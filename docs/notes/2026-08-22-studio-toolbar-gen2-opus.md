# Studio toolbar gen2 — Opus: a verb is drawn on its subject, or it is not drawn

Date: 2026-08-22

Status: one replacement design, written from first principles after the deletion of the standing
verb bank. Discussion input. Not a selected design, not a shipping decision, not an implementation
plan, and not a ranking of anyone else's replacement.

Scope: what replaces the deleted bank, and nothing else. No Core type, no VEL word, no Engine
signature, and no `AGENTS.md` invariant is changed or proposed for change here. Where this note and
`docs/principles.md`, `docs/architecture.md`, `docs/interaction.md`, or `AGENTS.md` disagree, the
spec wins.

Isolation, stated up front so the reader knows what this is worth: this was written without reading
the other gen2 toolbar proposals or the observer and chair threads. It is one line of reasoning
carried to the end, not a synthesis and not an arbitration. Read it as a single opinion with its
receipts attached.

## The premise this starts from

**Deleted, unanimously:** the fixed, always-visible bank of locus-taking `SemanticEdit`s in the
Studio actions row — `Set In`, `Set Out`, `Split at Playhead`, `Delete Selected Clip`, `Gain -3 dB`,
`Fade`.

**Still open, and answered below:** what — if anything — takes their place; whether a non-verb
session strip stays global and what belongs on it; and whether the one utterance is allowed to
commit.

**Not open, and not reopened here:**

- Overlap resolves through Projection-Local's candidate list on the projection that was touched.
- A click on a video clip points the **source clip** and keeps its identity. No promotion to the
  containing scene.
- One locus, one Engine legal set, one utterance.
- No silent retarget: a control never rewrites a target while the displayed locus stays put.
- Title-shaped Inspector fields appear only when here is Title.
- No per-view selection.

Every route proposed below is a route for a locus the user pointed at. None of them changes what
pointing means.

## 1. What the bank actually was

The routing table on `main` reads, for the deleted controls:

```rust
(Projection::Toolbar, LocusKind::Source) => vec!["trim", "set-gain", "set-fade"],
(Projection::Toolbar, LocusKind::Scene)  => vec!["split", "delete"],
```

Put that next to every other row and the defect is structural rather than stylistic. The other three
projections that route anything — Timeline, Canvas, Inspector — each **commit against something they
drew**: a clip, an overlay, a title's text. The rest draw loci and route no verbs at all. `Toolbar`
is the only entry that commits without drawing. It has no hit region that names a locus, so it can
never produce an identity-bearing point; the target of every verb it routes arrives from somewhere
else.

That is the whole disease, and it is worth naming precisely, because "the toolbar is a verb home"
is not the defect — a verb home that can name its target would be fine.

> **A control whose target is not on screen has an implicit argument. The bank had six of them.**

Three consequences follow, and all three were observable before deletion.

**It is present when it is illegal.** Six controls stand whether or not the Engine can name a target
for them. The product's own rule is fail-closed; the bank turned that into fail-*after-the-click*.
The user spends a gesture to be told the control should not have been there. `refuse_edit` exists
entirely to service this: it is a well-written function whose whole job is to apologize for a row of
buttons.

**It parameterizes from state the gesture did not touch.** `Set In` and `Set Out` read the playhead.
`Split at Playhead` reads the playhead. So the identity came from "wherever here happens to be" and
the time came from "wherever the playhead happens to be", and neither is what the hand did. Note
that trim and split already had drawn routes with both arguments in the gesture — the clip edge and
the scene band. The bank was a *second* route to an existing edit, and it was the weaker one.

**It has no display, so it cannot be contradicted.** A button shows a label. It never shows the
value it is about to overwrite, so a wrong result looks exactly like a right one. This is the
consequence that turned out to matter most, below.

### What the fixture says

Run the two audio-ish buttons against the checked-in Alpha fixture,
`examples/gameplay-commentary/main.vel`, through `Engine::propose` / `Engine::apply_proposal` — the
exact path `StudioSession::set_gain` and `set_fade` take:

