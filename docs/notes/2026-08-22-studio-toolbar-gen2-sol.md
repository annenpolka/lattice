# Studio toolbar gen2 — Sol: route-bearing instruments

Date: 2026-08-22

Status: one replacement proposal for discussion. It is not a selected design, an implementation
plan, or a claim that the non-verb session/application strip has settled membership.

## Starting deletion

Delete the fixed, always-visible bank of locus-taking edits: Set In/Out, Split at Playhead, Delete
Selected Clip, Gain -3 dB, and Fade. Moving the same controls into another permanent row, an
Inspector, or the utterance would preserve the bank under a different layout and is not this
proposal.

The replacement is **route-bearing instruments**:

> The Engine names the legal edits for the one committed locus. Studio discloses that answer in one
> utterance, but commits an edit only at an instrument-specific route that carries the same exact
> `LocusId`.

The route is part of the affordance's meaning, not a second license. A Timeline handle, local audio
band, scene/playhead intersection, or key binding may commit only a member of the current Engine
legal set. If the touched instrument cannot commit a legal edit, the utterance names its route; the
instrument does not borrow another target.

This replaces a noun-first toolbar question — “which button applies to selection?” — with a
route-first question — “where does this exact edit commit for here?” It does not create
projection-local selection or projection-local legality.

## Utterance contract

The utterance is **disclosure-only, never a commit surface**. It states:

1. the committed locus;
2. the Engine legal set for that locus;
3. the direct route for each currently routed edit; and
4. an explicit `unrouted` result, with a reason, for every legal edit that has no route.

Verb-looking text in the utterance is not clickable, focusable, or keyboard-activatable. This keeps
one utterance from becoming the deleted bank in sentence form. Parameter entry also stays at the
route: the utterance never owns a gain value, fade duration, split time, or delete confirmation.

An illustrative source utterance is:

> Here: `source:fight`. `SetGain` → this source's Timeline audio band. `SetFade` → this source's
> Timeline fade handles. Trim → this source's Timeline edges. Related scene `scene:demo`: Navigate
> to point it for scene edits.

The related scene is disclosure and Navigate, not an adopted target. Its legal edits are not merged
into the source's legal set.

## Required routes

| Edit | Exact target required before the route appears | Commit route | If the requirement is not met |
|---|---|---|---|
| `SetGain` | A source clip is the committed locus and the Engine licenses `SetGain` for that `LocusId`. | The pointed source block exposes a compact audio band in its Timeline projection. Dragging or entering a value there commits one `SetGain` to that source on release/confirm. | `unrouted` for the current locus. No scene-to-first-source lookup and no project-wide gain control. |
| `SetFade` | A source clip is the committed locus and the Engine licenses `SetFade` for that `LocusId`. | Fade handles live on that same source block's audio band. A handle gesture commits one `SetFade` to the carried source identity. | `unrouted` for the current locus. No inferred source binding. |
| `Split` | A scene is the committed locus, the playhead intersects it, and the Engine licenses `Split` at that time. | The scene block's playhead intersection becomes a split notch; activating the notch or the split key commits to that exact scene and explicit playhead time. | `unrouted`, stating whether the missing fact is a scene locus, an intersecting playhead, or Engine legality. A pointed source remains a source; Navigate to the related scene is explicit. |
| `Delete` | A scene is the committed locus and the Engine licenses `Delete` for that `LocusId`. | The Delete key while the scene block is the committed locus opens any required confirmation and then commits deletion of that exact scene. The scene block may mirror the key route in a local, transient overflow. | `unrouted` for the current locus. “Delete selected clip” is removed because it conceals both target kind and scope. |

Set In/Out has no replacement button. Source trim remains on the source block's left and right
Timeline edges, preserving the existing pointer-down/update/pointer-up gesture lifecycle. This note
does not propose a new semantic edit for it.

The compact audio band and transient scene overflow are local instruments, not persistent verb
homes. They appear only for the committed locus, carry its identity, and disappear when that locus
changes. Opening one cannot re-point. A locus change invalidates an in-flight parameter draft rather
than applying it to the new locus.

## Locked behavior carried unchanged

- Overlap remains Projection-Local: the candidate list stays on the touched projection, pointing
  remains unresolved until one candidate is chosen, and the choice establishes one shared locus.
  Route-bearing affordances do not appear for an unresolved point.
- Clicking a video clip points its source clip. It never promotes to the containing scene. Therefore
  gain, fade, and trim can become routed after that click; split and delete cannot silently target the
  related scene.
- There is one locus, one Engine legal set, and one disclosure utterance. Hover, focus, playhead, an
  open audio band, and an open overflow are not additional selections.
- Title fields appear only when the committed locus is Title. This proposal neither moves those
  fields nor treats their presence as precedent for a general property panel.
- Every commit carries the locus identity disclosed before invocation. A stale or missing identity
  fails closed; Studio never reverse-matches by label, source span, visible text, or “first related”
  object.

## Global strip boundary

A non-verb session/application strip may remain global, but this proposal does not settle its
membership. Save, Resolve, renderer choice, playback transport, export, Undo/Redo, or other candidates
are deliberately neither kept nor removed here. The boundary test is only that the strip cannot take
a locus and cannot commit a `SemanticEdit`.

## Falsifiers and open routing results

This proposal fails if any of the following is necessary:

- users must click verb text in the utterance to complete ordinary edits;
- a source click must be promoted to a scene to make split or delete reachable;
- gain or fade needs a global fallback when no exact source is pointed;
- the Delete key chooses a target from focus, hover, playhead, or a per-view selection;
- an affordance survives a locus change with its old value but commits to the new identity; or
- an Engine-legal edit is omitted instead of being reported with a route or explicit `unrouted`
  reason.

`unrouted` is an acceptable result, not permission to invent a route. Route-bearing instruments are
one candidate replacement for the deleted bank; this note does not rank it against other proposals
or select it for Studio.
