#!/usr/bin/env bash
# CHI-67 negative harness. Runs a real --miss-commit smoke and records the log.
# Reviewers read docs/artifacts/chi67-miss-commit.log: nonzero exit, no LINUX SMOKE OK.
#
#   DISPLAY=:1 ./scripts/studio-linux-smoke-miss.sh
#
# Not CHI-63. Not product Linux. Do not mark CHI-67 Done.

set -euo pipefail

Root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$Root"

out="${LATTICE_STUDIO_MISS_DIR:-$Root/docs/artifacts}"
mkdir -p "$out"
log="$out/chi67-miss-commit.log"
tmp="$(mktemp)"

unset WAYLAND_DISPLAY || true
export DISPLAY="${DISPLAY:-:1}"

set +e
./scripts/studio-linux-smoke.sh --fixture timeline-basic --miss-commit >"$tmp" 2>&1
ec=$?
set -e

{
  echo "==== chi67-miss-commit harness $(date -Iseconds) DISPLAY=${DISPLAY} exit=${ec} ===="
  cat "$tmp"
  echo ""
  echo "harness_exit=$ec"
} >"$log"
rm -f "$tmp"

if grep -q 'LINUX SMOKE OK' "$log"; then
  echo "CHI-67 miss harness FAIL: log contains LINUX SMOKE OK ($log)" >&2
  exit 2
fi
if [[ "$ec" -eq 0 ]]; then
  echo "CHI-67 miss harness FAIL: smoke exited 0 ($log)" >&2
  exit 2
fi
if ! grep -q 'LINUX SMOKE FAIL: missing timeline-pointer-commit' "$log"; then
  echo "CHI-67 miss harness FAIL: missing FAIL line ($log)" >&2
  exit 2
fi

echo "CHI-67 miss harness OK exit=$ec log=$log"
exit 0
