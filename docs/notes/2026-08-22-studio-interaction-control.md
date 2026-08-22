# Studio CONTROL model

Date: 2026-08-22

Status: Phase IV isolated author; one interaction model, not a vote and not an implementation

Scope: honesty and contract cleanup on **current** Studio chrome. Docs only. No Studio UI / GPUI code. No HTML sketch. No new pane grammar.

Chair (source of truth, read in full before this note):
[2026-08-22-studio-interaction-chair-note.md](https://github.com/annenpolka/lattice/blob/3fc72b7eaaf3c036ec3900510edf97ebdb330c1d/docs/notes/2026-08-22-studio-interaction-chair-note.md)

This note does not prototype gen1 candidates A–F. It answers the chair’s mutation ledger from one CONTROL stance: keep the Alpha shell recognizable, and stop the shell from lying about Lattice’s domain.

## Invariants (do not break)

- One shared locus is “here” across VEL, Canvas, Timeline, Review, SEQUENCE, and agent context. A view may have focus, hover, a playhead, or ephemeral gesture state. It does not own another semantic selection.
- Studio is an Engine client. Legal mutations are `SemanticEdit`s and source-backed rewrites.
- No GPUI in Core. FFmpeg is a backend. Project state stays text-first and Git-friendly.
- No in-process agent runtime or LLM SDK. Agent context is `locus + instruction`.
- No hidden magic. Every expansion remains explainable. A widget convenience is not permission to warp Core nouns (`LocusKind`, TimeMap, Freeze, AudioPlan).

## What this model is

CONTROL is the current Studio **named**.

The Alpha window already has a pane grammar, a gesture lifecycle, a shared `LocusId`, and an Engine propose/apply path. Those are the product. The failure is that several surfaces **pretend** to be something they are not: Inspector pretends every locus is a title; Scrub pretends time navigation is pointing; a SEQUENCE row pretends freeze is a locus; Review pretends only title text can be inspected; teal pretends every chrome role is “here.”

CONTROL keeps SEQUENCE | Canvas | VEL | Inspector | Timeline, Review nested in Inspector, and the wrapping command strip. It changes the contracts those pixels obey. It is one model, not a bug-fix list: every cleanup below follows from the same two clocks and the same verb source.

## What this model is not

- Not a new editor, NLE skin, design-tool skin, or IDE skin.
- Not hide-VEL, readout-only Timeline, automatic seek, a second playhead locus, `Core Freeze`, a time-scoped audio `SemanticEdit`, or a per-view selection.
- Not five-pane synchronization as an invariant. Coherence must survive a missing projection.
- Not gen1 candidates A–F restated as a mash-up.

## The model

Lattice Studio has two named clocks and one verb source.

| Clock or source | Name | Meaning |
|---|---|---|
| Semantic clock | **locus** | What I mean. One `LocusId` on the session. |
| Temporal clock | **playhead** | When I am looking. Session transport; `evaluate_at` time. |
| Verb source | **Engine** | The legal `SemanticEdit`s for that locus, plus available projections. |

Pointing writes the locus. Scrubbing writes the playhead. A named gesture or command may commit at most one `SemanticEdit` → one VEL rewrite → one compile → one Undo. Review may inspect an `EditProposal` that Engine already built. Review is never a gate.

If a surface cannot say which of those three it is doing, it is lying.

```text
point  → writes locus. Does not rewrite VEL. Does not seek.
scrub  → writes playhead. Does not rewrite VEL. Does not re-point.
verb   → one SemanticEdit, or a disclosed retarget, or a named refusal.
```

## Chrome that stays

Current Alpha grammar, unchanged as a layout claim:

```text
[ Lattice  ·  file.vel ]
[ Open Video · Set In/Out · Split · Delete · renderer/audio status · CPU · GPU
  Play · Pause · Seek · Scrub · Save · Undo · Redo · Resolve · Copy locus JSON
  Gain · Fade · Zoom ]

[ SEQUENCE | Canvas | VEL | Inspector ]
[ Timeline: Video / Audio / Text rails + playhead ]
```

Review remains a slot **inside Inspector**, as today. VEL remains a pane. Navigate (`Go to definition`, Copy locus JSON) remains optional. The boards that freeze the older capability story stay at [`docs/interaction.md`](../interaction.md); this note proposes the CONTROL contracts that current chrome should actually obey.

Header chrome today hardcodes `Scene demo` beside the file name even when the locus is a title. CONTROL: the header is project/file identity only. “Here” lives on the Inspector heading and the shared locus mark. The header is not a second selection.

## 1. What “here” is

“Here” is one `StudioSession` `LocusId`. Engine `inspect` / `LocusProjection` is how every client reads it. SEQUENCE, Canvas overlay, VEL caret highlight, Timeline clip mark, Inspector heading, Review `proposal.locus_id`, and `Copy locus JSON` all project that id.

A locus is a Core noun: `title`, `callout`, `source`, `placement`, `scene`, `sequence`, `media`, `speech`. It is not a tree row, not a teal rectangle, not a white clip border count, and not the playhead.

One definition may project to many rendered instances. Shared identity explains the relation. It does not make every instance the mutation target. Scope is the `SemanticEdit`, not the highlight.

Focus, hover, insertion marker, in-flight geometry, and playhead are **not** loci. They may coincide with the locus. Coincidence is not identity.

## 2. Playhead versus locus

These are independent by default.

| | Locus (“what I mean”) | Playhead (“when I am looking”) |
|---|---|---|
| Stored as | `LocusId` | `Time` |
| Written by | Point: SEQUENCE row with a real id, Canvas overlay, clip body, VEL caret, agent context | Scrub (ruler / rail background), Play / Pause / Seek / Scrub, Space-as-transport |
| Reads | Inspector, Review, Copy locus JSON, overlay bind, agent | Canvas `evaluate_at`, mix, overlay visibility |
| Rewrites VEL? | No | No |
| Pushes Undo? | No | No |

Current Studio couples them in two hidden places:

1. Scrub commit calls `point_from_timeline_time`, which collapses overlapping candidates with `specificity()/max_by_key` and replaces “here.” Audio-rail navigation can mint a Title locus as a side effect of looking.
2. `point_at` calls `sync_playhead_to_current`, so a SEQUENCE title click seeks the playhead into the title span. Timeline `point_scene` / `point_clip` do **not** seek. The same verb (“point”) is not one contract.

CONTROL takes chair **M2** and refuses both hidden couplings.

- Scrub, toolbar Seek / Scrub, and Play stepping move **only** the playhead.
- Point writes **only** the locus.
- When the locus has a timeline range that does not contain the playhead, Canvas names that fact and offers an explicit **Seek to locus**. That seek is a playhead write. It is not persisted as a second locus and not implied by pointing.

No automatic seek on locus change. No second playhead locus. Neither variant rewrites VEL.

## 3. How legal verbs appear

Kind is context. Kind is not the action switch.

Inspector is not a title form. It is the Engine readout for the current locus:

1. **Identity** — kind, label, `LocusId`.
2. **Provenance** — origin and defined-in. Always present; never a checkpoint.
3. **Projection inventory** (M1) — source span, timeline range, visual, TimeMap / freeze, scene/sequence context. Each field is present or **named absent** with a reason. No new pane per noun. Canvas is not a locus field; it is `evaluate_at` at the playhead.
4. **Legal verbs** (M3) — only edits Engine will accept for this locus, each labelled by scope.

### Scope labels (1→1 with existing `SemanticEdit`)

| Verb | Scope label | Legal on |
|---|---|---|
| Title text / title timing | definition | `LocusKind::Title` |
| Callout timing | this placement | `LocusKind::Callout` |
| `SetPosition`, `ResizeOverlay` | this placement | Title, Callout |
| `Trim` | this source (clip / time range) | Source |
| `SetGain` | this source (audio; **no Time scope**) | Source |
| `SetFade` | this source (video opacity envelope, not audio) | Source |
| `ReorderScene`, `Split`, `Delete` | scene order / scene | Scene |
| none | — | Sequence, Media, and any locus with no legal edit: say why |

Do not invent a time-scoped audio verb so Gain can look like Trim. `SetGain` has no Time field. `SetFade` is not that gap.

### Inspector honesty: title fallback and `adopt_locus_label`

Today the Inspector always paints a “Title text” field and `Apply edit` / `Review`. `adopt_locus_label` copies `locus.label` into that draft for **every** locus, including scene, source, and callout. A scene named `demo` becomes title-shaped. A callout labelled `Hold` becomes a title draft.

CONTROL:

- The title draft, `inspector.title`, `Apply edit` (title), and title `Review` exist only when the current locus is `LocusKind::Title`.
- `adopt_locus_label` copies `visual.text` (else the title label) and only then.
- A non-title locus shows its inventory and its own verbs, or “no mutation is legal: …”.
- Callout keeps callout verbs. It does not borrow Title.

### Toolbar retarget must be disclosed

`target_source_locus` / `target_scene_locus` can fire Gain, Fade, Trim, Split, or Delete against a noun that is **not** the displayed locus, and they can leave the displayed locus unmoved. That is a hidden second target, not context-sensitive kindness.

CONTROL keeps the current buttons (same strip). Before or as they apply:

- if the displayed locus can accept the edit, apply to that locus;
- else if a unique related target exists (scene’s source, title’s scene), **name it** (“Gain applies to source `fight`, not sequence `main`”) and apply;
- else fail closed.

Fail-closed is preferred when the related target is not unique. Helpful fallback and hidden second target are the same pixels until the target is spoken.

Direct Canvas / Timeline gestures already point before they commit (`point_clip` / overlay id). They stay fail-closed on kind: move/resize only title and callout.

## 4. Timeline regions: point, scrub, or mutate

Chair contradiction 1 is real: Timeline is not a readout and not a hidden mode. CONTROL takes **M4** as the contract already implied by rail-level hit testing, then removes the two silent doubles.

| Pixel region | Contract | Today | CONTROL |
|---|---|---|---|
| Ruler / rail **background** | scrub | scrub | scrub (playhead only) |
| **Video** clip body | point; drag past threshold begins `ReorderScene` | click points **scene**, discards clip id | point the **Source** (else Placement) that owns the clip; drag still reorders the scene |
| **Text** clip body | point that Title/Callout; drag moves overlay timing | point clip; drag `Title`/`Callout` timing | same |
| **Audio** clip body | point Source / Speech | track filter **clears clips**, so the body is always rail-scrub | point; rail background still scrubs |
| Trim / overlay **handle** | begin named trim / resize | rail capture + `hit_test`; no handle-local callback | same implementation; rest-state handle means this contract |
| Playhead line | display of the temporal clock | teal, same as Text clips and locus chrome | display only; not a hit target that points |
| Insertion / snap marks | ephemeral gesture | yellow / white | unchanged roles |

No region silently does two of {scrub, point, mutate} on pointer-down. Drag-after-point is allowed: pointer-down points (or begins a named gesture), pointer-move is ephemeral, pointer-up commits at most one edit. Escape cancels. Scrub has no Undo.

Audio-rail “always scrub” is current policy (`hit_clips_on_track` clears Audio clips). CONTROL treats that as a lie about a clip-shaped pixel, not as domain law.

Video→scene is the same class of lie: the user hit a clip and received a scene. Scene remains one click away on SEQUENCE. Clip identity stays available for Trim / Gain.

## 5. Overlap policy (M10)

Session stores one `LocusId`. `locus_at_timeline` / `locus_at_source` manufacture singularity with `max_by_key((specificity, tighter span))`. `Locus::specificity` is an ordinal (title/callout/speech = 4 … sequence/media = 0). Core does not claim overlapping nouns are totally ordered. That ranking is **interaction policy**, not a discovered domain fact.

Evidence that distinguishes the readings: the ordinal exists to break lookup ties; it is not a provenance, TimeMap, or explain property. A Title and a Scene overlapping at 3s remain two nouns after ranking. The rank only chooses which id the session keeps.

CONTROL **rejects M10 as a surface**. A candidate picker is a new editor affordance. Current chrome has no such pane, and CONTROL does not add one.

CONTROL **takes the diagnosis**:

- Ordinary pointing binds the **hit target** (section 4). Timeline-time collapse is not consulted.
- Scrub no longer points, so the common silent collapse (Audio rail → Title) disappears.
- VEL caret may still use `locus_at_source`. Nested source spans are definitional; specificity-plus-tighter-span is an acceptable policy there because the user pointed at bytes, not at a time with several projected nouns.
- After any point, every projection receives the same `LocusId`. Candidate lists are not project state.

If a later phase needs disambiguation, it must still commit one shared locus. That is a different model.

## 6. Freeze: drop the row

Freeze is a TimeMap rate-0 segment on a Source. It is already a temporal transformation, already explainable, already held by preview frames. It is **not** a `LocusKind`.

SEQUENCE currently emits a synthetic child `freeze:{source.id}` (e.g. `freeze:source:clip`) with `kind: "freeze"`. The row is never `selected` (id is not a `LocusId`). Click still calls `point_at` with that string. Inspect fails; layout fails. That is not a freeze feature. It is an unknown locus.

CONTROL takes **M6 variant A**: delete or ignore the unresolved row. Do not add `Core Freeze`. Do not mint `LocusKind::Freeze`.

Discover freeze on the **source** inventory: “TimeMap hold at 5.2s for 1.5s” (from existing segments), and via `lattice explain`. The callout/speech that share that interval remain their own loci.

Variant B (selectable freeze row) is rejected unless someone can reconstruct a real Core locus with source span, timeline range, provenance, and explain **without** distorting Core. The synthetic id already proved that test fails.

## 7. Review reuses the proposal; Review is not a gate

`apply_edit` / `apply_committed` already call `engine.propose`, then apply, then drop the `EditProposal`. Review is populated only by `propose_title_text`. Direct Trim / SetPosition / title Apply never appear there. The Review slot is title-shaped even when the last legal edit was not.

CONTROL takes **M7**.

Keep the proposal Engine already built. The existing Inspector Review slot shows one of two states:

| State | How it got there | Source | Actions |
|---|---|---|---|
| **Pending** | Review button / `propose_*` | unchanged until Apply | Apply / Reject (today’s contract: `base_revision` must still match) |
| **Committed evidence** | last `apply_committed` | already rewritten | no second Apply; Undo reverts; Reject is not a second history |

What Review shows, for any legal edit that actually produced a proposal (title text, Trim, SetPosition, ResizeOverlay, ReorderScene, …):

- current picture = the Canvas already on screen (`evaluate_at` at the playhead), named as current, not as a preview of `new_source`;
- semantic effect + **scope** from the `SemanticEdit`;
- `locus_id`;
- source diff (`vel_diff`).

Do not add Gain or TimeMap to Review unless that edit was proposed. Do not build a second compositor. Do not route everyday gestures through Review. Manipulate may finish on pointer-up. Agent paths may still Propose → Review → Apply. Those are two legal paths over the same noun, as [`docs/interaction.md`](../interaction.md) already says.

A stale pending proposal (source moved underneath `base_revision`) is named and rejected on Apply. That is existing Engine behavior. Surface it.

## 8. Space binding

Space is unbound as transport today. In VEL and the title draft it is a character. That absence is a missing **binding**, not a domain hole.

CONTROL: Space toggles Play / Pause **iff** keyboard focus is not a text-entry surface (`source_focus`, `title_focus`, and any later text field). When focus is text-entry, Space inserts a space.

This is chair F’s conventional binding under explicit focus rules. It is not a Core requirement. It does not rewrite VEL, does not re-point, does not seek, and does not skip the existing “Play waits for required PCM” rule. Play / Pause buttons stay. Space is another way to say those verbs.

## 9. How absence is disclosed

Empty Canvas is not one problem. CONTROL takes **M9**. Name the cause; do not style the causes into one black.

| State | Disclosure | Must not look like |
|---|---|---|
| Locus outside playhead | “not visible at {playhead}; active {start}–{end}” + Seek to locus | preview-off, layout fail |
| No visual projection | “this {kind} has no visual at any time” (Sequence, Media, some Sources) | broken preview |
| Renderer initializing | initializing | media missing |
| Layout / inspect failure | typed error (today’s freeze-click `layout failed`) | temporal absence |
| Preview disabled | `LATTICE_STUDIO_PREVIEW=0` (or equivalent) named | locus-outside-playhead |
| Media unavailable | typed missing source | implicit silence / black success |
| Compile diagnostic | show the diagnostic | empty stage |
| Typed renderer failure | DX12/CPU error stays typed; no silent CPU fallback | “preview just off” |

Keep overlay chrome when geometry is known and frame pixels are temporarily unavailable. `overlay_playhead_visible` is a span-vs-playhead test. It is not a proxy for `preview_image`. Preview-off must not hide a locus overlay that the plan already placed.

VEL may sliver or clip under width pressure (committed 800 capture shows missing Inspector/VEL; mechanism is unproven). CONTROL does not adopt a drop-pane policy and does not hide VEL. Navigate stays optional; source must remain explicable.

## 10. Teal-role, not a new skin

Teal (`#3dd6c6`) currently marks pane titles, filled commands, SEQUENCE / VEL selection, playhead, Text clips, Inspector accents, and Save / Apply / Resolve. Unrelated roles compete. That is chair C/D’s collision, not a request for a familiar-app reskin.

CONTROL takes **M8 as an audit**, not as a palette project. Withdraw teal from every role except one.

**Teal means the shared locus / “here.”**

| Role | Signal | Teal? |
|---|---|---|
| Locus mark (SEQUENCE row, overlay, Inspector identity) | teal text / fill | yes |
| Playhead | distinct transient line (not Text-clip fill) | no |
| Media kind (Video / Audio / Text rails) | rail-local hues; Text must leave teal | no |
| Primary commit (Save, Apply, Resolve) | filled button, non-teal token | no |
| Transport (Play / Pause / Seek / Scrub) | transport chrome, not locus | no |
| Destructive (Delete, Reject, errors) | existing red | no |
| Status (renderer / audio) | muted / error red (already) | no |
| Insertion marker | existing white | no |
| Snap | existing amber | no |
| Pane labels (`SEQUENCE`, `Inspector`, …) | muted chrome | no |

Same panels. Same buttons. Same three rails. Color answers “which interaction role is this?”, not “what product are we quoting?”. Width captures at 1400 / 1024 / 800 remain a measurement harness. They do not prove a 1024→800 causal chain and they do not pick a responsive redesign.

VEL’s static look is an affordance mismatch (`StudioSourceInputHandler` already edits). CONTROL: a focused VEL surface looks like text entry (caret / focus ring). That is role honesty, not a new editor.

## 11. Multiplicity without a new mode

Chair **M5** asks whether to emphasize definition or instance. CONTROL **rejects M5 as a mode switch**. Current chrome has no definition/instance toggle; adding one is a new editor.

CONTROL’s multiplicity rule is the highlight contract already implied by “one locus, several projections”:

- A clip is locus-marked iff it **is** the current locus (`clip.id` / `node_id`) or, for a Source locus, the clip’s placement binds that source.
- A Scene locus marks a scene envelope / tree row, not every child clip as the selected object.
- Title locus marks the title clip and the title overlay, not Video+Audio as if they were the title.

Simultaneous five-pane highlight is teaching, not an invariant. One visible projection plus a discoverable inventory is enough. Do not reverse-match by visible text. Duplicate labels are legal.

The disputed “Title → three clips” rail crop is not used as identity evidence. Scope is predicted from the verb table, not from white-border count.

## Mutation ledger

| Id | CONTROL | Why |
|---|---|---|
| **M1** inventory | **Take** | Disclose fields Engine already has. No new panes or kinds. Canvas stays `evaluate_at`. |
| **M2** locus ⊥ playhead | **Take** | Scrub does not call `point_from_timeline_time`. Point does not `sync_playhead_to_current`. Optional explicit Seek to locus. |
| **M3** scoped verbs | **Take** | Replaces title fallback. `adopt_locus_label` is Title-only. Toolbar retarget is named or fail-closed. |
| **M4** hit-region split | **Take** | Three non-overlapping pointer contracts. Audio body points. Handles stay rail-hit-tested. |
| **M5** definition/instance probe | **Reject as a UI mode** | Would add a second editor. Highlight leak is fixed by the mark rule; scope stays on `SemanticEdit`. |
| **M6** freeze | **Take A (drop row)** | Synthetic `freeze:source:clip` is not a locus. No `Core Freeze`. |
| **M7** Review breadth | **Take** | Retain `EditProposal` from `apply_committed`. Review stays optional inside Inspector. |
| **M8** role/width | **Take as harness** | Teal-role only. No skin. No wrap-causal story. |
| **M9** Canvas absence | **Take** | Named empty states. Overlay chrome survives missing pixels. |
| **M10** overlap picker | **Reject as a surface** | Specificity is policy, not domain law. Hit-target pointing + no scrub-repoint is the CONTROL resolution. A picker is new chrome. |

Dropped with the chair, still dropped here: hide VEL; readout-only Timeline; desaturation-as-design; new Placement/TimeMap panes or kinds; second playhead locus; Audio-as-new-`SemanticEdit`; handle-local trim callback; automatic seek.

## One falsifier

On `examples/gameplay-commentary/main.vel`, with current chrome (same five panes):

1. Point the **scene** (SEQUENCE `scene demo`) at playhead `0s`.
2. Scrub the **Audio rail background** through the title’s active interval.
3. Do not click a title clip, title overlay, or title tree row.

CONTROL is false if any of the following hold after step 2:

- the shared `LocusId` is no longer the scene (Scrub re-pointed via `point_from_timeline_time` / specificity);
- Inspector shows a Title text field populated from `adopt_locus_label`;
- a participant cannot say, **before** pointer-down on a later pixel, whether that pixel will move only the playhead, re-point “here”, or commit a `SemanticEdit`, and cannot name that edit’s scope (definition / this placement / this source / scene order).

A prettier Inspector that still applies Title to a Scene also falsifies the model. A freeze tree row that is selectable without a real Core locus also falsifies it. A Review that becomes mandatory for Trim / SetPosition also falsifies it.

## Implementability (notes only; no code in this PR)

All of this is reconstructible from existing Engine/Studio types. No Core noun changes. No GPUI types in Core.

| Contract | Current hook | CONTROL change (later code PR) |
|---|---|---|
| Title-only draft | `StudioView::adopt_locus_label`; Inspector always paints `inspector.title` | Gate on `LocusKind::Title`; copy `visual.text` |
| Scrub does not point | `interaction::commit` Scrub arm; `StudioSession::click_timeline` | Drop `point_from_timeline_time`; keep playhead write |
| Point does not seek | `point_at` → `sync_playhead_to_current` (tree, canvas). `point_scene` / `point_clip` already skip seek | One point contract: no seek. Add an explicit Seek-to-locus command |
| Hit-target point | Video commit `point_scene`; Audio `hit_clips_on_track` clears clips | Video click → Source/Placement id; Audio body participates in hit test |
| Freeze row | `layout::tree_from_compilation` `freeze:{source.id}` | Stop emitting the row; show TimeMap on source inventory |
| Review retains proposal | `apply_edit` builds `EditProposal` then drops it; `review` is title-only | Keep last proposal; ReviewView pending vs committed |
| Toolbar retarget | `target_locus_for` / `target_source_locus` / `target_scene_locus` | Disclose or fail closed; do not move displayed locus as a side effect |
| Space | `handle_key` has no `" "` | Toggle play/pause when focus ∉ text-entry |
| Teal-role | `TEAL` on pane titles, Play, playhead, Text clips, Save, … | Teal token only on locus mark |
| Canvas absence | `preview_image` early-return hides overlays | Separate preview-off / temporal absence / typed errors; keep overlay when geometry exists |
| Header | `header_bar` `"{file} · Scene demo"` | File/project only |

Tests that would lock the model later (not in this PR): Engine/session unit tests for “scrub does not change `current`”; Inspector/layout tests that a Scene locus has no `inspector.title`; tree tests that `freeze:*` is absent and source inventory names the hold segment; Review tests that Trim/`SetPosition` proposals survive `apply_committed` without becoming a gate; `VisualTestContext` Space-vs-VEL-focus; no new Core TimeMap algebra (freeze remains rate 0).

## Chair reply hooks

1. **Delete first:** M10-as-surface, then M5-as-mode. A picker and a definition/instance toggle violate “current chrome” and invent UI-owned selection/mode on top of one `LocusId`.
2. **M3 / M5 scopes:** table in §3. Highlight count is not scope. Title text = definition. Move/resize = this placement. Trim/Gain = this source. Reorder/Split/Delete = scene.
3. **M2 coupling:** none automatic. The only allowed coupling is an **explicit** Seek to locus (temporary playhead write, not persisted as locus state).
4. **M4 regions:** table in §4. Audio body is point, not a second scrub. No region does two of {scrub, point, mutate} on pointer-down.
5. **M6:** freeze is not directly selectable. Reconstruct it from Source TimeMap + explain. The synthetic row is dropped.
6. **M10:** specificity is interaction policy. The distinguishing evidence is that the ordinal lives on lookup only and is not a Core semantic of overlap. CONTROL removes the lookup from scrub/point-at-time rather than promoting it to a pane.
7. **Falsifier:** § above. Not a skin. Not a pane arrangement.

## Evidence (committed chrome; not new captures)

These shots establish current failure modes. They are not a replacement design. URLs are commit-pinned.

Title-shaped Inspector on a scene locus:

![Scene locus with Title text](https://github.com/annenpolka/lattice/blob/75c4674c6fa123451c69fa93de8c780eeb8dbb26/docs/screenshots/studio-observe-a-scene-locus.png?raw=true)

Callout held, Inspector still Title text:

![Callout with Title field](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-callout-hold.png?raw=true)

Synthetic freeze row after click:

![Freeze click layout failure](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-freeze-click.png?raw=true)

![Freeze row before click](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-freeze-node-before.png?raw=true)

Audio-rail scrub replacing “here” with a title:

![Audio rail scrub to title](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-audio-rail-scrub-to-title.png?raw=true)

Playhead outside title locus; Canvas empty at 0s (cause not unique from the pixels alone):

![Title at 0s empty Canvas](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-open-title-0s-empty-canvas.png?raw=true)

![Playhead outside locus](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-playhead-outside-locus.png?raw=true)

Title-only Review, no picture of the proposal:

![Title-only Review](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-review-title-propose.png?raw=true)

Toolbar Gain leaving the displayed locus unmoved:

![Gain fallback](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-toolbar-2026-08-22-after-gain-vel-inspector.png?raw=true)

Current pane grammar at default width (CONTROL keeps this grammar):

![Default 1400×840](https://github.com/annenpolka/lattice/blob/32da848e5bb38c10d0bd887d74c528d463ab300f/docs/screenshots/01_default_layout_1400x840.png?raw=true)

Packet 5’s uncommitted 640px VM measure is excluded, per the chair.
