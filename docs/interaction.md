# Interaction

Studio, the CLI, and external agents are clients of the same Core graph. They do not each invent a selection model. They share a **locus**.

Boards that freeze this model: [`mockups/studio/model.png`](mockups/studio/model.png), [`mockups/studio/scenario.png`](mockups/studio/scenario.png).

## Two principles

- **Provenance is always present. It must not obstruct.** Why a node exists (`Origin`, source span) is visible. Everyday edits do not stop for a review of that fact.
- **A locus survives projections.** The same semantic "here" is what Canvas, VEL, Timeline, and an agent are pointing at. One source definition may project to many rendered instances; the locus is the meaning, not a particular rectangle.

## Locus

A locus is Lattice's **here**: the editing target held across representations.

It is not an "object" in the GUI sense. A `title` written once in VEL may appear as a scene instance, a timeline span, and a canvas rectangle. Those are projections. The locus is the shared pointing.

`lattice-core::Locus` carries:

```text
Locus
- semantic identity
- source span
- timeline range
- visual projection (text / fit / opacity when present)
- scene / sequence context
- derived-from / provenance
```

GPUI types stay out of it. Studio projects a locus; it does not own it.

This is what an external agent should receive as context. Instruction alone is not enough:

```text
Agent receives: locus(title Hello · main.vel:16) + "これ変えて"
```

The same shape covers Canvas click, Timeline click, VEL cursor, "point the agent here", and Review's affected target.

## Three unordered capabilities

These are abilities over a locus, not a pipeline. There is no required `Navigate → Manipulate → Review`.

| Capability | Meaning | Everyday |
|---|---|---|
| **Manipulate** | Touch what is visible. Select, drag, type. | Select → move → done. |
| **Navigate** | Follow the same semantic target in another representation. Canvas ↔ VEL, timeline ↔ source, proposal ↔ affected locus. `Go to definition` is one Navigate, not the capability itself. | Optional. |
| **Review** | Inspect a proposed change as meaning, picture, and source, then Apply or Reject. The current VEL does not change until Apply. | Agent path. |

Navigate is never a gate. Daily editing may never open VEL. Agent editing may never drag. Writing VEL may never open Review.

## One scenario (not the only path)

Main line:

```text
point at title Hello on the canvas
  → provenance visible (Defined in main.vel:16), not a checkpoint
  → "change this" to an agent, with that locus
  → Review: picture + semantic proposal + VEL diff
  → Apply / Reject
```

Optional branch off the canvas: `Go to definition` into VEL, then back. Not step two of the main line.

Other legal paths over the same locus:

- Manipulate only: select → move → done
- Agent: point → instruction + locus → Review
- VEL: Navigate to definition → edit source → confirm on canvas

## Direct manipulation (Studio)

Studio timeline edits use an explicit gesture lifecycle. GPUI only translates pointer and key events. Session owns the viewport, playhead, and ephemeral proposal.

```text
pointer down  → begin
pointer move  → update ephemeral geometry (redraw only)
pointer up    → commit: one SemanticEdit → one VEL rewrite → one compile → one Undo
Escape        → cancel: discard proposal, VEL unchanged, no Undo
```

The playhead is transient editor state, not a locus. Scrubbing maps the latest pointer x through `TimelineViewport` (`time_at_x` / `x_at_time`). It does not rewrite VEL and does not push Undo. Preview extract runs off the UI path with generation coalescing: a stale frame never overwrites a newer request.

Clip-edge trim commits `SemanticEdit::Trim`. Scene body drag commits identity-based `SemanticEdit::ReorderScene`. Timeline title/callout body or edge drags commit timing edits. Canvas title/callout body drag projects the same locus into normalized Canvas Space and commits `SemanticEdit::SetPosition`; GPUI pixels never enter Core or VEL. Failed commit discards ephemeral geometry and restores the compiled layout.

Snapping uses a display-pixel threshold (not a fixed number of milliseconds). Alt temporarily disables snap. Selection remains the shared locus.

## Studio interaction test layers

Studio interaction correctness is split deliberately:

1. Core and Engine unit/property tests own semantic state transitions.
2. GPUI view/entity tests own view-local actions and focus behavior.
3. `VisualTestContext` tests use Studio's `UiDriver`: stable `debug_selector` names are resolved
   to rendered bounds, then real mouse/key/scroll input is dispatched through GPUI. Tests use
   bounds-relative points and assert the resulting Session/source state; screenshots are not the
   primary oracle.
4. Computer Use and `studio-smoke.ps1` remain the final OS boundary for native windows, file
   pickers, clipboard/IME, GPU/DPI appearance, audio devices, and other platform integration.

Interactive selectors are semantic and do not depend on visible labels or widget nesting. The
namespace uses names such as `toolbar.play`, `inspector.title`, `canvas.overlay.<locus>`,
`timeline.track.<track>`, `timeline.clip.<stable-id>`, `timeline.trim.<stable-id>.in`, and
`review.apply`. Tests must not record absolute screen coordinates: use center/relative points or
source/target selector drags. A drag always emits mouse-down, multiple mouse-moves, and mouse-up so
GPUI hit testing and the product drag threshold stay in the path.
