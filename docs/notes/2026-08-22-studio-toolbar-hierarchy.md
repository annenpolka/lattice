# Studio top bar visual hierarchy and perceptual grouping (live UI observation)

Date: 2026-08-22

This note records the direct visual and perceptual hierarchy observation of the CURRENT top-of-window chrome (`header_bar` + `actions_bar` / Toolbar) in Studio after the integrated verb-license spine shipped on `main` (#23 + #22 + #24 @ `85b589e`).

This observation focuses strictly through the **visual / hierarchy lens**: perceptual grouping, visual prominence, color weight, spatial density, and what reads as primary editorial controls versus leftover scaffolding chrome.

---

## 1. Top-of-Window Chrome Inventory

In the live application window (default 1400×840 client area), the top chrome consists of two stacked horizontal bars:

```text
┌────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Lattice  main.vel · Scene demo                                                           (header_bar)  │
├────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ [Open Video…] [Set In] [Set Out] [Split at Playhead] [Delete Selected Clip]                            │
│ Renderer · CPU ready  Audio · 48 kHz stereo active  [CPU] [GPU DX12] [Play] [Pause] [Seek] [Scrub]    │
│ [Save] [Undo] [Redo] [Resolve] [Copy locus JSON] [Gain -3 dB] [Fade] [Zoom In] [Zoom Out] (actions_bar)│
└────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Layer 1: `header_bar` (Window Identity)
- **Container**: `h(36px)`, background `PANEL` (`#141821`), bottom border `LINE` (`#2a3140`), horizontal padding `px_3` (12px).
- **Elements**:
  - **Product mark**: `"Lattice"` in bright `TEAL` (`#3dd6c6`), font weight default.
  - **Context label**: `"{file} · Scene demo"` in `MUTED` gray (`#8b95a8`), offset left by `ml_3` (12px).
- **Perceptual character**: Quiet, spacious, high-contrast branding with clear secondary project path context. Zero interactive controls.

### Layer 2: `actions_bar` (Toolbar Chrome)
- **Container**: Flex row with `flex_wrap()`, `items_center()`, `gap_1` (4px), padding `px_2` (8px) `py_1` (4px), background `PANEL` (`#141821`), bottom border `LINE` (`#2a3140`).
- **Elements in order (22 items total)**:
  1. `[Open Video…]` — Button, background `LINE` (`#2a3140`), text `TEXT` (`#e8edf5`).
  2. `[Set In]` — Button, background `LINE`, text `TEXT`.
  3. `[Set Out]` — Button, background `LINE`, text `TEXT`.
  4. `[Split at Playhead]` — Button, background `LINE`, text `TEXT`.
  5. `[Delete Selected Clip]` — Button, background `LINE`, text `TEXT`.
  6. `Renderer · <status>` — Unframed text label, text `MUTED` (`#8b95a8`) or error light-red (`#ff8f8f`), `px_2`.
  7. `Audio · <status>` — Unframed text label, text `MUTED` or error light-red, `px_2`.
  8. `[CPU]` — Button, background `TEAL` (`#3dd6c6`) when active / `LINE` when inactive, text `TEXT`.
  9. `[GPU DX12]` — Button, background `TEAL` when active / `LINE` when inactive, text `TEXT`.
  10. `[Play]` — Custom button, background ALWAYS `TEAL` (`#3dd6c6`), text `TEXT`, `px_3 py_1`.
  11. `[Pause]` — Button, background `LINE`, text `TEXT`.
  12. `[Seek]` — Button, background `LINE`, text `TEXT`.
  13. `[Scrub]` — Button, background `LINE`, text `TEXT`.
  14. `[Save]` — Button, background ALWAYS `TEAL` (`#3dd6c6`), text `TEXT`.
  15. `[Undo]` — Button, background `LINE`, text `TEXT`.
  16. `[Redo]` — Button, background `LINE`, text `TEXT`.
  17. `[Resolve]` — Button, background ALWAYS `TEAL` (`#3dd6c6`), text `TEXT`.
  18. `[Copy locus JSON]` — Button, background `LINE`, text `TEXT`.
  19. `[Gain -3 dB]` — Button, background `LINE`, text `TEXT`.
  20. `[Fade]` — Button, background `LINE`, text `TEXT`.
  21. `[Zoom In]` — Button, background `LINE`, text `TEXT`.
  22. `[Zoom Out]` — Button, background `LINE`, text `TEXT`.

---

## 2. Visual Hierarchy & Perceptual Grouping Observations

### A. The 2-Row Flex-Wrap Artifact (Spatial Instability)
At standard 1400px window width, the 22 toolbar items exceed the single-line horizontal budget. Because `actions_bar` uses `.flex_wrap()`, the bar splits into two visually distinct lines:
- **Line 1**: `[Open Video…]` through `[Scrub]` (13 elements: Project I/O, In/Out trims, Split/Delete, two status badges, Renderer mode toggles, Transport controls).
- **Line 2**: `[Save]` through `[Zoom Out]` (9 elements: Persistence, History, Asset Resolve, Agent JSON copy, Audio gain/fade step actions, Timeline zoom).

**Visual consequences**:
1. **Accidental vertical hierarchy**: `[Save]`, `[Undo]`, `[Redo]`, and `[Resolve]` visually appear on a lower tier than `[Open Video…]` and `[Play]`, suggesting they are secondary or sub-actions, even though Save and Resolve are fundamental project operations.
2. **Window-width volatility**: On narrower windows (e.g. 1024–1280px) or wider windows (1920px), the wrap breakpoint shifts dynamically, scattering functional pairs (such as `[Play]` vs `[Pause]` or `[Undo]` vs `[Redo]`) across line breaks.

### B. Color Weight & The Quad-Teal Collision
The studio UI relies on `TEAL` (`#3dd6c6`) as its single saturated accent color against dark slate (`#141821` / `#2a3140`). In the current toolbar, `TEAL` is applied unconditionally to four completely different conceptual mechanisms:

1. **Active Radio Toggle**: `[CPU]` (when CPU is chosen). Saturated teal signifies *state selection*.
2. **Primary Transport**: `[Play]`. Saturated teal signifies *transient timeline playback*.
3. **Document Persistence**: `[Save]`. Saturated teal signifies *source file write*.
4. **External Provider Generation**: `[Resolve]`. Saturated teal signifies *heavy asset generation and lock generation*.

**Perceptual consequences**:
- The user’s visual attention is pulled simultaneously to four corners of the top row (`CPU`, `Play`, `Save`, `Resolve`), with no color differentiation between passive state selection, ephemeral playback, irreversible disk writes, and external network/TTS generation.
- Transport controls lack internal cohesion: `[Play]` screams with teal brightness, while `[Pause]`, `[Seek]`, and `[Scrub]` immediately next to it fade into dark background gray (`LINE`).

### C. The Flat Linear Soup (Uniform Proximity & Missing Grouping)
Under Gestalt principles of visual grouping (Proximity and Similarity):
- Every button has identical visual treatment: dark gray background (`#2a3140`), light text (`#e8edf5`), `px_3 py_1` box, and identical 4px spacing (`gap_1`).
- There are **no visual delimiters, dividers, background chips, or category clustering**.
- Highly divergent functional domains are placed back-to-back with zero breathing room or visual boundary:
  - Destructive clip edit (`[Delete Selected Clip]`) directly abuts unframed engine status text (`Renderer · ...`).
  - Timeline fine positioning (`[Scrub]`) is visually adjacent across the wrap boundary to project file persistence (`[Save]`).
  - Developer / Agent debugging helper (`[Copy locus JSON]`) is sandwiched between compilation pipeline (`[Resolve]`) and clip audio gain (`[Gain -3 dB]`).
  - Audio parameter tweak (`[Fade]`) directly precedes Timeline viewport zoom (`[Zoom In]`).

### D. Unframed Status Text Orphans
`Renderer · CPU initializing` and `Audio · monitor explicitly disabled` are rendered as raw text elements without button backgrounds or status-pill containers.
- Sitting directly in the middle of a continuous button sequence, these text snippets look like misplaced labels or interrupted button rows.
- When renderer or audio encounters an error (`0xff8f8f` light red), the text shifts color but still lacks a distinct status-indicator container.

---

## 3. Primary Workflow vs. Leftover Scaffolding Chrome

Analyzing the 22 elements through the lens of visual affordance reveals two distinct layers of intent:

### 1. What Reads as Primary Editorial Spine
- **Project & Session Lifecycle**: `[Open Video…]`, `[Save]`, `[Undo]`, `[Redo]`. These read clearly as standard document-level controls.
- **Core Engine Lifecycle**: `[Resolve]`. Essential for resolving generated speech and fonts into `lattice.lock.json` before export.
- **Core Media Transport**: `[Play]`, `[Pause]`. Universal video player baseline.

### 2. What Reads as Leftover Chrome / Harness Scaffolding
- **`[Seek]` and `[Scrub]` as discrete buttons**:
  - In an NLE, seek and scrub are continuous gestures performed on the timeline ruler and playhead.
  - Discrete top-bar buttons labeled "Seek" (which hardcodes `seek(0s)`) and "Scrub" (which triggers `scrub(playhead)`) visually read like test harness triggers or headless smoke hooks rather than primary user interaction models.
- **Hardcoded step buttons (`[Gain -3 dB]`, `[Fade]`)**:
  - Sitting in the global window header, these specific step adjustments (-3 dB fixed delta, 500ms fixed fade) read as discrete command shortcuts rather than contextual inspector properties.
  - While PR #23/#24 established that audio source properties route to Toolbar when a source is focused, their static presence in the global header with fixed numbers makes them appear disconnected from the selected clip.
- **Developer/Agent Tooling (`[Copy locus JSON]`)**:
  - A utility for copying the current Locus JSON payload to clipboard. Visually occupies prime editorial real estate on the top toolbar alongside user creative actions.
- **Hardware Backend Switches (`[CPU]`, `[GPU DX12]`)**:
  - Rendering backend toggles exposed prominently in the main user action bar rather than in a settings menu, status bar, or export dialog.
- **Viewport Zoom Buttons (`[Zoom In]`, `[Zoom Out]`)**:
  - Global top-bar placement separates zoom controls from the timeline ruler they control at the bottom of the window.

---

## 4. Visual Alignment with Editor Panes

The Studio window beneath the top chrome is structured into distinct vertical and horizontal domains:
- **Left Pane (width 200px)**: SEQUENCE hierarchical tree.
- **Center Canvas**: Video preview viewport.
- **Right-Center Pane**: VEL code editor.
- **Right Pane (width 320px)**: Inspector and Spoken Utterance panel.
- **Bottom Bar**: Timeline tracks (Video, Audio, Text) and timeline ruler.

The current top toolbar does not spatially correlate with these lower domains:
- `[Zoom In]` / `[Zoom Out]` affect the bottom Timeline, but sit on Line 2 above the VEL/Inspector panes.
- `[Open Video…]` affects the project, but sits above the Sequence tree.
- `[Split]` / `[Delete]` affect the timeline selection, but sit horizontally adjacent to backend engine status text.

---

## Summary Matrix

| Item | Visual Treatment | Dominance | Semantic Category | Reads As |
|---|---|---|---|---|
| `Lattice` | Teal text, 36px bar | Low / Header | Branding | Primary Header |
| `Open Video…` | Dark gray button | Medium | Project I/O | Primary Editorial |
| `Set In` / `Set Out` | Dark gray button | Medium | Editorial In/Out | Primary Editorial |
| `Split at Playhead` | Dark gray button | Medium | Editorial Cut | Primary Editorial |
| `Delete Selected Clip` | Dark gray button | Medium | Editorial Delete | Primary Editorial |
| `Renderer · <status>` | Muted text string | Low / Orphaned | Engine Status | Inline telemetry |
| `Audio · <status>` | Muted text string | Low / Orphaned | Audio Status | Inline telemetry |
| `CPU` / `GPU DX12` | Teal (active) / Gray | High (when active) | Backend Config | Leftover config toggle |
| `Play` | Saturated Teal button | High | Transport | Primary Transport |
| `Pause` | Dark gray button | Medium | Transport | Primary Transport |
| `Seek` | Dark gray button | Medium | Transport | Leftover test button |
| `Scrub` | Dark gray button | Medium | Transport | Leftover test button |
| `Save` | Saturated Teal button | High | Persistence | Primary Document |
| `Undo` / `Redo` | Dark gray button | Medium | History | Primary Document |
| `Resolve` | Saturated Teal button | High | Asset Generation | Primary Engine |
| `Copy locus JSON` | Dark gray button | Medium | Agent Context | Leftover debug tool |
| `Gain -3 dB` | Dark gray button | Medium | Audio Property | Hardcoded step button |
| `Fade` | Dark gray button | Medium | Audio Property | Hardcoded step button |
| `Zoom In` / `Zoom Out`| Dark gray button | Medium | Viewport Nav | Detached view control |
