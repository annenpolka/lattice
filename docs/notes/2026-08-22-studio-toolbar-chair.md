# Studio toolbar talk — Sol chair

Date: 2026-08-22

Role: Phase II chair. This is a docs-only synthesis of six isolated observer
notes. It does not choose an observer, cast a Phase III vote, or propose Studio,
GPUI, or crate implementation.

## Question held by the chair

Should Studio have a **global top-of-window verb button row** under the locked
spine?

The observers converge on a narrow negative claim: the current, fixed,
always-visible bank of locus-taking `SemanticEdit` buttons does not make a
coherent global verb surface. They do **not** settle the larger chrome question.
A session/application strip may remain global without becoming a verb home, and
the replacement routes for currently toolbar-only verbs remain design work.

That distinction is the chair result, not a winner.

## Source pack

The notes were read from the six PR heads, not from `main`.

| PR | Observer note | HEAD read |
|---|---|---|
| [#25 — Studio global verb row review — semantics](https://github.com/annenpolka/lattice/pull/25) | `docs/notes/2026-08-22-studio-toolbar-semantics.md` | `09ae71842c513152b788d59a430f2260ae81be02` |
| [#26 — Studio toolbar observe — hierarchy](https://github.com/annenpolka/lattice/pull/26) | `docs/notes/2026-08-22-studio-toolbar-hierarchy.md` | `6ce30f47ee6a403006994503e8b2cb0dc5815d84` |
| [#27 — Studio toolbar observe — structure](https://github.com/annenpolka/lattice/pull/27) | `docs/notes/2026-08-22-studio-toolbar-structure.md` | `5e06101f53aed17aded3eb08758016693d13bd71` |
| [#28 — Studio toolbar observe — affordance](https://github.com/annenpolka/lattice/pull/28) | `docs/notes/2026-08-22-studio-toolbar-affordance.md` | `e99b03545e30b5a300cb695424d2219cb622989d` |
| [#29 — Studio toolbar observe — interaction](https://github.com/annenpolka/lattice/pull/29) | `docs/notes/2026-08-22-studio-toolbar-interaction.md` | `d7497ec9ea22a1ed8eed2a35f3caba68339ff98a` |
| [#30 — Studio toolbar observe — first principles](https://github.com/annenpolka/lattice/pull/30) | `docs/notes/2026-08-22-studio-toolbar-first-principles.md` | `99ad0d7ed8c93e384116693c011006677cb4175e` |

PR #30 uses the expected path.

## Six-lens ledger

`DELETE` below means remove a claim from the decision frame. It is not an
instruction to edit an observer branch or remove UI.

### #25 — semantics

- **KEEP:** legality, routing, and utterance are separate authorities. A global
  click has no intrinsic projection route, and a button must use the one here or
  refuse; it must never search for a more convenient locus.
- **STEAL:** the useful partition between locus-taking semantic edits and
  transport, viewport, persistence, history, Resolve, runtime, and agent-context
  commands. Co-location does not give these classes one semantic authority.
- **DELETE:** “the alternative is no global verb home” as though it also decides
  whether a non-verb global strip exists. It decides the verb object only.
- **MISREAD:** a legal verb with no current route is not thereby illegal or
  absent. Removing its button before supplying a route would reduce
  reachability, not improve the spine.
- **UNSTATED ASSUMPTION:** a top-row button is treated as a fourth projection.
  An explicit command can instead be a reader of the one locus and still
  fail-closed; whether that is a good rendered affordance is the disputed point,
  not a lock violation by definition.

### #26 — hierarchy

- **KEEP:** the current wrap creates accidental tiers, uniform proximity flattens
  unlike command classes, and teal carries incompatible meanings. These explain
  why the current strip is perceived as one global promise.
- **STEAL:** visual grouping can manufacture semantic grouping. The row teaches
  “everything here is equally and presently invocable” before Engine legality
  has said so.
- **DELETE:** cosmetic repair as an answer to the chair question. Dividers,
  stable wrapping, and color roles could improve chrome while leaving the false
  global verb promise intact.
- **MISREAD:** the proposed clean header allocation is a design sketch, not an
  observation forced by visual evidence. Visual hierarchy can reject the current
  aggregation without deciding the permanent home of Save, Resolve, renderer
  status, or transport.
- **UNSTATED ASSUMPTION:** standard NLE placement is taken as the natural end
  state. Lattice's text-first and explicit phase boundaries may justify
  unfamiliar placement if the behavior is disclosed.

### #27 — structure

- **KEEP:** there is no toolbar object or store today. `actions_bar` is a mixed
  flex; `Projection::Toolbar` is only a routing tag; `header_bar` and
  `actions_bar` own neither locus nor legality.
- **STEAL:** the inventory supplies the decisive type split: six controls stamp
  Toolbar and apply semantic edits, while fourteen buttons and two status items
  do unrelated session, view, runtime, phase, or utility work.
- **DELETE:** shared `action_button` construction and `toolbar.*` selector names
  as evidence that the controls form one product object. Those are implementation
  coincidences.
- **MISREAD:** “Timeline, Canvas, and Inspector are the commit surfaces” is
  descriptive only for routes already implemented. Sequence renders structural
  targets but currently commits none; Review and direct VEL editing are separate
  workflows and should not be silently erased by a three-surface slogan.
- **UNSTATED ASSUMPTION:** removing `Projection::Toolbar` is equivalent to
  removing the visible row. The routing tag, semantic buttons, and non-verb
  chrome are separable decisions.

### #28 — affordance

- **KEEP:** labels make concrete promises that conflict with actual targets:
  video click gives Source while Split needs Scene; “Delete Selected Clip”
  names a forbidden selection and a clip while the edit targets Scene; fixed
  gain/fade values present parameters as verbs.
- **STEAL:** a global, uniformly clickable row hides its context contract. A
  correct refusal protects state but does not retroactively make the invitation
  truthful.
- **DELETE:** familiar NLE vocabulary as sufficient proof of correct behavior.
  “Familiar” can itself be a damaging misread when Lattice means a different,
  explainable semantic edit.
- **MISREAD:** `Open Video…` replacing the current session is not, by itself,
  proof that the document is destroyed; persistence and replacement are
  different claims. The serious affordance defect is that “Open” does not
  disclose replacement.
- **UNSTATED ASSUMPTION:** first-time interpretation is inferred from conventions
  and one UI inspection, not measured with users. Treat the note as a strong
  heuristic audit, not usability-study evidence.

### #29 — interaction

- **KEEP:** runtime behavior exposes the contradiction sharply: the same stable
  button can be a lawful commit for one here and a lawful refusal for another,
  while the refusal is spoken away from the control.
- **STEAL:** “Toolbar commits” names an invented home for orphan verbs. The
  observed successful Scene split demonstrates current reachability, not a
  semantic need for a global verb object.
- **DELETE:** a legal clause becoming clickable as the automatic replacement.
  That would make Inspector/utterance an invocation surface and needs its own
  target, parameter, focus, and refusal account.
- **MISREAD:** an always-visible command using the one here is not silent
  retargeting when its target is disclosed. Its remaining defects are hidden
  ambient target, weak spatial routing, and repeated refusal; those are enough
  to challenge it without claiming it breaks a lock it may obey.
- **UNSTATED ASSUMPTION:** every useful edit must begin as pointer direct
  manipulation. Keyboard and command invocation may be globally reachable in
  input while remaining locus-scoped in semantics.

### #30 — first principles

- **KEEP:** the cleanest criterion in the pack is “does this control take a
  locus?” A locus-taking `SemanticEdit` should be local in semantics; document,
  session, transport, phase, runtime, and view commands need not be.
- **STEAL:** fixing membership, target/scope/effect disclosure, and route state
  makes a static global verb bank converge toward the existing utterance. Also
  keep the warning that removing the row now strands `set-gain`, `set-fade`,
  `split`, and `delete`.
- **DELETE:** the claimed impossibility for every global row. It proves that a
  **fixed union** cannot equal every Engine legal set; it does not mathematically
  exclude a locus-conditioned rendering in a fixed global location. The later
  usability argument against that rendering remains relevant.
- **MISREAD:** “the target is the thing under the cursor” is exact for direct
  Canvas/Timeline gestures but incomplete for Inspector fields, keyboard
  bindings, and commands. Those can explicitly operate on the one here without
  creating a second selection.
- **UNSTATED ASSUMPTION:** the Inspector can absorb property-shaped invocation
  without becoming overloaded, and the utterance can carry discovery once it is
  legible. The note correctly names legibility and complete local routes as
  prerequisites; they are not yet established outcomes.

## Tensions that survive synthesis

### 1. What “global” modifies

The pack sometimes treats global position, global membership, global legality,
and global target as one property. They are not:

- a control can occupy a fixed window location;
- its contents can be conditioned on the one locus;
- legality can still come only from Engine;
- invocation can still target only the disclosed here.

The observers strongly reject fixed global **membership** presented as standing
legality. They do not jointly disprove every locus-conditioned control rendered
in global **position**. Conversely, making the row contextual may turn it into a
duplicate rendering of the utterance and sacrifice the fixed membership that
made it useful. That product trade remains open.

### 2. Discovery versus invocation

The utterance is the complete discovery account; a button label is not. But
making the utterance clickable changes its role from disclosure to commit
surface. The pack offers three live possibilities without selecting among them:

1. utterance discloses only; projections supply all invocation;
2. Inspector renders locus-conditioned property invocation beside disclosure;
3. non-visual bindings invoke against the disclosed one here and speak refusal.

None permits a second legal set or silent target substitution.

### 3. Semantic purity versus present reachability

Four legal edits currently depend on Toolbar routing:
`set-gain`, `set-fade`, `split`, and `delete`. Calling them “debt” is persuasive,
but deleting their only route would make legal work unreachable. A transition
must preserve one named route per legal edit or explicitly speak that no route
exists. “No global verb row” is therefore a destination constraint, not by
itself a safe removal sequence.

### 4. Which local projection owns structural and property verbs

The pack agrees on Canvas geometry and Timeline time-shaped edits. It does not
settle:

- whether Scene `split`/`delete` commit on Timeline, Sequence, or both;
- whether source gain/fade commit through Timeline controls, Inspector
  properties, or both;
- whether Inspector invocation is limited to coordinate-free properties;
- how a command or binding exposes parameters without frozen magic values.

These are routing and interaction questions. They cannot be answered by moving
the current buttons cosmetically.

### 5. A global shell can survive without a global verb home

Save, Undo, Redo, Resolve, renderer choice/status, audio status, transport,
viewport controls, import/session replacement, and agent-context copy do not
share one authority merely because they take no locus. The first-principles
criterion permits global placement; hierarchy and affordance still question
their grouping, labels, prominence, and disclosure. “Not a locus-taking verb”
is necessary for coherent global chrome, not sufficient for one undifferentiated
row.

### 6. Expert speed does not settle visual semantics

A fixed rendered bank offers discoverability and muscle memory, but the current
bank is unstable under wrapping and freezes parameters. A keyboard binding or
command invoke can be global in input while gated by the one Engine legal set.
That may preserve speed without a standing visual claim, but the pack neither
specifies nor validates such a system.

## Phase III absorb

This section absorbs the six isolated vote addenda without converting their
different motions or test readings into one voice. It records concurrence,
contradictions, and residue. It is not a winner selection and does not open
Phase IV.

### Addenda read

| PR | Vote HEAD read | Motion kept distinct |
|---|---|---|
| [#25 — Studio global verb row review — semantics](https://github.com/annenpolka/lattice/pull/25) | `1e8ebf5639b2347432d60a21bb5737d07e800dd0` | DELETE the fixed `SemanticEdit` bank; KEEP a session strip grouped by authority; STEAL disclosure and commit into **Route Ledger + Projection Commit Affordance**. |
| [#26 — Studio toolbar observe — hierarchy](https://github.com/annenpolka/lattice/pull/26) | `7b1f1d67624c476707568d255d83c2fa61dfb08a` | DELETE the locus-taking bank; KEEP a single-tier session/header strip, especially Save, Undo, Redo, Resolve, and quiet status. |
| [#27 — Studio toolbar observe — structure](https://github.com/annenpolka/lattice/pull/27) | `c149d8c3620b2df3cc6c11a6b2130df00410bf1b` | DELETE the locus-taking bank; name a non-verb session strip grouped by owner; keep verbs on Timeline, Canvas, and Inspector. |
| [#28 — Studio toolbar observe — affordance](https://github.com/annenpolka/lattice/pull/28) | `c958c4ea8e2ec536d0740701ea13f530bbbcf134` | DELETE the global verb bank; STEAL transport to Timeline, session operations to a thin strip, and Resolve to a phase drawer. Current row: tests 1/2/6 FAIL, 3/4/5 PASS. |
| [#29 — Studio toolbar observe — interaction](https://github.com/annenpolka/lattice/pull/29) | `d730e572d6b320d2dbb9fd1d6b2fa550276c4652` | DELETE Set In/Out, Split, Delete, Gain, and Fade as a locus-taking bank; KEEP non-locus session chrome as a separate object; make no STEAL into another global verb home. |
| [#30 — Studio toolbar observe — first principles](https://github.com/annenpolka/lattice/pull/30) | `b8c25dd4866f1a0bfbfd8b35729bf8bf498fc844` | DELETE the locus-taking bank; STEAL verbs to local surfaces; keep a predicate-defined session strip. Current bank: tests 1/2/5/6 FAIL, 3/4 PASS. |

The PR #30 vote HEAD is confirmed as
`b8c25dd4866f1a0bfbfd8b35729bf8bf498fc844`.

### What is jointly rejected

All six addenda reject the same narrow object: a fixed, always-visible global
bank containing Set In, Set Out, Split at Playhead, Delete Selected Clip,
Gain -3 dB, and Fade as standing invitations to locus-taking
`SemanticEdit`s. None votes to repair that object through grouping, color,
renaming, enablement, or another global verb dock.

The rejection is narrower than “delete top chrome.” Every addendum permits some
non-locus global session/application chrome, although they disagree about its
membership and placement. They also agree that the current commit gate already
preserves Engine-only legality, one here, fail-closed refusal, and no hidden
target search or promotion. The vote is against the bank's invitation,
disclosure, route naming, and authority mix—not against those spine properties.

This concurrence does not select the #25 named object, the #28 redistribution,
the #30 local-surface motion, or any other packet as the shipping answer.

### Contradictions absorbed, not resolved

#### Test 5: named route versus nameable place

#28 scores the current row PASS because the utterance names Toolbar, Timeline,
Canvas, or Inspector and omits no legal edit. #29 scores the bank FAIL because
Toolbar becomes a circular home for its own orphan buttons. #30 also scores it
FAIL, qualified: `Toolbar` is named but is not a place in which the target can
be pointed, and the row's duplicate trim route is not the route the utterance
names. #25 and #26 score their proposed replacements PASS by requiring local or
explicitly `unrouted` disclosure; that is a requirement of those proposed
objects, not evidence that the local routes already exist.

The unresolved criterion is whether “named route” means any routing enum value
spoken without omission, or a projection where the target is rendered and a
commit can be initiated. The six addenda do not share that definition.

#### STEAL now, name debt, or redistribute command classes

#25 names Route Ledger + Projection Commit Affordance. #27 and #30 move verbs to
local surfaces. #28 additionally redistributes transport, Resolve, telemetry,
and developer controls to different objects. #29 deliberately offers no STEAL
for the six buttons and leaves the orphan routes as named leftovers rather than
smuggling a shipping choice into the vote.

These motions agree on DELETE but are not interchangeable. “STEAL to local
surfaces” needs target-specific gestures and parameter entry. “Leave spoken
unrouted” preserves the spine but reduces immediate reachability. “Route Ledger”
may remain disclosure or may become an invocation object; those are different
interaction contracts.

#### Session strip membership and authority

#26's retained strip is narrow and visually specific: Save, Undo, Redo, Resolve,
and status. #27 groups a broader non-verb strip by state owner. #28 removes
transport to Timeline and Resolve to a phase drawer. #29 keeps transport,
renderer request, Save, Resolve, Open Video, Zoom, and locus copy eligible for a
separate global object. #30 defines membership by the predicate
`commits_semantic_edit == false` and then groups by authority.

Thus all six permit a session strip, but there is no joint strip design.
“Non-locus-taking” is the shared admission test; it does not settle grouping,
rank, disclosure, or physical location.

#### Test 3 describes a gate, not a truthful invitation

The vote tables phrase test 3 differently. #28 and #30 mark Engine-only legality
PASS. #27 and #29 distinguish a gate pass from a surface failure because the
bank authors an invitation set before consulting Engine. These are compatible
facts attached to different tests: Engine remains the only commit authority,
while the rendered bank can still imply a broader offer. The chair does not
convert this wording difference into a vote split about the spine.

### What remains open

#### Gain and fade

The addenda do not choose between a Timeline audio binding control, an Inspector
property with explicit dB/duration, or more than one projection route. Any
answer must disclose the source target and parameter, use the one here, and
avoid frozen magic values masquerading as complete verbs.

#### Split and delete

The addenda do not choose Timeline, Sequence, or both for Scene structural
commits. Video click remains Source, so neither action may silently promote the
pointed clip to its related Scene. A chosen route must make the Scene target
explicit before commit.

#### Utterance as disclosure or commit

#25's Route Ledger is explicitly the one utterance, while its projection
affordances commit. #28 treats Inspector as the primary disclosure surface.
#29 refuses to STEAL the buttons into another global home. #30 says correcting a
global row converges on the utterance/Inspector but leaves actual route ownership
undecided.

The open question is whether an utterance clause only names legal and unrouted
work, navigates to a committing projection, or itself becomes an Inspector
commit affordance. Making it clickable is not a neutral presentation change: it
must define focus, target visibility, parameter entry, and refusal without
creating a second legal set or utterance.

#### Transition and reachability

DELETE is unanimous as an object judgment, not as an immediate implementation
sequence. `set-gain`, `set-fade`, `split`, and `delete` currently rely on
Toolbar routes. Before those routes disappear, each needs a named local route
or an explicit `unrouted` clause. The addenda disagree on whether spoken
unrouted is an acceptable intermediate product state; none authorizes silently
dropping a legal edit.

### Phase III boundary

The chair absorbs the unanimous rejection and preserves the disagreements
above. There is still no observer winner, replacement winner, implementation
motion, or Phase IV decision.

## Chair tests retained

The Phase II decision frame remains useful for reading the absorbed votes,
without reopening the spine:

1. Does the object render a standing invitation for a locus-taking edit?
2. Is target, scope, effect, parameter, and committing projection disclosed
   before commit?
3. Does Engine remain the only legality authority?
4. Does the gesture/command use the one here and fail closed, with no target
   search or promotion?
5. Is every legal edit reachable through at least one named route, or explicitly
   spoken as currently unrouted?
6. Are non-verb global controls grouped by their actual authority rather than by
   implementation convenience?

This frame rejects cosmetic toolbar polish as an answer. It does not pick
Timeline, Sequence, Inspector, utterance clauses, bindings, or any observer as
the winner.

## Locks held

- Overlap remains Projection-Local: failed point → candidates on that
  projection; no cross-surface modal; one `LocusId` after pick.
- Video click points the source clip; there is no Scene promotion.
- There is one locus, one Engine legal set, and one utterance. Gesture is
  routing. Studio speaks when legality differs from routing. There is no silent
  `target_*_locus`.
- Scrub does not call `point_from_timeline_time`.
- Title fields appear only on Title.
- There is no freeze row, Core Freeze, per-view selection, or GPUI in Core.

## Out of scope

No toolbar styling, layout, widget design, Studio/GPUI/crate implementation,
observer-branch edits, observer-PR merges, observer ranking, winner selection,
a new chair vote, or Phase IV.
