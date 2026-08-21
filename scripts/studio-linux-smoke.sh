#!/usr/bin/env bash
# Agent/Linux Studio smoke. Not a product Linux target and not CHI-63.
#
# Demonstrated CHI-64 path: an existing X11 session (Cursor Cloud XFCE is
# DISPLAY=:1). Xvfb is an explicit fallback, not that path.
#
# Builds lattice-studio, launches --ui-fixture with preview/audio detached,
# waits for open_window ok / first paint, captures the identified Studio
# window (not ${DISPLAY}.0), asserts nonblank pixels, then optionally
# clicks Play and scrub-drags using app-emitted smoke_geom
# (play / ruler / rail / tracks / canvas). debug_selector is a test-only
# no-op in the product binary and is not used here.
#
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture drag-valid
#   DISPLAY=:1 ./scripts/studio-linux-smoke.sh --no-interact
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

usage() {
  cat <<'EOF'
Usage: ./scripts/studio-linux-smoke.sh [options]

  --fixture NAME   timeline-basic | drag-valid | drag-invalid | dense-project
  --no-interact    skip OS click/drag (still requires a visible nonblank window)
  --allow-xvfb     start Xvfb if DISPLAY is unset (not the CHI-64 demonstrated path)
  --release        cargo build --release
  --smoke-ms N     LATTICE_STUDIO_SMOKE_MS watchdog (default 25000)
  --wait-seconds N process wait budget (default smoke-ms/1000 + 20)
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
  need_cmd ffmpeg ffmpeg
  need_cmd python3 python3
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

preflight_packages
maybe_set_gcc_linker

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

if [[ -z "${WAYLAND_DISPLAY:-}" ]]; then
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

stamp="$(date +%Y%m%d-%H%M%S)"
out="${LATTICE_STUDIO_SMOKE_DIR:-$target_root/studio-linux-smoke}"
mkdir -p "$out"
log="$out/studio-linux-smoke-$stamp.log"
stdout="$out/studio-linux-smoke-$stamp.stdout.log"
stderr="$out/studio-linux-smoke-$stamp.stderr.log"
state="$out/studio-linux-smoke-$stamp.state.json"
geom="$out/studio-linux-smoke-$stamp.geom.json"
shot="$out/studio-linux-smoke-$stamp.png"
shot_after="$out/studio-linux-smoke-$stamp-after.png"

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
for _ in $(seq 1 40); do
  win="$(xdotool search --pid "$pid" --name 'Lattice Studio' 2>/dev/null | head -n 1 || true)"
  if [[ -z "$win" ]]; then
    win="$(xdotool search --name 'Lattice Studio' 2>/dev/null | head -n 1 || true)"
  fi
  if [[ -n "$win" ]]; then
    break
  fi
  sleep 0.25
done
[[ -n "$win" ]] || fail "xdotool could not find a Lattice Studio window"
xdotool windowactivate --sync "$win"
xdotool windowfocus --sync "$win" || true
sleep 0.2
eval "$(xdotool getwindowgeometry --shell "$win")"
[[ "${WIDTH:-0}" -gt 200 && "${HEIGHT:-0}" -gt 200 ]] || fail "Studio window geometry is implausible: ${WIDTH:-?}x${HEIGHT:-?}"
# xdotool geometry includes the WM frame. GPUI widget bounds are client-local.
# _NET_FRAME_EXTENTS is left,right,top,bottom.
extents="$(xprop -id "$win" _NET_FRAME_EXTENTS 2>/dev/null | sed -n 's/.* = //p')"
frame_left=0
frame_right=0
frame_top=0
frame_bottom=0
if [[ -n "$extents" ]]; then
  IFS=', ' read -r frame_left frame_right frame_top frame_bottom <<<"$extents"
fi
client_x=$((X - frame_left))
client_y=$((Y - frame_top))
echo "window id=$win frame=${X},${Y} ${WIDTH}x${HEIGHT} extents=${frame_left},${frame_right},${frame_top},${frame_bottom} client=${client_x},${client_y}"
printf '%s\n' "{\"id\":\"$win\",\"x\":$X,\"y\":$Y,\"w\":$WIDTH,\"h\":$HEIGHT,\"client_x\":$client_x,\"client_y\":$client_y,\"frame_top\":$frame_top,\"display\":\"$DISPLAY\"}" >"$out/studio-linux-smoke-$stamp.window.json"

capture_window() {
  local dest="$1"
  ffmpeg -y -hide_banner -loglevel error \
    -f x11grab -video_size "${WIDTH}x${HEIGHT}" -i "${DISPLAY}+${X},${Y}" \
    -frames:v 1 "$dest"
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
echo "screenshot $shot ($(wc -c <"$shot") bytes) window=${WIDTH}x${HEIGHT}+${X}+${Y}"
assert_nonblank "$shot"

json_field() {
  python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get(sys.argv[2],""))' "$1" "$2"
}

wait_log() {
  local pattern="$1"
  local label="$2"
  local budget="${3:-8}"
  local until=$((SECONDS + budget))
  while (( SECONDS < until )); do
    if [[ -f "$log" ]] && grep -q "$pattern" "$log"; then
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
  [[ "$lx" -ge 0 && "$lx" -le "$WIDTH" ]] || fail "client X $lx outside ${WIDTH}x${HEIGHT}"
  [[ "$ly" -ge 0 && "$ly" -le "$HEIGHT" ]] || fail "client Y $ly outside ${WIDTH}x${HEIGHT}"
  xdotool windowactivate --sync "$win"
  xdotool mousemove --sync "$sx" "$sy"
  sleep 0.05
  xdotool mousedown 1
  sleep 0.05
  xdotool mouseup 1
}

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
  # GPUI toolbar bounds can include the CSD/title offset. Try the recorded
  # center, then the same point minus the verified frame top.
  for adj in 0 "-${frame_top}"; do
    try_y=$((play_y + adj))
    if [[ "$try_y" -lt 0 || "$try_y" -gt "$HEIGHT" ]]; then
      continue
    fi
    echo "click Play at client ${play_x},${try_y} (geom center ${play_y}, adj=${adj})"
    click_client "$play_x" "$try_y"
    sleep 0.35
    if grep -q 'semantic_state .*\"reason\":\"play\"' "$log"; then
      play_hit=1
      break
    fi
  done
  if [[ "$play_hit" -ne 1 ]]; then
    fail "standalone Play click (reason=play) missed recorded smoke_geom and frame-adjusted points"
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
  echo "scrub-drag client ${from_x},${rail_y} -> ${to_x},${rail_y} (ruler)"
  xdotool mousemove --sync $((client_x + from_x)) $((client_y + rail_y))
  sleep 0.05
  xdotool mousedown 1
  sleep 0.15
  xdotool mousemove --sync $((client_x + to_x)) $((client_y + rail_y))
  sleep 0.15
  xdotool mouseup 1
  wait_log 'semantic_state .*\"reason\":\"timeline-pointer-begin\"' "in-flight timeline-pointer-begin" 6
  # Fail-closed: a miss that never commits must not print LINUX SMOKE OK.
  wait_log 'semantic_state .*\"reason\":\"timeline-pointer-commit\"' "timeline-pointer-commit" 6
  after_playhead="$(json_field "$state" "playhead")"
  echo "after scrub: playhead=$after_playhead"
  if [[ "$after_playhead" == "$before_playhead" ]]; then
    fail "playhead did not change after verified-ruler scrub ($before_playhead)"
  fi

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
  echo "WARN: missing smoke quit (process was stopped by the script after the window was observed)"
fi

echo ""
echo "LINUX SMOKE OK fixture=$Fixture display=$DISPLAY path=$display_path pid=$pid shot=$shot log=$log"
exit 0
