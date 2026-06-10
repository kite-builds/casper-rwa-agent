# CasperRWA-Agent

**An autonomous agent that settles real-world-asset (RWA) rent on Casper, paying per-signal via x402.**

Built for the [Casper Agentic Buildathon 2026](https://dorahacks.io/hackathon/casper-agentic-buildathon/detail) — Innovation Track (Agentic AI × DeFi × RWA).

---

## The idea in one paragraph

Imagine a rental property split into digital shares owned by many people. Normally a
human landlord has to check when rent is due, collect it, and split it fairly between
every owner. **CasperRWA-Agent replaces that human with an autonomous AI agent.** The
agent (1) pays a tiny fee — using the [x402](https://www.x402.org) agent-payment
standard — every time it asks an oracle *"is rent due, and what is the asset worth?"*,
and (2) when rent is due, fires a single on-chain transaction on Casper that collects
the rent and distributes it pro-rata to every shareholder automatically. No human in
the loop.

## Why Casper

- **WebAssembly-native L1** with a live **x402 facilitator** — the agent's pay-per-use
  loop settles natively on-chain, not on a bolted-on sidechain.
- **[Odra](https://odra.dev) smart-contract framework** (Rust) — the settlement
  contract is type-safe Rust compiled to Casper WASM.

## Architecture

```
                 ┌──────────────────────────┐
                 │   Autonomous Agent loop   │
                 │  (off-chain, headless)    │
                 └───────────┬──────────────┘
            x402 pay-per-call │            │ on-chain tx
       ┌───────────────▼─────┐      ┌─────▼────────────────────┐
       │  x402 Rent Oracle    │      │  RwaVault (Odra → Casper) │
       │  GET /rent-signal    │      │  deposit_rent + distribute│
       │  (HTTP 402 paywall)  │      │  pro-rata to shareholders │
       └──────────────────────┘      └───────────────────────────┘
```

Three components (all original code for this Buildathon):

| # | Component | Stack | Status |
|---|-----------|-------|--------|
| 1 | **`RwaVault`** on-chain settlement contract | Rust / Odra → Casper WASM | ✅ built + 8/8 tests + **deployed to testnet** ([DEPLOYMENT.md](./DEPLOYMENT.md)) |
| 2 | **x402 Rent Oracle** — `GET /rent-signal` behind an HTTP-402 paywall | Rust / axum (`agent/`) | ✅ runs; HTTP-402 exact scheme |
| 3 | **Autonomous agent** — pay → query → settle loop | Rust (`agent/`) | ✅ end-to-end, fires real on-chain settlement |

**Live on Casper testnet — every leg of the loop is on-chain:** contract package
`82e45926c39c0d42166a8bce66770b2cbcab2448dc61a8b3622acca09f2ea059`. One autonomous
cycle (2026-06-10) produced three real casper-test transactions:

| step | tx |
|------|----|
| x402 micro-payment for the oracle query (2.5 CSPR) | [`6ae8c1b1…39ae380`](https://testnet.cspr.live/transaction/6ae8c1b1079a593f1f077324cb1b251f4c02c3df446decedaa5770edd39ae380) |
| `deposit_rent` (200 CSPR) | [`4dde8fc0…4b1968e`](https://testnet.cspr.live/transaction/4dde8fc0793e20bc37f8b284c416d3785a744bed63882580e9e88ca354b1968e) |
| `distribute` (120 + 80 CSPR pro-rata) | [`5945080a…dae076f`](https://testnet.cspr.live/transaction/5945080aec1c5a3f319f3856a60183262b1c82bc2bed00c9a0af1f19ddae076f) |

The x402 micro-payment uses the spec's payer-signs / facilitator-broadcasts model
(the Casper analogue of EIP-3009): the agent pre-signs a native transfer, and the
bundled facilitator validates + broadcasts it and returns the real tx hash as the
`X-PAYMENT-RESPONSE` receipt. Full tx hashes + explorer links in
[DEPLOYMENT.md](./DEPLOYMENT.md). Run the loop: `cd agent && ./run_loop.sh`.

## The `RwaVault` contract

`src/rwa_vault.rs`. The on-chain settlement layer.

- `init(asset)` — deploy the vault for one tokenized asset; deployer becomes owner.
- `register_shareholder(holder, share_units)` — owner registers fractional owners.
- `deposit_rent()` *(payable)* — anyone (the agent) deposits native CSPR rent into a pool.
- `distribute()` — permissionless; splits the entire pool **pro-rata** to all shareholders
  in one transaction (integer-division dust retained for the next round). **This is the
  transaction-producing on-chain settlement.**
- Views: `asset`, `get_owner`, `shares_of`, `total_shares`, `shareholder_count`,
  `rent_pool`, `total_distributed`.
- Events: `ShareholderRegistered`, `RentDeposited`, `Distributed`.

## Build & test

Requires Rust + the Casper/Odra toolchain (`cargo-odra`, `wasm32-unknown-unknown`,
pinned nightly in `rust-toolchain`).

```bash
cargo odra test       # run the unit-test suite on the Odra VM
cargo odra build      # produce wasm/RwaVault.wasm (the deployable artifact)
```

Current test status: **8/8 passing** (init, registration, access control, payable
deposit, pro-rata distribution, dust retention, revert paths).

## Roadmap to submission (2026-06-30)

- [x] `RwaVault` contract + unit tests + WASM build
- [x] Deploy `RwaVault` to **Casper testnet** (real `distribute()` tx hash) ← eligibility floor
- [x] x402 rent-oracle service (HTTP 402)
- [x] Autonomous agent loop (pay → query → settle), end-to-end
- [x] x402 micro-payment settles **on-chain** (payer-signed transfer, facilitator-broadcast)
- [x] Demo video + DoraHacks submission ([BUIDL #44481](https://dorahacks.io/buidl/44481))
- [ ] Optional: swap `FACILITATOR_URL` to the production mainnet facilitator (sponsored credentials)

## License

MIT — see [LICENSE](./LICENSE).
