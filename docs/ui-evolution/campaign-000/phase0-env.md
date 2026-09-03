# UI Evolution Campaign 000 — Phase 0 Environment Proof

**Recorded:** 2026-09-03T06:23Z (UTC)  
**Agent run:** `bc-a61b1663-0168-46e5-b097-c7268bc02129`  
**Outcome:** **SUCCESS (A)** — Studio GUI visible, screenshot captured, build OK.

## Starting ref

| Field | Value |
|---|---|
| Branch | `main` |
| Full SHA | `de789601c458537263caf9802cc6e421156f92fb` |

## Cursor Cloud environment

| Field | Value |
|---|---|
| Build ID | `bld-20260902-a535ed0b-2a28-406d-b7af-1133cb188ff7` |
| Environment ID | `35cc6dd6-9d43-11f1-a7d1-d6b4613131ce` |
| Display | `DISPLAY=:1` (existing XFCE/Xfwm X11 session) |
| Vulkan ICD | Mesa lavapipe (`/usr/share/vulkan/icd.d/lvp_icd.json`); `VK_ICD_FILENAMES` unset |
| Desktop resolution | 1920×1200 (session); Studio client 1400×840 |

## Build / launch (documented paths)

**Build (workspace crate):**

```bash
cargo build -p lattice-studio --features window
```

**Direct launch (Linux agent fixture entry):**

```bash
DISPLAY=:1 \
  LATTICE_STUDIO_PREVIEW=0 \
  LATTICE_STUDIO_AUDIO_MONITOR=0 \
  LATTICE_STUDIO_RENDERER=cpu \
  /workspace/target/debug/lattice-studio --ui-fixture timeline-basic
```

**Smoke harness (used for this proof):**

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture timeline-basic --no-interact
```

References: `docs/studio-linux-smoke.md`, `scripts/studio-linux-smoke.sh`, `crates/lattice-studio/src/main.rs` (window bounds `1400×840`).

## Fixture / project to reach usable timeline

| Item | Value |
|---|---|
| Fixture | `--ui-fixture timeline-basic` |
| Materialized VEL | `/tmp/lattice-studio-ui-fixtures/timeline-basic/main.vel` |
| Intent | One scene, title + callout, idle playhead at `0s`, duration `4s` |
| Media resolve | Not required for UI-only smoke (`LATTICE_STUDIO_PREVIEW=0`) |

Windows dogfood path (unchanged): `lattice-studio path\to\main.vel` with resolved gameplay-commentary fixture.

## Phase 0 run results

| Gate | Result | Notes |
|---|---|---|
| Build | **PASS** | `cargo build -p lattice-studio --features window` — 1.58s (prebuilt workspace) |
| Launch | **PASS** | `open_window ok`, `first paint`, `LINUX SMOKE OK` |
| GUI visible | **PASS** | Window XID `27262977`, client `1400×840+1+57`, title area shows `Lattice main.vel · Scene demo` |
| Screenshot | **PASS** | Nonblank PNG: 249 unique colors, 77959 bytes |
| Input (optional) | skipped | `--no-interact` for Phase 0; full interact path available in smoke script |

## Log / artifact paths (this run)

| Artifact | Path |
|---|---|
| Screenshot (uploaded) | `/opt/cursor/artifacts/phase0_studio_timeline_basic.png` |
| Screenshot (run dir) | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.png` |
| Studio log | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.log` |
| stdout | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.stdout.log` (empty) |
| stderr / trace | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.stderr.log` |
| semantic_state | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.state.json` |
| smoke_geom | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.geom.json` |
| window identity | `/workspace/target/studio-linux-smoke/run.8BvSN1/studio.window.json` |
| Smoke transcript | `/tmp/phase0-smoke-run.log` |

## Initial semantic state (timeline-basic)

```json
{"locus":{"id":"demo:title:1","kind":"title","label":"Hello"},"playhead":"0s","duration":"4s","playing":false,"interaction":"idle","gesture":{"kind":"none"},"drag":null,"fixture":"timeline-basic","reason":"first-paint"}
```

## STOP conditions (not triggered)

- Display missing → **not applicable** (`DISPLAY=:1` present)
- Build fail → **not triggered**
- Missing deps → **not triggered** (mesa-vulkan, gcc, ffmpeg, xdotool, x11-utils preflight OK)
- `NoSupportedDeviceFound` → **not triggered** (lavapipe ICD sufficient)
- Xvfb fallback → **not used** (documented fallback: `./scripts/studio-linux-smoke.sh --allow-xvfb`)

## Blockers for later phases

None for GUI presence. Known constraints for campaign planning:

- Linux path is agent-enabling smoke only (`docs/studio-linux-smoke.md`); product dogfood remains Windows 11 / macOS.
- Preview/audio detached in smoke (`LATTICE_STUDIO_PREVIEW=0`, `LATTICE_STUDIO_AUDIO_MONITOR=0`).
- Window size is fixed at launch (`1400×840`); not user-resizable via CLI flag.
- Computer-use / interact clicks require `smoke_geom` + verified `xwininfo` client bounds (see CHI-66/67 docs).
