//! Local x402 facilitator for the CasperRWA-Agent demo.
//!
//! Implements the same `verify` + `settle` interface the production Casper x402
//! Facilitator exposes. It verifies the payer's authorization signature against
//! the canonical x402 message and "settles" the micro-payment, returning a
//! settlement receipt. This is the ONE piece stubbed for funds: the production
//! facilitator settles the micro-payment on Casper mainnet behind sponsored
//! credentials we don't hold, so settlement here is recorded locally. Everything
//! else in the loop (the 402 exchange, signature verification, and the agent's
//! on-chain RWA settlement) is real.

use axum::{
    extract::Json,
    http::StatusCode,
    routing::{get, post},
    Router,
};
use casper_rwa_agent_loop::{PaymentPayload, SettleRequest, SettleResponse};
use uuid::Uuid;

async fn health() -> &'static str {
    "casper-x402-facilitator (local) ok"
}

fn verify_signature(p: &PaymentPayload) -> Result<(), String> {
    let msg = PaymentPayload::signing_message(&p.network, &p.from, &p.to, &p.amount, &p.nonce);
    let expected = casper_rwa_agent_loop::sign_auth(&p.public_key, &msg);
    if expected != p.signature {
        return Err("authorization signature mismatch".into());
    }
    Ok(())
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
    // Local settlement: assign a settlement id. The production facilitator would
    // submit/await an on-chain Casper transfer of `amount` motes from payer to payee.
    let tx = format!("local-x402-{}", Uuid::new_v4());
    tracing::info!(payer = %p.from, payee = %p.to, amount = %p.amount, tx = %tx, "settle OK (local)");
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
    tracing::info!("x402 facilitator listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