```text
after one Gain -3 dB click:
      gain fight by --3
    recompiles: OK, diagnostics=0
    audio clip gain_db=Some(3)

after one Fade click: byte-identical=true vel_diff="@@ no line changes @@"

baseline audio clip gain_db=Some(-3)
baseline video clip fade_in=Some(1/2 s) fade_out=None
```

Both results are worse than "no-op".

**`Gain -3 dB` corrupts the fixture and no one is told.** The fixture already reads
`gain fight by -3`. `apply_gain` splices `db.to_string()` over the modifier value span, that span
covers `3` and not `-3`, and the result is `gain fight by --3`. It parses. It compiles with **zero
diagnostics**. The commentary bed goes from −3 dB to **+3 dB** — six decibels the wrong way, from a
button labelled with the value it was supposed to set. This is a defect in `apply_gain`'s span, not
in the toolbar, and it would survive the bank's deletion untouched; it is recorded in §9 as a
finding rather than fixed here. It belongs in *this* section for one reason: **there is nothing on
screen that shows the gain, so nothing on screen disagrees.** A control drawn on the audio block
with `-3 dB` written on it would have shown the value jump to `+3` the instant it committed. The
bank's missing display is what turns a semantic corruption into a silent one.

**`Fade` writes nothing and still costs an Undo.** `Time::milliseconds(500)` normalizes to `1/2 s`
and prints as `0.5s`, which is exactly what line 17 already says. The proposal succeeds, the diff is
`@@ no line changes @@`, the applied source is byte-identical — and `StudioSession::apply_edit`
calls `push_undo()` before it applies, unconditionally, so the observable effect of the button on
the flagship fixture is one Undo entry and one recompile.

One more thing the same run settles, because it decides where two of the four required routes go:

```text
LOCUS   Source id=source:fight      legal=["trim", "set-gain", "set-fade"]
LOCUS    Scene id=scene:demo        legal=["split", "delete", "reorder-scene"]
LOCUS Placement id=demo:video:4     legal=[]     label="source:fight"
LOCUS Placement id=demo:audio:5     legal=[]     label="source:fight"
```

One source binding projects to **two** placements — a Video placement carrying `visual.fade_in` and
an Audio placement carrying `audio.gain_db` — and the placements themselves have empty legal sets.
So `fade` is a *visual* fade on the video projection and `gain` is an audio property, the bank sat
them side by side as if they were the same kind of thing, and the locus that actually carries both
verbs is the source binding that neither block is. The video-click lock is what already bridges
that: the drawn block is a placement, and pointing it yields the source. Everything below is that
bridge used deliberately instead of once.

## 2. The rule

> **A verb's control is drawn on its target, or it is not drawn at all. Where no projection draws
> the target, the verb is legal-and-unrouted, and the utterance says so with the typed reason.**

Read the two halves separately, because they do different work. The first half deletes standing
verb chrome without deleting capability. The second half is what keeps the first half from turning
routing into legality — which is the failure the spine already names.

Three corollaries, in the order they bite:

**C1 — Availability is drawn by the subject, so there is nothing to refuse.** Fail-closed becomes
fail-*early*. You cannot click a verb whose target the Engine cannot name, because that verb has no
control on screen. `refuse_edit` does not disappear — the keyboard, an agent, and the CLI still
arrive without a pointer — but it stops being the ordinary outcome of ordinary clicking.

**C2 — Identity is never implicit. Other arguments must merely be drawn.** This is narrower than
"every argument comes from the gesture", and deliberately so. The playhead is drawn; a time taken
from it is visible. What the bank got wrong was identity: nothing on screen tied `Split at Playhead`
to a scene. So a keyboard accelerator that splits at the playhead is admissible, and a button that
splits "the current scene" is not.

**C3 — The keyboard accelerates handles; it does not add routes.** An accelerator is live exactly
while its handle is drawn, and commits exactly what that handle commits. Without this, deleting the
bank just regrows it in the keymap, invisibly, and "one locus, one legal set, one utterance" quietly
becomes "one utterance and a second set of things that also work."

### This is not a new interaction concept

Canvas already obeys the rule. There is no `Resize Selected Overlay` button; there are four corner
handles drawn on the overlay that is here, each committing one `ResizeOverlay` on pointer-up,
Escape-cancellable, bound to `TimelineClip.id` / `LocusId` rather than to visible text. Nobody
proposed a toolbar button for it, and the reason is not that resize is special — it is that Canvas
draws its subject.

