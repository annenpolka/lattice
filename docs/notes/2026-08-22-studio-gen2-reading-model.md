# Gen2 interaction model — the Reading

Date: 2026-08-22

Status: second-generation interaction model, submitted independently to the chair round

Scope: one interaction model. Not a UI proposal, not a mutation list, not a pane arrangement.

Input of record: [studio interaction chair note](https://github.com/annenpolka/lattice/blob/3fc72b7/docs/notes/2026-08-22-studio-interaction-chair-note.md).
Spec of record: [`docs/interaction.md`](../interaction.md), [`docs/principles.md`](../principles.md), `AGENTS.md`.
When this note disagrees with the spec, the spec wins; where it proposes to change the spec, it says so.

---

## 1. The model in one page

The editor holds exactly one **Reading**.

A Reading is the editor's complete, spoken answer to a single question:

```text
what am I pointing at, when am I looking, at what scope may I act,
what will each act actually target, and what is missing here and why?
```

Every clause of that answer is derived from Engine/Core data and is stated. Nothing in it is
inferred behind the user's back. That is the whole model; the rest of this note is its consequences.

The spine is one rule about guessing:

> **Ranking is allowed only where its consequence is a reversible pointing.
> Where the consequence is a source rewrite, the editor must not rank — it derives, or it fails
> closed and says what it needs.**

That single asymmetry resolves the chair note's live tension between M10 (disclose overlapping
candidates) and M3 (fail-closed verbs). Pointing at an overlapping timeline time may be ranked,
because the cost of a wrong rank is a locus that moves again on the next keystroke and never
touches VEL. Choosing which source binding a `Trim` will cut may not be ranked, because the cost
of a wrong rank is a committed rewrite of someone's project.

Three cases exhaust the model's resolution policy:

| Situation | Policy | Example in this repo |
|---|---|---|
| **Unique** target derivable from the locus's own fields | adopt it and name it | `SemanticEdit::Split` from a Title locus whose `scene_id` is `demo` |
| **Ranked** candidates, consequence is pointing only | adopt the top, state the ranking, one key steps through | overlapping `title`/`callout`/`video` clips at one timeline time |
| **Ambiguous**, no domain order, consequence is a rewrite | do not adopt; require an explicit pointing | `Trim` from a Title locus in a scene with two bindings |

Today the third row is a silent `find(..)` on the first match in document order:
`target_source_locus` falls through to "the first `LocusKind::Source` in the project", and
`primary_source_name` falls through to "the first `Item::Binding` in the scene body". Those two
lines are where the "hidden second target" in Candidate E's toolbar observation actually lives.

### Where a Reading lives

A Reading is **not a second selection**. It holds no identity that the session does not already
hold. It is a pure function of Engine data plus transient session state:

```text
Reading = f(Compilation, LocusId, playhead, viewport facts)
```

It is recomputed after every compile, never persisted, never diffed, never written to VEL, never in
Undo. Proposed home, following the precedent that `LocusProjection` is a Core type while
`loci_from_project` is Engine code:

- `lattice-core` owns the data shape (`Reading`, `LegalEdit { edit, scope, target: LocusId }`,
  `Absence`). Pure data, no GPUI, no backend.
- `lattice-engine` owns the derivation (`Engine::reading(&Compilation, &LocusId, Time)`), beside
  `inspect` and `locus_at_timeline`.
- `lattice-cli` serializes it under the existing global `--json`. Studio renders it.

That placement matters for more than tidiness: **the Reading is the exact payload an external agent
needs.** `locus + instruction` is the invariant, and today an agent receiving a locus must guess
edit legality and target the same way Studio guesses. One derivation in Engine removes the guess
from both clients at once, with no agent runtime and no LLM SDK in the repo.

---

## 2. What "here" is

"Here" is one `LocusId` in the session. Unchanged. No per-view semantic selection, no second
selection for the playhead, no new `LocusKind`.

What changes is that "here" is never presented as a bare highlight. A Reading always names three
things about it:

