# Studio interaction model chair note

Date: 2026-08-22

Status: discussion input; observation and reconstruction only

Scope: interaction-model mutations, not a UI proposal

## Chair frame

This note does not select a candidate or preserve the current pane layout. It extracts claims that
can be varied independently and tested against Lattice's domain model.

The fixed boundary is:

- One shared locus is “here” across VEL, Canvas, Timeline, Review, and agent context. A view may have
  focus, hover, a playhead, or ephemeral gesture state, but it does not own another semantic
  selection.
- Studio is an Engine client. Legal mutations are `SemanticEdit`s and source-backed rewrites, not
  GPUI-shaped domain shortcuts.
- Core remains free of GPUI, FFmpeg remains a backend, and project state remains text-first and
  Git-friendly.
- External agents receive locus plus instruction. Studio does not acquire an agent runtime or LLM
  SDK.
- Magic may compress work, but the expansion and its origin must remain explainable.
- A UI observation is not permission to change domain semantics to make a widget convenient.

The screenshots establish real failure modes, but not by themselves the correct replacement.

## Candidate extraction

### Candidate A — implementation-grounded

- **KEEP:** Treat one session and one `LocusId` as the semantic selection authority. Keep the
  observations that scene loci receive title-shaped Inspector controls, the freeze row can produce
  `freeze:source:clip` and then fail layout, and Review currently shows a title/source diff without
  the affected picture.
- **CHANGE:** Turn “Inspector by `LocusKind`” into “actions and projections supported by this locus.”
  Kind-switching alone can still produce a large closed set of special-case forms. Treat hiding VEL
  as an optional projection policy, not as removal of source/provenance access.
- **DELETE:** Delete synthetic selectable rows whose identity cannot resolve to a Core locus. Delete
  title-shaped editing and title-only Review for non-title loci.
- **STEAL:** Steal cheap forks as an evaluation method: vary one projection or action policy while
  leaving Engine semantics fixed. Steal the demand that Trim, SetPosition, and audio changes reach
  Review as meaningful proposals, but do not make Review a gate for direct manipulation.
- **MISREAD:** “Audio rail on `SemanticEdit`” conflates pointing in time with changing the project.
  Scrubbing is transient playhead state and must not rewrite VEL. An explicit gain/timing gesture may
  commit a `SemanticEdit`; an ordinary rail click must not secretly do so.
- **UNSTATED ASSUMPTION:** A permanent Inspector and a hidden VEL pane are assumed to be the best
  homes for locus-specific actions and provenance. Neither follows from the one-locus model.

