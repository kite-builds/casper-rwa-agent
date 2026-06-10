//! x402 facilitator for the CasperRWA-Agent — settles micro-payments ON-CHAIN.
//!
//! Implements the same `verify` + `settle` interface the production Casper x402
//! Facilitator exposes. Per the x402 spec the facilitator is an untrusted
//! convenience service anyone can run: the payer pre-signs the payment
//! transaction and the facilitator validates + broadcasts it (it can never
//! move funds the payer didn't authorize — the Casper analogue of EIP-3009
//! `transferWithAuthorization`).
//!
//! `/settle` flow when the payload carries `signed_tx`:
//!   1. verify the x402 authorization signature,
//!   2. validate the pre-signed Casper TransactionV1 against the payment
//!      requirements (chain, payer, payee, amount),
//!   3. broadcast it via `account_put_transaction` on the configured node,
//!   4. await on-chain execution and return the REAL transaction hash as the
//!      settlement receipt.
//! Without `signed_tx` it falls back to a local (off-chain) receipt so dry
//! runs work offline.
//!
//! Env: `NODE_RPC` (default `https://node.testnet.casper.network/rpc`),
//! `FACILITATOR_PORT` (default 8403).

use axum::{
    extract::Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use casper_rwa_agent_loop::{PaymentPayload, SettleRequest, SettleResponse};
use uuid::Uuid;

fn node_rpc() -> String {
    std::env::var("NODE_RPC").unwrap_or_else(|_| "https://node.testnet.casper.network/rpc".into())
}

async fn health() -> &'static str {
    "casper-x402-facilitator ok (on-chain settlement: signed_tx)"
}

fn verify_signature(p: &PaymentPayload) -> Result<(), String> {
    let msg = PaymentPayload::signing_message(&p.network, &p.from, &p.to, &p.amount, &p.nonce);
    let expected = casper_rwa_agent_loop::sign_auth(&p.public_key, &msg);
    if expected != p.signature {
        return Err("authorization signature mismatch".into());
    }
    Ok(())
}

/// Validate the pre-signed transfer against what the payer authorized, then
/// broadcast and await execution. Returns the on-chain transaction hash.
async fn settle_onchain(p: &PaymentPayload, b64tx: &str) -> Result<(String, u64), String> {
    let tx: serde_json::Value = casper_rwa_agent_loop::from_b64_json(b64tx)
        .map_err(|e| format!("signed_tx decode failed: {e}"))?;
    let payload = &tx["Version1"]["payload"];

    // Bind the broadcast to exactly what the x402 authorization covers.
    if payload["chain_name"].as_str() != Some(p.network.as_str()) {
        return Err("signed_tx chain does not match payment network".into());
    }
    if payload["initiator_addr"]["PublicKey"].as_str() != Some(p.public_key.as_str()) {
        return Err("signed_tx initiator does not match payer".into());
    }
    let args = payload["fields"]["args"]["Named"]
        .as_array()
        .ok_or("signed_tx missing transfer args")?;
    let arg = |name: &str| {
        args.iter()
            .find(|a| a[0].as_str() == Some(name))
            .and_then(|a| a[1]["parsed"].as_str().map(String::from))
    };
    if arg("target").as_deref() != Some(p.to.as_str()) {
        return Err("signed_tx target does not match payee".into());
    }
    if arg("amount").as_deref() != Some(p.amount.as_str()) {
        return Err("signed_tx amount does not match authorized amount".into());
    }

    // Broadcast.
    let node = node_rpc();
    let http = reqwest::Client::new();
    let put = serde_json::json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "account_put_transaction",
        "params": { "transaction": tx }
    });
    let resp: serde_json::Value = http
        .post(&node)
        .json(&put)
        .send()
        .await
        .map_err(|e| format!("node unreachable: {e}"))?
        .json()
        .await
        .map_err(|e| format!("node response: {e}"))?;
    let hash = resp["result"]["transaction_hash"]["Version1"]
        .as_str()
        .ok_or_else(|| format!("node rejected transaction: {}", resp["error"]))?
        .to_string();
    tracing::info!(tx = %hash, "broadcast accepted; awaiting on-chain execution");

    // Await execution (casper-test block time is a few seconds).
    for _ in 0..24 {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let q = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "info_get_transaction",
            "params": { "transaction_hash": { "Version1": hash }, "finalized_approvals": true }
        });
        let r: serde_json::Value = match http.post(&node).json(&q).send().await {
            Ok(resp) => resp.json().await.map_err(|e| format!("poll response: {e}"))?,
            Err(_) => continue,
        };
        let info = &r["result"]["execution_info"];
        if info.is_object() {
            let exec = &info["execution_result"]["Version2"];
            if let Some(err) = exec["error_message"].as_str() {
                return Err(format!("on-chain execution failed: {err}"));
            }
            let block = info["block_height"].as_u64().unwrap_or(0);
            return Ok((hash, block));
        }
    }
    Err(format!("timed out awaiting execution of {hash}"))
}