The bank existed for exactly the verbs whose subject was drawn on a rail that had never grown
handles. So this is not a new model competing with the old one. **It is Canvas's existing grammar
finished on the Timeline.** That matters for cost: the gesture lifecycle, the one-edit-per-pointer-up
contract, the Escape path, and the `LocusId` binding are all already specified and already shipped
for the Canvas case.

## 3. The routes

Required by the brief, and answered without hedging. Every row is a gesture on a region that draws
the target.

| Verb | Route | Region that draws the target | Identity from | Other arguments |
|---|---|---|---|---|
| `set-gain` | **Timeline · Audio rail · gain line on the source's audio block, vertical drag** | the audio block, which is the source binding's audio projection | the block → its `source_id` → `source:fight` | Δy → absolute dB |
| `set-fade` | **Timeline · Video rail · head wedge on the source clip, horizontal drag** | the video block, which is the source binding's visual projection | the same `LocusId` as the gain line | Δx → `fade_in` |
| `split` | **Timeline · scene band · cut lane, click** | the scene band | the band | pointer x → timeline time → source time |
| `delete` | **Timeline · scene band · delete handle drawn while that band is here, click** | the scene band | the band | none — `SemanticEdit::Delete` has no fields |

And the rows that already complied, restated so the table is the whole story:

| Verb | Route |
|---|---|
| `trim` | Timeline · Video rail · clip in/out edge drag. The playhead is a **snap target**, not an implicit argument. `Set In` / `Set Out` are absorbed here, not replaced. |
| `reorder-scene` | Timeline · scene band body drag |
| `title`, `callout` | Timeline · overlay body/edge drag; `title` text on the Inspector when here is Title |
| `set-position`, `resize-overlay` | Canvas · overlay body drag and corner handles |

Four details that are load-bearing rather than decorative:

**The gain line spans the whole block, because the scope is the whole binding.** `legal_edits_for`
gives `set-gain` the scope `source-binding`. Core has no time-scoped audio edit. So the line is
drawn edge to edge and a drag never produces a range — drawing a partial-width ramp would simulate a
gap that Core does not have, which is the one thing the Audio rail is specifically forbidden to do.
The line also displays the current value, which is what §1 says the bank lacked.

**`SetGain` is absolute, and the drawn control finally says so.** The deleted button read
`Gain -3 dB`, which any editor parses as a relative trim. `SemanticEdit::SetGain { db }` sets; twice
is not −6. A line at a height with a number on it is the honest presentation of an absolute value.

**There is no tail wedge.** `SemanticEdit::SetFade` carries `fade_in` only. `TimelineClip.fade_out`
exists, `evaluate` honours it, and nothing in VEL or in the edit surface can set it. Drawing a tail
handle would invent the missing half. The absence is spoken instead: this is "the missing instrument
speaks" applied to a Core gap that is named rather than filled.

**The cut lane needs no new mapping.** `apply_split` wants a source time inside the binding's
`media[start..end]`. `split_at_playhead` obtains one today by calling `map_timeline_to_source` on the
playhead. The lane calls the same function on the pointer x. The argument becomes visible without
becoming new.

### Unrouted, and what it does not mean

A verb with no drawn subject is **legal and unrouted**. It is not absent, not illegal, and not
silently missing. Named cases:

- **`too-small-to-draw`.** A clip a few pixels wide cannot host an in-edge, an out-edge and a head
  wedge at legible size. The handles are not drawn overlapping; the verb is spoken as unrouted at
  this zoom, and `Zoom In` — already on the session strip — is the affordance that restores it.
- **`no-drawn-block`.** If a rail draws no block for a binding, the verb that lives on that block
  has nowhere to sit. This is the mirror of the existing obligation that a drawn block must be
  pointable: an undrawn subject must be *spoken*, not silently dropped.
- **`structurally-absent`.** The Engine names nothing at all. `legal_edits_for` returns `[]` for
  `Speech`, `Placement`, `Sequence`, and `Media`. Under the bank this was the worst state in the
  product: six controls, all clickable, all refusing.

