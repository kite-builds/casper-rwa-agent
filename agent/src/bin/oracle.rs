//! x402-gated rent oracle for the CasperRWA-Agent.
//!
//! `GET /rent-signal` is paywalled with HTTP 402 (x402 "exact" scheme). On the
//! first hit it returns `402 Payment Required` with a [`PaymentRequirements`]
//! object. The agent pays (via the facilitator) and retries with an `X-PAYMENT`
//! header; the oracle asks the facilitator to settle, then returns the
//! [`RentSignal`] plus an `X-PAYMENT-RESPONSE` receipt.

use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use casper_rwa_agent_loop::{
    from_b64_json, PaymentPayload, PaymentRequiredBody, PaymentRequirements, RentSignal,
    SettleRequest, SettleResponse, X_PAYMENT, X_PAYMENT_RESPONSE,
};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    network: String,
    price_motes: String,
    pay_to: String,
    facilitator: String,
    http: reqwest::Client,
}

async fn health() -> &'static str {
    "casper-rwa rent oracle (x402-gated) ok"
}

fn requirements(state: &AppState, nonce: &str) -> PaymentRequirements {
    PaymentRequirements {
        scheme: "exact".into(),
        network: state.network.clone(),
        max_amount_required: state.price_motes.clone(),
        resource: "/rent-signal".into(),
        description: "Rent-due + valuation signal for the tokenized RWA".into(),
        asset: "CSPR".into(),
        pay_to: state.pay_to.clone(),
        facilitator: state.facilitator.clone(),
        nonce: nonce.to_string(),
    }
}

/// The actual rent signal (would read a real data feed in production).
fn current_signal() -> RentSignal {
    // Deterministic demo policy: rent is due, 200 CSPR owed, asset valued at 480k.
    RentSignal {
        rent_due: true,
        valuation_cspr: 480_000,
        rent_amount_cspr: 200,
        as_of: chrono_now(),
    }
}

fn chrono_now() -> String {
    // Avoid pulling chrono: use a coarse UNIX timestamp ISO-ish stamp.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

async fn rent_signal(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    let nonce = Uuid::new_v4().to_string();

    let Some(pay_hdr) = headers.get(X_PAYMENT) else {
        // No payment yet -> 402 with requirements.
        let body = PaymentRequiredBody {
            x402_version: 1,
            error: "payment required".into(),
            accepts: vec![requirements(&state, &nonce)],
        };
        tracing::info!("402 issued, nonce {nonce}");
        return (StatusCode::PAYMENT_REQUIRED, Json(serde_json::json!(body))).into_response();
    };

    // Decode and settle the presented payment.
    let payload: PaymentPayload = match pay_hdr
        .to_str()
        .ok()
        .and_then(|s| from_b64_json::<PaymentPayload>(s).ok())
    {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"malformed X-PAYMENT"})),
            )
                .into_response()
        }
    };

    let settle_req = SettleRequest {
        requirements: requirements_for_payment(&state, &payload),
        payment: payload.clone(),
    };
    let settle: SettleResponse = match state
        .http
        .post(format!("{}/settle", state.facilitator))
        .json(&settle_req)
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(s) => s,
            Err(e) => return bad_gateway(&format!("facilitator response: {e}")),
        },
        Err(e) => return bad_gateway(&format!("facilitator unreachable: {e}")),
    };

    if !settle.success {
        tracing::warn!("settlement rejected: {:?}", settle.error);
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(serde_json::json!({"error": settle.error})),
        )
            .into_response();
    }

    tracing::info!(tx = %settle.tx, payer = %settle.payer, "payment settled; serving signal");
    let signal = current_signal();
    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        X_PAYMENT_RESPONSE,
        casper_rwa_agent_loop::b64_json(&settle).parse().unwrap(),
    );
    (StatusCode::OK, resp_headers, Json(serde_json::json!(signal))).into_response()
}

/// Rebuild the requirements the client must have paid against (echo its nonce).
fn requirements_for_payment(state: &AppState, p: &PaymentPayload) -> PaymentRequirements {
    let mut r = requirements(state, &p.nonce);
    r.max_amount_required = state.price_motes.clone();
    r
}

fn bad_gateway(msg: &str) -> axum::response::Response {
    (
        StatusCode::BAD_GATEWAY,
        Json(serde_json::json!({ "error": msg })),
    )
        .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();
    let port: u16 = std::env::var("ORACLE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8402);
    let state = Arc::new(AppState {
        network: std::env::var("X402_NETWORK").unwrap_or_else(|_| "casper-test".into()),
        // price of one rent signal: 2.5 CSPR = 2_500_000_000 motes (the chain
        // minimum for a native transfer, so the micro-payment settles on-chain).
        price_motes: std::env::var("ORACLE_PRICE_MOTES").unwrap_or_else(|_| "2500000000".into()),
        pay_to: std::env::var("ORACLE_PAY_TO").unwrap_or_else(|_| {
            // oracle operator public key (payment address for the transfer).
            "01345219a3c91e0e2cce865d0706bc0840b1549a05a0abe160a49726f2b596483d".into()
        }),
        facilitator: std::env::var("FACILITATOR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8403".into()),
        // Long timeout: /settle blocks while the facilitator broadcasts the
        // micro-payment and awaits on-chain execution.
        http: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .expect("http client"),
    });

    let app = Router::new()
        .route("/", get(health))
        .route("/rent-signal", get(rent_signal))
        .with_state(state);

    let addr = format!("127.0.0.1:{port}");
    tracing::info!("rent oracle listening on http://{addr} (x402-gated /rent-signal)");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
