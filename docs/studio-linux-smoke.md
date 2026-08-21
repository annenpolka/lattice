# Linux Studio smoke (agent enabling path)

This is **not** product Linux support. Parent CHI-19 still holds: Studio dogfood is Windows 11 x64. CHI-54 only needs Ubuntu agents — including Cursor Cloud — to close `implement → launch → interact → screenshot → inspect semantic state`.

CHI-63 (`VisualTestContext` / `UiDriver`) stays the later reduction of computer-use for ordinary button and drag correctness. This document is the OS-boundary computer-use / process smoke.

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

Windows launch is unchanged: `lattice-studio path\to\main.vel` and `scripts/studio-debug.ps1` / `scripts/studio-smoke.ps1`.

## Environment

The agent VM needs an X11 `DISPLAY`. A running XFCE/Xfwm session is enough. If `DISPLAY` is unset, `scripts/studio-linux-smoke.sh` starts Xvfb.

GPUI 0.2 opens a real window and expects Vulkan. On a machine without a hardware ICD, install Mesa's software implementation (`mesa-vulkan-drivers`) and point `VK_ICD_FILENAMES` at `lvp_icd.x86_64.json`. The smoke script does this when the file exists and also sets `LIBGL_ALWAYS_SOFTWARE=1`.

Link-time packages already required by Linux CI remain: `libxcb1-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`. A C++ linker (`g++` / `libstdc++`) is also required to link GPUI.

UI-only smoke detaches media backends:

```text
LATTICE_STUDIO_PREVIEW=0
LATTICE_STUDIO_AUDIO_MONITOR=0
LATTICE_STUDIO_RENDERER=cpu
```

Audio monitoring is Windows-only; the Linux stub already returns `UnsupportedPlatform`. Preview extract is optional and is the usual launch blocker when FFmpeg/media is missing.

## Observable semantic state

Each launch writes `semantic_state {json}` lines to the durable Studio log (`LATTICE_STUDIO_LOG`). When `LATTICE_STUDIO_STATE` is set, the latest snapshot is also written as pretty JSON.

The snapshot is a hook over existing session fields:

- current locus (`id` / `kind` / `label`) or `null`
- focused entity when a window is available (`studio`, `vel.editor`, `inspector.title`)
- playhead and playing flag
- active interaction/mode
- drag source / target / validity when a gesture is in flight

It is emitted on `open`, `first-paint`, timeline pointer commit, and canvas drag/resize commit. It is not a permanent on-canvas debug HUD.

## Reproducing the smoke

```bash
./scripts/studio-linux-smoke.sh
./scripts/studio-linux-smoke.sh --fixture drag-valid
./scripts/studio-linux-smoke.sh --no-interact
```

The script:

1. Builds `lattice-studio`.
2. Launches `--ui-fixture` with preview/audio detached and a smoke watchdog.
3. Waits for `open_window ok` and `first paint`.
4. Captures the X11 display to `target/studio-linux-smoke/*.png` via `ffmpeg -f x11grab`.
5. Optionally clicks once and performs one horizontal scrub-style drag with `xdotool`.
6. Asserts `semantic_state` lines for `open` and `first-paint`.

Artifacts stay under `target/studio-linux-smoke/` (gitignored with the rest of `target/`).

## Spike results (Cursor Cloud Ubuntu)

Recorded on the CHI-54 implementing agent (`DISPLAY=:1`, XFCE/Xfwm 1920×1200, Mesa lavapipe ICD `/usr/share/vulkan/icd.d/lvp_icd.json`).

| Gate | Result | Notes |
|---|---|---|
| build | pass | `cargo build -p lattice-studio --features window` after `libxkbcommon*-dev` + `g++`/`libstdc++` |
| launch | pass | `open_window ok`; `LATTICE_STUDIO_PREVIEW=0` and `LATTICE_STUDIO_AUDIO_MONITOR=0` |
| visible | pass | Window title `Lattice Studio · CPU`; sequence / Canvas / VEL / Inspector / Timeline all draw |
| screenshot | pass | `ffmpeg -f x11grab` artifact under `target/studio-linux-smoke/` |
| input | pass | Computer-use click on Play (`play samples` at 0s) then a timeline clip drag; playhead ended at `4s` and locus moved `demo:title:1` → `scene:demo` |

Classification notes:

- **Environment:** `vulkaninfo --summary` failed with `X_CreateWindow BadMatch`. That is a WSI probe issue, not Studio. GPUI still opened a window through lavapipe.
- **App platform-coupling (fixed in this path):** a title-only VEL cannot flatten (`timeline has no video clip`), so fixtures include a `media` + video clip in Core IR. The media file itself is not required for UI-only smoke.
- **Environment / input tool:** `xdotool` can miss the 640px timeline rail. Computer-use is the reliable agent input path; the script still tries one click + one scrub-style drag.

Initial fixture semantic state (stable across opens):

```json
{"locus":{"id":"demo:title:1","kind":"title","label":"Hello"},"playhead":"0s","playing":false,"interaction":"idle","drag":null}
```

After the agent Play click + timeline drag:

```json
{"locus":{"id":"scene:demo","kind":"scene","label":"demo"},"playhead":"4s","playing":false,"interaction":"idle","reason":"timeline-pointer-commit"}
```

## Non-goals

No Linux packaging, installer, macOS, WSL-as-supported-runtime, Linux UX polish, realtime playback, hardware encode/decode, GPU renderer foundation, or in-process LLM SDK.
