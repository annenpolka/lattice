# Studio top row, read against the Engine (first principles)

Date: 2026-08-22
Observed at: `85b589ec260554f851c214731e607c7727c7cae8` (main, after the
INTEGRATED verb-license spine and its leftover fix).

## Lens and scope

One lens only: **Engine / `SemanticEdit` / legal set.** The question is not what
the top row looks like or which affordances it could grow. It is:

> Studio is a client of the Engine. The Engine names what is legal for `here`.
> What, then, must the top-of-window button row *be*?

This is an observation, not a design and not an implementation. There is no
proposed row here, no widget inventory offered as a product, and no chrome. The
inventory in §2 is organised by *which layer answers for a control*, because
that is the only thing the lens can see; it is evidence for §4, not a catalogue.

Held fixed, not reopened: overlap is Projection-Local with one `LocusId` after
the pick; a video click points the source clip; one locus, one Engine legal set,
one utterance, gesture is routing, and the gap is spoken; no silent
`target_*_locus`; scrub does not `point_from_timeline_time`; Title fields only on
Title; no freeze row; no Core Freeze; no per-view selection; no GPUI in Core.

## 1. How it was observed

The row was read live, not from source alone. `lattice-studio` was built with
`--features window` and run on the agent X11 path
(`scripts/studio-linux-smoke.sh`, `DISPLAY=:1`, lavapipe, preview and audio
detached) against the `timeline-basic` fixture at the commit above. The unique
viewable client was identified by `_NET_WM_PID` and captured with
`ffmpeg -window_id`. `LINUX SMOKE OK`.

Two captures give the two loci: first paint (`here = title "Hello"`) and after
the SEQUENCE scene click (`here = scene "demo"`). A second run clicked two
top-row verb buttons directly at measured client bounds. Claims below cite
either the captured pixels, `semantic_state` from the run log, or source at this
commit.

Button geometry was recovered from the capture itself (`LINE` `0x2a3140` /
`TEAL` `0x3dd6c6` background runs), which resolved 11 rectangles on the first
wrapped line and 9 on the second — exactly the 20 buttons `actions_bar`
constructs, in order. The two remaining children carry no background.

## 2. What is in the row, by who answers for it

`StudioView::render` stacks two rows above the body: `header_bar` (branding, no
controls) and `actions_bar` (22 children: 20 buttons, 2 status `div`s). Every
button except Play is built by one helper, `action_button`, with identical
padding and one of two background colours; Play is a hand-rolled equivalent that
additionally publishes its bounds for the smoke.

Grouped by the layer that decides what the control means:

| Answers to | Controls | Produces |
|---|---|---|
| `legal_edits_for(here)` | Set In, Set Out, Split at Playhead, Delete Selected Clip, Gain -3 dB, Fade | `Trim`, `Split`, `Delete`, `SetGain`, `SetFade` |
| an Engine phase or project I/O | Resolve, Open Video…, Save | `resolve` + lock write, `import_media` + `open`, source write |
| volatile session history | Undo, Redo | source stack pop/push, recompile |
| Studio view only | CPU, GPU DX12, Play, Pause, Seek, Scrub, Zoom In, Zoom Out | no `SemanticEdit` |
| read-only projection | Copy locus JSON | `Engine::inspect` |
| nothing (display) | `Renderer · …`, `Audio · …` | renderer / audio error channel |

Five control layers plus a display channel, all at one visual rank. Teal marks
three unrelated things: selected mode (CPU), emphasis (Save, Resolve), and
transport (Play).

## 3. What the Engine offers that the row does not read

`legal_edits_for(locus)` returns `Vec<LegalEdit>`, and each element is a
four-tuple: `verb`, `target: LocusId`, `scope`, `effect`. `is_legal_verb` is the
same data reduced to a bool. `AbsenceReason` types the eight ways a verb can be
absent or routed. `commit_projection(verb, kind)` names a surface. All of it is
pure over `here` and available before any click.

The top row reads none of it at render time. The only consumer of the legal set
in the window is `utterance_block`, in the Inspector. The row's single contact
with legality is `is_legal_verb`, inside `target_locus_for`, *after* the click.

## 4. Findings

### 4.1 The row is invariant under `here`; the legal set is not

The whole top chrome — `header_bar` plus both wrapped lines of `actions_bar`,
rows 0–109 — is byte-identical between the two loci:

