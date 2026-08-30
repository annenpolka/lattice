#!/usr/bin/env bash
# Local macOS process smoke for the GPUI Studio window.
#
# This is an OS-integration check, not a replacement for #[gpui::test]. It
# launches the deterministic timeline-basic fixture with media preview and
# device audio disabled, waits for Studio's own bounded watchdog to quit, then
# validates the durable trace and semantic-state snapshot.
#
#   ./scripts/studio-smoke-macos.sh
#   ./scripts/studio-smoke-macos.sh --smoke-ms 12000
#   ./scripts/studio-smoke-macos.sh --release

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "MACOS SMOKE FAIL: this script requires macOS (Darwin)" >&2
  exit 1
fi

Root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$Root"

SmokeMs=8000
WaitSeconds=0
Release=0

usage() {
  cat <<'EOF'
Usage: ./scripts/studio-smoke-macos.sh [options]

  --smoke-ms N      Studio watchdog duration in milliseconds (default 8000)
  --wait-seconds N  outer process deadline (default smoke-ms/1000 + 20)
  --release         build and run the release binary
  -h, --help        show this help

The script always uses --ui-fixture timeline-basic with CPU rendering,
LATTICE_STUDIO_PREVIEW=0, and LATTICE_STUDIO_AUDIO_MONITOR=0.
Rustup runs Cargo with the channel declared in rust-toolchain.toml.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --smoke-ms)
      SmokeMs="${2:?missing --smoke-ms value}"
      shift 2
      ;;
    --wait-seconds)
      WaitSeconds="${2:?missing --wait-seconds value}"
      shift 2
      ;;
    --release)
      Release=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ ! "$SmokeMs" =~ ^[1-9][0-9]*$ ]]; then
  echo "MACOS SMOKE FAIL: --smoke-ms must be a positive integer" >&2
  exit 2
fi
if [[ ! "$WaitSeconds" =~ ^[0-9]+$ ]]; then
  echo "MACOS SMOKE FAIL: --wait-seconds must be a non-negative integer" >&2
  exit 2
fi

fail() {
  echo "" >&2
  echo "MACOS SMOKE FAIL: $*" >&2
  if [[ -n "${log:-}" && -f "$log" ]]; then
    echo "----- studio log -----" >&2
    tail -n 80 "$log" >&2 || true
  fi
  if [[ -n "${stdout:-}" && -f "$stdout" ]]; then
    echo "----- stdout -----" >&2
    tail -n 80 "$stdout" >&2 || true
  fi
  if [[ -n "${stderr:-}" && -f "$stderr" ]]; then
    echo "----- stderr -----" >&2
    tail -n 80 "$stderr" >&2 || true
  fi
  exit 1
}

if ! command -v rustup >/dev/null 2>&1; then
  fail "rustup is not on PATH; install rustup and the repository toolchain"
fi
toolchain="$(sed -nE 's/^[[:space:]]*channel[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' rust-toolchain.toml | head -n 1)"
[[ -n "$toolchain" ]] || fail "cannot read the channel from rust-toolchain.toml"
cargo_cmd=(rustup run "$toolchain" cargo)
profile=debug
cargo_args=(build -p lattice-studio --bin lattice-studio --features window)
if [[ "$Release" -eq 1 ]]; then
  profile=release
  cargo_args+=(--release)
fi

echo "building lattice-studio ($profile)..."
"${cargo_cmd[@]}" "${cargo_args[@]}"