/// POST /verify — checks the authorization is well-formed and correctly signed.
async fn verify(Json(req): Json<SettleRequest>) -> (StatusCode, Json<SettleResponse>) {
    let p = &req.payment;
    if p.scheme != req.requirements.scheme || p.network != req.requirements.network {
        return fail(p, "scheme/network mismatch");
    }
    if p.amount != req.requirements.max_amount_required {
        return fail(p, "amount does not match requirements");
    }
    if p.nonce != req.requirements.nonce {
        return fail(p, "nonce mismatch (possible replay)");
    }
    if let Err(e) = verify_signature(p) {
        return fail(p, &e);
    }
    tracing::info!(payer = %p.from, amount = %p.amount, "verify OK");
    (
        StatusCode::OK,
        Json(SettleResponse {
            success: true,
            network: p.network.clone(),
            tx: String::new(),
            payer: p.from.clone(),
            amount: p.amount.clone(),
            error: None,
        }),
    )
}

/// POST /settle — verifies then settles the micro-payment, returning a receipt.
async fn settle(Json(req): Json<SettleRequest>) -> (StatusCode, Json<SettleResponse>) {
    let p = &req.payment;
    if let Err(e) = verify_signature(p) {
        return fail(p, &e);
    }
    let tx = match &p.signed_tx {
        Some(b64tx) => match settle_onchain(p, b64tx).await {
            Ok((hash, block)) => {
                tracing::info!(payer = %p.from, payee = %p.to, amount = %p.amount,
                    tx = %hash, block, "settle OK (ON-CHAIN, casper testnet)");
                hash
            }
            Err(e) => return fail(p, &e),
        },
        None => {
            // Offline fallback: local receipt (dry runs without a signed tx).
            let tx = format!("local-x402-{}", Uuid::new_v4());
            tracing::info!(payer = %p.from, payee = %p.to, amount = %p.amount, tx = %tx,
                "settle OK (local fallback, no signed_tx)");
            tx
        }
    };
    (
        StatusCode::OK,
        Json(SettleResponse {
            success: true,
            network: p.network.clone(),
            tx,
            payer: p.from.clone(),
            amount: p.amount.clone(),
            error: None,
        }),
    )
}

fn fail(p: &PaymentPayload, msg: &str) -> (StatusCode, Json<SettleResponse>) {
    tracing::warn!(payer = %p.from, "settle/verify FAILED: {msg}");
    (
        StatusCode::PAYMENT_REQUIRED,
        Json(SettleResponse {
            success: false,
            network: p.network.clone(),
            tx: String::new(),
            payer: p.from.clone(),
            amount: p.amount.clone(),
            error: Some(msg.to_string()),
        }),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();
    let port: u16 = std::env::var("FACILITATOR_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8403);
    let app = Router::new()
        .route("/", get(health))
        .route("/verify", post(verify))
        .route("/settle", post(settle));
    let addr = format!("127.0.0.1:{port}");
    tracing::info!("x402 facilitator listening on http://{addr} (node: {})", node_rpc());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
