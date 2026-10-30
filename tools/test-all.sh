#!/usr/bin/env bash
# Everything that checks ray, in order, with a line each:
#   the unit tests (cargo test: under ten seconds once the GPU's shaders are
#     cached; with a GPU they include the CPU-against-GPU comparison),
#   the CPU-against-GPU comparison of every saved scene (tools/parity.sh),
#   every scene generator (tools/generators-check.sh),
#   the MCP server against a headless viewer (tools/mcp-check.sh),
#   the interface, headless (tools/gui-check.sh: a few minutes).
# Exit 1 when any of them failed; each one's output is kept in the folder
# named at the end. Nothing is written to saves/ or the user's home folders.
#   tools/test-all.sh [--quick]   (--quick: without the interface's checks)
set -uo pipefail
cd "$(dirname "$0")/.."
logs=$(mktemp -d -t ray-test-all-XXXX)
cargo build --release > "$logs/build.log" 2>&1 || { echo "FAIL build (see $logs/build.log)"; exit 1; }
failed=0
step() {
  local name=$1
  shift
  local t0=$SECONDS
  if "$@" > "$logs/$name.log" 2>&1; then
    printf 'ok   %-12s %4ds  %s\n' "$name" $((SECONDS - t0)) "$(grep -E 'test result|passed|all passed' "$logs/$name.log" | tail -1)"
  else
    printf 'FAIL %-12s %4ds  see %s\n' "$name" $((SECONDS - t0)) "$logs/$name.log"
    failed=$((failed + 1))
  fi
}
step unit cargo test --release
step parity tools/parity.sh "$logs/parity"
step generators tools/generators-check.sh
step mcp tools/mcp-check.sh
[ "${1:-}" = --quick ] || step gui tools/gui-check.sh
echo "logs: $logs"
exit $((failed > 0))
