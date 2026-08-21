# Linux Studio smoke (agent enabling path)

This is **not** product Linux support. Parent CHI-19 still holds: Studio dogfood is Windows 11 x64. CHI-54 only needs Ubuntu agents — including Cursor Cloud — to close `implement → launch → interact → screenshot → inspect semantic state`.

CHI-63 (`VisualTestContext` / `UiDriver`) stays the later reduction of computer-use for ordinary button and drag correctness. This document is the OS-boundary computer-use / process smoke. Do not reimplement CHI-63 here.

Do not mark CHI-54 / CHI-64 / CHI-65 / CHI-66 / CHI-67 Done from this document alone.

## What the path is

```text
cargo build -p lattice-studio --features window
lattice-studio --ui-fixture timeline-basic
```

`--ui-fixture` materializes an ordinary VEL document and opens it through `StudioSession`. It does not invent a second project model.

| Fixture | Intent |
|---|---|
| `timeline-basic` | One scene, title + callout, idle playhead |
| `drag-valid` | Two title scenes; canvas/timeline move has a legal target |
| `drag-invalid` | One short title pinned at `0s`; left-edge trim has no legal room |
| `dense-project` | Four scenes, each with title + callout |

Windows launch is unchanged: `lattice-studio path\to\main.vel` and `scripts/studio-debug.ps1` / `scripts/studio-smoke.ps1`. Those scripts still pass a single VEL path as the process argument. `--ui-fixture` is an additional Linux-agent entry; it does not replace the VEL-path launch.

## Environment (CHI-64)

CHI-54's display target is **X11**. The demonstrated Cursor Cloud path is an already-running XFCE/Xfwm session:

```bash
DISPLAY=:1
```

`scripts/studio-linux-smoke.sh` does **not** start Xvfb when `DISPLAY` is set. Xvfb is a labeled fallback only (`--allow-xvfb` when `DISPLAY` is unset). An Xvfb run is not a substitute for the `DISPLAY=:1` demonstration.

GPUI 0.2 opens a real window and expects Vulkan. `LATTICE_STUDIO_PREVIEW=0` and `LATTICE_STUDIO_RENDERER=cpu` skip live frame extract / select the CPU compositor. They do **not** skip Blade / wgpu window init. `NoSupportedDeviceFound` is fatal.

On a machine without a hardware ICD, install Mesa's software implementation:

```bash
sudo apt-get install -y mesa-vulkan-drivers
```

After that package, `/usr/share/vulkan/icd.d/` contains lavapipe (`lvp_icd.json`). The window opens **without** setting `VK_ICD_FILENAMES`. The smoke script does not auto-export that variable. Leave a caller-supplied value alone.

Do **not** gate CHI-64 on `vulkaninfo`. It can fail with `X_CreateWindow BadMatch` while GPUI still opens a window through lavapipe.

The script preflights the ICD directory and fails with the `mesa-vulkan-drivers` install line when it is empty. It does not run `vulkaninfo`.

## Clang / libstdc++ linker (not CHI-64)

Cloud VMs often ship `cc` as clang. `rustc` then asks that clang driver to `-lstdc++`, and the link fails even when `g++` and `libstdc++-*-dev` are installed.

The smoke script — not Cargo.toml, not `.cargo/config.toml` — sets:

```bash
RUSTFLAGS=-C linker=gcc
```

when default `cc` looks like clang and `gcc` exists. If `gcc` is missing, it fails and names `g++` / `gcc`. If `RUSTFLAGS` already contains `-C linker=`, the script leaves it alone.

This is **not** a CHI-64 Done criterion. Chief is filing a separate child. Do not bake the linker into workspace Cargo config.

## Pinned agent packages

`.cursor/environment.json` pins the packages the next Ubuntu agent must have so the smoke path does not regress:

- `mesa-vulkan-drivers`
- `libxkbcommon-dev` / `libxkbcommon-x11-dev` (`libxkbcommon*`)
- `g++` (pulls `gcc` / libstdc++)
- `xvfb`
- `xdotool`
- `x11-utils` (`xprop` / `xwininfo` for PID window identity and client bounds)
- `ffmpeg`

`libxcb1-dev` is included next to `libxkbcommon*` because Linux CI already needs it to link GPUI. The smoke script preflights the same set and prints the apt line when one is missing.

UI-only smoke detaches media backends:

```text
LATTICE_STUDIO_PREVIEW=0
LATTICE_STUDIO_AUDIO_MONITOR=0
LATTICE_STUDIO_RENDERER=cpu
```

Audio monitoring is Windows-only; the Linux stub already returns `UnsupportedPlatform`. Preview extract is optional and is the usual launch blocker when FFmpeg/media is missing.

## Observable semantic state (CHI-66)

Each launch writes `semantic_state {json}` lines to the durable Studio log (`LATTICE_STUDIO_LOG`). When `LATTICE_STUDIO_STATE` is set, the latest snapshot is also written as pretty JSON. A write failure is logged as `semantic_state write failed` and fails the smoke. The env var being unset is a no-op.

The snapshot is a hook over existing session fields:

- current locus (`id` / `kind` / `label`) or `null`
- focused entity when known (`studio`, `vel.editor`, `inspector.title`), including on commit snapshots
- playhead, duration, and playing flag
- active interaction/mode
- drag source / target / validity while a gesture is in flight

It is emitted on `open`, `first-paint`, Play, timeline pointer begin/update/commit, and canvas drag/resize begin/update/commit. Begin/update lines exist so source/target/validity is visible before commit resets `gesture` to `none`. It is not a permanent on-canvas debug HUD.