Unrouted is a Studio-routing fact, not an Engine fact. The CLI and an agent commit against the
`LocusId` directly and are not bound by any of this. That is the same separation the spine draws
between legality and routing, applied to the case where routing has no surface.

## 4. The utterance is disclosure-only

**Answer: disclosure-only. The utterance never commits a `SemanticEdit`.**

Four reasons, in descending order of how much they would hurt to get wrong.

**A clickable clause is the bank with better prose.** Everything in §1 applies verbatim to
"`split` — committed on Timeline `[do it here]`": permanently present, target supplied implicitly by
here, no display of what it would change. Deleting a row of buttons and growing the same authority
inside the sentence that explains the row's absence would be the most expensive kind of no-op.

**A speaker that is also an instrument cannot report a missing instrument.** The utterance exists to
make the gap between the Engine legal set and the gesture-committable set observable. If it can
close that gap itself, the gap stops being detectable — every route hole becomes commit-able from
the sentence, and the routing table stops being falsifiable. "The missing instrument speaks" needs
the speaker not to be an instrument.

**One utterance has to survive having no pointer.** The same derivation is consumed by Studio, by
`inspect --json`, and by an agent. A reading can be identical across all three. A commit surface
cannot, and the moment it diverges there are two answers to "what may I do here", which is the
condition `AGENTS.md` forbids by making the CLI a first-class client.

**It keeps the one-pointer-up contract trivially true.** Every commit path is a handle with a
gesture lifecycle. There is no second commit path with different cancellation semantics, no
"Escape while a clause is focused" question, and no clause that has to decide what its ephemeral
geometry is.

**The one carve-out, drawn tightly.** The utterance may carry `Navigate` and explicit seek. Both
are non-edits: they change no source, push no Undo, and do not change here. The line is mechanical
rather than tasteful:

> **The utterance may move the eye. It may never move the source.**

That is checkable by construction — nothing reachable from the utterance may reach `apply_edit` —
and it is why the routing clause in scene B ends with "Navigate shows the band; your click points
it." Two explicit acts, zero silent retargets. The user's own pointing is what changes here, which
is exactly what the no-silent-retarget lock asks for.

**Its home is layout, not model.** A persistent line is fine; a floating popup anchored at the
witness is not required and should not be inherited as an invariant, since the note that proposed
anchoring conceded it does not solve occlusion. What the model requires is that the home is legible
as a reading — text, no controls — so that "the utterance is the new bank" cannot become true by
appearance.

## 5. The session strip

Membership is not settled, and this note does not settle it. What it offers is a test, because a
list without a test regrows.

> A control may stand permanently only if **(a)** it commits no `SemanticEdit`, and **(b)** its
> subject is the session, the document, or the application — something that is always present.

Applying it to the row as it exists today, as an input rather than a decision:

| Control | (a) no `SemanticEdit` | (b) always-present subject | Reading |
|---|---|---|---|
| `Play`, `Pause`, `Seek`, `Scrub` | yes | transport | stands |
| `Zoom In`, `Zoom Out` | yes | viewport | stands — and is the affordance for `too-small-to-draw` |
| `Save`, `Resolve`, `Open Video…` | yes | document / phase | stands |
| `CPU`, `GPU DX12`, renderer + audio status | yes | explicit renderer request, device state | stands |
| `Undo`, `Redo` | no — they mutate source | session history | stands, and is the one row that needs an argument |
| `Copy locus JSON` | yes | **here** | stands only if it fails closed and speaks when here is unset |
| the six deleted verbs | no | a locus | fails both — this is the deletion |

`Undo` / `Redo` deserve the argument rather than a checkmark. They mutate source, so they fail (a)
as written. They pass the rule the test is *for*: their subject is the volatile session history,
which is always present, and they take no `LocusId`, so they cannot invent a target or retarget
anything. The honest statement of the test is therefore about targets rather than about mutation —
*no standing control may name a locus it did not draw* — and Undo/Redo name none.

`Copy locus JSON` is the interesting row and the reason the test is not just "no edits". It reads on
here, which means it can be pressed when here is unset. It is not a verb, so it does not fall to the
rule; but it inherits the same obligation the rule enforces everywhere else — say what is missing
rather than doing something plausible.

