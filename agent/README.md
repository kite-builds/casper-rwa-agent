# CasperRWA-Agent — autonomous x402 settlement loop

The "robot landlord": an autonomous agent that pays-per-query an x402-gated rent
oracle, and when rent is due, fires a **real on-chain settlement** on the deployed
`RwaVault` contract (Casper testnet), paying every shareholder pro-rata. No human
in the loop.

## Components (all new code)

| Binary | Role |
|--------|------|
| `oracle` | x402-gated rent oracle. `GET /rent-signal` returns `402 Payment Required` + price; after a valid `X-PAYMENT`, settles via the facilitator and returns `{rent_due, valuation, rent_amount}`. |
| `facilitator` | x402 facilitator implementing the production `verify` + `settle` interface. **Settles the micro-payment ON-CHAIN**: validates the payer's pre-signed Casper transfer against the payment requirements, broadcasts it via `account_put_transaction`, awaits execution, and returns the real tx hash as the receipt. |
| `agent` | The autonomous loop: observe → pay → decide → settle. |

## The loop

```
observe : GET /rent-signal            -> 402 + PaymentRequirements (exact scheme)
pay     : sign x402 authorization + pre-sign the native CSPR transfer,
          retry with X-PAYMENT header;
          oracle -> facilitator /settle -> facilitator broadcasts the transfer
          ON-CHAIN (casper-test), awaits execution
          -> 200 OK + X-PAYMENT-RESPONSE receipt (real tx hash) + signal
decide  : read rent_due / valuation from the paid signal
settle  : if rent due, fire REAL on-chain `distribute` on RwaVault (testnet)
```

x402 "exact" on Casper works like EIP-3009 `transferWithAuthorization` on EVM
rails: the payer signs the transfer transaction itself, and the facilitator —
an untrusted convenience service anyone can run, per the x402 spec — validates
it against the requirements (chain / payer / payee / amount) and broadcasts it.
The facilitator can never move funds the payer didn't authorize.

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

## What's real

**Everything is on-chain.** The entire x402 HTTP exchange (402 → sign →
X-PAYMENT → verify/settle → X-PAYMENT-RESPONSE), nonce binding + signature
verification, the **micro-payment settlement** (a real native CSPR transfer on
casper-test, broadcast by the facilitator, real tx hash in the receipt), and
the agent's **on-chain RWA settlement** — `deposit_rent` + `distribute` are
real Casper testnet transactions on the deployed vault (hashes printed each
run; see ../DEPLOYMENT.md).

Proven loop run (2026-06-10), all three txs on casper-test:

| step | tx |
|------|----|
| x402 micro-payment (2.5 CSPR, agent → oracle) | [`6ae8c1b1…39ae380`](https://testnet.cspr.live/transaction/6ae8c1b1079a593f1f077324cb1b251f4c02c3df446decedaa5770edd39ae380) |
| `deposit_rent` (200 CSPR) | [`4dde8fc0…4b1968e`](https://testnet.cspr.live/transaction/4dde8fc0793e20bc37f8b284c416d3785a744bed63882580e9e88ca354b1968e) |
| `distribute` (120 + 80 CSPR pro-rata) | [`5945080a…dae076f`](https://testnet.cspr.live/transaction/5945080aec1c5a3f319f3856a60183262b1c82bc2bed00c9a0af1f19ddae076f) |

The production Casper x402 Facilitator (mainnet, sponsored credentials) exposes
the same `verify`/`settle` interface — swapping `FACILITATOR_URL` moves the
micro-payment rail to mainnet with no code changes. `--dry-run` skips both
on-chain legs and settles with a local receipt for offline testing.

## Config (env vars)

`ORACLE_PORT`, `FACILITATOR_PORT`, `FACILITATOR_URL`, `ORACLE_PRICE_MOTES`
(default 2.5 CSPR — the chain minimum for a native transfer), `ORACLE_PAY_TO`
(payee public key), `X402_NETWORK`, `NODE_RPC` (facilitator's Casper node),
`CASPER_CLIENT` (path to `casper-client`). Agent flags: `--oracle`, `--payer`,
`--public-key`, `--secret-key`, `--holder1`, `--holder2`, `--contract-dir`,
`--dry-run`.
