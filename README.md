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
| x402 micro-payment for the oracle query (2.5 CSPR) | [`4e2fa6e6…f84ae03`](https://testnet.cspr.live/transaction/4e2fa6e652d23ba2e93bb01ed8e8b97e8ce9d869638e8dac375b29485f84ae03) |
| `deposit_rent` (200 CSPR) | [`ccebb404…7b9cfb`](https://testnet.cspr.live/transaction/ccebb4049f2799df5eff0f93c92405fa5368899fb74d23bd9038e92b7e7b9cfb) |
| `distribute` (120 + 80 CSPR pro-rata) | [`5b74241e…1048d3a`](https://testnet.cspr.live/transaction/5b74241ec0920107bb62675a052c2a9d526f4a82f8fc3ecc4735b6e221048d3a) |

(These are the txs from the cycle shown in the [demo video](https://casperrwa-agent-demo.surge.sh/); a prior identical cycle is recorded in [DEPLOYMENT.md](./DEPLOYMENT.md).)

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

## Why this matters (real-world applicability)

Tokenized real-world assets (RWAs) are one of crypto's fastest-growing categories, but
the **operational layer** — collecting yield and distributing it to fractional owners —
is still manual and trust-heavy. CasperRWA-Agent demonstrates that an autonomous agent
can run that operational layer end-to-end: it pays for the data it consumes with x402
micropayments and settles the resulting pro-rata distribution on-chain, with no human in
the loop. Rent is the first instance; the same pattern covers dividends, bond coupons,
and royalty splits.

**Why Casper specifically:** it is a WebAssembly-native L1 with a **live x402 facilitator**,
so the agent's pay-per-use loop settles natively rather than on a bolted-on payments rail —
the agent and its data purchases live on the same chain as the settlement.

## Roadmap

**Shipped for the Buildathon (2026-06):**
- [x] `RwaVault` contract + 8/8 unit tests + WASM build
- [x] Deployed `RwaVault` to **Casper testnet** with real `distribute()` settlement txs
- [x] x402 rent-oracle service (HTTP 402, exact scheme)
- [x] Autonomous agent loop (observe → pay → decide → settle), end-to-end
- [x] x402 micro-payment settles **on-chain** (payer-signed transfer, facilitator-broadcast)
- [x] Demo video + landing page + DoraHacks submission ([BUIDL #44481](https://dorahacks.io/buidl/44481))

**Path to a real deployment (post-Buildathon):**
- [ ] Swap `FACILITATOR_URL` to the production Casper **mainnet** x402 facilitator
      (sponsored credentials; no code change — the interface is identical).
- [ ] Issue fractional shares as a **CEP-18** token so ownership is transferable, and add a
      multi-asset vault registry (one agent operating many properties).
- [ ] Replace the demo rent signal with a **real oracle feed** (off-chain rent/valuation data
      behind the x402 paywall).
- [ ] Generalize beyond rent to dividends / coupon / royalty distributions — an "agentic
      operations layer" for RWAs on Casper.

**Vision:** a general autonomous operations layer for tokenized real-world assets on Casper,
where every recurring payout event is settled per-event by an agent that pays its own way.

## License

MIT — see [LICENSE](./LICENSE).
