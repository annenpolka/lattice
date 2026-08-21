#!/usr/bin/env bash
# Agent/Linux Studio smoke. Not a product Linux target and not CHI-63.
#
# Demonstrated CHI-64 path: an existing X11 session (Cursor Cloud XFCE is
# DISPLAY=:1). Xvfb is an explicit fallback, not that path.
#
# Builds lattice-studio, launches --ui-fixture with preview/audio detached,
# waits for open_window ok / first paint, identifies the unique viewable
# top-level Studio client by PID / _NET_WM_PID (never title, WM_CLASS,
# or largest-area), captures that XID with ffmpeg -window_id (not a
# root-rectangle grab of ${DISPLAY}.0), asserts nonblank pixels, then
# optionally clicks Play and scrub-drags using app-emitted smoke_geom
# (play / ruler / rail / tracks / canvas) offset by verified client
# bounds. Event checks read only bytes appended after each action.
# debug_selector is a test-only no-op in the product binary and is not
# used here.
#
#   ./scripts/studio-linux-smoke.sh --self-test
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture drag-valid
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh --no-interact
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh --miss-commit  # CHI-67 negative
#   ./scripts/studio-linux-smoke.sh --allow-xvfb   # labeled fallback only
#
# Windows dogfood remains scripts/studio-smoke.ps1 / studio-debug.ps1.

set -euo pipefail

Root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$Root"

Fixture="timeline-basic"
Interact=1
Release=0
SmokeMs=25000
WaitSeconds=0
AllowXvfb=0
AllowWaylandX11=0
MissCommit=0
SelfTest=0

usage() {
  cat <<'EOF'
Usage: ./scripts/studio-linux-smoke.sh [options]

  --fixture NAME   timeline-basic | drag-valid | drag-invalid | dense-project
  --no-interact         skip OS click/drag (still requires a visible nonblank window)
  --miss-commit         CHI-67 negative: off-widget click, omit timeline-pointer-commit, expect FAIL
  --allow-xvfb          start Xvfb if DISPLAY is unset (not the CHI-64 demonstrated path)
  --allow-wayland-x11   labeled X11-under-Wayland path when WAYLAND_DISPLAY is set
  --release        cargo build --release
  --smoke-ms N     LATTICE_STUDIO_SMOKE_MS watchdog (default 25000)
  --wait-seconds N process wait budget (default smoke-ms/1000 + 20)
  --self-test      run window-identity unit tests and exit
  -h, --help       show this help

Environment:
  Demonstrated path: an existing X11 DISPLAY (Cursor Cloud XFCE = :1).
  LATTICE_STUDIO_PREVIEW=0 and LATTICE_STUDIO_AUDIO_MONITOR=0 are forced.
  PREVIEW=0 / RENDERER=cpu do not skip GPUI/Blade window init.
  mesa-vulkan-drivers (lavapipe ICD in /usr/share/vulkan/icd.d/) is enough;
  this script does not set VK_ICD_FILENAMES and does not gate on vulkaninfo.
  If default cc is clang, RUSTFLAGS=-C linker=gcc is set so rustc can link
  libstdc++. That is not baked into Cargo.toml / .cargo/config.toml.
  NoSupportedDeviceFound is fatal. Missing timeline-pointer-commit fails.
  Missing smoke quit fails (not WARN). Populated WAYLAND_DISPLAY fails unless
  --allow-wayland-x11 is passed.
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
    --allow-xvfb)
      AllowXvfb=1
      shift
      ;;
    --allow-wayland-x11)
      AllowWaylandX11=1
      shift
      ;;
    --miss-commit)
      MissCommit=1
      shift
      ;;
    --self-test)
      SelfTest=1
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

json_field() {
  python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get(sys.argv[2],""))' "$1" "$2"
}

window_py="$Root/scripts/studio-linux-smoke-window.py"

run_window_self_test() {
  python3 "$window_py" --self-test
}

need_cmd() {
  local bin="$1"
  local packages="$2"
  if ! command -v "$bin" >/dev/null 2>&1; then
    fail "missing $bin. Install: sudo apt-get install -y $packages"
  fi
}

