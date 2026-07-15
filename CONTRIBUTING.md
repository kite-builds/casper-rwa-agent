# Contributing to CasperRWA-Agent

Thanks for your interest in contributing! This project is an autonomous RWA rent-settlement agent built on the Casper Network (Odra smart contracts + an x402 micro-payment agent loop).

## Development setup

Prerequisites: Rust (see `rust-toolchain`), the [Odra](https://odra.dev) toolchain, and a Casper Testnet key (never commit keys - see `.gitignore`).

```bash
cargo build
cargo test
cargo fmt --all
cargo clippy --all-targets --all-features
```

Deployment steps for the `RwaVault` contract on Casper Testnet are in [`DEPLOYMENT.md`](./DEPLOYMENT.md).

## Pull requests

1. Fork and create a feature branch.
2. Keep changes focused; add tests where it makes sense.
3. Ensure `cargo build`, `cargo test`, `cargo fmt --check`, and `cargo clippy` pass.
4. Open a PR with a clear description of the change and its motivation.

## Reporting bugs / security issues

- Functional bugs: open a GitHub issue.
- Security vulnerabilities: **do not** open a public issue - see [`SECURITY.md`](./SECURITY.md).

By participating you agree to abide by our [Code of Conduct](./CODE_OF_CONDUCT.md).
