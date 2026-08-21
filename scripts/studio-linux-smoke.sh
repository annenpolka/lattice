#!/usr/bin/env bash
# Agent/Linux Studio smoke. Not a product Linux target and not CHI-63.
#
# Builds lattice-studio, launches a deterministic --ui-fixture window with
# preview/audio detached, waits for the durable log, screenshots DISPLAY,
# and optionally injects one click plus one scrub-style drag via xdotool.
#
#   ./scripts/studio-linux-smoke.sh
#   ./scripts/studio-linux-smoke.sh --fixture drag-valid
#   ./scripts/studio-linux-smoke.sh --no-interact
#
# Windows dogfood remains scripts/studio-smoke.ps1 / studio-debug.ps1.

set -euo pipefail

Root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$Root"

Fixture="timeline-basic"
Interact=1
Release=0
SmokeMs=20000
WaitSeconds=0

usage() {
  cat <<'EOF'
Usage: ./scripts/studio-linux-smoke.sh [options]

  --fixture NAME   timeline-basic | drag-valid | drag-invalid | dense-project
  --no-interact    skip xdotool click/drag (still requires a visible window)
  --release        cargo build --release
  --smoke-ms N     LATTICE_STUDIO_SMOKE_MS watchdog (default 20000)
  --wait-seconds N process wait budget (default smoke-ms/1000 + 15)
  -h, --help       show this help

Environment:
  DISPLAY must already be an X11 session, or Xvfb will be started.
  LATTICE_STUDIO_PREVIEW=0 and LATTICE_STUDIO_AUDIO_MONITOR=0 are forced.
  Software Vulkan (lavapipe) is selected when its ICD is present.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --fixture)
      Fixture="${2:?missing --fixture name}"
      shift 2
      ;;
    --no-interact)
      Interact=0
      shift
      ;;
    --release)
      Release=1
      shift
      ;;
    --smoke-ms)
      SmokeMs="${2:?missing --smoke-ms value}"
      shift 2
      ;;
    --wait-seconds)
      WaitSeconds="${2:?missing --wait-seconds value}"
      shift 2
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

fail() {
  echo ""
  echo "LINUX SMOKE FAIL: $*"
  exit 1
}

profile=debug
cargo_args=(build -p lattice-studio --features window)
if [[ "$Release" -eq 1 ]]; then
  profile=release
  cargo_args+=(--release)
fi

echo "building lattice-studio ($profile)..."
cargo "${cargo_args[@]}"