preflight_packages() {
  need_cmd gcc "g++ gcc"
  need_cmd g++ "g++ gcc"
  if [[ ! -e /usr/include/xkbcommon/xkbcommon.h ]] && ! pkg-config --exists xkbcommon 2>/dev/null; then
    fail "missing libxkbcommon headers. Install: sudo apt-get install -y libxkbcommon-dev libxkbcommon-x11-dev"
  fi
  local icds=()
  if [[ -d /usr/share/vulkan/icd.d ]]; then
    shopt -s nullglob
    icds=(/usr/share/vulkan/icd.d/*.json)
    shopt -u nullglob
  fi
  if [[ ${#icds[@]} -eq 0 ]]; then
    fail "no Vulkan ICD in /usr/share/vulkan/icd.d/. Install: sudo apt-get install -y mesa-vulkan-drivers. lavapipe is enough; do not set VK_ICD_FILENAMES and do not gate on vulkaninfo."
  fi
  echo "vulkan ICD present (${#icds[@]}): ${icds[*]}"
  need_cmd xdotool xdotool
  need_cmd xprop "x11-utils"
  need_cmd xwininfo "x11-utils"
  need_cmd ffmpeg ffmpeg
  need_cmd python3 python3
  run_window_self_test || fail "studio-linux-smoke-window.py --self-test failed"
  if [[ "$AllowXvfb" -eq 1 && -z "${DISPLAY:-}" ]]; then
    need_cmd Xvfb xvfb
  fi
}

# Cloud VMs often ship cc=clang. rustc then passes -lstdc++ to clang and
# link fails even with g++ / libstdc++-*-dev installed. gcc as the rustc
# linker works. Do not put this in Cargo.toml or .cargo/config.toml.
maybe_set_gcc_linker() {
  if [[ "${RUSTFLAGS:-}" == *"-C linker="* || "${RUSTFLAGS:-}" == *"-Clinker="* ]]; then
    echo "RUSTFLAGS already names a linker; leaving as-is"
    return 0
  fi
  local cc_ver
  cc_ver="$(cc --version 2>/dev/null | head -n1 || true)"
  if [[ "$cc_ver" == *[Cc]lang* ]]; then
    if ! command -v gcc >/dev/null 2>&1; then
      fail "default cc is clang ($cc_ver) but gcc is missing. rustc -lstdc++ fails with the clang driver. Install: sudo apt-get install -y g++ gcc. Re-run; this script sets RUSTFLAGS=-C linker=gcc. Do not bake the linker into Cargo.toml."
    fi
    export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C linker=gcc"
    echo "note: default cc is clang; setting RUSTFLAGS=-C linker=gcc so rustc can link libstdc++ (not Cargo.toml)"
  fi
}

if [[ "$SelfTest" -eq 1 ]]; then
  need_cmd python3 python3
  run_window_self_test
  exit 0
fi

preflight_packages
maybe_set_gcc_linker

if [[ -n "${WAYLAND_DISPLAY:-}" && "$AllowWaylandX11" -ne 1 ]]; then
  fail "WAYLAND_DISPLAY is set (${WAYLAND_DISPLAY}). Demonstrated CHI-64 path is X11 DISPLAY=:1 without Wayland. Unset WAYLAND_DISPLAY or pass --allow-wayland-x11 for a labeled X11-under-Wayland path."
fi

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

display_path="existing-x11"
if [[ -z "${DISPLAY:-}" ]]; then
  if [[ "$AllowXvfb" -ne 1 ]]; then
    fail "DISPLAY is unset. Demonstrated CHI-64 path is an existing X11 session (DISPLAY=:1 on Cursor Cloud). Pass --allow-xvfb only for a labeled Xvfb fallback."
  fi
  echo "DISPLAY unset; starting Xvfb :99 (CHI-64 fallback, not the demonstrated DISPLAY=:1 path)"
  export DISPLAY=:99
  display_path="xvfb-fallback"
  Xvfb :99 -screen 0 1920x1200x24 >/tmp/lattice-studio-xvfb.log 2>&1 &
  xvfb_pid=$!
  sleep 0.5
  if ! kill -0 "$xvfb_pid" 2>/dev/null; then
    fail "Xvfb failed to start; see /tmp/lattice-studio-xvfb.log"
  fi
fi

if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
  if [[ "$AllowWaylandX11" -ne 1 ]]; then
    fail "WAYLAND_DISPLAY is set (${WAYLAND_DISPLAY}). Demonstrated CHI-64 path is X11 DISPLAY=:1 without Wayland. Unset WAYLAND_DISPLAY or pass --allow-wayland-x11 for a labeled X11-under-Wayland path."
  fi
  display_path="x11-under-wayland"
  echo "WAYLAND_DISPLAY=${WAYLAND_DISPLAY}; taking labeled X11-under-Wayland path"
elif [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
  unset WAYLAND_DISPLAY || true
fi

# Do not force VK_ICD_FILENAMES. After mesa-vulkan-drivers, lavapipe is
# selected from /usr/share/vulkan/icd.d/ without this variable. Leave a
# caller-supplied value alone. Do not run vulkaninfo (BadMatch is not fatal).
if [[ -n "${VK_ICD_FILENAMES:-}" ]]; then
  echo "VK_ICD_FILENAMES already set; leaving as-is"
else
  echo "VK_ICD_FILENAMES unset; using ICD directory (mesa-vulkan-drivers / lavapipe)"
fi

# One directory per process. Second-resolution names plus append-mode
# LATTICE_STUDIO_LOG would let a sibling run's timeline-pointer-commit
# greenwash a miss.
smoke_root="${LATTICE_STUDIO_SMOKE_DIR:-$target_root/studio-linux-smoke}"
mkdir -p "$smoke_root"
out="$(mktemp -d "$smoke_root/run.XXXXXX")"
log="$out/studio.log"
stdout="$out/studio.stdout.log"
stderr="$out/studio.stderr.log"
state="$out/studio.state.json"
geom="$out/studio.geom.json"
shot="$out/studio.png"
shot_after="$out/studio-after.png"
window_json="$out/studio.window.json"
: >"$log"

export LATTICE_STUDIO_LOG="$log"
export LATTICE_STUDIO_STATE="$state"
export LATTICE_STUDIO_GEOM="$geom"
export LATTICE_STUDIO_PREVIEW=0
export LATTICE_STUDIO_AUDIO_MONITOR=0
export LATTICE_STUDIO_AUTOPLAY=0
export LATTICE_STUDIO_SMOKE_MS="$SmokeMs"
export LATTICE_STUDIO_RENDERER="${LATTICE_STUDIO_RENDERER:-cpu}"
export RUST_BACKTRACE="${RUST_BACKTRACE:-1}"

{
  echo "==== studio-linux-smoke $(date -Iseconds) fixture=$Fixture preview=off audio=off display=${DISPLAY} path=${display_path} vulkan=${VK_ICD_FILENAMES:-default} ===="
} >>"$log"

echo "starting $exe --ui-fixture $Fixture"
echo "  log    $log"
echo "  state  $state"
echo "  geom   $geom"
echo "  shot   $shot"
echo "  display $DISPLAY ($display_path)"

"$exe" --ui-fixture "$Fixture" >"$stdout" 2>"$stderr" &
pid=$!
echo "pid $pid"
cleanup() {
  kill "$pid" 2>/dev/null || true
  if [[ -n "${xvfb_pid:-}" ]]; then
    kill "$xvfb_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

if [[ "$WaitSeconds" -le 0 ]]; then
  WaitSeconds=$((SmokeMs / 1000 + 20))
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
    if grep -Eq 'NoSupportedDeviceFound|panicked at' "$log" "$stderr" 2>/dev/null; then
      fail "lattice-studio died during GPU/window init (NoSupportedDeviceFound is not hidden)"
    fi
    fail "lattice-studio exited before the window became ready"
  fi
  sleep 0.25
done
if [[ "$ready" -ne 1 ]]; then
  kill "$pid" 2>/dev/null || true
  fail "timed out waiting for open_window ok / first paint on DISPLAY=$DISPLAY ($display_path)"
fi

win=""
identify_window() {
  local extra=()
  if [[ -n "${1:-}" ]]; then
    extra=(--expect-id "$1")
  fi
  python3 "$window_py" identify --pid "$pid" --out "$window_json" --display "$DISPLAY" "${extra[@]}"
}

load_window_json() {
  win="$(json_field "$window_json" "id")"
  client_x="$(json_field "$window_json" "client_x")"
  client_y="$(json_field "$window_json" "client_y")"
  WIDTH="$(json_field "$window_json" "w")"
  HEIGHT="$(json_field "$window_json" "h")"
  frame_left="$(json_field "$window_json" "frame_left")"
  frame_right="$(json_field "$window_json" "frame_right")"
  frame_top="$(json_field "$window_json" "frame_top")"
  frame_bottom="$(json_field "$window_json" "frame_bottom")"
  frame_x="$(json_field "$window_json" "frame_x")"
  frame_y="$(json_field "$window_json" "frame_y")"
  wm_class="$(json_field "$window_json" "wm_class")"
  X="$client_x"
  Y="$client_y"
}

refresh_window() {
  identify_window "${win:-}" || return 1
  load_window_json
  [[ "${WIDTH:-0}" -ge 200 && "${HEIGHT:-0}" -ge 200 ]] || return 1
}

found_window=0
for _ in $(seq 1 40); do
  if identify_window; then
    load_window_json
    found_window=1
    break
  fi
  sleep 0.25
done
[[ "$found_window" -eq 1 && -n "$win" && -s "$window_json" ]] || fail "no unique viewable Studio client (_NET_WM_PID=$pid); title / largest-area are not identities"
[[ "${WIDTH:-0}" -ge 200 && "${HEIGHT:-0}" -ge 200 ]] || fail "Studio client geometry is implausible: ${WIDTH:-?}x${HEIGHT:-?}"
# xdotool getwindowgeometry is decoration-inflated / origin-ambiguous.
# Clicks use verified xwininfo client bounds + smoke_geom (GPUI pixels
# are client-local). Do not treat a Play hit as license to switch later
# clicks onto the WM frame origin — the ruler is too thin for that.
echo "window id=$win pid=$pid identity=net_wm_pid wm_class=${wm_class:-?} frame=${frame_x},${frame_y} extents=${frame_left},${frame_right},${frame_top},${frame_bottom} client=${client_x},${client_y} ${WIDTH}x${HEIGHT}"
xdotool windowactivate --sync "$win"
xdotool windowfocus --sync "$win" || true
sleep 0.2

capture_window() {
  local dest="$1"
  refresh_window || fail "Studio XID $win (_NET_WM_PID=$pid) is no longer the unique viewable client"
  # Bind the grab to the identified XID. A root rectangle at a cached
  # origin can capture another window or wallpaper after move/unmap.
  if ! ffmpeg -y -hide_banner -loglevel error \
    -f x11grab -window_id "$win" -video_size "${WIDTH}x${HEIGHT}" \
    -i "$DISPLAY" \
    -frames:v 1 "$dest"
  then
    fail "ffmpeg -window_id $win capture failed; root-rectangle grab is not a fallback"
  fi
  [[ -s "$dest" ]] || fail "screenshot was not written: $dest"
}

assert_nonblank() {
  local dest="$1"
  python3 - "$dest" <<'PY'
import struct, sys, zlib
from pathlib import Path

path = Path(sys.argv[1])
data = path.read_bytes()
if data[:8] != b"\x89PNG\r\n\x1a\n":
    raise SystemExit(f"not a PNG: {path}")
pos = 8
width = height = None
rows = []
while pos + 8 <= len(data):
    length, ctype = struct.unpack(">I4s", data[pos : pos + 8])
    chunk = data[pos + 8 : pos + 8 + length]
    pos += 12 + length
    if ctype == b"IHDR":
        width, height, bit, color, *_ = struct.unpack(">IIBBBBB", chunk)
        if bit != 8 or color not in (2, 6):
            raise SystemExit(f"unsupported PNG {width}x{height} bit={bit} color={color}")
    elif ctype == b"IDAT":
        rows.append(chunk)
    elif ctype == b"IEND":
        break
raw = zlib.decompress(b"".join(rows))
bpp = 3 if color == 2 else 4
stride = 1 + width * bpp
if len(raw) < stride * height:
    raise SystemExit(f"truncated PNG payload {len(raw)}")
colors = {}
dark = bright = 0
total = width * height
for y in range(height):
    row = raw[y * stride + 1 : (y + 1) * stride]
    for x in range(width):
        r, g, b = row[x * bpp : x * bpp + 3]
        key = (r >> 4, g >> 4, b >> 4)
        colors[key] = colors.get(key, 0) + 1
        luma = (int(r) + int(g) + int(b)) / 3
        if luma < 64:
            dark += 1
        if luma > 80:
            bright += 1
unique = len(colors)
top = max(colors.values()) / total
if unique < 12:
    raise SystemExit(f"blank/flat capture: {unique} quantized colors in {width}x{height}")
if top > 0.97:
    raise SystemExit(f"near-solid capture: dominant color occupies {top:.3f} of {width}x{height}")
if dark < total * 0.02 or bright < total * 0.005:
    raise SystemExit(
        f"missing Studio contrast: dark={dark} bright={bright} unique={unique} {width}x{height}"
    )
print(f"nonblank {path} {width}x{height} unique={unique} dark={dark} bright={bright}")
PY
}

capture_window "$shot"
echo "screenshot $shot ($(wc -c <"$shot") bytes) window_id=$win ${WIDTH}x${HEIGHT}+${X}+${Y}"
assert_nonblank "$shot"

log_bytes() {
  if [[ -f "$log" ]]; then
    wc -c <"$log" | tr -d '[:space:]'
  else
    echo 0
  fi
}

log_suffix_has() {
  local offset="$1"
  local pattern="$2"
  local start=$((offset + 1))
  [[ -f "$log" ]] || return 1
  tail -c +"$start" "$log" | grep -q "$pattern"
}

wait_log_since() {
  local offset="$1"
  local pattern="$2"
  local label="$3"
  local budget="${4:-8}"
  local until=$((SECONDS + budget))
  while (( SECONDS < until )); do
    if log_suffix_has "$offset" "$pattern"; then
      return 0
    fi
    if ! kill -0 "$pid" 2>/dev/null; then
      fail "Studio exited while waiting for $label"
    fi
    sleep 0.1
  done
  fail "missing $label"
}

click_client() {
  local lx="$1"
  local ly="$2"
  local sx=$((client_x + lx))
  local sy=$((client_y + ly))
  [[ "$lx" -ge 0 && "$lx" -lt "$WIDTH" ]] || fail "local X $lx outside ${WIDTH}x${HEIGHT}"
  [[ "$ly" -ge 0 && "$ly" -lt "$HEIGHT" ]] || fail "local Y $ly outside ${WIDTH}x${HEIGHT}"
  xdotool windowactivate --sync "$win"
  xdotool windowfocus --sync "$win" || true
  # Root = verified xwininfo client origin + smoke_geom. Do not use
  # xdotool --window (WM coordinate spaces differ) or the frame origin.
  xdotool mousemove --sync "$sx" "$sy"
  sleep 0.05
  xdotool mousedown 1
  sleep 0.05
  xdotool mouseup 1
}

if [[ "$MissCommit" -eq 1 ]]; then
  echo "CHI-67 negative miss: off-widget click at client 8,8; omit timeline-pointer-commit"
  miss_mark="$(log_bytes)"
  click_client 8 8
  sleep 0.5
  if log_suffix_has "$miss_mark" 'semantic_state .*\"reason\":\"timeline-pointer-commit\"'; then
    fail "miss path unexpectedly produced timeline-pointer-commit (cannot prove fail-closed)"
  fi
  fail "missing timeline-pointer-commit (CHI-67 negative miss; fail-closed)"
fi

if [[ "$Interact" -eq 1 ]]; then
  geom_deadline=$((SECONDS + 8))
  while (( SECONDS < geom_deadline )) && [[ ! -s "$geom" ]]; do
    sleep 0.1
  done
  [[ -s "$geom" ]] || fail "Studio did not write smoke_geom widget bounds ($geom)"
  # First flex layout can wrap the toolbar; wait for smoke_geom to reflow.
  sleep 0.5

  play_local="$(python3 - "$geom" <<'PY'
import json,sys
p=json.load(open(sys.argv[1]))["play"]
print(int(p["x"]+p["w"]/2), int(p["y"]+p["h"]/2))
PY
)"
  read -r play_x play_y <<<"$play_local"
  play_hit=0
  # Demonstrated XFCE/Xfwm CSD only: Play's GPUI bounds can sit
  # frame_top below the visible pixels. This is not a general WM
  # transform. Ruler/tracks stay on raw smoke_geom. Skip the retry
  # when extents top is 0 so the same pixel is not clicked twice.
  play_adjs=(0)
  if [[ "${frame_top:-0}" -gt 0 ]]; then
    play_adjs+=("-$frame_top")
  fi
  for adj in "${play_adjs[@]}"; do
    try_y=$((play_y + adj))
    if [[ "$try_y" -lt 0 || "$try_y" -ge "$HEIGHT" ]]; then
      continue
    fi
    play_mark="$(log_bytes)"
    echo "click Play at client ${client_x},${client_y} + smoke_geom ${play_x},${try_y} (geom center ${play_y}, adj=${adj})"
    click_client "$play_x" "$try_y"
    sleep 0.4
    if log_suffix_has "$play_mark" 'semantic_state .*\"reason\":\"play\"'; then
      play_hit=1
      echo "Play hit (reason=play) on verified client bounds adj=${adj}"
      break
    fi
  done
  if [[ "$play_hit" -ne 1 ]]; then
    fail "standalone Play click (reason=play) missed smoke_geom on verified xwininfo client bounds"
  fi
  before_playhead="$(json_field "$state" "playhead")"
  before_locus="$(python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); print((s.get("locus") or {}).get("id",""))' "$state")"
  echo "after Play: playhead=$before_playhead locus=$before_locus"

  drag="$(python3 - "$geom" <<'PY'
import json,sys
ruler=json.load(open(sys.argv[1])).get("ruler")
if not ruler:
    raise SystemExit("no ruler bounds in smoke_geom")
print(int(ruler["x"]+ruler["w"]*0.20), int(ruler["y"]+ruler["h"]/2), int(ruler["x"]+ruler["w"]*0.80))
PY
)"
  read -r from_x rail_y to_x <<<"$drag"
  [[ "$from_x" -ge 0 && "$from_x" -lt "$WIDTH" ]] || fail "ruler from-x $from_x outside ${WIDTH}x${HEIGHT}"
  [[ "$to_x" -ge 0 && "$to_x" -lt "$WIDTH" ]] || fail "ruler to-x $to_x outside ${WIDTH}x${HEIGHT}"
  [[ "$rail_y" -ge 0 && "$rail_y" -lt "$HEIGHT" ]] || fail "ruler y $rail_y outside ${WIDTH}x${HEIGHT}"
  echo "scrub-drag client ${client_x},${client_y} + ruler ${from_x},${rail_y} -> ${to_x},${rail_y}"
  scrub_mark="$(log_bytes)"
  xdotool windowactivate --sync "$win"
  xdotool windowfocus --sync "$win" || true
  xdotool mousemove --sync $((client_x + from_x)) $((client_y + rail_y))
  sleep 0.08
  xdotool mousedown 1
  sleep 0.15
  xdotool mousemove --sync $((client_x + to_x)) $((client_y + rail_y))
  sleep 0.15
  xdotool mouseup 1
  wait_log_since "$scrub_mark" 'semantic_state .*\"reason\":\"timeline-pointer-begin\"' "in-flight timeline-pointer-begin" 6
  # Fail-closed: a miss that never commits must not print LINUX SMOKE OK.
  wait_log_since "$scrub_mark" 'semantic_state .*\"reason\":\"timeline-pointer-commit\"' "timeline-pointer-commit" 6
  after_playhead="$(json_field "$state" "playhead")"
  commit_playhead="$(python3 "$window_py" playhead --log "$log" --offset "$scrub_mark" --reason timeline-pointer-commit)"
  duration="$(json_field "$state" "duration")"
  echo "after scrub: playhead=$after_playhead commit=$commit_playhead duration=$duration"
  # Drag ends at 80% of the ruler. Playback after Play is not evidence:
  # a missed drag still advances the playhead while playing.
  python3 "$window_py" ruler-commit --playhead "$commit_playhead" --duration "$duration" \
    || fail "timeline-pointer-commit playhead $commit_playhead is not a ruler-target commit (duration=$duration); playback drift is not evidence"

  clip="$(python3 - "$geom" <<'PY'
import json,sys
g=json.load(open(sys.argv[1]))
tracks=g.get("tracks") or []
track=next((t for t in tracks if t.get("name")=="Video"), tracks[0] if tracks else None)
if track is None:
    raise SystemExit("no timeline track bounds in smoke_geom")
print(int(track["x"]+track["w"]*0.55), int(track["y"]+track["h"]/2), track["name"])
PY
)"
  read -r clip_x clip_y track_name <<<"$clip"
  echo "click $track_name clip at client ${clip_x},${clip_y} (locus transition)"
  click_client "$clip_x" "$clip_y"
  after_locus="$before_locus"
  locus_deadline=$((SECONDS + 6))
  while (( SECONDS < locus_deadline )); do
    after_locus="$(python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); print((s.get("locus") or {}).get("id",""))' "$state")"
    if [[ -n "$after_locus" && "$after_locus" != "$before_locus" ]]; then
      break
    fi
    sleep 0.1
  done
  echo "after clip click: locus=$after_locus (was $before_locus)"
  if [[ -z "$after_locus" ]]; then
    fail "locus missing after timeline clip click"
  fi
  if [[ "$after_locus" == "$before_locus" ]]; then
    fail "locus did not transition after verified Video-track click ($after_locus)"
  fi

  scene_deadline=$((SECONDS + 6))
  while (( SECONDS < scene_deadline )); do
    if python3 - "$geom" <<'PY'
import json,sys
g=json.load(open(sys.argv[1]))
raise SystemExit(0 if any(n.get("kind")=="scene" for n in (g.get("tree") or [])) else 1)
PY
    then
      break
    fi
    sleep 0.1
  done
  scene="$(python3 - "$geom" <<'PY'
import json,sys
g=json.load(open(sys.argv[1]))
nodes=g.get("tree") or []
scene=next((n for n in nodes if n.get("kind")=="scene"), None)
if scene is None:
    raise SystemExit("no SEQUENCE scene bounds in smoke_geom")
print(int(scene["x"]+scene["w"]/2), int(scene["y"]+scene["h"]/2), scene.get("id",""), scene.get("label",""))
PY
)"
  read -r scene_x scene_y scene_id scene_label <<<"$scene"
  echo "click SEQUENCE scene $scene_id ($scene_label) at client ${scene_x},${scene_y}"
  scene_mark="$(log_bytes)"
  click_client "$scene_x" "$scene_y"
  scene_deadline=$((SECONDS + 6))
  while (( SECONDS < scene_deadline )); do
    after_locus="$(python3 -c 'import json,sys; s=json.load(open(sys.argv[1])); print((s.get("locus") or {}).get("id",""))' "$state")"
    if [[ "$after_locus" == "$scene_id" ]] && log_suffix_has "$scene_mark" 'semantic_state .*\"reason\":\"tree-select\"'; then
      break
    fi
    sleep 0.1
  done
  echo "after SEQUENCE scene click: locus=$after_locus (want $scene_id)"
  if [[ "$after_locus" != "$scene_id" ]]; then
    fail "SEQUENCE scene click did not select $scene_id (locus=$after_locus)"
  fi
  # Let SEQUENCE / VEL / Inspector paint the scene selection before capture.
  sleep 0.6
  capture_window "$shot_after"
  assert_nonblank "$shot_after"
  echo "after-screenshot $shot_after"
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
tail -n 60 "$log" || true

log_text="$(cat "$log")"
if grep -Eq 'PANIC|panicked at|fatal runtime error|NoSupportedDeviceFound' <<<"$log_text"; then
  fail "Studio log contains a panic, fatal runtime error, or NoSupportedDeviceFound"
fi
if grep -q "semantic_state write failed" <<<"$log_text"; then
  fail "LATTICE_STUDIO_STATE write failed"
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
if [[ "$Interact" -eq 1 ]]; then
  if ! grep -q 'semantic_state .*\"reason\":\"play\"' <<<"$log_text"; then
    fail "missing standalone Play click effect (reason=play)"
  fi
  if ! grep -q 'semantic_state .*\"reason\":\"timeline-pointer-begin\"' <<<"$log_text"; then
    fail "missing in-flight timeline-pointer-begin"
  fi
  if ! grep -q 'semantic_state .*\"reason\":\"timeline-pointer-commit\"' <<<"$log_text"; then
    fail "missing timeline-pointer-commit"
  fi
fi
if ! grep -q "smoke quit" <<<"$log_text"; then
  fail "missing smoke quit (fail-closed; a killed process must not greenwash a miss)"
fi

echo ""
echo "LINUX SMOKE OK fixture=$Fixture display=$DISPLAY path=$display_path pid=$pid shot=$shot log=$log"
exit 0
