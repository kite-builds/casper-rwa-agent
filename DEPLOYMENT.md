# CasperRWA-Agent — Testnet Deployment

Live, verifiable deployment of the `RwaVault` contract on **Casper testnet**
(`casper-test`), with a full on-chain rent-settlement flow. All hashes below are
real Casper 2.0 transactions (TransactionV1) and resolve on the public explorer.

## Network
- Chain: `casper-test`
- RPC node: `https://node.testnet.casper.network/rpc` (api 2.0.0, protocol 2.2.1)

## Deployer account
- Public key: `01719330c30e154207e1fbcab076854787355a1fe51968ff33e517ef811396e235`
- Account hash: `account-hash-8a78d764e32047715f610ac3687e7c971b607a58e1f982024cbe61ffd16bd399`
- Funded with 5000 CSPR from the cspr.live testnet faucet
  (faucet tx `19e93d62788066d2ff5eaebc04ab3159cc96997485d61dd3e88f13036f575a8c`).

## Deployed contract
- Contract package hash:
  `contract-package-82e45926c39c0d42166a8bce66770b2cbcab2448dc61a8b3622acca09f2ea059`
- Explorer (contract package):
  https://testnet.cspr.live/contract-package/82e45926c39c0d42166a8bce66770b2cbcab2448dc61a8b3622acca09f2ea059

## On-chain transactions (the required transaction-producing component)

| Step | Entry point | Transaction hash | Explorer |
|------|-------------|------------------|----------|
| Install | `init` (asset = "Oslo duplex, Storgata 1") | `b86d5c161033ead1ea6f40d9733558e4b130f6d3fb01152e05e2d196088c096a` | [link](https://testnet.cspr.live/transaction/b86d5c161033ead1ea6f40d9733558e4b130f6d3fb01152e05e2d196088c096a) |
| Register shareholder #1 (60 units) | `register_shareholder` | `36e5b97e89464f8ba99bec93845d36866ed4065755411e0049fac29fd9ab86f0` | [link](https://testnet.cspr.live/transaction/36e5b97e89464f8ba99bec93845d36866ed4065755411e0049fac29fd9ab86f0) |
| Register shareholder #2 (40 units) | `register_shareholder` | `f88ff0fa6b0ddf34efa237b21d51895cc0922e1bf950c23ac46885d147fd8536` | [link](https://testnet.cspr.live/transaction/f88ff0fa6b0ddf34efa237b21d51895cc0922e1bf950c23ac46885d147fd8536) |
| Deposit 100 CSPR rent | `deposit_rent` (payable) | `74a8d6bf995958dbc59dce95874f1c8969b28568983131d60151f8d4e6ab3be9` | [link](https://testnet.cspr.live/transaction/74a8d6bf995958dbc59dce95874f1c8969b28568983131d60151f8d4e6ab3be9) |
| **Settlement: distribute pro-rata** | `distribute` | `dbadda3e43e101c66171e74c5d6d4f2237b98f05b0070f003e0a80bdc70bbf75` | [link](https://testnet.cspr.live/transaction/dbadda3e43e101c66171e74c5d6d4f2237b98f05b0070f003e0a80bdc70bbf75) |

### Settlement proof (distribute tx `dbad…bf75`, block 8113788, no error)
The `distribute` call emitted two native CSPR transfers — the 60/40 pro-rata split of the
100 CSPR rent pool, with zero integer-division dust:

- `account-hash-8a78d764…` (60 units) received **60,000,000,000 motes** (60 CSPR)
- `account-hash-b383c7cc…` (40 units) received **40,000,000,000 motes** (40 CSPR)

Contract state after: `total_distributed = 100,000,000,000 motes`, `rent_pool = 0`.

## Agent-driven settlement cycle (Milestone B)

The autonomous agent (`agent/`) then ran a full observe → pay (x402) → decide → settle
cycle and fired these on its own (rent signal said 200 CSPR due):

| Step | Entry point | Transaction hash | Explorer |
|------|-------------|------------------|----------|
| Deposit 200 CSPR rent | `deposit_rent` | `020b7b3667e8c4a535d9409e38132ee1854f670fe9cfd056fcf501cc909bd4ec` | [link](https://testnet.cspr.live/transaction/020b7b3667e8c4a535d9409e38132ee1854f670fe9cfd056fcf501cc909bd4ec) |
| **Settlement: distribute** | `distribute` | `7fd096effdecae47e0dc3ddbf3720493973bc75243f02209ffdd04a41626ffcb` | [link](https://testnet.cspr.live/transaction/7fd096effdecae47e0dc3ddbf3720493973bc75243f02209ffdd04a41626ffcb) |

Distribute `7fd0…ffcb` (block 8113832, no error) transferred 120 CSPR + 80 CSPR (60/40).
Lifetime `total_distributed` after this cycle: **300 CSPR**.

## Fully on-chain x402 cycle (2026-06-10)

The facilitator was upgraded to settle the x402 micro-payment **on-chain**: the agent
pre-signs a native CSPR transfer (payer-signs / facilitator-broadcasts, the Casper
analogue of EIP-3009), the facilitator validates it against the payment requirements,
broadcasts via `account_put_transaction`, and returns the real tx hash in the
`X-PAYMENT-RESPONSE` receipt. One autonomous cycle produced **three** on-chain txs:

| Step | Transaction hash | Explorer |
|------|------------------|----------|
| **x402 micro-payment** (2.5 CSPR, agent → oracle, block 8135906) | `6ae8c1b1079a593f1f077324cb1b251f4c02c3df446decedaa5770edd39ae380` | [link](https://testnet.cspr.live/transaction/6ae8c1b1079a593f1f077324cb1b251f4c02c3df446decedaa5770edd39ae380) |
| Deposit 200 CSPR rent | `4dde8fc0793e20bc37f8b284c416d3785a744bed63882580e9e88ca354b1968e` | [link](https://testnet.cspr.live/transaction/4dde8fc0793e20bc37f8b284c416d3785a744bed63882580e9e88ca354b1968e) |
| **Settlement: distribute** | `5945080aec1c5a3f319f3856a60183262b1c82bc2bed00c9a0af1f19ddae076f` | [link](https://testnet.cspr.live/transaction/5945080aec1c5a3f319f3856a60183262b1c82bc2bed00c9a0af1f19ddae076f) |

Oracle payee account `account-hash-e2039a5a78f02b0962a327c7d4f7c6e7ffcccc2ab4c131a476c935f4b5ae1273`
received the 2.5 CSPR query fee on-chain. Lifetime `total_distributed`: **500 CSPR**.

## How to reproduce
1. `source ~/.casper-build-env.sh && source ~/.cargo/env`
2. `cargo odra build` (produces `wasm/RwaVault.wasm`)
3. Populate `.env` (see `.env.example`): node address, `casper-test`, secret key path.
4. Deploy: `cargo run --bin casper_rwa_agent_cli -- deploy --deploy-mode default`
5. Settle: `cargo run --bin casper_rwa_agent_cli -- scenario settle --holder1 <acct-hash> --holder2 <acct-hash> --rent 100`

The autonomous agent (see `agent/`) drives steps 4–5 programmatically after paying an
x402-gated rent oracle.
