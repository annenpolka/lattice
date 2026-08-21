# Linux Studio smoke (agent enabling path)

This is **not** product Linux support. Parent CHI-19 still holds: Studio dogfood is Windows 11 x64. CHI-54 only needs Ubuntu agents — including Cursor Cloud — to close `implement → launch → interact → screenshot → inspect semantic state`.

CHI-63 (`VisualTestContext` / `UiDriver`) stays the later reduction of computer-use for ordinary button and drag correctness. This document is the OS-boundary computer-use / process smoke.

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

GPUI 0.2 opens a real window and expects Vulkan. `LATTICE_STUDIO_PREVIEW=0` skips live frame extract. It does **not** skip Blade / wgpu GPU init. On a machine without a hardware ICD, install Mesa's software implementation (`mesa-vulkan-drivers`) and point `VK_ICD_FILENAMES` at `lvp_icd.json` / `lvp_icd.x86_64.json`. The smoke script does this when the file exists and also sets `LIBGL_ALWAYS_SOFTWARE=1`. Those variables are a hypothesis that has worked on this Cloud VM; they must not hide `NoSupportedDeviceFound`. If GPU init fails, the process dies and the script fails.

Link-time packages already required by Linux CI remain: `libxcb1-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`. A C++ linker (`g++` / `libstdc++`) is also required to link GPUI.

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

Widget bounds used by the OS smoke (Play, ruler, tracks) are written to `LATTICE_STUDIO_GEOM` as `smoke_geom` JSON. Those are window-local GPUI pixels; the script adds the verified X11 window origin.

## Reproducing the smoke

Documented CHI-64 / CHI-67 command (this Cloud VM):

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture timeline-basic
```

Other forms:

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture drag-valid
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --no-interact
./scripts/studio-linux-smoke.sh --allow-xvfb   # fallback only; not the demonstrated path
```

The script:

1. Builds `lattice-studio`.
2. Requires an existing `DISPLAY` unless `--allow-xvfb` is passed.
3. Launches `--ui-fixture` with preview/audio detached and a smoke watchdog.
4. Waits for `open_window ok` and `first paint`.
5. Identifies the Studio window with xdotool and captures **that window**, not `${DISPLAY}.0`.
6. Asserts the PNG is a nonblank Studio frame (color diversity / contrast), not merely a non-empty file.
7. With interact: clicks Play from `smoke_geom` (must emit `reason=play`), scrub-drags the ruler (must emit begin + commit and move the playhead), then clicks the Video clip (must change locus). Percent positions are fractions of verified widget bounds, then offset by the verified window origin. Missing `timeline-pointer-commit` fails the script.

Artifacts stay under `target/studio-linux-smoke/` (gitignored with the rest of `target/`). PR-visible evidence is copied to `docs/screenshots/`.

## Spike results (Cursor Cloud Ubuntu)

Recorded on the CHI-54 implementing agent (`DISPLAY=:1`, XFCE/Xfwm 1920×1200, Mesa lavapipe ICD `/usr/share/vulkan/icd.d/lvp_icd.json`).

| Gate | Result | Notes |
|---|---|---|
| build | pass | `cargo build -p lattice-studio --features window` after `libxkbcommon*-dev` + `g++`/`libstdc++` |
| launch | pass | `open_window ok`; `LATTICE_STUDIO_PREVIEW=0` and `LATTICE_STUDIO_AUDIO_MONITOR=0` |
| visible | pass | Window title `Lattice Studio · CPU`; sequence / Canvas / VEL / Inspector / Timeline all draw |
| screenshot | pass | Window-cropped `ffmpeg -f x11grab` of the identified Studio window; nonblank-pixel check |
| input | in script | Fail-closed xdotool using `smoke_geom` + verified client bounds. Manual Computer Use is supporting evidence only |

Classification notes:

- **Environment:** `vulkaninfo --summary` failed with `X_CreateWindow BadMatch`. That is a WSI probe issue, not Studio. GPUI still opened a window through lavapipe.
- **App platform-coupling (fixed in this path):** a title-only VEL cannot flatten (`timeline has no video clip`), so fixtures include a `media` + video clip in Core IR. The media file itself is not required for UI-only smoke.
- **CHI-64 remainder:** a human still needs to look at the committed Studio-window PNGs. Process start + PNG byte count is not enough.
- **CHI-67 remainder:** a green percent-xdotool script without `timeline-pointer-commit` / playhead / locus assertions is not Done. CHI-63 UiDriver is not reimplemented here.

Initial fixture semantic state (stable across opens; see `UiFixture::expected_initial`):

```json
{"locus":{"id":"demo:title:1","kind":"title","label":"Hello"},"playhead":"0s","duration":"4s","playing":false,"interaction":"idle","drag":null}
```

## Non-goals

No Linux packaging, installer, macOS, WSL-as-supported-runtime, Linux UX polish, realtime playback, hardware encode/decode, GPU renderer foundation, or in-process LLM SDK.