## 6. What this asks the code to stop doing

Consequences of the rule that are mechanical enough to be tested, listed so the note is not a mood.
None of these is implemented here.

- **`routed_verbs(Projection::Toolbar, _)` is empty for every `LocusKind`.** Not "smaller" — empty.
- **`commit_projection` never returns `Toolbar`,** and no spoken clause ever contains
  `committed on Toolbar`.
- **`commit_projection`'s `None` must be honoured.** `speak_legal_vs_routed` currently does
  `.unwrap_or(Projection::Timeline)`, so an unrouted verb is spoken as "committed on Timeline". That
  is a fallback that manufactures a false route — the same species as the deleted
  `target_source_locus` fallthrough, one layer up in the sentence. Unrouted must reach §3's clause,
  with its reason.
- **An empty legal set must be spoken.** `utterance()` builds its clauses by iterating the legal
  set, so a locus with no legal edits — `Speech`, `Placement`, `Sequence`, `Media` — produces no
  clauses at all. Today that is a blank line where `structurally-absent` belongs. Silence is the one
  answer the utterance is not allowed to give.
- **Handles bind by `LocusId`,** never by visible label or span. `crates/lattice-studio/tests/layout.rs`
  already builds duplicate overlays with identical labels and spans; that fixture is the stress test
  for handles exactly as it is for overlap cards.
- **The audio block's point resolves to the same `LocusId` as the video block's.** Both are
  projections of one source binding; two loci for one binding would be per-view selection wearing a
  rail. The video half of this is locked. The audio half is this note's decision, and it is stated
  here rather than assumed.

## 7. What this will be misread as

**"You deleted gain, fade, split and delete."** The most likely and the most expensive. Four verbs
gained drawn routes; nothing lost a route. What died is the class of control whose identity argument
was implicit. If the change ships and this is what users say, the routes were drawn badly — not the
rule.

**"Handles are a hover toolbar."** They will accrete if nothing forbids it. The governor is narrow
and should be stated wherever the rule is: **one region, one verb, arguments from that region.** A
region that would host a second verb is a bank with a smaller footprint.

**"The utterance is the new bank."** Answered in §4 with a mechanical falsifier rather than a
promise, because a promise is what everyone makes here.

**"Playhead editing is gone."** It is snapping now, plus accelerators that are live while their
handle is drawn. `Set In` at the playhead survives as a trim edge that snaps; what does not survive
is a control that reads the playhead while nothing on screen says which clip it will cut.

**"This is Projection-Local Verbs by another name."** No, and the distinction is the one the spine
turns on. Legality stays global and Engine-owned; a verb with no drawn handle anywhere is still
legal and is still spoken. Only *drawing* is local. The rule constrains routing and touches license
nowhere.

**"A drawn handle means the edit will succeed."** It means the Engine named the verb legal for that
locus. §9 records a case where those come apart.

## 8. What would kill this

**The refusal count.** The rule predicts that deleting the bank removes refusals rather than
capability. Measure: after the change, can a user produce "X is not legal for here" using only a
pointer? Every such path is a surviving bank. This is the primary falsifier and it is cheap.

**Gestures per commit.** If trim, split, gain and fade cost more gestures on
`examples/gameplay-commentary/main.vel` than the bank did *for verbs the bank actually committed
correctly*, the rule bought honesty with throughput and has to be re-argued. Note that the fixture
currently offers zero such verbs for the audio pair, which makes the comparison easier than it
should be — that is a fact about §9, not a defence.

**First-hour discoverability.** Handles are invisible until here is the thing. The first-time editor
prior expects a toolbar, and this model has none. Mitigations are real but unproven: rest state
predicting the role on hover, and the utterance naming the region in words. If a first-time user
cannot find `split` within a minute, the rule is right and the drawing is insufficient — but if they
cannot find it after being told where it lives, the rule is wrong.

**Handle density.** Four verbs want room on one clip. `too-small-to-draw` is an honest answer and an
ugly one, and if it fires at ordinary zoom on ordinary material then the rail needs a different
layout, not a different rule.

**The dense-tie fixture.** Two invocations with identical spans and distinct `LocusId`s already
exist in `layout.rs`. Handles drawn on both must stay distinguishable and must not commit against
the wrong one.