```
here = title "Hello"  rows 0..109 sha256 836321c98ef4869f0550c18408972ec547e1b2eaa86aec303f7eedfc9cdc6d7b
here = scene "demo"   rows 0..109 sha256 836321c98ef4869f0550c18408972ec547e1b2eaa86aec303f7eedfc9cdc6d7b
identical: True   differing subpixels: 0
```

The first differing row anywhere in the window is 151 (x 889–1085), inside the
Inspector column. Between those two frames the Engine legal set changed
completely — from
`title, set-position, resize-overlay` to `split, delete, reorder-scene` — and
the row that hosts the verbs did not move a pixel.

This is the whole finding in one line. Everything below is why it matters.

### 4.2 At first paint, what is offered and what is legal are disjoint

`here = demo:title:1` (`kind: title`). From the run log:

```
"legal":[{"verb":"title",...},{"verb":"set-position",...},{"verb":"resize-overlay",...}]
"routed":["title"]
```

The row offers `trim` (twice, as Set In and Set Out), `split`, `delete`,
`set-gain`, `set-fade`. `routed_verbs(Projection::Toolbar, LocusKind::Title)` is
empty, and a unit test on main pins it empty. So at first paint:

- six live, identically-styled verb buttons, none of which can commit;
- three legal verbs, none of which the row offers;
- intersection: empty.

The row is not a degraded rendering of the legal set. It is unrelated to it.

### 4.3 Legality is known before the click and spoken only after

`legal_edits_for` is a pure function of the locus. Nothing about `split` on a
Title depends on the click. Yet the row renders `split` as an ordinary live
button and discovers the answer in `target_locus_for` afterwards.

A rendered, cursor-pointer button is a claim that a verb exists and is
invocable. When it is not, the row has made a false claim and then retracted it
in prose. "Magic is allowed. Hidden behavior is not" cuts both ways: a false
affordance is worse than a hidden one, because the user has already committed
intent to it.

### 4.4 The refusal has no channel, and does not reach the screen

`speak_toolbar` writes the refusal into `last_render`. `last_render` is rendered
at the bottom of the 240px Inspector as `format!("wrote {path}")` — a field whose
template says a file was written. With `EngineError::Edit`'s `#[error("edit: {0}")]`,
the sentence Studio actually renders for a refused verb is:

```
wrote edit: set-gain is not legal for title "Hello" (needs-source-binding).
```

Live, clicking `Gain -3 dB` at `here = title "Hello"` logged exactly
`toolbar: edit: set-gain is not legal for title "Hello"
(needs-source-binding).` and changed **three rows of pixels** in the Inspector
(677–679) — glyph tops at the pane's clip boundary. The remaining pixel
differences in the frame are the mouse cursor's old and new positions. The
Inspector already clips the Engine's own utterance mid-sentence above that
boundary; the refusal is appended below the fold.

So the top row's verbs commit through a fail-closed gate that speaks correctly
and is never read. The Engine spoke; the screen did not.

### 4.5 `Projection::Toolbar` is a category error

`Projection` enumerates Timeline, Canvas, Source, Inspector, Review, Tree,
Toolbar. The first six each *render the locus*: you are looking at the thing, and
in Timeline, Canvas, and Tree you can point at it by coordinate, so a projection
can be the surface an ambiguous point resolves on. The toolbar renders no locus
at all. Nothing in the row calls `point_at`; in the view the only callers are the
SEQUENCE tree node click and the overlap candidate pick, and the timeline clip
hit path points through `point_video_clip`.

Making it a peer variant has a visible consequence, because `commit_projection`
returns it. Live, at `here = source:clip`:

```
set-gain → source:clip (source-binding: set gain on this source) is legal for
this source, committed on Toolbar — not implied absent here.
```

and at `here = scene:demo`, `"routed":[]` with both `split` and `delete`
"committed on Toolbar".

"Committed on Canvas" is an instruction: go there, the thing is there, point it.
"Committed on Toolbar" is not a place. It is always present and it contains no
locus. The same sentence form is doing two different jobs and the reader cannot
tell which.

### 4.6 Four verbs have no locus-anchored home anywhere

`set-gain`, `set-fade`, `split`, and `delete` route only through Toolbar. Since
the toolbar has no pointing geometry, those four legal edits have no surface
where their target is visible at the moment of invocation. `delete` is legal on
a Scene, targets that Scene, and its sole invocation path is a global button
labelled for a clip. The target is off-screen by construction.