target_root="${CARGO_TARGET_DIR:-$Root/target}"
if [[ "$target_root" != /* ]]; then
  target_root="$Root/$target_root"
fi
exe="$target_root/$profile/lattice-studio"
[[ -x "$exe" ]] || fail "missing $exe"

if [[ -z "${DISPLAY:-}" ]]; then
  echo "DISPLAY unset; starting Xvfb :99"
  export DISPLAY=:99
  Xvfb :99 -screen 0 1920x1200x24 >/tmp/lattice-studio-xvfb.log 2>&1 &
  xvfb_pid=$!
  trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
  sleep 0.5
  if ! kill -0 "$xvfb_pid" 2>/dev/null; then
    fail "Xvfb failed to start; see /tmp/lattice-studio-xvfb.log"
  fi
fi

if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
  unset WAYLAND_DISPLAY || true
fi

lavapipe=""
for candidate in \
  /usr/share/vulkan/icd.d/lvp_icd.x86_64.json \
  /usr/share/vulkan/icd.d/lvp_icd.json
do
  if [[ -f "$candidate" ]]; then
    lavapipe="$candidate"
    break
  fi
done
if [[ -n "$lavapipe" && -z "${VK_ICD_FILENAMES:-}" ]]; then
  export VK_ICD_FILENAMES="$lavapipe"
fi
export LIBGL_ALWAYS_SOFTWARE="${LIBGL_ALWAYS_SOFTWARE:-1}"

stamp="$(date +%Y%m%d-%H%M%S)"
out="${LATTICE_STUDIO_SMOKE_DIR:-$target_root/studio-linux-smoke}"
mkdir -p "$out"
log="$out/studio-linux-smoke-$stamp.log"
stdout="$out/studio-linux-smoke-$stamp.stdout.log"
stderr="$out/studio-linux-smoke-$stamp.stderr.log"
state="$out/studio-linux-smoke-$stamp.state.json"
shot="$out/studio-linux-smoke-$stamp.png"

export LATTICE_STUDIO_LOG="$log"
export LATTICE_STUDIO_STATE="$state"
export LATTICE_STUDIO_PREVIEW=0
export LATTICE_STUDIO_AUDIO_MONITOR=0
export LATTICE_STUDIO_AUTOPLAY=0
export LATTICE_STUDIO_SMOKE_MS="$SmokeMs"
export LATTICE_STUDIO_RENDERER="${LATTICE_STUDIO_RENDERER:-cpu}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

{
  echo "==== studio-linux-smoke $(date -Iseconds) fixture=$Fixture preview=off audio=off display=${DISPLAY} vulkan=${VK_ICD_FILENAMES:-default} ===="
} >>"$log"

echo "starting $exe --ui-fixture $Fixture"
echo "  log   $log"
echo "  state $state"
echo "  shot  $shot"
echo "  display $DISPLAY"

"$exe" --ui-fixture "$Fixture" >"$stdout" 2>"$stderr" &
pid=$!
echo "pid $pid"

if [[ "$WaitSeconds" -le 0 ]]; then
  WaitSeconds=$((SmokeMs / 1000 + 15))
fi

ready=0
deadline=$((SECONDS + WaitSeconds))
while (( SECONDS < deadline )); do
  if [[ -f "$log" ]] && grep -q "open_window ok" "$log" && grep -q "first paint" "$log"; then
    ready=1
    break
  fi
  if ! kill -0 "$pid" 2>/dev/null; then
    wait "$pid" || true
    echo "----- studio log -----"
    tail -n 80 "$log" 2>/dev/null || true
    echo "----- stderr -----"
    tail -n 80 "$stderr" 2>/dev/null || true
    fail "lattice-studio exited before the window became ready"
  fi
  sleep 0.25
done
if [[ "$ready" -ne 1 ]]; then
  kill "$pid" 2>/dev/null || true
  fail "timed out waiting for open_window ok / first paint"
fi

if ! command -v ffmpeg >/dev/null; then
  fail "ffmpeg is required to capture the DISPLAY screenshot"
fi
size="$(xdpyinfo | awk '/dimensions:/ { print $2; exit }')"
[[ -n "$size" ]] || fail "could not read X11 dimensions from DISPLAY=$DISPLAY"
ffmpeg -y -hide_banner -loglevel error -f x11grab -video_size "$size" -i "${DISPLAY}.0" -frames:v 1 "$shot"
[[ -s "$shot" ]] || fail "screenshot was not written: $shot"
echo "screenshot $shot ($(wc -c <"$shot") bytes)"

if [[ "$Interact" -eq 1 ]]; then
  command -v xdotool >/dev/null || fail "xdotool is required for --interact (pass --no-interact to skip)"
  win=""
  for _ in $(seq 1 40); do
    win="$(xdotool search --name 'Lattice Studio' 2>/dev/null | head -n 1 || true)"
    if [[ -n "$win" ]]; then
      break
    fi
    sleep 0.25
  done
  [[ -n "$win" ]] || fail "xdotool could not find a Lattice Studio window"
  xdotool windowactivate --sync "$win"
  eval "$(xdotool getwindowgeometry --shell "$win")"
  echo "window id=$win x=${X} y=${Y} ${WIDTH}x${HEIGHT}"
  # Toolbar Play sits on the first row, right of the CPU/DX12 toggles.
  click_x=$((X + WIDTH * 72 / 100))
  click_y=$((Y + HEIGHT * 8 / 100))
  echo "click play-ish ${click_x},${click_y}"
  xdotool mousemove --sync "$click_x" "$click_y" click 1
  sleep 0.3
  # Timeline rail is TIMELINE_WIDTH (640) plus a 64px label, left-aligned
  # in the bottom bar — not the full 1400px window.
  from_x=$((X + 120))
  to_x=$((X + 520))
  rail_y=$((Y + HEIGHT * 92 / 100))
  echo "scrub-drag ${from_x},${rail_y} -> ${to_x},${rail_y}"
  xdotool mousemove --sync "$from_x" "$rail_y" mousedown 1
  xdotool mousemove --sync "$to_x" "$rail_y"
  xdotool mouseup 1
  sleep 0.5
fi

deadline=$((SECONDS + WaitSeconds))
while kill -0 "$pid" 2>/dev/null && (( SECONDS < deadline )); do
  if grep -q "smoke quit" "$log"; then
    break
  fi
  sleep 0.25
done
if kill -0 "$pid" 2>/dev/null; then
  kill "$pid" 2>/dev/null || true
  sleep 0.2
fi
wait "$pid" || true

echo ""
echo "----- studio log tail -----"
tail -n 40 "$log" || true

log_text="$(cat "$log")"
if grep -Eq 'PANIC|panicked at|fatal runtime error' <<<"$log_text"; then
  fail "Studio log contains a panic or fatal runtime error"
fi
if ! grep -q "open_window ok" <<<"$log_text"; then
  fail "missing open_window ok"
fi
if ! grep -q "first paint" <<<"$log_text"; then
  fail "missing first paint"
fi
if ! grep -q 'semantic_state .*\"reason\":\"open\"' <<<"$log_text"; then
  fail "missing semantic_state reason=open"
fi
if ! grep -q 'semantic_state .*\"reason\":\"first-paint\"' <<<"$log_text"; then
  fail "missing semantic_state reason=first-paint"
fi
if [[ "$Interact" -eq 1 ]] && ! grep -q 'semantic_state .*\"reason\":\"timeline-pointer-commit\"' <<<"$log_text"; then
  echo "WARN: no timeline-pointer-commit semantic_state (xdotool may have missed the ruler)"
fi
if ! grep -q "smoke quit" <<<"$log_text"; then
  echo "WARN: missing smoke quit (process was stopped by the script after the window was observed)"
fi

echo ""
echo "LINUX SMOKE OK fixture=$Fixture pid=$pid shot=$shot log=$log"
exit 0