Widget bounds used by the OS smoke are written to `LATTICE_STUDIO_GEOM` as `smoke_geom` JSON: `play`, `ruler`, `rail`, `tracks`, and `canvas` when CHI-66 has measured them. Those are window-local GPUI pixels; the script adds the verified `xwininfo` client origin (not `xdotool getwindowgeometry`).

`debug_selector` is a test-only no-op in the product binary. The OS smoke must not depend on it. Drive click/drag coordinates from the app-emitted geoms only.

Window identity is PID / `_NET_WM_PID` (CHI-82). `xdotool search --name` is not a fallback. `xdotool getwindowgeometry` is decoration-inflated and origin-ambiguous; the script records verified `xwininfo` client bounds plus `_NET_FRAME_EXTENTS` and uses those with `smoke_geom`.

## Reproducing the smoke

Documented CHI-64 / CHI-67 command (this Cloud VM):

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture timeline-basic
```

Other forms:

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture drag-valid
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --no-interact
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --miss-commit   # CHI-67 negative; must FAIL
DISPLAY=:1 ./scripts/studio-linux-smoke-miss.sh            # records docs/artifacts/chi67-miss-commit.log
./scripts/studio-linux-smoke.sh --allow-xvfb   # fallback only; not the demonstrated path
```

A populated `WAYLAND_DISPLAY` is not a silent X11 path. The script fails unless `WAYLAND_DISPLAY` is unset or `--allow-wayland-x11` is passed (labeled X11-under-Wayland). Missing `smoke quit` fails; it is not a WARN that can greenwash a miss.

The script:

1. Preflights `g++` / `gcc`, `libxkbcommon` headers, a Vulkan ICD directory, `xdotool`, `xprop` / `xwininfo`, `ffmpeg`, and `python3`.
2. If `cc` is clang, sets `RUSTFLAGS=-C linker=gcc` (not Cargo.toml).
3. Builds `lattice-studio`.
4. Requires an existing `DISPLAY` unless `--allow-xvfb` is passed.
5. Launches `--ui-fixture` with preview/audio detached and a smoke watchdog.
6. Waits for `open_window ok` and `first paint`.
7. Identifies the Studio window by process PID / `_NET_WM_PID` (never title substring) and captures **that client area**, not `${DISPLAY}.0`.
8. Asserts the PNG is a nonblank Studio frame (color diversity / contrast), not merely a non-empty file.
9. With interact: clicks Play from `smoke_geom` (must emit `reason=play`), scrub-drags the ruler (must emit begin + **commit** and move the playhead), then clicks the Video clip (must change locus). Percent positions are fractions of verified widget bounds, then offset by the verified `xwininfo` client origin. Missing `timeline-pointer-commit` **fails** the script. `--miss-commit` deliberately clicks off-widget and must exit nonzero with no `LINUX SMOKE OK`. Missing `smoke quit` also fails.

Artifacts stay under `target/studio-linux-smoke/` (gitignored with the rest of `target/`). PR-visible evidence is copied to `docs/screenshots/`.

## Spike results (Cursor Cloud Ubuntu)

Recorded on the CHI-54 implementing agent (`DISPLAY=:1`, XFCE/Xfwm 1920×1200, Mesa lavapipe ICD `/usr/share/vulkan/icd.d/lvp_icd.json` present; `VK_ICD_FILENAMES` is not required).

| Gate | Result | Notes |
|---|---|---|
| build | pass | `cargo build -p lattice-studio --features window`. Clang `cc` needs `RUSTFLAGS=-C linker=gcc` (script, not CHI-64). |
| launch | pass | `open_window ok`; `LATTICE_STUDIO_PREVIEW=0` and `LATTICE_STUDIO_AUDIO_MONITOR=0` |
| visible | pass | Window title `Lattice Studio · CPU`; sequence / Canvas / VEL / Inspector / Timeline all draw |
| screenshot | pass | Window-cropped `ffmpeg -f x11grab` of the identified Studio window; nonblank-pixel check |
| input | in script | Fail-closed xdotool using `smoke_geom` + verified client bounds. Manual Computer Use is supporting evidence only |

Classification notes:

- **Environment:** `vulkaninfo --summary` can fail with `X_CreateWindow BadMatch`. That is a WSI probe issue, not Studio. Do not gate on it. GPUI still opened a window through lavapipe after `mesa-vulkan-drivers`.
- **App platform-coupling (fixed in this path):** a title-only VEL cannot flatten (`timeline has no video clip`), so fixtures include a `media` + video clip in Core IR. The media file itself is not required for UI-only smoke.
- **CHI-64 remainder:** a human still needs to look at the committed Studio-window PNGs. Process start + PNG byte count is not enough. Lavapipe is proven for window init; it is not a product GPU path.
- **CHI-67 remainder:** this-head rerun and the `--miss-commit` negative path are the remaining Linux-agent evidence. Missing commit / missing `smoke quit` are fail-closed. CHI-63 UiDriver is not reimplemented here. Not a Windows closer.

Initial fixture semantic state (stable across opens; see `UiFixture::expected_initial`):

```json
{"locus":{"id":"demo:title:1","kind":"title","label":"Hello"},"playhead":"0s","duration":"4s","playing":false,"interaction":"idle","drag":null}
```

## Non-goals

No Linux packaging, installer, macOS, WSL-as-supported-runtime, Linux UX polish, realtime playback, hardware encode/decode, GPU renderer foundation, or in-process LLM SDK.