### 4.7 `commit_projection` is a function over a many-to-many relation

`routed_verbs(Timeline, Source)` and `routed_verbs(Toolbar, Source)` both contain
`trim`. `commit_projection` searches `[Timeline, Toolbar, …]` and returns the
first hit, so `trim` resolves to Timeline. Live, at `here = source:clip`, the
`trim` clause reads "this Timeline gesture commits it" — and Set In / Set Out,
which commit `Trim` from the toolbar, are named nowhere in any utterance.

The spine's rule is that the gap between legality and routing is spoken. A verb
with two real routes cannot be described by a single "committed on X", so today
one of its two routes is silently unspeakable. Either routes are plural, or the
row must not duplicate a route the Timeline owns.

### 4.8 Labels discard the Engine's answer, and one names a forbidden concept

The Engine hands over `(verb, target, scope, effect)`. Every label keeps at most
the verb:

- **Delete Selected Clip** — names a *selection*, which the model does not have,
  and names the wrong kind: `Delete` is legal on a Scene and targets a Scene.
  Two independent errors in three words.
- **Split at Playhead** — names the parameter and hides the target. The playhead
  is "transient editor state, not a locus"; it is a fine argument to
  `Split { at }` and a poor subject for the label.
- **Gain -3 dB**, **Fade** — see 4.9.

A verb affordance in an Engine client has an ordering forced on it by
`LegalEdit`: the target is what makes the verb true, so it cannot be the field
that gets dropped.

### 4.9 Parameters are baked in, so these are samples, not verbs

`SetGain { db }`, `SetFade { fade_in }`, `Trim { in_point, out_point }`, and
`Split { at }` are parameterised. The row hardcodes `-3`, `500ms`, the playhead,
and (for `Seek`) `Time::ZERO`. The Engine's effect strings describe settable
quantities — "set gain on this source" — not decrements. A button that fixes the
value is one instance of the edit presented as the edit, which is how "gain"
comes to mean "−3 dB, repeatedly."

### 4.10 Identity is derived from the visible label

`docs/interaction.md` states selectors "are semantic and do not depend on visible
labels or widget nesting." `action_selector` is a `match` on the label string,
and `action_button` also takes its GPUI `id` from the label. Renaming a button
silently moves it to `toolbar.unknown` — which is shared, so two renamed buttons
collide on one selector. The same table also issues `inspector.apply` and
`inspector.review`, so the row's identity function is not even row-scoped.

For an Engine client this is the same defect as 4.8 one level down: the stable
name of a verb affordance should derive from the verb and its target, which the
Engine supplies, not from display text.

### 4.11 The verb row has no stable geography

`actions_bar` is `.flex().flex_wrap()`. Live at 1400px it wraps to two lines,
splitting the six semantic buttons across both (Set In through Delete Selected
Clip on line 1; Gain -3 dB and Fade on line 2, after Copy locus JSON). Positions
move with window width; `studio-linux-smoke.sh` has to wait out the reflow
before it can trust `smoke_geom`. A fixed global palette's one advantage over a
locus-anchored surface is that it is always in the same place, and wrapping
spends it.

### 4.12 The one locus-shaped string in the top chrome is a literal

`header_bar` renders `format!("{file} · Scene demo")`. Live, directly above six
verb buttons, it read `main.vel · Scene demo` while `here` was
`title "Hello"`. The toolbar is the only surface with no geometry to show you
where you are, so it is the one place `here` must be legible; instead the space
is occupied by a hardcoded string that is wrong.

### 4.13 The counterexample already in the row

`Copy locus JSON` reads `here`, calls `Engine::inspect`, and reports
`"no current locus"` when there is none — before doing anything. It is the only
top-row control that is locus-aware and the only one that declines gracefully.
The row is capable of this; it is not applied to the verbs.

## 5. What the top row must be, if Studio is an Engine client

Derived from §4, as constraints rather than a design. None of these picks a
widget.

- **R1 — the verb region is a rendering of `legal_edits_for(here)`.** Its
  cardinality varies with `here`; it is empty when `here` is unset. A static verb
  list cannot be correct, because legality is not static.
- **R2 — each verb carries the Engine's `target`, `scope`, and `effect`.** No
  re-authored noun, and never a selection word.
