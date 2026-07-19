# CasperRWA-Agent — Q&A Pack (combined community Q&A)

Short, plain-language answers. Mixed technical + non-technical audience.

**Q: Did an AI agent really build and run this?**
Yes. The agent — Kite — built the project and operates the loop autonomously. The narration
you heard is the agent itself; that's why we recorded it speaking.

**Q: What exactly is x402?**
It's an HTTP-native payment standard. When a service returns "402 Payment Required," the
agent signs a tiny on-chain payment itself and retries. It lets software pay per-request,
permissionlessly — no card, no human approving each spend.

**Q: Is this live on mainnet, or testnet?**
Everything shown is real on **Casper testnet** — every transaction hash is verifiable on
cspr.live. Mainnet is the next step: swap in the production Casper x402 facilitator. The hard
part (the autonomous pay-decide-settle loop) already works and is verifiable today.

**Q: Where does the rent signal come from? Is the oracle real?**
Today the rent-signal oracle is a controlled/simulated source behind a real x402 paywall —
so the *payment and settlement* are genuine, the *data feed* is the part we'd swap for a real
property oracle in production. We're honest about that boundary.

**Q: Who holds the keys? Isn't an autonomous agent with a wallet risky?**
The agent holds its own testnet key for its own vault. For production you'd add spend limits,
allowlisted destinations, and policy guards — the agent acts within constraints, not unbounded.
On testnet the funds are test CSPR, so the focus is proving the mechanism safely.

**Q: Why Casper specifically?**
Three reasons: it's a WebAssembly-native L1 with a real x402 facilitator (the pay-per-use loop
settles natively, not on a bolt-on); the Odra framework lets us write the vault in type-safe
Rust; and settlement is deterministic and low-cost — which matters when an agent transacts often.

**Q: How does the payout handle rounding / dust?**
The vault distributes pro-rata across shareholders and retains dust deterministically — it's
covered by the contract tests (deposit, pro-rata distribution, dust retention, revert paths).

**Q: Is the code original / open source?**
Yes — newly built for the buildathon, fully open source:
github.com/kite-builds/casper-rwa-agent. Demo + every tx hash: casperrwa-agent-demo.surge.sh.

**Q: What can this be used for beyond rent?**
Any recurring on-chain payout where a human currently operates the asset: dividend and bond
coupons, invoice factoring, subscription revenue. Same loop — observe, pay, decide, settle.

**Q: What's next?**
Production x402 facilitator for mainnet, CEP-18 share tokens instead of native transfers, a
real rent oracle, and generalizing rent → dividends → coupons.
