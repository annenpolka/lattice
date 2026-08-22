# Studio toolbar gen2 — CONTROL

Date: 2026-08-22

Status: one isolated CONTROL candidate. Not a selected design. Not a shipping
decision. Not an implementation. This note does not pick a winner among gen2
skins and does not invent a new ontology.

What this is: the honest current Studio chrome **minus** the fixed always-visible
bank of locus-taking `SemanticEdit` buttons. A thin session/application strip
may remain global. CONTROL keeps only what already exists that is not those six
buttons. It does not add a new kind of chrome, a new pane, a new verb home, or a
new pointing rule.

Sketch stills (PNG, no mp4):

- [Full chrome after the bank](https://github.com/annenpolka/lattice/blob/18f4a8a6921ab401a8e974d82058f6ebab7ea071/docs/sketches/toolbar-gen2-control/still-full-chrome.png?raw=true)
- [Thin session strip](https://github.com/annenpolka/lattice/blob/18f4a8a6921ab401a8e974d82058f6ebab7ea071/docs/sketches/toolbar-gen2-control/still-session-strip.png?raw=true)
- [Source here: gain/fade unrouted](https://github.com/annenpolka/lattice/blob/18f4a8a6921ab401a8e974d82058f6ebab7ea071/docs/sketches/toolbar-gen2-control/still-source-unrouted.png?raw=true)
- [Scene here: split/delete unrouted](https://github.com/annenpolka/lattice/blob/18f4a8a6921ab401a8e974d82058f6ebab7ea071/docs/sketches/toolbar-gen2-control/still-scene-unrouted.png?raw=true)
- [Overlap lock unchanged](https://github.com/annenpolka/lattice/blob/18f4a8a6921ab401a8e974d82058f6ebab7ea071/docs/sketches/toolbar-gen2-control/still-overlap.png?raw=true)
- [Title fields only on Title](https://github.com/annenpolka/lattice/blob/18f4a8a6921ab401a8e974d82058f6ebab7ea071/docs/sketches/toolbar-gen2-control/still-title-inspector.png?raw=true)

The HTML source for those stills is
[`docs/sketches/toolbar-gen2-control/index.html`](../sketches/toolbar-gen2-control/index.html).

## Shared premise (closed before this note)

Unanimous delete: the fixed always-visible bank of locus-taking `SemanticEdit`
controls. In current Studio those six buttons are, in `actions_bar` order:

| Label | `debug_selector` | Session call | Engine verb | Current `routed_verbs` home |
|---|---|---|---|---|
| Set In | `toolbar.set-in` | `set_in_at_playhead` → `Trim` | `trim` | Toolbar (Source) — Timeline already routes `trim` via clip-edge handles |
| Set Out | `toolbar.set-out` | `set_out_at_playhead` → `Trim` | `trim` | same |
| Split at Playhead | `toolbar.split` | `split_at_playhead` → `Split` | `split` | Toolbar (Scene) only |
| Delete Selected Clip | `toolbar.delete-clip` | `delete_selected_clip` → `Delete` | `delete` | Toolbar (Scene) only |
| Gain -3 dB | `toolbar.gain-minus-3` | `set_gain(-3)` → `SetGain` | `set-gain` | Toolbar (Source) only |
| Fade | `toolbar.fade` | `set_fade(500ms)` → `SetFade` | `set-fade` | Toolbar (Source) only |

A non-verb session/application strip **may** remain global. Membership of that
strip is **not settled**. CONTROL does not settle it. CONTROL keeps the strip
members that already exist and are not those six buttons. It does not add
kinds of chrome that are not already on `main`.

Locks — restated, not reopened:

1. Overlap UI is Projection-Local: a coordinate that names several loci lists
   candidates on the touched Timeline only. One shared `LocusId` after the pick.
   No cross-surface modal.
2. A video clip click points the **source clip** and keeps that identity. Here
   is not promoted to the containing Scene.
3. One locus, one Engine legal set, one utterance.
4. No silent retarget. When legality ≠ what the touched projection can commit,
   the difference is spoken.
5. Title-shaped Inspector fields appear only when here is Title.
6. No per-view selection. Focus, hover, playhead, and in-flight gesture are not
   a second semantic "here".

Pointing is not open in this note. Verb-license *skins* are not compared here.

## What current chrome actually is

`StudioView::render` on `main` (HEAD `85b589e`) composes, top to bottom:

1. `header_bar` — brand `Lattice` plus `{file} · Scene demo`. Not a verb bank.
2. `actions_bar` — one wrapping row. This is the current global strip. It mixes
   session/application controls with the six locus-taking buttons named above.
3. `body` — four panes, left to right: SEQUENCE, Canvas, VEL, Inspector.
4. `timeline_bar` — ruler, overlap candidate cards (when unresolved), tracks.

The six buttons are not a separate widget. They sit in the same wrapping row as
Open Video, renderer/audio status, transport, Save/Undo/Redo, Resolve, Copy
locus JSON, and Zoom. CONTROL's subtraction is therefore a hole in that row,
not the deletion of a distinct "toolbar pane".

Inspector already hosts the one utterance as a disclosure block
(`inspector.utterance`): Here, pointing, legal set, "this gesture commits …",
and spoken clauses. The block has no click handler. It is not a commit surface
today.

## CONTROL subtraction

Delete only these six always-visible locus-taking buttons:

- Set In
- Set Out
- Split at Playhead
- Delete Selected Clip
- Gain -3 dB
- Fade

Keep every other control that already exists in `actions_bar`, the header, the
four panes, and the timeline. Do not relocate the six verbs onto Inspector,
utterance, Timeline context chrome, SEQUENCE, or a new dock. Do not add a
command palette, a radial menu, a property sheet for gain/fade, or keyboard
verbs that do not already exist.

After the subtraction, the global strip that remains is exactly this membership
(current labels, current selectors):

| Label / chrome | `debug_selector` | Kind | Why it stays in CONTROL |
|---|---|---|---|
| Open Video… | `toolbar.import` | application | already present; not a locus-taking `SemanticEdit` |
| `Renderer · {status}` | `toolbar.renderer-status` | session status | already present; read-only |
| audio status | `toolbar.audio-status` | session status | already present; read-only |
| CPU | `toolbar.renderer.cpu` | session | already present; renderer request, not a locus edit |
| GPU DX12 | `toolbar.renderer.gpu-dx12` | session | already present; explicit request, no silent fallback |
| Play | `toolbar.play` | session transport | already present; playhead is not a locus |
| Pause | `toolbar.pause` | session transport | already present |
| Seek | `toolbar.seek-start` | session transport | already present |
| Scrub | `toolbar.scrub` | session transport | already present; scrub does not re-point |
| Save | `toolbar.save` | session | already present; writes working source |
| Undo | `toolbar.undo` | session | already present; volatile working-session history |
| Redo | `toolbar.redo` | session | already present |
| Resolve | `toolbar.resolve` | session phase | already present; Resolve is a phase, not a locus verb |
| Copy locus JSON | `toolbar.copy-locus` | session / agent | already present; copies current projection JSON |
| Zoom In | `toolbar.zoom-in` | session viewport | already present; viewport is not a locus |
| Zoom Out | `toolbar.zoom-out` | session viewport | already present |

That list is CONTROL's strip. It is not a proposal for the settled membership
of a future session strip. Other candidates may drop or regroup these. CONTROL
only refuses to invent replacements.

Header, SEQUENCE, Canvas, VEL, Inspector, Timeline, Review Apply/Reject, Go to
definition, Title fields-when-Title, overlap cards, clip-edge trim handles,
Canvas move / four-corner resize, and scene-body reorder stay as they are.

## Routes after the bank is gone

`routed_verbs` on `main` names only gesture paths the UI can actually commit.
Toolbar is the **only** commit projection for `set-gain`, `set-fade`, `split`,
and `delete`. Timeline already commits `trim` (source clip edges), overlay
time (`title` / `callout`), and `reorder-scene`. Canvas commits geometry.
Inspector commits title text. VEL is a real editor (source rewrite) and is not
a `routed_verbs` home for those four Engine verbs.

CONTROL deletes the Toolbar commit path and does not add another. Therefore:

| Engine verb | Legal here (Engine, unchanged) | Remaining commit route in CONTROL | Spoken after the bank |
|---|---|---|---|
| `set-gain` | Source (`source-binding`) | **unrouted** | legal for this source; no remaining chrome commits it |
| `set-fade` | Source (`source-binding`) | **unrouted** | legal for this source; no remaining chrome commits it |
| `split` | Scene (`scene`) | **unrouted** | legal for this scene; no remaining chrome commits it |
| `delete` | Scene (`scene`) | **unrouted** | legal for this scene; no remaining chrome commits it |
| `trim` | Source (`source-range`) | Timeline clip-edge handles (already present) | still committed on Timeline |
| `reorder-scene` | Scene (`sequence`) | Timeline scene-body drag (already present) | still committed on Timeline |

`Set In` / `Set Out` are Toolbar-shaped `trim` at the playhead. CONTROL deletes
those two buttons. Clip-edge `trim` on Timeline remains. CONTROL does not add a
playhead-trim gesture to replace them. Playhead-at-in / playhead-at-out as
named chrome is gone; edge trim is not.

There is no keyboard route today for `set-gain`, `set-fade`, `split`, or
`delete`. `handle_key` on `main` owns Undo/Redo, Escape (cancel in-flight
gesture), and zoom. CONTROL does not add keys.

Session methods `set_gain`, `set_fade`, `split_at_playhead`, and
`delete_selected_clip` remain callable in process. CONTROL is a chrome
subtraction, not an Engine or Session deletion. With no remaining selector,
those methods are not a user route.

VEL source edits can still rewrite `gain`, `fade`, `game[start..end]`, or
scene structure as text. That is source editing, not a named `routed_verbs`
commit for `set-gain` / `set-fade` / `split` / `delete`. CONTROL does not
reclassify the VEL pane as those verbs' home.

## Utterance is disclosure-only

The utterance is **disclosure-only**. It is not a commit surface.

On `main`, `utterance_block` renders Here, pointing, the Engine legal set, the
touched projection's routed set, and spoken clauses. Nothing in that block
invokes `apply_edit`. Toolbar buttons, Timeline/Canvas gestures, and Inspector
title Apply are the commit surfaces.

CONTROL keeps that contract. After the bank is gone, spoken clauses for
`set-gain`, `set-fade`, `split`, and `delete` disclose Engine legality and that
no remaining chrome commits them. They do not become buttons. Clicking a
spoken line does not apply a `SemanticEdit`. CONTROL does not turn the
utterance into a verb list, a Compass, or an Inspector form.

A missing route is spoken, not claimed as present. CONTROL does not keep
`routed_verbs(Toolbar, …)` listing verbs whose buttons no longer exist, and
does not pretend Timeline or Inspector now commit them.

## What CONTROL does not change

- One locus / one Engine legal set / one utterance.
- Touched projection is routing only.
- Video clip click points the source clip.
- Overlap candidates stay on the touched Timeline.
- Scrub / playhead do not re-point.
- Title Inspector fields only when here is Title.
- No selectable freeze tree row. Freeze remains a `TimeMap` reading.
- No silent retarget when a remaining session button is pressed against the
  wrong here. Session methods that still exist stay fail-closed; CONTROL just
  removes their chrome.
- Review stays ungated. Direct manipulation still commits without it.
- Navigate stays optional.

CONTROL is not Compass chrome, not Projection-Local as a product skin, and not
the Reading five-pane product. It is current Studio with the verb bank cut out.

## What this note refuses

- Inventing a new ontology (no new `LocusKind`, no new capability, no new
  selection, no new verb-license rule).
- Filling the four unrouted verbs with a new home so CONTROL looks complete.
- Picking a winner among other gen2 candidates.
- Implementing Studio, GPUI, Engine, or `routed_verbs` in this PR.
- Reopening the locks listed above.

The hole is the candidate. Other notes may propose a home for `set-gain`,
`set-fade`, `split`, and `delete`. CONTROL's job is to show the current chrome
without the bank, name the missing routes, and leave the utterance as
disclosure.