- **R3 — legality resolves before the click.** `target_locus_for` /
  `refuse_edit` stay as the fail-closed gate, but they become the unreachable
  path for anything the row displayed as invocable, not the row's primary
  explanation channel.
- **R4 — three states must be distinguishable before the click**, using data the
  Engine already returns: invocable here; legal but routed to a named surface
  (`commit_projection`); not legal for this kind, with its `AbsenceReason`.
  Showing only the first is implied absence and is forbidden.
- **R5 — a spoken route must name a surface that can be pointed in.** Either
  `commit_projection` stops being able to answer "Toolbar", or `set-gain`,
  `set-fade`, `split`, and `delete` stop having the toolbar as their only route.
  4.5 and 4.6 are one problem.
- **R6 — routes are plural or unique.** A verb with two real commit paths cannot
  be described by a single "committed on X" (4.7).
- **R7 — parameterised edits need a value channel.** A verb affordance with the
  value fixed is a constant, not the verb.
- **R8 — rank follows layer.** A control that rewrites VEL, pushes Undo, and
  recompiles must not share a visual rank with one that scales a viewport float.
  The user's model of "what did I just change" is built from that ranking.
- **R9 — Engine phases are their own rank.** Resolve performs provider I/O and
  writes the lock; the constitution insists that boundary be explicit, and it is
  currently a button between Redo and Copy locus JSON.
- **R10 — the renderer/audio error channel leaves the verb row.** A
  `RequireGpuDx12` typed error is load-bearing and currently surfaces as red
  inline text between two buttons in a wrapping row.
- **R11 — `here` is legible at the row**, because the row is the one surface
  without geometry of its own; and the hardcoded `Scene demo` cannot stand in
  for it.
- **R12 — affordance identity derives from verb and target**, not from display
  text (4.10).

## 6. What does not follow

Guards, because the obvious over-correction breaks a lock.

- **Not "hide what is illegal."** R1 is not "show only the routable verbs." That
  is implied absence, which the spine forbids outright. The row must be able to
  disclose a verb it cannot commit — the defect is that today it *offers* it
  instead of disclosing it.
- **Not a second legal set.** Studio must keep reading `legal_edits_for`. R1 asks
  the row to render the set the Engine already returned, not to compute one.
- **Not a second utterance.** `session.utterance()` is computed once. The
  Inspector renders it today; a second reader of the *same* `Utterance` value is
  not a second utterance. The lock forbids a second legal set and a second
  selection, not a second renderer of the one answer.
- **Not a second selection.** The row has no locus of its own and must not
  acquire one. It reads `here`; it does not point.
- **Not a relaxation of the commit gate.** Everything in §5 is additive to
  `target_locus_for`. Pre-click legality is a truthfulness requirement, never a
  substitute for fail-closed commit.

## 7. Named leftover: Seek and Scrub placement

Named only because the top bar is involved, and not resolved here. `Seek`
(`toolbar.seek-start`) and `Scrub` (`toolbar.scrub`) sit in the verb row at verb
rank while being transport. `Scrub` correctly re-affirms the playhead without
re-pointing — the lock holds. `Seek` is labelled for a verb and hardcodes
`Time::ZERO`, which is the 4.9 defect again. Both are instances of R8, not a
separate question.

## 8. Not decided here

Whether the verb region is a row, a strip near `here`, or something anchored to
the projection that owns the locus. Whether `Projection::Toolbar` is removed,
renamed, or given geometry. Where `set-gain` / `set-fade` / `split` / `delete`
acquire a locus-anchored route. What the value channel for a parameterised edit
looks like. All of that is design; this note only fixes the constraints any
answer has to satisfy.

## Evidence

- `docs/screenshots/toolbar-observe-top-row.png` — the subject: rows 0–109,
  `header_bar` plus both wrapped lines of `actions_bar`.
- `docs/screenshots/toolbar-observe-here-title.png` — full window,
  `here = title "Hello"`; Inspector shows the legal set the row does not offer.
- `docs/screenshots/toolbar-observe-here-scene.png` — full window,
  `here = scene "demo"`; identical top row, `split` / `delete` "committed on
  Toolbar".
- `docs/screenshots/toolbar-observe-refusal-invisible.png` — Inspector column
  before (left) and after (right) a refused top-row verb click; the red gutter
  marks rows 677–679, the entire visible response.
