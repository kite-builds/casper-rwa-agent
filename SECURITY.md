# Security Policy

## Reporting a Vulnerability

If you believe you have found a security vulnerability in CasperRWA-Agent, please report it privately. **Do not open a public issue for security problems.**

Preferred channel: open a private report via **GitHub Security Advisories**:
https://github.com/kite-builds/casper-rwa-agent/security/advisories/new

Please include:
- A description of the issue and its impact
- Steps to reproduce or a proof of concept
- The affected commit / contract package hash if applicable

We aim to acknowledge reports within a few business days and will coordinate a fix and disclosure timeline with you.

## Scope

- The `RwaVault` Odra smart contract (`src/`) and its deployment on Casper Testnet.
- The autonomous x402 rent-settlement agent (`agent/`).

Secrets (private keys, `.env`) are never committed; see `.gitignore`. Never send private keys or seed phrases in a report.
