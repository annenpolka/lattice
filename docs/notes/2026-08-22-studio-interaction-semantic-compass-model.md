# Studio interaction model: semantic compass

Date: 2026-08-22

Status: Phase IV discussion input; one second-generation interaction model

Scope: domain-derived interaction behavior, not a pane layout or widget proposal

Source: [Studio interaction model chair note](https://raw.githubusercontent.com/annenpolka/lattice/3fc72b7/docs/notes/2026-08-22-studio-interaction-chair-note.md)

## Thesis

Studio should behave as a **semantic compass**.

The compass has one center: the committed `LocusId`. Everything else is a reading relative to that
center:

- `LocusProjection` says where the center can be found in source, Core, and timeline data.
- `evaluate_at(playhead)` says what is observable at the current session time.
- `TimeMap` says which content time that observation samples, including a rate-zero freeze.
- legal `SemanticEdit`s say which state transitions the Engine can perform from the center.
- an `EditProposal` is a revision-bound preview of one such transition, not another center.

This is not an object-selection model. A definition, placement, clip, and rendered rectangle can be
related without becoming one interchangeable object. Nor is it a collection of synchronized panes:
the model still works when only one projection is present.

## The compass state

The interaction state is conceptually:

```text
Compass
  center: one committed LocusId
  projection ledger: present and absent readings of that locus
  playhead: transient timeline time
  observation: evaluate_at(playhead)
  temporal reading: local time -> TimeMap -> content time
  verbs: Engine-derived legal SemanticEdits with exact target and scope
  draft: optional EditProposal bound to locus_id + base_revision
  probe: optional, ephemeral set of unresolved pointing candidates
```

Only `center` is semantic “here.” `playhead`, `draft`, and `probe` are session state. They neither
create a second selection nor enter project source.

### What “here” is

“Here” is the one committed semantic address named by `LocusId`, together with the identity,
context, and provenance carried by its `Locus`. It answers:

> If an edit or an external instruction says “this,” which semantic node does “this” mean?

It does not mean “all pixels currently highlighted,” “the most specific item under the clock,” or
“whatever target makes a command succeed.” A projection may be absent, and one locus may have
several related instances, without changing here.

The compass distinguishes **identity** from **relation**. `derived_from`, scene/sequence context,
stable clip identity, source span, and timeline span describe paths from the center. Following one
of those paths can explicitly commit a new center. Merely displaying the relation cannot silently
change it.

## Playhead and locus

The playhead answers “when am I observing?” The locus answers “what do I mean?” They are independent.

Scrub moves only the playhead. It does not rewrite VEL, create Undo, or call temporal specificity a
semantic selection. Pointing at an identified projected item commits its locus and leaves the
playhead fixed. When the locus is inactive at the playhead, the compass retains both facts:

```text
Here: title “Checkpoint”
Observed at: 0s
Visible range: 1s–4s
Result: temporally absent
Available navigation: seek to 1s
```

Seek is an explicit navigation verb. It moves the playhead to a disclosed time and retains the
locus. It is the only ordinary coupling in this model; locus changes never auto-seek, and scrub
never auto-points.

The current picture is not a projection field of `Locus`. It is the result of
`evaluate_at(playhead)`. The compass may therefore report a valid visual locus and no current
pixels without contradiction.

For a timed item, the temporal reading exposes both timeline-local time and mapped content time.
A rate-zero `TimeMapSegment` is reported as a freeze:

```text
timeline 12.0s -> item local 2.0s -> content 37.4s (held for 1.5s)
```

Freeze is thus an explained property of time evaluation, not a synthetic selectable object.

## Projection ledger

`LocusProjection` is treated as a ledger of evidence, not a demand for simultaneous surfaces. Each
reading has one of three outcomes:

1. **Present** — identity and data needed to navigate or explain the projection are available.
2. **Structurally absent** — this kind of locus has no such projection, for example a media locus
   with no source span.
3. **Currently unavailable** — the projection exists in principle but cannot be produced now, with
   a typed reason.

The ledger exposes identity before multiplicity. For a one-to-many relationship it states, for
example, “definition with three placements,” then names each stable relation and its scope. It does
not imply that editing the definition edits placement geometry, or that pointing at one clip
selects every clip.

Navigate follows a ledger entry. It is optional and never blocks provenance or editing.

## Legal verbs are transitions from here

Studio does not populate a form by `LocusKind`, and it does not search for a nearby source or scene
when a command lacks a target. It asks the Engine which named transitions can be constructed for
the committed locus.

Each exposed verb must disclose four things before invocation:

```text
verb = SemanticEdit variant
target = exact LocusId or stable related identity
scope = semantic extent changed by the rewrite
effect = plain-language description of the transition
```

Scope is not inferred from highlight count. It must be one of the extents already named by the
domain relation involved: definition, placement, clip/time range, scene order, source, or TimeMap.
Examples include:

- title text or parameters: the exact invocation/definition represented by the target locus;
- `SetPosition` and `ResizeOverlay`: one placement;
- `Trim`, `Split`, and clip deletion: one stable clip and its time range;
- `ReorderScene`: one scene within one sequence order;
- `SetGain`: only the exact source or audio placement the Engine can name;
- freeze: a TimeMap transformation exposed through an existing source/placement relation.

These labels are explanatory outputs, not new `LocusKind`s. If the Engine cannot name the exact
target and scope for an existing `SemanticEdit`, the verb is absent and the reason is shown. The
model rejects action fallback whose effective target differs from here.

A gesture may manipulate ephemeral geometry, but pointer-up submits at most one named
`SemanticEdit`, producing one source-backed rewrite, compile, and Undo entry. Escape discards the
ephemeral state.

### Direct transition and proposed transition

The same legal verb has two transaction policies:

- **Direct:** commit the semantic edit immediately through the Engine.
- **Proposed:** retain the generated `EditProposal`; current source remains unchanged until Apply.

Proposal policy depends on the initiating workflow, not on the edit kind. Review is therefore broad
but not a gate. It can explain title, trim, placement, or any other genuinely supported edit using
the affected locus, semantic effect and scope, current observation, and VEL diff.

Apply succeeds only when both the proposal's `locus_id` still identifies its target and
`base_revision` matches current source. Otherwise the proposal is disclosed as stale; Studio does
not retarget it.

## Pointing and overlap: singularity is manufactured

One shared locus is an invariant after pointing completes. It is not evidence that every timeline
coordinate contains one naturally preferred semantic target.

`specificity()/max_by_key` manufactures a winner from overlapping title, placement, source, and
scene candidates. Kind rank and short span can be useful interaction defaults, but they are not a
domain proof that the discarded candidates mean less.

The compass uses two pointing paths:

1. A projection with stable identity, such as a clip body or Canvas overlay, commits that identity
   directly as here.
2. A coordinate-only probe, such as pointing into an overlap without item identity, asks the Engine
   for all semantically distinct candidates at that coordinate.

The candidate set is ephemeral. It reports each candidate's identity, relation, span, provenance,
and legal edit scopes. If exactly one candidate remains, it is committed. If several remain,
pointing is unresolved until one is chosen or the probe is cancelled. No view owns the candidates,
and no projection changes selection early. Once chosen, the result is the one session locus seen by
every client.

Repeated choice may be accelerated by an explicit user preference or a reversible interaction
default, but that policy must remain visible in `lattice explain`; it cannot be promoted to Core
semantic ordering merely to avoid disambiguation.

## Absence is a result, not empty chrome

Every requested reading returns either content or a reason. The minimum reason vocabulary is:

- `not-applicable`: the locus has no source, timeline, or visual projection;
- `outside-playhead`: it has a visual/timeline projection but is inactive at the observation time;
- `preview-disabled`;
- `renderer-initializing` or typed renderer failure;
- `media-unavailable`;
- `compile-diagnostic`;
- `layout-unavailable`;
- `stale-proposal`;
- `unresolved-pointing`: several candidate loci remain.

The reason includes the origin that can explain it and only actions legal for that state. For
example, `outside-playhead` can offer explicit Seek; missing media cannot masquerade as black
success; a compile diagnostic cannot become “no visual projection.”

When geometry is known but frame pixels are unavailable, the ledger can still expose the locus and
its normalized placement. That does not synthesize a picture or turn a renderer failure into
success.

## Decision on M1–M10

The mutations are evaluated as consequences of the semantic compass, not assembled as independent
features.

| Mutation | Decision | Reason |
|---|---|---|
| M1 projection inventory | Take, as the projection ledger | Partial projection is intrinsic to `LocusProjection`; present and absent readings must be explainable without new panes or kinds. |
| M2 independent locus and playhead | Take | Identity and observation time answer different questions. Only explicit Seek couples them, and it moves time without replacing here. |
| M3 scope-labelled legal verbs | Take, fail-closed | A verb exists only when the Engine can name its `SemanticEdit`, exact target, and rewrite scope. Hidden fallback contradicts one here. |
| M4 timeline hit-region split | Take as a pointing contract | Coordinate probes navigate time, identity-bearing projections point, and named gestures edit. A single input cannot silently do two of these. The particular pixels remain editor convention. |
| M5 definition/instance multiplicity | Take as relation disclosure | One-to-many is expressed by stable identity and provenance, not synchronized highlighting. Each edit still declares one exact scope. |
| M6 freeze honesty | Take Variant A; reject a freeze locus | Freeze is a rate-zero `TimeMap` segment with provenance and explain output. A synthetic row identity would distort the domain for a tree. |
| M7 Review breadth without gating | Take | `EditProposal` already gives a locus-bound, revision-bound transaction. It can explain every supported verb while direct edits remain legal. |
| M8 role and width harness | Reject from the interaction model | It is useful evaluation instrumentation, but it neither derives from domain semantics nor decides identity, time, scope, or mutation legality. Results may falsify a presentation, not this model. |
| M9 Canvas absence taxonomy | Take and generalize | `evaluate_at`, projection availability, compile, media, and renderer state have different causes. Collapsing them makes hidden magic. |
| M10 overlap candidates | Take, with identity-first bypass | Silent specificity is interaction policy, not discovered semantic singularity. Stable projected identity resolves directly; coordinate-only ambiguity is disclosed before one shared locus is committed. |

## One falsifier

This model is falsified if, in semantically verified overlap and one-to-many fixtures, participants
who are shown the committed locus, independent playhead, projection/absence reasons, and exact verb
scope still cannot predict **which source-backed extent will change** better than with silent
specificity and target fallback—or if correct prediction requires maintaining different semantic
selections per view.

That result would refute the model's central claim that one explicit semantic address plus
domain-derived readings is sufficient for understandable interaction. A preference for different
chrome, slower pointing alone, or failure of one pane arrangement would not.
