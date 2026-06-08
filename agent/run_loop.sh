#!/usr/bin/env bash
# One-shot driver for the CasperRWA-Agent autonomous loop.
# Starts the facilitator + oracle, runs one agent cycle, tears down.
#
# Usage: ./run_loop.sh [--dry-run]
set -euo pipefail
cd "$(dirname "$0")"

cargo build --quiet

./target/debug/facilitator >/tmp/casper_facilitator.log 2>&1 &
FAC=$!
./target/debug/oracle >/tmp/casper_oracle.log 2>&1 &
ORC=$!
trap 'kill $FAC $ORC 2>/dev/null || true' EXIT

for _ in $(seq 1 30); do
  curl -s http://127.0.0.1:8403/ >/dev/null 2>&1 \
    && curl -s http://127.0.0.1:8402/ >/dev/null 2>&1 && break
  sleep 0.3
done

echo "== facilitator: $(curl -s http://127.0.0.1:8403/)"
echo "== oracle:      $(curl -s http://127.0.0.1:8402/)"
echo "== running autonomous agent cycle =="
./target/debug/agent --contract-dir .. "$@"
