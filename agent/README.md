# CasperRWA-Agent — autonomous x402 settlement loop

The "robot landlord": an autonomous agent that pays-per-query an x402-gated rent
oracle, and when rent is due, fires a **real on-chain settlement** on the deployed
`RwaVault` contract (Casper testnet), paying every shareholder pro-rata. No human
in the loop.

## Components (all new code)

| Binary | Role |
|--------|------|
| `oracle` | x402-gated rent oracle. `GET /rent-signal` returns `402 Payment Required` + price; after a valid `X-PAYMENT`, settles via the facilitator and returns `{rent_due, valuation, rent_amount}`. |
| `facilitator` | Local x402 facilitator implementing the production `verify` + `settle` interface (the one piece stubbed for funds — see below). |
| `agent` | The autonomous loop: observe → pay → decide → settle. |

## The loop

```
observe : GET /rent-signal            -> 402 + PaymentRequirements (exact scheme)
pay     : sign x402 authorization,    -> retry with X-PAYMENT header
          oracle -> facilitator /settle  -> 200 OK + X-PAYMENT-RESPONSE receipt + signal
decide  : read rent_due / valuation from the paid signal
settle  : if rent due, fire REAL on-chain `distribute` on RwaVault (testnet)
```

## Run it

```bash
source ~/.casper-build-env.sh && source ~/.cargo/env
cd agent && cargo build

# terminal 1 + 2 (or background):
./target/debug/facilitator      # :8403
./target/debug/oracle           # :8402  (x402-gated /rent-signal)

# dry run (observe + pay + decide, no on-chain tx):
./target/debug/agent --dry-run

# full autonomous cycle incl. real on-chain settlement:
./target/debug/agent --contract-dir ..
```

Or use the one-shot driver: `./run_loop.sh` (starts both services, runs the agent,
tears down).

## What's real vs stubbed

- **Real:** the entire x402 HTTP exchange (402 → sign → X-PAYMENT → verify/settle →
  X-PAYMENT-RESPONSE), nonce binding + signature verification, and the agent's
  **on-chain RWA settlement** — `deposit_rent` + `distribute` are real Casper testnet
  transactions on the deployed vault (hashes printed each run; see ../DEPLOYMENT.md).
- **Stubbed (documented):** the facilitator's *settlement of the micro-payment*. The
  production Casper x402 Facilitator settles micro-payments on Casper **mainnet** behind
  sponsored buildathon credentials we don't hold; the local facilitator records the
  settlement and returns a faithful receipt instead. Swapping `FACILITATOR_URL` to the
  Casper facilitator endpoint is the only change needed to go fully on-rail.

## Config (env vars)

`ORACLE_PORT`, `FACILITATOR_PORT`, `FACILITATOR_URL`, `ORACLE_PRICE_MOTES`,
`ORACLE_PAY_TO`, `X402_NETWORK`. Agent flags: `--oracle`, `--payer`, `--public-key`,
`--holder1`, `--holder2`, `--contract-dir`, `--dry-run`.
