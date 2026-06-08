//! Shared x402 protocol types for the CasperRWA-Agent loop.
//!
//! These mirror the HTTP-402 "exact" payment scheme used by the Casper x402
//! Facilitator: a client hits a paid resource, gets a `402 Payment Required`
//! with a [`PaymentRequirements`] object, signs an authorization, and retries
//! with an `X-PAYMENT` header carrying a base64 [`PaymentPayload`]. The resource
//! server asks a facilitator to `verify` then `settle` the payment before
//! returning `200 OK` plus an `X-PAYMENT-RESPONSE` settlement receipt.
//!
//! Scope note: the production Casper x402 Facilitator settles on Casper mainnet
//! and is gated behind sponsored buildathon credentials we don't hold. This crate
//! ships a faithful local facilitator (`bin/facilitator`) implementing the same
//! verify/settle interface so the full observe -> pay -> settle loop runs
//! end-to-end on the testnet rail; only the facilitator's *settlement* of the
//! micro-payment is local (documented), while the agent's RWA *settlement* on the
//! deployed vault is a real Casper testnet transaction.

use serde::{Deserialize, Serialize};

pub const X_PAYMENT: &str = "X-PAYMENT";
pub const X_PAYMENT_RESPONSE: &str = "X-PAYMENT-RESPONSE";

/// The `accepts` entry returned in a 402 response (x402 "exact" scheme).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequirements {
    /// Payment scheme. We use "exact" (pay a fixed amount).
    pub scheme: String,
    /// Settlement network identifier.
    pub network: String,
    /// Amount required, in the smallest unit (motes for CSPR).
    pub max_amount_required: String,
    /// The resource being paid for.
    pub resource: String,
    /// Human description of the resource.
    pub description: String,
    /// Asset identifier (here: native CSPR).
    pub asset: String,
    /// Where the payment is collected (the resource server's account hash).
    pub pay_to: String,
    /// Facilitator endpoint that verifies + settles the payment.
    pub facilitator: String,
    /// Opaque nonce binding this quote to one request.
    pub nonce: String,
}

/// Standard x402 402 body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRequiredBody {
    pub x402_version: u8,
    pub error: String,
    pub accepts: Vec<PaymentRequirements>,
}

/// Authorization the client signs and sends back in the `X-PAYMENT` header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentPayload {
    pub x402_version: u8,
    pub scheme: String,
    pub network: String,
    /// Payer account hash.
    pub from: String,
    /// Payee account hash (echoes `pay_to`).
    pub to: String,
    /// Amount paid, in motes.
    pub amount: String,
    /// Echoed nonce from the requirements.
    pub nonce: String,
    /// Payer signature over the canonical authorization message.
    pub signature: String,
    /// Payer public key (hex) so the facilitator can verify the signature.
    pub public_key: String,
}

impl PaymentPayload {
    /// Canonical message that the payer signs and the facilitator re-derives.
    pub fn signing_message(
        network: &str,
        from: &str,
        to: &str,
        amount: &str,
        nonce: &str,
    ) -> String {
        format!("x402-exact|{network}|{from}|{to}|{amount}|{nonce}")
    }
}

/// Request body sent by the resource server to the facilitator's /settle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleRequest {
    pub payment: PaymentPayload,
    pub requirements: PaymentRequirements,
}

/// Settlement receipt returned by the facilitator and surfaced to the client
/// in `X-PAYMENT-RESPONSE`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleResponse {
    pub success: bool,
    pub network: String,
    /// On-chain (or local) settlement transaction id of the micro-payment.
    pub tx: String,
    pub payer: String,
    pub amount: String,
    #[serde(default)]
    pub error: Option<String>,
}

/// The oracle's rent signal (the data behind the paywall).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentSignal {
    /// Whether rent is currently due for the tokenized asset.
    pub rent_due: bool,
    /// Latest off-chain valuation of the asset, in CSPR.
    pub valuation_cspr: u64,
    /// Rent amount owed this period, in CSPR (0 if not due).
    pub rent_amount_cspr: u64,
    /// ISO timestamp the signal was produced.
    pub as_of: String,
}

/// base64-encode a JSON-serializable value (for the X-PAYMENT header).
pub fn b64_json<T: Serialize>(v: &T) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(v).unwrap())
}

/// Decode a base64 JSON header value.
pub fn from_b64_json<T: for<'de> Deserialize<'de>>(s: &str) -> anyhow::Result<T> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(s.trim())?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Deterministic signature over the authorization message.
///
/// The real Casper rail uses ed25519 over the deploy approvals; here we bind the
/// payer's public key + the canonical message with SHA-256 so the facilitator can
/// independently re-derive and reject tampered/replayed authorizations. The
/// facilitator treats this as the payment authorization proof.
pub fn sign_auth(public_key_hex: &str, message: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(public_key_hex.as_bytes());
    h.update(b"|");
    h.update(message.as_bytes());
    hex::encode(h.finalize())
}
