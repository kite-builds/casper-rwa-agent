//! CasperRWA-Agent — the autonomous rent-settlement loop.
//!
//! No human in the loop. Each cycle the agent:
//!   1. OBSERVE  — GET the x402-gated rent oracle; receive a 402 + price quote.
//!   2. PAY      — sign an x402 authorization and retry with `X-PAYMENT`; the
//!                 oracle settles via the facilitator and returns the signal.
//!   3. DECIDE   — read `rent_due` / valuation from the paid signal.
//!   4. SETTLE   — if rent is due, fire a REAL on-chain `distribute` settlement
//!                 on the deployed RwaVault (Casper testnet) via the proven
//!                 odra-cli path, paying out pro-rata to all shareholders.
//!
//! The on-chain settlement (step 4) is a real Casper testnet transaction. The
//! x402 micro-payment (steps 1-2) uses the real protocol with a local facilitator
//! (see facilitator.rs for what is/ isn't on-chain).

use std::process::Command;

use casper_rwa_agent_loop::{
    b64_json, from_b64_json, PaymentPayload, PaymentRequiredBody, RentSignal, SettleResponse,
    X_PAYMENT, X_PAYMENT_RESPONSE,
};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Autonomous x402 rent-settlement agent for RwaVault on Casper")]
struct Args {
    /// Oracle base URL.
    #[arg(long, default_value = "http://127.0.0.1:8402")]
    oracle: String,
    /// Agent payer account hash (the on-chain caller / x402 payer).
    #[arg(
        long,
        default_value = "account-hash-8a78d764e32047715f610ac3687e7c971b607a58e1f982024cbe61ffd16bd399"
    )]
    payer: String,
    /// Agent payer public key hex (for x402 signature verification).
    #[arg(
        long,
        default_value = "01719330c30e154207e1fbcab076854787355a1fe51968ff33e517ef811396e235"
    )]
    public_key: String,
    /// Two shareholder account hashes for the settlement (registered if needed).
    #[arg(
        long,
        default_value = "account-hash-8a78d764e32047715f610ac3687e7c971b607a58e1f982024cbe61ffd16bd399"
    )]
    holder1: String,
    #[arg(
        long,
        default_value = "account-hash-b383c7cc23d18bc1b42406a1b2d29fc8dba86425197b6f553d7fd61375b5e446"
    )]
    holder2: String,
    /// Path to the contract crate root (where the odra-cli runs).
    #[arg(long, default_value = "..")]
    contract_dir: String,
    /// Skip the real on-chain settlement (dry run; observe+pay only).
    #[arg(long)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_target(false).init();
    let args = Args::parse();
    let http = reqwest::blocking::Client::new();
    let url = format!("{}/rent-signal", args.oracle);

    // --- 1. OBSERVE ---
    tracing::info!("[observe] GET {url}");
    let r = http.get(&url).send()?;
    if r.status() != reqwest::StatusCode::PAYMENT_REQUIRED {
        anyhow::bail!("expected 402 from oracle, got {}", r.status());
    }
    let req: PaymentRequiredBody = r.json()?;
    let pr = req
        .accepts
        .first()
        .ok_or_else(|| anyhow::anyhow!("no payment requirements in 402"))?;
    tracing::info!(
        "[observe] 402: pay {} motes ({} {}) to {} via {}",
        pr.max_amount_required,
        pr.scheme,
        pr.network,
        pr.pay_to,
        pr.facilitator
    );

    // --- 2. PAY (sign x402 authorization) ---
    let msg = PaymentPayload::signing_message(
        &pr.network,
        &args.payer,
        &pr.pay_to,
        &pr.max_amount_required,
        &pr.nonce,
    );
    let signature = casper_rwa_agent_loop::sign_auth(&args.public_key, &msg);
    let payload = PaymentPayload {
        x402_version: 1,
        scheme: pr.scheme.clone(),
        network: pr.network.clone(),
        from: args.payer.clone(),
        to: pr.pay_to.clone(),
        amount: pr.max_amount_required.clone(),
        nonce: pr.nonce.clone(),
        signature,
        public_key: args.public_key.clone(),
    };
    tracing::info!("[pay] signed authorization, retrying with {X_PAYMENT}");
    let paid = http
        .get(&url)
        .header(X_PAYMENT, b64_json(&payload))
        .send()?;
    if !paid.status().is_success() {
        anyhow::bail!("payment retry failed: {} {}", paid.status(), paid.text()?);
    }
    if let Some(rcpt) = paid.headers().get(X_PAYMENT_RESPONSE) {
        if let Ok(s) = rcpt.to_str() {
            if let Ok(receipt) = from_b64_json::<SettleResponse>(s) {
                tracing::info!(
                    "[pay] settled: tx {} amount {} motes on {}",
                    receipt.tx,
                    receipt.amount,
                    receipt.network
                );
            }
        }
    }
    let signal: RentSignal = paid.json()?;
    tracing::info!(
        "[decide] signal: rent_due={} valuation={} CSPR rent_amount={} CSPR (as_of {})",
        signal.rent_due,
        signal.valuation_cspr,
        signal.rent_amount_cspr,
        signal.as_of
    );

    // --- 3. DECIDE ---
    if !signal.rent_due || signal.rent_amount_cspr == 0 {
        tracing::info!("[decide] rent not due -> no settlement this cycle");
        return Ok(());
    }
    if args.dry_run {
        tracing::info!(
            "[settle] DRY RUN: would distribute {} CSPR rent on-chain",
            signal.rent_amount_cspr
        );
        return Ok(());
    }

    // --- 4. SETTLE (real Casper testnet transaction) ---
    tracing::info!(
        "[settle] rent due -> firing on-chain distribute of {} CSPR via odra-cli",
        signal.rent_amount_cspr
    );
    let status = Command::new("cargo")
        .current_dir(&args.contract_dir)
        .args([
            "run",
            "--quiet",
            "--bin",
            "casper_rwa_agent_cli",
            "--",
            "scenario",
            "settle",
            "--holder1",
            &args.holder1,
            "--holder2",
            &args.holder2,
            "--rent",
            &signal.rent_amount_cspr.to_string(),
        ])
        .status()?;
    if !status.success() {
        anyhow::bail!("on-chain settlement failed (exit {:?})", status.code());
    }
    tracing::info!("[settle] on-chain distribute submitted; see explorer links above");
    tracing::info!("[done] autonomous cycle complete: observe -> pay -> decide -> settle");
    Ok(())
}