1. **Subject** — `LocusKind`, label, `node_id`, and the citation `origin` + `source_span`
   (`main.vel:16`), or the stated reason there is no span.
2. **Witness** — which projection produced this pointing: a rail item, a VEL caret, a canvas
   overlay, a tree row, the CLI, an agent. The witness is view-local provenance for the pointing
   itself, not a selection.
3. **Reason** — why this locus and not another. For a derived target: the field that derived it
   (`scene_id`, `sequence_id`, `derived_from`). For a ranked pointing: the rank and the ties.

Coherence, not simultaneity, is the invariant (chair Challenge 1). Exactly one Reading exists, and
it is rendered adjacent to the witness that last produced the locus. Other projections may cite the
same locus, but they do not each grow a verb list. A Reading must remain complete and legible when
only one projection is visible, and must make the others reachable by Navigate — which stays
optional and is never a gate.

The Reading also kills a class of bug rather than styling around it: because every field in it is
born from Engine data at render time, no editable draft can outlive the locus it belongs to. Today
`adopt_locus_label` copies `locus.label` into a persistent `title_draft` for *any* locus kind, which
is why a Scene locus is offered a populated "Title text" box
([scene locus with title control](https://github.com/annenpolka/lattice/blob/75c4674c6fa123451c69fa93de8c780eeb8dbb26/docs/screenshots/studio-observe-a-scene-locus.png?raw=true)).
There is no `title_draft` in this model. There is a text field only while a legal text verb is open.

---

## 3. Playhead versus locus

| | Locus | Playhead |
|---|---|---|
| Question | which one? | which moment? |
| Meaning | **what I mean** | **when I am looking** |
| Lifetime | until re-pointed | continuous, transient |
| Owner | session, shared by all clients | session, Studio A/V clock |
| In VEL | referenced by span | never |
| In Undo | never (pointing is not an edit) | never |
| Changed by | pointing, candidate cycling, Navigate, CLI/agent | transport, scrub, seek, playback clock |

The model makes them **orthogonal by default and both always named**.

**Scrub stops pointing.** This is a real behavior change and the one I most want tested. Today
`commit_timeline_pointer` on a `TimelineGesture::Scrub` calls
`session.point_from_timeline_time(session.playhead)`, so moving through time silently rewrites the
shared semantic "here". The sharpest case is the Audio rail: clip pointing there is disabled
outright (`hit_clips_on_track` clears every candidate for `"Audio"`), yet the scrub commit still
re-points through `locus_at_timeline`, which searches *all* loci — so dragging on audio can land the
shared locus on a title
([audio rail scrub re-points to a title](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-audio-rail-scrub-to-title.png?raw=true)).
Under this model, time navigation is not a pointing act. Scrub moves the playhead and nothing else.

Coupling exists, but only as a named verb the user asks for, in either direction, with no source
effect and no Undo entry:

- **Seek to here** — playhead ← `locus.timeline_span.start`. Offered exactly when the locus has a
  `timeline_span` that does not contain the playhead. Never automatic (chair Challenge 2).
- **Read the playhead** — locus ← a candidate at the current time. This is the M10 surface. The
  user asks; time never asks on the user's behalf.

Consequently the case that reads as a bug today becomes a sentence: a title selected at `0s` while
active `2s–5s` is stated as "*active 2s–5s; you are at 0s (before it starts)*" with **Seek to here**
offered, instead of a black stage
([title at 0s, empty canvas](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-open-title-0s-empty-canvas.png?raw=true),
[playhead outside locus](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-playhead-outside-locus.png?raw=true)).

**Space is not the editor's identity.** The loop of this editor is read → name → commit, and no step
of it needs a transport key. Space may be bound to play/pause as a conventional default when the
root is focused and no text entry is active — it is currently bound to nothing — but it is a default,
not a pillar, and it is not what makes the thing an editor. Transport witnesses time; it does not
select, mean, or mutate.

---

## 4. How legal verbs appear

**Verbs are derived from legality, scoped, and either reachable or absent-with-a-reason.**

There is no permanent property grid. A Reading lists the `SemanticEdit`s that are legal for this
locus, each labelled with its real scope and its real target `LocusId`. Choosing a verb is what
creates its input (a text field, a time field, a number), seeded from Engine data at that instant;
dismissing the verb destroys the input. Direct gestures are pre-named verbs: pointer-down already
declares the verb, the scope, and the target, and the Reading shows them during the drag, before
commit. Pointer-up still commits exactly one `SemanticEdit` → one rewrite → one compile → one Undo
entry, and Escape still cancels with VEL unchanged.

Every scope label below is a group of fields that already exists in `lattice-core::SemanticEdit`.
No label is a UI-invented category. Reachability is computed only from the locus's own fields.

| Verb (scope label) | `SemanticEdit` | Target kind | Reachable when |
|---|---|---|---|
| Retype title — *definition text* | `Title { text }` | Title | `kind == Title` |
| Move / lengthen title — *this placement's timing* | `Title { at, duration }` | Title | `kind == Title` |
| Set title opacity — *this placement* | `Title { opacity }` | Title | `kind == Title` |
| Move / lengthen callout — *this placement's timing* | `Callout { at, duration }` | Callout | `kind == Callout` |
| Place overlay — *this placement's geometry* | `SetPosition` | Title, Callout | `visual.is_some()` |
| Resize overlay — *this placement's geometry* | `ResizeOverlay` | Title, Callout | `visual.is_some()` |
| Trim in/out — *this source binding* | `Trim` | Source | `kind == Source`, or `derived_from` names one Source, or the scene declares exactly one binding |
| Split — *this scene, at a source time* | `Split { at }` | Scene | `kind == Scene`, or `scene_id` names one |
| Delete — *this scene* | `Delete` | Scene | same |
| Reorder — *this sequence's order* | `ReorderScene { before }` | Scene | same, and `sequence_id` is present |
| Set gain — *this scene's named source* | `SetGain { db }` | Source | same rule as `Trim` |
| Fade in — *this source's video, anchored at scene start* | `SetFade { fade_in }` | Source | same rule as `Trim` |

Two things this table refuses to do:

- **It refuses to retarget across the project.** `target_source_locus` and `target_scene_locus`
  currently end in `loci.into_iter().find(kind == Source | Scene)` — the first match anywhere in the
  project. From a `sequence:main` locus, `SetGain` therefore rewrites some scene's first binding
  while the displayed locus never moves
  ([toolbar fallback](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-toolbar-2026-08-22-after-gain-vel-inspector.png?raw=true)).
  Those two fallthroughs are deleted. A Sequence locus simply has no gain verb; it has "gain needs a
  source binding — point at a scene."
- **It refuses to lie about anchoring.** `SetFade` is rewritten as `fade {source} { at 0s for … }`,
  so its scope is the scene's source video fade-in, not "fade at the playhead." The label says so.

### Named gaps, not simulated verbs

Where Core has no verb, the Reading says so instead of borrowing a field:

- **No time-scoped audio edit exists.** `SetGain` is scene/source scoped and `SetFade` is a video
  opacity envelope. An audio placement's Reading states "no legal time-scoped audio edit in Core"
  and offers nothing. This is the chair note's warning about concealing a Core gap, honored.
- **No TimeMap edit variant exists.** `freeze` is explained, not edited (§7).
- Speech text is a resolve/provider concern, not a `SemanticEdit`.

### The honest weak point

M3's falsifier is that scope labels "cannot map one-to-one to existing `SemanticEdit`s." They do not,
and I will not hide it: `SemanticEdit::Title { text, at, duration, opacity }` bundles a definition
edit and two placement edits into one variant, so three of my labels map to one variant. I take the
weaker claim that survives: **each label names a field group that already exists in Core**, so the
categories are still Core's, not the UI's. The clean fix is a Core split
(`SetTitleText` / `SetOverlayTiming` / `SetOverlayOpacity`), which is a Core decision and not
something an interaction model should force. The evidence that decides it: if the three labels
cannot be predicted as distinct scopes by participants while sharing one variant, split the variant;
if they can, the bundling is only a serialization detail.

### Where verbs live

Scope determines home. This is a hierarchy derived from the model, not a repaint.

| Verb class | Home | Examples |
|---|---|---|
| Locus verbs | the Reading, at the witness | the table above |
| Session verbs | with the witness of time | play, pause, seek, scrub, zoom, renderer request |
| Project verbs | with the project | save, import, resolve, export |
| Not verbs at all | never in a command strip | renderer status, audio status, diagnostics, dirty state |

---

## 5. Overlap policy (M10)

I take M10 as **policy, not probe**, in the form that keeps ordinary pointing cheap:

1. Pointing at a timeline time gathers candidates exactly as today: loci whose `timeline_span`
   contains `t`. Only loci with a `timeline_span` qualify, so Scene, Source, Sequence and Media are
   not candidates.
2. The top candidate under the existing `(specificity(), narrower span)` key is committed
   **immediately** to the one shared locus. No modal, no prompt, no obstruction.
3. The Reading then **states the collapse**: how many readings were at `t`, which one was taken,
   why, and what the others were. One key steps the committed locus through them, one at a time.
4. Candidates are never persistent project state, never per-view selection, never in Undo. After a
   step, every projection receives the same `LocusId`.

This is the "ranked" row of the spine rule: ranking is permitted here precisely because the
consequence is a pointing that costs one keystroke to correct and never touches VEL.

**Chair hook 6 — is specificity domain ordering or interaction policy?** Interaction policy, and the
code says so. `Locus::specificity()` is a hand-assigned `u8` per `LocusKind` with `Title`,
`Callout` and `Speech` all at `4` and `Sequence`/`Media` both at `0`; a total order cannot be
recovered from those ties, and the tiebreaker `i64::MAX - span.duration` is a statement about
pointing convenience, not about meaning. The real domain relations are elsewhere and are partial:
`derived_from`, `scene_id`, `sequence_id`, and `TimeSpan::contains`. Specificity flattens a partial
order into a total one so that a click can be answered in one frame. That is a legitimate thing to
do — and exactly why it must be spoken rather than be the only record of the choice.

The distinguishing evidence, if the chair wants it settled empirically: construct a fixture where
two candidates tie on both specificity and duration (two `title` invocations with identical spans —
`crates/lattice-studio/tests/layout.rs` already builds duplicate overlays with distinct
`LocusId`s). If the domain ordered these, the tie would be breakable from semantic data. It is not;
today the winner is whichever `max_by_key` happens to keep.

---

## 6. How absence is disclosed

Absence is a **clause of the Reading**, not a pane state, not a black rectangle, and never one
undifferentiated "empty." I take M9 and M1 as a single vocabulary, each entry traceable to data that
already exists.

| Clause | Grounded in | What is said |
|---|---|---|
| Not applicable to this kind | `Option` fields on `Locus` | "a Scene has no timeline range of its own; its clips do" |
| Temporal absence | `timeline_span` present, playhead outside | "active 2s–5s; you are at 0s" + **Seek to here** |
| No visual projection | `visual == None` (audio, Source, Scene) | "audio placement: no picture" |
| Authored by convention | `Provenance::convention`, `source_span == None` | "created by `convention commentary`" + the explain event |
| Preview disabled | `PREVIEW=0` launch state | "preview off" — not a failure |
| Renderer not ready | `RendererInitError::{Unavailable, Initialization}` | typed stage and message; a `RequireGpuDx12` refusal is never a silent CPU frame |
| Media or asset unavailable | `ExportError::{MissingMedia, MissingSource, StaleFont, MissingFont}`, `AudioMixError::{MissingWindowSource, MissingGeneratedAsset, SourceUnavailable}`, `LAT-RES-001/002/005` | the missing thing, by name |
| Compile diagnostics | `Compilation::diagnostics`, `has_errors()` | the code and span; "picture is from the last good compile" |
| Frame not computed yet | preview generation in flight | "computing" — keep the last still, explicit retry |
| Nothing projected here | empty candidate set at `t` | "nothing projected at 5.2s on the Audio rail" |
| Verb absent | unreachable target, or no Core support | what it needs, or that Core has no such edit |

Two hard rules follow.

**Black is never an answer.** All eleven clauses above are separately producible today, and several
can hold at once (a compile diagnostic *and* a locus outside the playhead). An empty stage collapses
every one of them into the same pixel
([empty canvas at 0s](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-open-title-0s-empty-canvas.png?raw=true)).

**Geometry outlives pixels.** Today the canvas renders overlays only inside
`if let Some(preview_image)`; with no frame the stage becomes `div().flex_1()` and the locus loses
its overlay and its handles at exactly the moment the user needs the explanation. When normalized
geometry is known, the overlay and its affordances persist independently of frame availability.

---

## 7. Structural consequences

These are entailments of §1–§6, not extra mutations.

**The Inspector pane is deleted.** A property grid must have a subject at all times, so it invents
one; that invention is `adopt_locus_label` plus an unconditional "Title text" field plus
`target_*_locus`. Verbs generated from legality cannot lie about legality, so nothing needs the
grid.

**Five panes stop being the product.** Coherence is a property of one Reading, not of five
simultaneous highlights. This also removes the width inversion the chair note flags: Canvas can only
lose width to a fixed 240px Inspector rail if that rail exists. The remaining measurement work is
M8's harness, which selects no skin.

**VEL is protected, not hidden.** This is not hide-VEL, which the chair note rejects and I reject
too. Two guards: the Reading cites `origin` + `source_span` (or names why there is none) at all
times, so source identity is continuously present even when text is off-screen; and the route to the
source text must be persistent and non-collapsing — VEL is the only surface the layout policy may
not drop under pressure. Source cannot become inexplicable.

**Review becomes a Reading of a proposal, not a pane and not a gate.** `apply_committed` already
builds an `EditProposal` (`locus_id`, `description`, `edit`, `vel_diff`, `new_source`,
`base_revision`) and throws it away. Keep it: a proposed Reading shows the same subject, scope,
target, and absence clauses, plus the diff and the current picture, and it can go stale via
`base_revision` → `EngineError::StaleProposal`. Direct manipulation still commits without Review
(chair contradiction 3). This is what generalizes Review past the title-only diff
([title-only Review](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-review-title-propose.png?raw=true)).

**Freeze becomes explanation, not a target.** The tree's `format!("freeze:{}", source.id)` row is a
synthetic non-locus; clicking it calls `point_at` with an id no `Locus` has, `current_locus()` finds
nothing, and nothing is reported
([freeze click failure](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-freeze-click.png?raw=true)).
Under this model, **selectable implies resolvable**: any row that can be pointed at must resolve to
a real `LocusId`. Freeze is disclosed instead as a clause on the Source locus's Reading — a zero-rate
`TimeMap` segment with its existing explain event ("TimeMap hold (rate 0)",
`Origin::Builtin { name }`), and its own named gap: Core has no TimeMap edit. No `Core Freeze`, no
synthetic identity, and freeze does not become undiscoverable.

**Timeline hit regions (M4), with the scrub overlap removed.** Three contracts, no region doing two
things: ruler and rail background *witness time* (scrub — playhead only, no locus change, no Undo);
a projected item body *points* (commits a Reading); an explicit handle or edit affordance *begins a
named gesture*. Keep rail-level `capture_any_mouse_down` and `gesture::hit_test`; add no
handle-local callback — `crates/lattice-studio/tests/layout.rs` already asserts rail-level capture
so that "clip children must not steal timeline pointer-down from the rail", and M4 says to exercise
the existing path first. The one overlap that exists today — rail background scrubbing *and*
re-pointing — is gone by §3.

One rest-state mismatch the contracts expose: `layout.rs` draws audio clips on the Audio rail, but
`hit_clips_on_track` clears every candidate there, so a clip-shaped block cannot be pointed at. The
model resolves it in the direction of honesty rather than erasure — audio placements already carry a
`timeline_span` and are already loci, so they become pointable, and their Reading carries the named
gap ("no legal time-scoped audio edit in Core"). A drawn block that cannot be pointed at, and a
pointable thing with a silently empty verb list, are the same lie told two different ways.

---

## 8. Mutation ledger (M1–M10)

| | Verdict | Why |
|---|---|---|
| **M1** projection inventory | **take** — it is the Reading's body | Absence clauses replace kind-specific forms. Rejected reading: inventory as panes. Canvas stays an `evaluate_at` surface, not a locus field. |
| **M2** independent locus and playhead | **take, strengthened** | Not just "compare" — scrub stops calling `point_from_timeline_time`, and coupling exists only as **Seek to here** / **Read the playhead**. No auto-seek, no second locus. |
| **M3** scope-labelled legal verbs | **take**, with fail-closed targets | §4. Deletes the project-wide `target_*_locus` fallthrough. Its falsifier partly bites on `Title`'s bundled variant; stated openly rather than papered over. |
| **M4** timeline hit-region split | **take, narrowed** | §7. Three contracts, no handle-local callback, and the scrub/point overlap removed at the source rather than by re-labelling. |
| **M5** definition/instance multiplicity | **reject as a mutation, absorb as a clause** | Multiplicity belongs in the Reading ("this definition projects to N clips: ids…"), bound by `TimelineClip.id`, never reverse-matched by text. Simultaneous highlighting is not the invariant, and the disputed rail crop settles nothing. |
| **M6** freeze honesty | **take Variant A** | §7. Non-selectable explanatory structure; selectable implies resolvable. |
| **M7** Review breadth without gating | **take** | §7. Keep the already-built `EditProposal`; Review is a projection with staleness, not a checkpoint. |
| **M8** role and width harness | **take as harness only** | It measures; it chooses nothing. The model's own testable claim is that one Reading stays complete when only one projection is visible. |
| **M9** Canvas absence taxonomy | **take, generalized** | §6. Absence is a Reading clause everywhere, not a Canvas special case, and geometry outlives pixels. |
| **M10** overlapping candidates | **take as policy** | §5. Rank → state → step. Specificity is interaction policy, not domain order. |

Rejected outright, and each for a reason already in the note: per-view semantic selection; a second
playhead locus; automatic seek on locus change; new `LocusKind`s or Placement/TimeMap panes;
`Core Freeze`; Audio-as-`SemanticEdit`; a handle-local trim callback; hide-VEL; readout-only
Timeline; colour or desaturation as a standalone mutation; any in-repo agent runtime or LLM SDK.

Rejected additionally by this model: the always-visible Inspector, the five-pane composition as the
product, and Space as the editor's identity.

---

## 9. The conventional assumption I broke

**Broken: NLE clip-as-object selection — that pointing yields an object, and that the object has a
property sheet.** Its entailment, the always-visible Inspector, goes with it.

Lattice's Core never had that object. `Locus` is a pointing with `Option` fields, not a thing with
properties. `TimelineClip` is a flatten artifact of `flatten_project`, not a document node.
`SemanticEdit` targets definitions, source bindings, scenes and sequences — never a clip id. The
object was introduced by the UI, and the chair note's shared assumption 2 — "the current Inspector
lies about legal edits" — is the recurring cost of maintaining it. The lie has three parts, each a
requirement of permanence: a permanent grid needs a subject (`adopt_locus_label`), a permanent field
needs a verb (the unconditional "Title text"), and a permanent verb needs a target
(`target_source_locus` → first match in the project). Remove the object and none of the three has
anywhere to live.

**Why the model still holds without it.** Every invariant survives, and several get stronger:

- One shared locus is still "here", still one `LocusId` in the session, and a Reading adds no
  identity of its own. Candidate cycling commits to that same field.
- Studio is still an Engine client. The Reading is derived in Engine and consumed identically by
  Studio, the CLI under `--json`, and an external agent. Legal mutations remain `SemanticEdit`s and
  source-backed rewrites, one per gesture, with Escape cancelling.
- Core stays GPUI-free: the type is data, the derivation is Engine, FFmpeg stays a backend, and
  Canvas geometry stays normalized.
- Project state stays text-first and Git-friendly; a Reading is volatile and adds no store.
- Nothing hidden: the model's only new obligation is to *say* what the current implementation
  already decides silently.

The one thing the model gives up is the reassurance of a box that is always there. That is a real
loss, and §11 says what it would cost.

---

## 10. Falsifier

One, and it is aimed at the spine.

> **On a fixture whose scene declares two bindings (`game[…] as a`, `bgm[…] as b`), take the verbs
> that currently retarget — `Trim`, `SetGain`, `SetFade` invoked while a Title locus is current, and
> `SetGain` invoked while a Sequence locus is current. If the fail-closed Reading form ("gain needs a
> source binding; scene `demo` declares 2 — point at one") produces *more* wrong or abandoned edits
> than today's silent first-match fallback, the spine is wrong.**

If that result comes back, "never rank where the consequence is a rewrite" is costing correctness,
the guess is doing real work, and the model must retreat to ranking-plus-after-the-fact-disclosure —
which is a materially weaker and less honest design that I would then have to accept. The measure is
committed-edit correctness against a stated intent, not preference and not time-on-task.

Each mutation I took also keeps its own falsifier from the chair note; this one is the model's.

---

## 11. What this model does not fix

- **Discoverability.** A closed verb list of two to six items, with absences named, is not a search
  box over a global command table — it carries scope, target, and reasons, which no palette does.
  But a user who learned "the box is always in the corner" loses that. If verbs must be summoned to
  be seen, some will not be found. This is the first thing to measure after §10.
- **Keystroke cost.** Candidate stepping is cheap only if the ties are few. Dense overlays could
  make it tedious; the fixture to test is duplicate overlays with identical spans.
- **Occlusion.** A Reading anchored to its witness can cover the picture it explains. That is a real
  layout problem this note deliberately does not solve, because solving it in pixels is not a model.
- **Core gaps stay gaps.** Naming "no time-scoped audio edit" and "no TimeMap edit" makes them
  observable. It does not fill them, and this model must not be read as license to invent either.
- **`Title` variant bundling** (§4) remains an open Core question.

---

## 12. Answers to the chair's participant hooks

1. **Delete first:** M5 as a standalone mutation. It experiments with highlighting, but Challenge 1
   already establishes that simultaneity is not the invariant, and its strongest fixture is
   semantically disputed. Its content survives as a Reading clause.
2. **Predicted scopes (M3, M5):** `Title{text}` = definition text. `Title{at,duration}` = this
   placement's timing. `Title{opacity}` = this placement. `Callout{at,duration}` = this placement's
   timing. `SetPosition`/`ResizeOverlay` = this placement's geometry. `Trim` = this source binding's
   in/out. `Split`/`Delete` = this scene. `ReorderScene` = this sequence's order. `SetGain` = this
   scene's named source. `SetFade` = this source's video fade-in, anchored at scene start.
3. **Coupling (M2):** only two, both explicit, both temporary, neither persisted: **Seek to here**
   (playhead ← locus) and **Read the playhead** (locus ← candidate at `t`). Scrub does not couple.
4. **Regions (M4):** *scrub* — the ruler and empty rail background on every rail. *point* — a
   projected clip body on the Video, Text and (newly) Audio rails. *mutate* — trim handles on video
   clips, and body or edge drags on title and callout clips. No region does two; scrub no longer
   also points, and no rail draws a block it refuses to be pointed at.
5. **Freeze (M6):** not directly selectable. It is a zero-rate `TimeMap` segment on a Source, so it
   has no independent source span, no timeline span of its own, and no Core edit. It is disclosed on
   the Source locus's Reading with its existing explain event. Selectable implies resolvable.
6. **Specificity (M10):** interaction policy, not domain ordering — §5, with the tie fixture that
   distinguishes the readings.
7. **Falsifier:** §10.
