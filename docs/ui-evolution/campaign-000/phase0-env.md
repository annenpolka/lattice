# UI Evolution Campaign 000 — Phase 0 Environment Proof

**Linear:** CHI-95  
**Recorded:** 2026-09-03T06:25Z (UTC)  
**Agent run:** `bc-a61b1663-0168-46e5-b097-c7268bc02129`  
**Outcome:** **SUCCESS (A)** — Studio GUI visible, screenshots + semantic_state via CCA Linux smoke.

## Starting ref

| Field | Value |
|---|---|
| Branch | `main` |
| Expected SHA | `de789601c458537263caf9802cc6e421156f92fb` |
| Verified `origin/main` | `de789601c458537263caf9802cc6e421156f92fb` (match) |

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

**Primary smoke harness (CHI-95 critical path):**

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture timeline-basic
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture drag-valid
```

References: `docs/studio-linux-smoke.md`, `fixtures/studio-ui/`, `scripts/studio-linux-smoke.sh`, `crates/lattice-studio/src/main.rs` (window bounds `1400×840`).

## Fixture / project to reach usable timeline

| Item | Value |
|---|---|
| Fixture source | `fixtures/studio-ui/` (deterministic, no generated media) |
| timeline-basic | One scene, title + callout, idle playhead at `0s`, duration `4s` |
| drag-valid | Two title scenes; canvas/timeline move has a legal target |
| Materialized VEL | `/tmp/lattice-studio-ui-fixtures/<fixture>/main.vel` |
| Media resolve | Not required for UI-only smoke (`LATTICE_STUDIO_PREVIEW=0`) |

Windows/macOS dogfood is **not** the Phase 0 path.

## Phase 0 run results

| Gate | timeline-basic | drag-valid |
|---|---|---|
| Build | **PASS** (0.30s) | **PASS** (0.26s) |
| Launch | **PASS** (`open_window ok`, `first paint`) | **PASS** |
| GUI visible | **PASS** (1400×840 client) | **PASS** |
| Screenshot | **PASS** (77959 B, 249 unique colors) | **PASS** (83858 B, 237 unique colors) |
| semantic_state | **PASS** (`reason=first-paint`) | **PASS** (`reason=first-paint`) |
| Interact | **PASS** (Play / scrub / clip / tree) | **PASS** |
| Exit | `LINUX SMOKE OK` | `LINUX SMOKE OK` |

## Log / artifact paths (CHI-95 run)

**timeline-basic** (`run.vw3j13`)

| Artifact | Path |
|---|---|
| Screenshot (uploaded) | `/opt/cursor/artifacts/chi95_phase0_timeline_basic.png` |
| Screenshot (run dir) | `/workspace/target/studio-linux-smoke/run.vw3j13/studio.png` |
| After-interact | `/workspace/target/studio-linux-smoke/run.vw3j13/studio-after.png` |
| Studio log | `/workspace/target/studio-linux-smoke/run.vw3j13/studio.log` |
| semantic_state | `/workspace/target/studio-linux-smoke/run.vw3j13/studio.state.json` |
| Transcript | `/tmp/phase0-chi95-timeline-basic.log` |

**drag-valid** (`run.6troZ6`)

| Artifact | Path |
|---|---|
| Screenshot (uploaded) | `/opt/cursor/artifacts/chi95_phase0_drag_valid.png` |
| Screenshot (run dir) | `/workspace/target/studio-linux-smoke/run.6troZ6/studio.png` |
| After-interact | `/workspace/target/studio-linux-smoke/run.6troZ6/studio-after.png` |
| Studio log | `/workspace/target/studio-linux-smoke/run.6troZ6/studio.log` |
| semantic_state | `/workspace/target/studio-linux-smoke/run.6troZ6/studio.state.json` |
| Transcript | `/tmp/phase0-chi95-drag-valid.log` |

## semantic_state snapshots (first-paint)

**timeline-basic:**
```json
{"locus":{"id":"demo:title:1","kind":"title","label":"Hello"},"playhead":"0s","duration":"4s","playing":false,"interaction":"idle","fixture":"timeline-basic","reason":"first-paint"}
```

**drag-valid:**
```json
{"locus":{"id":"left:title:1","kind":"title","label":"Alpha"},"playhead":"0s","duration":"4s","playing":false,"interaction":"idle","fixture":"drag-valid","reason":"first-paint"}
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