## 9. Findings this surfaced

Recorded because they are true today and independent of whether anything here ships. Not fixed in
this note.

1. **`apply_gain` corrupts an existing negative gain.** `gain fight by -3` + `SetGain { db: -3 }`
   → `gain fight by --3`, which compiles clean and evaluates to **+3 dB**. The modifier value span
   appears to exclude the sign. This is reachable from Studio today, on the flagship fixture, from a
   button labelled with the value it fails to set. It is the most serious item on this page.
2. **`SetFade` at the fixture's own value is an empty diff that still consumes Undo.**
   `Engine::propose` succeeds with `@@ no line changes @@`, and `StudioSession::apply_edit` pushes
   Undo before applying, unconditionally.
3. **`fade_out` is reachable in Core and unreachable from the edit surface.** `TimelineClip.fade_out`
   is honoured by `evaluate`; no VEL word and no `SemanticEdit` field sets it.
4. **`legal_edits_for(Scene)` is looser than `apply_split`'s preconditions.** `split` is named legal
   for any Scene, while `apply_split` additionally requires a binding shaped `media[start..end]`.
   A cut lane drawn from the band's own range mostly hides this, which is precisely why it is worth
   writing down: the rule can mask an over-broad legal set instead of exposing it.
5. **`source:speech-nice-freeze` is a `Source` locus with `trim` / `set-gain` / `set-fade` legal,
   whose only drawn block is the generated speech placement.** Whether pointing that block should
   yield the `Speech` locus (empty legal set) or the source binding is a question the rule forces
   into the open. It is pointing, so this note names it and stops.

## 10. Scenes

Rendered from [`docs/sketches/toolbar-gen2-opus/index.html`](../sketches/toolbar-gen2-opus/index.html)
(`?scene=a|b|c|d`). Stills, not video.

**A — the bank that is deleted.** Six controls, no drawn subjects, and the refusals that arrive
after the click.

<img alt="Scene A — the standing verb bank, with its refusals" src="https://github.com/annenpolka/lattice/blob/02541549efb4547991a6598cea9ab38b2f0d3c54/docs/sketches/toolbar-gen2-opus/stills/scene-a.png?raw=true" />

**B — here is `source:fight`.** Trim edges and the fade wedge on the Video rail, the gain line on
the Audio rail, one `LocusId` across both, and the scene's verbs spoken as a relation.

<img alt="Scene B — three handles drawn on the source clip's two projections" src="https://github.com/annenpolka/lattice/blob/02541549efb4547991a6598cea9ab38b2f0d3c54/docs/sketches/toolbar-gen2-opus/stills/scene-b.png?raw=true" />

**C — here is `scene:demo`.** Cut lane, delete handle, band body: three regions, three verbs, and
the source-binding affordance instead of an invented target.

<img alt="Scene C — cut lane and delete handle on the scene band" src="https://github.com/annenpolka/lattice/blob/02541549efb4547991a6598cea9ab38b2f0d3c54/docs/sketches/toolbar-gen2-opus/stills/scene-c.png?raw=true" />

**D — here is the generated speech placement.** The Engine names nothing, so nothing is drawn, and
the empty legal set is spoken rather than left blank.

<img alt="Scene D — an empty legal set, drawn as nothing and spoken as structurally-absent" src="https://github.com/annenpolka/lattice/blob/02541549efb4547991a6598cea9ab38b2f0d3c54/docs/sketches/toolbar-gen2-opus/stills/scene-d.png?raw=true" />

## What this note does not do

- It does not implement anything. No Studio, Engine, Core, or VEL change is made or planned here.
- It does not reopen either pointing lock, and it does not select or rank a verb-license model.
- It does not settle session-strip membership. §5 offers a test and one reading of the current row.
- It does not fix the defects in §9, and it does not invent Core to close them: no time-scoped audio
  edit, no `fade_out` edit variant, no new legality for a placement.
- It does not add a typed reason to `AbsenceReason` on its own authority. `too-small-to-draw` and
  `no-drawn-block` are named as routing reasons a design would need; adding them is a decision for
  whoever ships this, with an explain event and an origin, not a side effect of a note.