target_root="${CARGO_TARGET_DIR:-$Root/target}"
if [[ "$target_root" != /* ]]; then
  target_root="$Root/$target_root"
fi
exe="$target_root/$profile/lattice-studio"
[[ -x "$exe" ]] || fail "missing executable $exe"

run_dir="$(mktemp -d -t lattice-studio-macos-smoke)"
log="$run_dir/studio.log"
stdout="$run_dir/studio.stdout.log"
stderr="$run_dir/studio.stderr.log"
state="$run_dir/studio.state.json"
pid=""

cleanup() {
  if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi
  case "$(basename "$run_dir")" in
    lattice-studio-macos-smoke.*)
      rm -rf -- "$run_dir"
      ;;
    *)
      echo "refusing to clean unexpected temporary path: $run_dir" >&2
      ;;
  esac
}
trap cleanup EXIT

: >"$log"
export LATTICE_STUDIO_LOG="$log"
export LATTICE_STUDIO_STATE="$state"
export LATTICE_STUDIO_PREVIEW=0
export LATTICE_STUDIO_AUDIO_MONITOR=0
export LATTICE_STUDIO_AUTOPLAY=0
export LATTICE_STUDIO_SMOKE_MS="$SmokeMs"
export LATTICE_STUDIO_RENDERER=cpu
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

echo "starting $exe --ui-fixture timeline-basic"
echo "  smoke ${SmokeMs}ms preview=off audio=off renderer=cpu"
echo "  artifacts $run_dir (removed on exit)"

"$exe" --ui-fixture timeline-basic >"$stdout" 2>"$stderr" &
pid=$!

if [[ "$WaitSeconds" -eq 0 ]]; then
  WaitSeconds=$(((SmokeMs + 999) / 1000 + 20))
fi

deadline=$((SECONDS + WaitSeconds))
while kill -0 "$pid" 2>/dev/null; do
  if (( SECONDS >= deadline )); then
    kill "$pid" 2>/dev/null || true
    sleep 0.2
    kill -9 "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
    pid=""
    fail "Studio was still running after ${WaitSeconds}s; its smoke watchdog did not quit"
  fi
  sleep 0.25
done

set +e
wait "$pid"
status=$?
set -e
pid=""

echo ""
echo "----- studio log tail -----"
tail -n 60 "$log" || true

[[ "$status" -eq 0 ]] || fail "Studio exited with status $status"

process_text="$(cat "$log" "$stdout" "$stderr" 2>/dev/null || true)"
if grep -Eiq 'PANIC|panicked at|fatal runtime error|preview worker panic|NoSupportedDeviceFound|open_window failed|semantic_state write failed|(^|[[:space:]])fatal:' <<<"$process_text"; then
  fail "Studio log/stdout/stderr contains a panic, fatal error, or failed window/state write"
fi
grep -q "open_window ok" "$log" || fail "missing open_window ok"
grep -q "first paint" "$log" || fail "missing first paint"
grep -q 'semantic_state .*"reason":"first-paint"' "$log" || \
  fail "missing semantic_state reason=first-paint"
grep -q "audio monitor explicitly disabled" "$log" || \
  fail "LATTICE_STUDIO_AUDIO_MONITOR=0 was not observed"
grep -q "smoke quit" "$log" || \
  fail "missing smoke quit; an externally terminated process must not pass"

[[ -s "$state" ]] || fail "LATTICE_STUDIO_STATE did not produce a non-empty file"
if command -v jq >/dev/null 2>&1; then
  jq -e '
    type == "object"
    and .reason == "first-paint"
    and .fixture == "timeline-basic"
  ' "$state" >/dev/null || fail "semantic state is invalid or lacks the expected fixture/reason"
else
  # The snapshot contains legal JSON null values that macOS plutil cannot
  # convert to its plist model. Keep jq optional and verify serde_json's two
  # smoke-contract fields when it is unavailable.
  grep -Eq '"reason"[[:space:]]*:[[:space:]]*"first-paint"' "$state" || \
    fail "semantic state lacks reason=first-paint"
  grep -Eq '"fixture"[[:space:]]*:[[:space:]]*"timeline-basic"' "$state" || \
    fail "semantic state lacks fixture=timeline-basic"
fi

echo ""
echo "MACOS SMOKE OK fixture=timeline-basic pid completed state=first-paint"