Evidence: [title locus and stale header](https://github.com/annenpolka/lattice/blob/75c4674c6fa123451c69fa93de8c780eeb8dbb26/docs/screenshots/studio-observe-a-title-locus.png?raw=true),
[scene locus with title control](https://github.com/annenpolka/lattice/blob/75c4674c6fa123451c69fa93de8c780eeb8dbb26/docs/screenshots/studio-observe-a-scene-locus.png?raw=true),
[freeze click failure](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-freeze-click.png?raw=true),
[title-only Review](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-review-title-propose.png?raw=true).

### Candidate B — semantic

- **KEEP:** Keep the formulation “one locus, several projections” and the distinction between locus
  and playhead. Keep Source, Placement, TimeMap, and current-time visibility as missing perceptual
  projections. Keep the finding that the freeze tree node is not currently a valid Core locus.
- **CHANGE:** Replace “every projection must be visibly present” with “every available projection is
  discoverable, and absence is explained.” A locus can legitimately have no Canvas projection at the
  current playhead or no editable placement.
- **DELETE:** Delete any visual implication that moving the playhead changes the locus, or that
  changing the locus must seek the playhead. Delete the unresolved freeze node rather than masking
  its failure.
- **STEAL:** Steal the before/after freeze reconstruction and the side-by-side current-picture,
  semantic proposal, and source-diff framing for Review.
- **MISREAD:** A missing picture at `0s` while a title active at `1s` is not necessarily a broken
  projection. The break is failing to explain “not visible at current playhead” or offering a hidden
  auto-seek, not the temporal absence itself.
- **UNSTATED ASSUMPTION:** Users need Source, Placement, TimeMap, and picture simultaneously. The
  domain requires those facets to be reachable and coherent, not permanently tiled.

Evidence: [shared locus](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-shared-locus.png?raw=true),
[playhead outside locus](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-playhead-outside-locus.png?raw=true),
[freeze before](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-freeze-node-before.png?raw=true),
[freeze after](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-freeze-node-after.png?raw=true),
[Review with current Canvas](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-review-diff-current-canvas.png?raw=true).

### Candidate C — visual hierarchy

- **KEEP:** Keep the observations that high-saturation teal makes unrelated actions compete, an
  empty-black Canvas communicates neither loading nor temporal absence, VEL reads as static, and the
  1024-wide toolbar wraps without preserving command grouping.
- **CHANGE:** Convert the color critique from “less teal” to “assign color by interaction role.”
  Selection, playhead, primary commit, renderer state, and media kind must not borrow one signal.
  Convert “VEL looks static” into a state/affordance question: read-only projection, editable source,
  focused range, and stale source need distinguishable treatment.
- **DELETE:** Delete equal visual weight for global commands, transient transport, semantic edits,
  status, and destructive actions. Delete unlabeled empty states.
- **STEAL:** Steal compact-width screenshots as a constraint test for every semantic mutation.
- **MISREAD:** Canvas blackness is not purely a hierarchy defect. It can represent an unavailable
  frame, a locus outside the playhead, renderer initialization, or a failed layout; styling cannot
  safely merge those states.
- **UNSTATED ASSUMPTION:** The existing five-pane composition and top toolbar remain fixed while only
  visual weight changes. The layout is not a domain invariant.

Evidence: [default 1400×840](https://github.com/annenpolka/lattice/blob/32da848/docs/screenshots/01_default_layout_1400x840.png?raw=true),
[compact 1024×768](https://github.com/annenpolka/lattice/blob/32da848/docs/screenshots/08_window_compact_1024x768.png?raw=true).

### Candidate D — affordance

- **KEEP:** Keep the five-surface synchronization as evidence that the shared locus already has a
  strong perceptual signal. Keep the findings that playhead and Text clips share a color, the ruler
  lacks a distinct scrub affordance, renderer/audio status is mixed into commands, and 800px silently
  removes useful projections.
- **CHANGE:** Preserve cross-projection coherence without requiring five simultaneous panes. Give the
  playhead a unique transient signal and expose transport hit targets separately from clips and trim
  handles. Move status out of action labels while keeping typed renderer failure observable.
- **DELETE:** Delete width-driven disappearance with no indication of hidden facets. Delete color as
  the sole carrier of locus, playhead, media kind, or action priority.
- **STEAL:** Steal in-flight versus committed scrub comparison as a state-transition test, and the
  800px capture as a disclosure test.
- **MISREAD:** The strong signal is not “five panes” itself; it is coherent projection from one
  semantic target. Five simultaneous highlights can also amplify a wrong or stale locus.
- **UNSTATED ASSUMPTION:** Selection synchronization is always more important than the distinction
  among locus, keyboard focus, hovered projection, playhead, and ephemeral proposal.

Evidence: [scrub committed](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/08_timeline_scrub_committed.png?raw=true),
[scrub in flight](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/07_timeline_scrub_in_flight.png?raw=true),
[title locus across panes](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/01_overview_title_locus_selected.png?raw=true),
[ultracompact 800×600](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/17_window_ultracompact_800x600.png?raw=true).

### Candidate E — first principles

- **KEEP:** Keep “one semantic definition may have many rendered projections” as the key departure
  from object-centric NLE selection. Keep the proposal that legal verbs come from the locus and
  become `SemanticEdit`s. Keep the toolbar-fallback observation.
- **CHANGE:** Treat `LocusProjection` as a description to reconstruct and test, not a new UI-owned
  domain object. Change “timeline is a readout” to “timeline is a projection that may also host
  explicit semantic gestures”; current contracts require trim, reorder, and timing edits there.
- **DELETE:** Delete toolbar actions that silently reinterpret a scene or sequence as a title. Delete
  the claim that one-locus/many-looks and many-clips/one-look are mutually exclusive editor classes;
  Lattice still has clips and instance-bound edits.
- **STEAL:** Steal the 1→many projection fixtures and require each mutation to state whether it edits
  a definition, one instance, a placement, or a TimeMap.
- **MISREAD:** Three highlighted spans do not prove that a single action should mutate all three.
  Shared semantic identity explains the relation; edit scope still depends on the legal
  `SemanticEdit`, stable clip identity, scene/sequence context, and provenance.
- **UNSTATED ASSUMPTION:** A facet model automatically yields understandable controls. It does not
  answer priority, disclosure, conflict among projections, or how unavailable facets are explained.

Evidence: [title locus to three clips](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-selection-2026-08-22-title-locus-3-clips.png?raw=true),
[scene locus to two clips](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-selection-2026-08-22-scene-locus-2-clips.png?raw=true),
[video click selects scene](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-videoclick-2026-08-22-after-scene-locus.png?raw=true),
[toolbar fallback](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-toolbar-2026-08-22-after-gain-vel-inspector.png?raw=true).

### Candidate F — interaction

- **KEEP:** Keep the observed mapping from Video click to scene locus; always-scrub Audio rail;
  title-only Inspector/Review; temporally empty Canvas; unbound Space; non-interactive drawn trim
  handles; and overlay chrome disappearing when preview is off. These are separate defects or policy
  decisions, not one solution.
- **CHANGE:** Define explicit contracts for each hit region and key binding. A rail background can
  scrub, a clip body can point at a locus, and a handle can begin a semantic gesture, but overlap
  resolution must be visible and testable. Keep overlay chrome independent from decoded-frame
  availability when a locus has a visual projection.
- **DELETE:** Delete controls that advertise unavailable interaction, including trim handles without
  pointer-down behavior. Delete title editing fallback for scene, sequence, or source loci.
- **STEAL:** Steal the hold-state and preview-off observations as lifecycle tests, and the
  `title × 0s` fixture as a test of playhead/locus independence.
- **MISREAD:** “Audio rail is always Scrub and also rewrites here” contains incompatible contracts.
  Scrub must remain transient and source-preserving. A gain or timing edit needs a distinct gesture,
  an explicit proposal, and one commit on pointer-up.
- **UNSTATED ASSUMPTION:** Conventional NLE bindings such as Space-to-play and visible trim handles
  are self-explanatory in Lattice. They are plausible defaults, but must coexist with focus,
  text-entry, and locus projection rules rather than bypass them.

Evidence: [title at 0s and empty Canvas](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-open-title-0s-empty-canvas.png?raw=true),
[Video click selects scene](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-video-click-scene-demo.png?raw=true),
[Audio rail scrub](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-audio-rail-scrub-to-title.png?raw=true),
[callout hold](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-callout-hold.png?raw=true),
[drawn trim handles](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-trim-handles-drawn.png?raw=true).

## Shared assumptions

The following are shared by most candidates, though not all are domain requirements:

1. **One locus should remain legible across projections.** A, B, D, E, and F say this directly; C
   diagnoses visual competition that currently weakens it.
2. **The current Inspector lies about legal edits.** A, B, E, and F identify title-shaped fallback;
   D's affordance critique supports separating selection from available action.
3. **Playhead and semantic locus are currently under-differentiated.** B, D, E, and F expose this
   through Canvas visibility, timeline color, rail behavior, and projection multiplicity.
4. **The current toolbar/pane hierarchy is accidental.** A, C, D, E, and F find command fallback,
   wrapping, status mixing, dropped panes, or controls with no interaction.
5. **Review is semantically too narrow.** A, B, E, and F expect more than a title-only textual diff,
   although they differ on when Review should appear.
6. **The UI needs to disclose why a projection exists or is absent.** A and B show invalid or missing
   projections; C shows ambiguous emptiness; E and F show misleading fallback and chrome lifecycle.

## Domain necessity versus editor convention

| Domain necessity | Editor convention to keep mutable |
|---|---|
| One semantic locus shared by all clients | Five panes visible at once |
| Playhead is transient session state, not a locus | Timeline at the bottom with three colored rails |
| One direct gesture commits at most one `SemanticEdit`, rewrite, compile, and Undo entry | Handles, modifier keys, toolbar buttons, and Space as the chosen affordances |
| Scrubbing does not rewrite VEL | Clicking a rail background is the scrub gesture |
| Definition, instance, placement, timing, and provenance retain their actual scopes | A permanent right-side Inspector |
| Navigate is optional; provenance remains available | VEL always shown, always hidden, or opened in place |
| Review preserves the current source until Apply and can show meaning, picture, and diff | Review as a pane, drawer, mode, or transient surface |
| Renderer/audio failures remain typed and observable | Status rendered inside the command strip |
| Canvas geometry stays normalized and backend-neutral | Teal boxes, corner handles, and overlay chrome |
| Project history remains source-backed and Git-friendly | Save/Undo/Redo placement and visual hierarchy |

## Semantic contradictions to resolve, not smooth over

1. **Timeline as readout versus timeline as editor.** E calls it a readout; the interaction contract
   requires trim, reorder, and timing gestures on it. The useful question is which regions merely
   project time and which begin a named semantic gesture.
2. **Audio rail scrub versus audio mutation.** A and F approach the same rail as both time navigation
   and source rewrite. Those cannot share an undisclosed gesture. Scrub has no Undo; gain/timing has
   exactly one source-backed commit.
3. **Review as richer evidence versus Review as mandatory checkpoint.** A correctly asks for
   non-title proposals in Review, but direct manipulation is allowed to finish without Review.
   Richness and mandatory routing are independent switches.
4. **One locus versus one edit scope.** B and E correctly emphasize one semantic “here,” but a locus
   surviving several projections does not imply that every legal edit changes every instance.
   `SetPosition`, Trim, definition text, and TimeMap edits have different scopes.
5. **Canvas emptiness versus automatic seeking.** B and F expose a title selected outside its active
   range. Seeking to it may help, but silently coupling selection and playhead would erase a required
   distinction. Explain absence before adding navigation.
6. **Hide VEL versus expose Source.** A's cheap fork can reduce clutter, while B requires Source to be
   perceptible. These are compatible only if source/provenance remains discoverable and Navigate
   stays optional rather than disappearing.
7. **Freeze in the tree versus freeze as a locus.** A, B, and the failure screenshot agree that the
   displayed row has no resolvable Core identity. Either the row projects a real semantic locus with
   provenance, or it is non-selectable explanatory structure; a UI-only fake identity is not a third
   semantic category.

## Challenges to shared assumptions

### Challenge 1: must all projections highlight simultaneously?

Shared identity requires coherent answers, not equal visual emphasis. Simultaneous five-way
highlighting may teach the model, but it may also obscure the active manipulation surface, multiply
a stale selection, and fail at 800px. A useful model must survive when only one projection is visible
and make the other projections discoverable without creating another selection.

### Challenge 2: should locus changes move the playhead?

Several observations treat an empty Canvas after locus selection as failure. Auto-seek would make the
picture gratifying but would hide the distinction between “what I mean” and “when I am looking.”
Lattice needs both. The assumption should be tested with explicit “not visible at 0s; active
1s–4s” disclosure and an optional seek action before any coupling is adopted.

### Challenge 3: is locus kind the right action switch?

Most candidates assume the Inspector should branch on `LocusKind`. Kind is necessary context but may
be too coarse: the same title definition can have several placements, and a scene locus may expose
source, ordering, TimeMap, or media actions. The stronger candidate is an Engine-derived set of legal
semantic edits plus available projections, with reasons for unavailable actions.

### Challenge 4: is the timeline the privileged place to understand multiplicity?

The timeline makes 1→many projections visible, but it can overstate clip instances and understate
source/provenance. A locus model should also be understandable with Timeline collapsed, through a
compact projection count and explicit paths to instance, placement, time, and source.

## Mutations for the next discussion round

These are probes, not a combined design. Each can be critiqued or reconstructed independently.

### M1 — Projection manifest

For the current locus, expose a small semantic inventory: Definition, Instances, Placement, TimeMap,
Provenance, current-time Picture, and Legal edits. Show “not applicable” or a reasoned absence instead
of an empty generic field.

- Tests assumption: facet disclosure is more useful than `LocusKind`-specific forms.
- Falsifier: participants cannot predict edit scope or still interpret every instance as selected
  for mutation.
- Invariant hook: inventory is reconstructed from Engine/Core data; it is not a GPUI-owned model.

### M2 — Independent locus and playhead indicators

Keep the locus fixed while scrubbing. Keep the playhead fixed while pointing at an off-time locus.
When the Canvas has no current projection, state its active range and offer an explicit seek.

- Tests assumption: users can understand two kinds of “here” when both are named.
- Falsifier: users repeatedly mutate the wrong temporal target even with range disclosure.
- Invariant hook: no scrub rewrite, no automatic hidden seek.

### M3 — Scope-labelled legal verbs

Replace generic title fallback with only Engine-supported verbs, each labelled by scope:
“definition,” “this placement,” “this clip,” “scene order,” or “TimeMap.” For unsupported loci, show
why no mutation is legal rather than borrowing a title field.

- Tests assumption: legal verbs are a better organizing unit than a permanent Inspector.
- Falsifier: the labels require UI-specific semantic categories or cannot map one-to-one to existing
  `SemanticEdit`s.
- Invariant hook: Studio remains an Engine client and commits one semantic edit per gesture.

### M4 — Timeline hit-region split

Define three non-overlapping contracts: ruler/rail background scrubs; a projected item points at a
locus; an explicit handle or edit affordance begins a named semantic gesture. Compare a readout-only
variant with a manipulation-enabled variant using the same fixture.

- Tests assumption: the timeline can be both projection and editor without hidden mode.
- Falsifier: participants cannot tell whether pointer-down will seek, select, or rewrite before doing
  it.
- Invariant hook: pointer-up is the only commit; Escape cancels; scrub has no Undo.

### M5 — Definition/instance multiplicity probe

Use the 1-title→3-clips and 1-scene→2-clips fixtures. In one mutation, emphasize the definition and
summarize instance count; in another, emphasize the pointed instance while preserving the shared
locus and definition relationship. Ask participants to predict the result of text, placement, trim,
and timing edits before applying anything.

- Tests assumption: simultaneous highlighting is necessary to communicate 1→many projection.
- Falsifier: either mutation makes edit scope less predictable than the current synchronized state.
- Invariant hook: stable clip/locus identity and provenance, never reverse matching by visible text.

### M6 — Freeze honesty probe

Variant A removes the freeze tree row and exposes freeze through its real source/time projection.
Variant B keeps a selectable row only if it resolves to a real Core locus with source span, timeline
range, provenance, and explain output.

- Tests assumption: users need freeze as a tree target rather than as an explained temporal
  transformation.
- Falsifier: removing it makes freeze undiscoverable, while a real locus cannot be defined without
  distorting Core semantics.
- Invariant hook: no synthetic UI-only `freeze:source:clip` identity.

### M7 — Review breadth without Review gating

For title text, Trim, SetPosition, gain, and TimeMap proposals, show current picture/time, semantic
effect and scope, and source diff. Run the same legal edit once as direct manipulation and once as a
proposal requiring Apply/Reject.

- Tests assumption: richer Review generalizes beyond title without becoming the everyday edit path.
- Falsifier: picture adds no decision value for a class of edits, or proposal scope cannot be
  explained without backend details.
- Invariant hook: source remains unchanged until Apply on proposal paths; direct edits remain legal.

### M8 — Role-based hierarchy and compact disclosure

Give locus, playhead, primary commit, destructive action, transport, renderer status, and media kind
distinct channels. At 1400, 1024, and 800 widths, collapse facets into disclosed navigation rather
than silently dropping VEL or Inspector.

- Tests assumption: visual competition and compact failure come from role collision, not merely teal
  saturation.
- Falsifier: participants lose cross-projection coherence once all panes are not simultaneous.
- Invariant hook: typed renderer/audio status remains visible and is never presented as successful
  fallback.

### M9 — Canvas absence taxonomy

Render distinct explanations for: locus outside playhead, no visual projection, renderer
initializing, layout failure, preview disabled, and media unavailable. Preserve locus overlay
affordances when geometry is known and frame pixels are temporarily unavailable.

- Tests assumption: “empty Canvas” is one problem.
- Falsifier: Engine cannot distinguish these states without introducing UI-specific semantics.
- Invariant hook: missing media and renderer failure remain observable; no implicit success or
  synthetic picture.

## Hooks for participant replies

Participants should respond independently rather than converge in this round:

1. Which mutation would you delete first, and which invariant or observation does it violate?
2. For M3 and M5, name the exact scope of each predicted edit: definition, instance, placement,
   clip/time range, scene order, or TimeMap.
3. For M2, specify whether any user action should couple locus and playhead, and whether that coupling
   is explicit, temporary, or persisted.
4. For M4, identify every pixel region that can scrub, point, or mutate; no region may silently do
   two of them.
5. For M6, either reconstruct a real freeze locus from existing semantics or argue that freeze should
   not be directly selectable.
6. State what evidence would falsify your preferred mutation. Do not answer with a skin or a pane
   arrangement alone.

No mutation is accepted, merged, or ranked by this note.
