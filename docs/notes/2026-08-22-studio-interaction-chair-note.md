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

## Follow-up packet handling

Five isolated packets arrived after the first note. They are not votes to total:

| Packet | Participant lens | Candidate authored by that participant |
|---|---|---|
| 1 | Flash A, visual/perceptual | Candidate C — hierarchy (#7) |
| 2 | Flash B, visual/perceptual | Candidate D — affordance (#9) |
| 3 | Grok A, cheap implementation | Candidate A — structure (#6) |
| 4 | Grok B, cheap implementation | Candidate F — interaction (#10), not Candidate B |
| 5 | Opus, adversarial plus one mutation | Candidate E — first principles (#8) |

Candidate B remains Candidate B — semantic (#11). No authorship is inferred for it from these
packets. Each packet is retained as an independent challenge: agreement below means compatible
claims, not a majority result. Packet 5's VM-only 640px measurement and artifact are excluded because
they are not committed evidence.

## Candidate extraction

### Candidate A — structure (#6)

- **KEEP:** Treat one session and one `LocusId` as the current semantic selection authority. Keep the
  observations that scene loci receive title-shaped Inspector controls and Review currently shows a
  title/source diff without the affected picture. Code-grounded packets also retain the synthetic
  freeze identity diagnosis, while the screenshot alone establishes only that clicking the row can
  end in layout failure.
- **CHANGE:** Turn “Inspector by `LocusKind`” into “actions and projections supported by this locus.”
  Kind-switching alone does not fix `adopt_locus_label` populating title text for non-title loci.
  Correct the freeze reconstruction: the observed click fails on an unknown locus; a successful
  rebind is not immediate and follows replacement of the working compilation.
- **DELETE:** Delete hide-VEL as a proposed cheap mutation. Delete unresolved selectable rows rather
  than inventing `Core Freeze`, and delete title-shaped editing for non-title loci.
- **STEAL:** Steal cheap forks as an evaluation method: vary one projection or action policy while
  leaving Engine semantics fixed. Reuse the `EditProposal` already built during committed edits to
  examine Trim and SetPosition in Review; do not make Review a direct-manipulation gate.
- **MISREAD:** “Audio rail on `SemanticEdit`” is not a cheap, defined audio edit. Current Scrub
  changes playhead and can re-point the locus through `point_from_timeline_time`, but does not rewrite
  VEL. `SetGain` has no Time scope, while `SetFade` is a video-opacity envelope; inventing a
  time-scoped audio verb here would conceal a Core gap.
- **UNSTATED ASSUMPTION:** One `LocusId` being stored in Session proves that singularity is naturally
  resolved at every overlapping timeline time. The current choice can be manufactured by
  specificity ranking. Hiding VEL also assumes that the only mutable source string can disappear
  without weakening explainability.

Evidence: [title locus and stale header](https://github.com/annenpolka/lattice/blob/75c4674c6fa123451c69fa93de8c780eeb8dbb26/docs/screenshots/studio-observe-a-title-locus.png?raw=true),
[scene locus with title control](https://github.com/annenpolka/lattice/blob/75c4674c6fa123451c69fa93de8c780eeb8dbb26/docs/screenshots/studio-observe-a-scene-locus.png?raw=true),
[freeze click failure](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-freeze-click.png?raw=true),
[title-only Review](https://github.com/annenpolka/lattice/blob/225f8497f9fa008850a74c276cbf03f84d56906a/docs/screenshots/studio-observe-a-review-title-propose.png?raw=true).

### Candidate B — semantic (#11)

- **KEEP:** Keep the formulation “one locus, several projections” and the distinction between locus
  and playhead as a domain goal. Keep missing Source, Placement, TimeMap, and current-time visibility
  as diagnoses: packet 5 reports TimeMap content time being computed then dropped, Source having no
  timeline span, and Video click dropping clip identity. Keep freeze as not currently a Core locus.
- **CHANGE:** Replace “every projection must be visibly present” with “every available projection is
  discoverable, and absence is explained.” Do not turn missing facets into panes, new
  `LocusKind`s, or a second playhead locus. Media, Sequence, and Scene can legitimately lack some
  source, timeline, or visual fields.
- **DELETE:** Delete per-view semantic selection and any second-selection repair for playhead versus
  locus. Delete the unresolved freeze row rather than masking its identity failure.
- **STEAL:** Steal the before/after freeze reconstruction and the side-by-side current-picture,
  semantic proposal, and source-diff framing for Review. Treat Review as a projection of an
  `EditProposal` carrying a `locus_id`, including the possibility that this proposal goes stale.
- **MISREAD:** A missing picture at `0s` while a title active at `1s` is not necessarily a broken
  projection, and Canvas is an `evaluate_at` surface rather than merely another
  `LocusProjection` pane. The failure is inability to distinguish temporal absence from preview-off,
  initialization, layout, media, or compile failure.
- **UNSTATED ASSUMPTION:** The philosophy “one locus projected” is already what every screenshot
  perceptually demonstrates. Some Core nouns have no projection fields, Canvas can early-return, and
  Source is already represented as a SEQUENCE row; the model must explain partial projection without
  multiplying panes.

Evidence: [shared locus](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-shared-locus.png?raw=true),
[playhead outside locus](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-playhead-outside-locus.png?raw=true),
[freeze before](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-freeze-node-before.png?raw=true),
[freeze after](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-freeze-node-after.png?raw=true),
[Review with current Canvas](https://github.com/annenpolka/lattice/blob/5deb7c7ae7191e3030879e63524fa109625a1add/docs/screenshots/semantic-review-diff-current-canvas.png?raw=true).

### Candidate C — hierarchy (#7)

- **KEEP:** Keep the observations that high-saturation teal makes unrelated actions compete, an
  empty-black Canvas communicates no cause, VEL looks less editable than surrounding surfaces, and
  the toolbar wraps without preserving command grouping.
- **CHANGE:** Convert the color critique from “less teal” to “assign color by interaction role.”
  Teal currently spans pane labels, filled commands, tree/VEL selection, playhead, and Text clips.
  VEL is editable through `StudioSourceInputHandler`; its static appearance is an affordance mismatch,
  not evidence of read-only behavior. Correct the width claim: wrapping is visible already at the
  default 1400 capture, so 1024 is not established as the unique trigger.
- **DELETE:** Delete equal visual weight for global commands, transient transport, semantic edits,
  status, and destructive actions. Delete this paint critique as a route to selecting a familiar
  NLE, design-tool, or IDE skin.
- **STEAL:** Steal teal-role collision as an audit and width captures as constraints, not as proof of
  a particular responsive cause. Retain teal only where a tested semantic role needs it, rather than
  treating desaturation as the mutation.
- **MISREAD:** Canvas blackness is not purely a hierarchy defect. It can represent an unavailable
  frame, preview disabled, a locus outside the playhead, renderer initialization, layout failure,
  media failure, or an undisplayed compile diagnostic. Styling cannot safely merge those states.
- **UNSTATED ASSUMPTION:** The existing five-pane composition and top toolbar remain fixed while only
  visual weight changes, and 1024 begins a wrap→800-drop causal chain. Neither follows from the
  captures; even the 1400 toolbar wraps.

Evidence: [default 1400×840](https://github.com/annenpolka/lattice/blob/32da848/docs/screenshots/01_default_layout_1400x840.png?raw=true),
[compact 1024×768](https://github.com/annenpolka/lattice/blob/32da848/docs/screenshots/08_window_compact_1024x768.png?raw=true).

### Candidate D — affordance (#9)

- **KEEP:** Keep the synchronized title capture as evidence of a strong grouping signal where all
  projections render. Keep the findings that playhead and Text clips share teal, insertion/selection
  whites also collide, the ruler lacks a persistent thumb, and renderer/audio status is mixed into
  commands.
- **CHANGE:** Preserve cross-projection coherence without requiring five simultaneous panes. Give the
  playhead a unique transient signal and expose transport hit targets separately from clips and trim
  handles. Treat the 800 capture as evidence that VEL/Inspector are not visible in that frame, not
  proof that a deliberate “drop panes” policy exists; overflow, slivering, and clipping remain
  competing reconstructions.
- **DELETE:** Delete five-pane synchronization as an invariant. Canvas can early-return when
  `preview_image` is absent, making four visible surfaces, and per-track highlighting can leak policy.
  Delete color-only change as a standalone semantic mutation.
- **STEAL:** Steal in-flight versus committed scrub comparison as a state-transition test, and the
  800px capture as a visibility/overflow test without using packet 5's uncommitted 640px artifact.
- **MISREAD:** “800 drops panes” overstates what one screenshot proves. The stronger concern is that
  fixed and flexible regions can invert domain priority, allowing Canvas to disappear before a
  fixed-width rail, whatever the exact mechanism.
- **UNSTATED ASSUMPTION:** Selection synchronization is always more important than the distinction
  among locus, keyboard focus, hovered projection, playhead, insertion marker, and ephemeral
  proposal—and that every projection participates at every state.

Evidence: [scrub committed](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/08_timeline_scrub_committed.png?raw=true),
[scrub in flight](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/07_timeline_scrub_in_flight.png?raw=true),
[title locus across panes](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/01_overview_title_locus_selected.png?raw=true),
[ultracompact 800×600](https://github.com/annenpolka/lattice/blob/6e8dce4/docs/screenshots/17_window_ultracompact_800x600.png?raw=true).

### Candidate E — first principles (#8)

- **KEEP:** Keep “one semantic definition may have many rendered projections” as the key departure
  from object-centric NLE selection. Keep the proposal that legal verbs come from the locus and
  become `SemanticEdit`s. Keep the toolbar-target divergence: fallback may be deliberate and quiet,
  but Gain on a Sequence can target a source while leaving the displayed locus unmoved.
- **CHANGE:** Do not make panes follow a UI-owned `LocusProjection` abstraction: visual content lives
  on `Locus`, while Canvas evaluates the timeline at the playhead. Change “timeline is a readout” to
  “timeline is an interactive projection supporting point, reorder, trim, and scrub.”
- **DELETE:** Delete readout-only Timeline and toolbar actions that invent an undisclosed target.
  Delete the claim that the supplied rail crop proves “one Title→three clips”: one packet identifies
  it as Scene→three clips, and the crop alone cannot settle semantic identity.
- **STEAL:** Steal the general 1→many inversion, verb=`SemanticEdit`, and the already-built
  `EditProposal`. Require every claimed multiplicity fixture and edit scope to be identified from
  semantic data, not labels or rail geometry.
- **MISREAD:** Highlight counts do not prove which definition owns the spans or that one action should
  mutate them all. Shared identity explains a relation; scope still depends on legal
  `SemanticEdit`, stable clip identity, scene/sequence context, and provenance.
- **UNSTATED ASSUMPTION:** A facet model automatically yields understandable controls. It does not
  answer priority, disclosure, conflict, or absent facets. It also assumes the fixed-width Timeline
  is the natural star while Canvas absorbs width pressure.

Evidence: [disputed 1→three rail crop](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-selection-2026-08-22-title-locus-3-clips.png?raw=true),
[scene locus to two clips](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-selection-2026-08-22-scene-locus-2-clips.png?raw=true),
[video click selects scene](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-videoclick-2026-08-22-after-scene-locus.png?raw=true),
[toolbar fallback](https://github.com/annenpolka/lattice/blob/7fb71a7c95fdce4228bc5e9f3320da466e388f20/docs/artifacts/studio-toolbar-2026-08-22-after-gain-vel-inspector.png?raw=true).

### Candidate F — interaction (#10)

- **KEEP:** Keep the observed mapping from Video click to scene locus; always-scrub Audio rail;
  title-only Inspector/Review; temporally empty Canvas; unbound Space; and overlay chrome disappearing
  when preview is off. Keep drawn trim edges as an ambiguous rest-state affordance, not as proof that
  trim is non-interactive.
- **CHANGE:** Define explicit contracts for each hit region and key binding. A rail background can
  scrub, a clip body can point at a locus, and a handle can begin a semantic gesture, but overlap
  resolution must be visible and testable. Current trim can be reached through rail-level
  `capture_any_mouse_down` plus hit testing without a handle-local callback. Separate preview-image
  absence from `overlay_playhead_visible`.
- **DELETE:** Delete the inference “no handle-local `on_mouse_down` means fake trim”; it would
  duplicate existing rail hit testing. Delete title editing fallback for scene, sequence, or source
  loci, and delete a new Audio→Trim semantic shortcut.
- **STEAL:** Steal the hold-state and preview-off observations as lifecycle tests, and the
  `title × 0s` fixture as a multi-cause Canvas test. Steal Space→play as a conventional binding to
  test under explicit focus rules, not as a domain requirement.
- **MISREAD:** “Audio rail is always Scrub and also rewrites here” does not mean source rewrite.
  Scrub leaves VEL unchanged but currently calls `point_from_timeline_time`; specificity can choose a
  Title locus, so the shared semantic “here” changes as a side effect of time navigation.
- **UNSTATED ASSUMPTION:** Conventional NLE bindings such as Space-to-play and visible trim handles
  are self-explanatory in Lattice. They are plausible defaults, but must coexist with focus,
  text-entry, and locus projection rules. Video→scene also discards clip identity, which may matter
  for later instance-scoped verbs.

Evidence: [title at 0s and empty Canvas](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-open-title-0s-empty-canvas.png?raw=true),
[Video click selects scene](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-video-click-scene-demo.png?raw=true),
[Audio rail scrub](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-audio-rail-scrub-to-title.png?raw=true),
[callout hold](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-callout-hold.png?raw=true),
[drawn trim handles](https://github.com/annenpolka/lattice/blob/827e35a454363e2f9663c5a39037642ef6ca0e87/docs/screenshots/observe-b-trim-handles-drawn.png?raw=true).

## Shared assumptions

The following recur across candidates and packets, though recurrence is not a vote and not all are
domain requirements:

1. **One locus should remain legible across projections.** A, B, D, E, and F say this directly; C
   diagnoses visual competition that currently weakens it.
2. **The current Inspector lies about legal edits.** A, B, E, and F identify title-shaped fallback.
   Follow-up packets locate both visible form reuse and `adopt_locus_label`; `LocusKind` branching
   alone may still be insufficient.
3. **Playhead and semantic locus are currently under-differentiated.** B, D, E, and F expose this
   through Canvas visibility, timeline color, rail behavior, and projection multiplicity.
4. **The current toolbar/pane hierarchy is accidental.** A, C, D, E, and F find target fallback,
   wrapping, status mixing, visibility loss, or weak rest-state affordance. The packets dispute the
   causal explanation, not the mismatch.
5. **Review is semantically too narrow.** A, B, E, and F expect more than a title-only textual diff,
   although they differ on when Review should appear.
6. **The UI needs to disclose why a projection exists or is absent.** A and B show invalid or missing
   projections; C shows ambiguous emptiness; E and F show target divergence and chrome lifecycle.
7. **One current locus is desirable, but its resolution is under-examined.** Most inputs treat one
   `LocusId` as foundational. Packet 5 challenges the specificity-based collapse of overlapping
   candidates, while no packet supports per-view semantic selection.

## Domain necessity versus editor convention

| Domain necessity | Editor convention to keep mutable |
|---|---|
| One semantic locus shared by all clients | Five panes visible at once |
| Playhead is transient session state, not a locus | Timeline at the bottom with three colored rails |
| One direct gesture commits at most one `SemanticEdit`, rewrite, compile, and Undo entry | Handles, modifier keys, toolbar buttons, and Space as the chosen affordances |
| Scrubbing does not rewrite VEL | Whether scrub also re-points the shared locus |
| Definition, instance, placement, timing, and provenance retain their actual scopes | New facet panes or new `LocusKind`s |
| Navigate is optional; provenance remains available | VEL always shown, always hidden, or opened in place |
| Review preserves the current source until Apply and can show meaning, picture, and diff | Review as a pane, drawer, mode, or transient surface |
| Renderer/audio failures remain typed and observable | Status rendered inside the command strip |
| Canvas geometry stays normalized and backend-neutral | Teal boxes, corner handles, and overlay chrome |
| Project history remains source-backed and Git-friendly | Save/Undo/Redo placement and visual hierarchy |

## Semantic contradictions to resolve, not smooth over

1. **Timeline as readout versus timeline as editor.** E calls it a readout; the interaction contract
   requires trim, reorder, and timing gestures on it. The useful question is which regions merely
   project time and which begin a named semantic gesture.
2. **Audio rail scrub versus locus mutation.** The first note incorrectly called this a source
   rewrite. Scrub leaves VEL unchanged, but `point_from_timeline_time` can replace the current locus
   using specificity, so time navigation also changes semantic “here.” Separately, there is no
   time-scoped audio `SemanticEdit`; Gain and video Fade do not fill that gap.
3. **Review as richer evidence versus Review as mandatory checkpoint.** A correctly asks for
   non-title proposals in Review, but direct manipulation is allowed to finish without Review.
   Richness and mandatory routing are independent switches.
4. **One locus versus one edit scope.** B and E correctly emphasize one semantic “here,” but a locus
   surviving several projections does not imply that every legal edit changes every instance.
   `SetPosition`, Trim, definition text, and TimeMap edits have different scopes.
5. **Canvas emptiness versus automatic seeking.** B and F expose a title selected outside its active
   range, but PREVIEW=0, renderer state, overlay gating, layout failure, missing media, and undisplayed
   compile diagnostics can produce compatible screenshots. Explain the cause before adding
   navigation or styling.
6. **Hide VEL versus expose Source.** Hiding VEL is rejected as a mutation by packets from different
   lenses. The remaining contradiction is narrower: VEL is the mutable source and provenance surface,
   yet visually appears static and may be absent under width pressure. Navigate remains optional,
   but source cannot become inexplicable.
7. **Freeze in the tree versus freeze as a locus.** Code-grounded packets report a synthetic
   `freeze:source:clip`; visual packets correctly note that pixels alone prove only a row and a
   failure. Either the row resolves through existing semantics or is non-selectable explanatory
   structure. Do not add `Core Freeze` for a tree convenience.
8. **Five-pane synchronization versus partial projection.** The synchronized title shot is strong,
   but Canvas can early-return and some locus kinds lack fields. Coherence must survive absent
   projections; simultaneity is not the invariant.
9. **Toolbar fallback versus locus-derived verbs.** Existing source/scene fallback is not wholly
   context-blind, but can target something other than the displayed locus without moving it.
   “Helpful fallback” and “hidden second target” are two readings of the same behavior.
10. **Trim affordance versus trim implementation.** Missing handle-local pointer code does not mean
    trim is broken because rail-level capture and hit testing exist. The remaining issue is whether
    rest-state pixels communicate that contract.
11. **One locus versus overlapping candidates.** Session stores one `LocusId`, while timeline
    resolution can obtain several candidates and collapse them with `specificity()/max_by_key`.
    Shared semantic selection forbids per-view selection; it does not settle whether overlap should
    be silently ranked, explicitly disambiguated, or represented by a different Core concept.

## Challenges to shared assumptions

### Challenge 1: must all projections highlight simultaneously?

Shared identity requires coherent answers, not equal visual emphasis. Simultaneous multi-surface
highlighting may teach the model, but it may also obscure the active manipulation surface, multiply
a stale selection, and fail under width or preview pressure. A useful model must survive when only
one projection is visible and make the others discoverable without creating another selection.

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

### Challenge 5: is a singular timeline locus discovered or manufactured?

The invariant says clients share one semantic “here”; it does not say overlapping candidates are
naturally ordered. If `max_by_key(specificity)` manufactures singularity, the editor may hide a real
choice before any view can explain it. Returning candidates for one surface to present is not a
per-view selection model, provided the committed result remains the one shared locus.

## Mutations for the next discussion round

These are probes, not a combined design. Each can be critiqued or reconstructed independently.

### Mutation ledger after follow-up

- **Kept and narrowed:** M1, M2, M3, M4, M5, M6, M7, and M9.
- **Dropped as standalone mutations:** hide VEL; readout-only Timeline; color/desaturation alone;
  new Placement/TimeMap panes or kinds; a second playhead locus; Audio-as-`SemanticEdit`; a new
  handle-local trim callback; automatic seek on locus change.
- **Downgraded to a measurement harness:** M8. Role and width observations constrain semantic
  mutations but do not choose a skin or prove a 1024→800 causal chain.
- **Added:** M10, exposing overlapping timeline locus candidates before choosing the one shared
  locus.

### M1 — Projection inventory without new panes or kinds

For the current locus, reconstruct only fields already available from Engine/Core: semantic identity,
source/provenance when present, timeline range when present, visual data when present, and legal
edits. Show absence with a reason. Do not require one pane per noun, invent Placement/TimeMap kinds,
or pretend Canvas is just another locus field; Canvas still comes from `evaluate_at`.

- Tests assumption: facet disclosure is more useful than `LocusKind`-specific forms.
- Falsifier: participants cannot predict edit scope or still interpret every instance as selected
  for mutation.
- Invariant hook: inventory is reconstructed from Engine/Core data; it is not a GPUI-owned model.

### M2 — Independent locus and playhead behavior

Compare current Scrub—playhead move plus `point_from_timeline_time`—against a mutation that moves only
the playhead and retains the shared locus. Keep the playhead fixed while pointing at an off-time
locus. When Canvas has no current projection, state the active range and offer explicit seek.

- Tests assumption: users can understand two kinds of “here” when both are named.
- Falsifier: users repeatedly mutate the wrong temporal target even with range disclosure.
- Invariant hook: neither variant rewrites VEL; no automatic hidden seek or second locus.

### M3 — Scope-labelled legal verbs

Replace generic title fallback with only Engine-supported verbs, each labelled by scope:
“definition,” “this placement,” “this clip,” “scene order,” or “TimeMap.” For unsupported loci, show
why no mutation is legal rather than borrowing a title field. Compare fail-closed behavior with
existing `target_source_locus` / `target_scene_locus` fallback, and disclose when the action target
differs from the displayed locus.

- Tests assumption: legal verbs are a better organizing unit than a permanent Inspector.
- Falsifier: the labels require UI-specific semantic categories or cannot map one-to-one to existing
  `SemanticEdit`s.
- Invariant hook: Studio remains an Engine client and commits one semantic edit per gesture.

### M4 — Timeline hit-region split

Define three non-overlapping contracts: ruler/rail background scrubs; a projected item points at a
locus; an explicit handle or edit affordance begins a named semantic gesture. Exercise existing
rail-level capture and hit testing before proposing any handle-local event code. Compare rest-state
predictability, not readout-only versus interactive Timeline.

- Tests assumption: the timeline can be both projection and editor without hidden mode.
- Falsifier: participants cannot tell whether pointer-down will seek, select, or rewrite before doing
  it.
- Invariant hook: pointer-up is the only commit; Escape cancels; scrub has no Undo.

### M5 — Definition/instance multiplicity probe

Use semantically verified 1→many fixtures. Do not rely on the disputed rail crop's filename or labels
to decide whether it is Title→three or Scene→three. In one mutation, emphasize the definition and
summarize verified instances; in another, emphasize the pointed instance while preserving the shared
locus and definition relationship. Ask participants to predict text, placement, trim, and timing
scope before applying anything.

- Tests assumption: simultaneous highlighting is necessary to communicate 1→many projection.
- Falsifier: either mutation makes edit scope less predictable than the current synchronized state.
- Invariant hook: stable clip/locus identity and provenance, never reverse matching by visible text.

### M6 — Freeze honesty probe

Variant A removes or ignores the unresolved freeze tree row and exposes freeze through existing
source/time semantics. Variant B keeps a selectable row only if it resolves through an existing Core
locus with source span, timeline range, provenance, and explain output. Neither adds `Core Freeze`.

- Tests assumption: users need freeze as a tree target rather than as an explained temporal
  transformation.
- Falsifier: removing it makes freeze undiscoverable, while a real locus cannot be defined without
  distorting Core semantics.
- Invariant hook: no synthetic UI-only `freeze:source:clip` identity.

### M7 — Review breadth without Review gating

Retain the `EditProposal` already built during `apply_committed` instead of discarding it. For
existing legal edits such as title text, Trim, and SetPosition, show current picture/time, semantic
effect and scope, and source diff. Add gain or TimeMap only when an actual legal edit carries the
claimed scope; do not manufacture a time-scoped audio verb for Review.

- Tests assumption: richer Review generalizes beyond title without becoming the everyday edit path.
- Falsifier: picture adds no decision value for a class of edits, or proposal scope cannot be
  explained without backend details.
- Invariant hook: source remains unchanged until Apply on proposal paths; direct edits remain legal.

### M8 — Role and width measurement harness

Audit locus, playhead, insertion marker, primary commit, destructive action, transport, renderer
status, and media kind as separate roles. At committed 1400, 1024, and 800 captures, record wrapping,
slivering, clipping, overflow, and absent surfaces without assuming one causes the next. Exclude the
uncommitted VM-only 640px measure.

- Tests assumption: semantic mutations remain legible when teal and simultaneous panes are not doing
  all grouping work.
- Falsifier: the audit cannot distinguish an actual responsive policy from viewport clipping or
  unavailable projection.
- Invariant hook: this harness selects no palette, skin, or pane arrangement.

### M9 — Canvas absence taxonomy

Render distinct explanations for: locus outside playhead, no visual projection, renderer
initializing, layout failure, preview disabled, media unavailable, and compile diagnostics. Preserve
locus overlay affordances when geometry is known and frame pixels are temporarily unavailable; do
not infer a cause from a PREVIEW=0 screenshot.

- Tests assumption: “empty Canvas” is one problem.
- Falsifier: Engine cannot distinguish these states without introducing UI-specific semantics.
- Invariant hook: missing media and renderer failure remain observable; no implicit success or
  synthetic picture.

### M10 — Overlapping-locus candidate probe

Remove only the silent `max_by_key(specificity)` collapse from timeline locus lookup for the probe.
Return the candidate list to one experimental surface, show the reason and scope for each candidate,
and commit exactly one result back to the shared session locus. Compare that with current automatic
specificity using overlapping title, scene, source, and clip ranges.

- Tests assumption: singular locus at a timeline time is domain-found rather than UI-manufactured.
- Falsifier: candidates are semantically equivalent, or explicit choice adds no scope predictability
  while materially obstructing ordinary pointing.
- Invariant hook: candidate presentation is not persistent project state and not per-view selection;
  after choice, every projection receives the same `LocusId`.

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
6. For M10, state whether specificity is domain ordering or merely current interaction policy, and
   identify the evidence that distinguishes those readings.
7. State what evidence would falsify your preferred mutation. Do not answer with a skin or a pane
   arrangement alone.

No packet is merged into a vote. No mutation is accepted, merged, or ranked by this note.
