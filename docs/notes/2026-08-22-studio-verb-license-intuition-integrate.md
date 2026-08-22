# Verb license: intuitive for whom, and is there one coherent model?

Date: 2026-08-22

Status: a reading of three existing gen2 interaction notes. Not a selected design, not a shipping
decision, not an implementation plan.

Scope: the verb-license question only. No Studio, GPUI, Core, or sketch change is proposed here, and
none is implied by anything below.

Locked constraint: **the overlap UI is not open in this note.** Resolution of a point that names
several loci is fixed to Projection-Local Verbs — a candidate list on the projection that was
touched, no cross-surface modal, one shared `LocusId` after the pick — as drawn in scene D of its
[sketch](https://github.com/annenpolka/lattice/blob/39fc7827e69a4b1d70608402ea1b01eefe7a05a1/docs/sketches/projection-local/index.html).
Compass's coordinate probe, the Reading's rank-then-step, and click-to-commit are therefore out as
overlap UI and are proposed nowhere below. The lock covers overlap only. It does not select a
verb-license model, which is why the two questions are still open questions.

## What is being read

Three second-generation interaction notes, each answering "what licenses a legal verb?" differently
while keeping the same fixed boundary: **"here" is one committed `LocusId`, there is no per-view
semantic selection, Studio is an Engine client, and domain nouns are not warped for widget
convenience.**

| Model | Note | Sketch | Its rule, in one line |
|---|---|---|---|
| Semantic Compass | [note](https://github.com/annenpolka/lattice/blob/0550794d549fe597ab7b3b6a950968592518347a/docs/notes/2026-08-22-studio-interaction-semantic-compass-model.md) | [sketch](https://github.com/annenpolka/lattice/blob/0d9876a94a9e28a665131200b7b1cb708dd0535f/docs/sketches/compass/index.html) | Touch points here; verbs live in one Engine list, fail-closed. Surfaces do not license verbs. |
| Projection-Local Verbs (Model 2 only) | [note](https://github.com/annenpolka/lattice/blob/b8243e7792059d91188f6a953269002fea666d3e/docs/notes/2026-08-22-studio-interaction-gen2-weighted-subject-and-projection-verbs.md) | [sketch](https://github.com/annenpolka/lattice/blob/39fc7827e69a4b1d70608402ea1b01eefe7a05a1/docs/sketches/projection-local/index.html) | The touched projection licenses verbs. Canvas ∩ Title = `SetPosition`; Timeline ∩ Title ≠ `SetPosition`. |
| The Reading | [note](https://github.com/annenpolka/lattice/blob/70b61f25eb42fb32535389ccb7a51b050deb4f20/docs/notes/2026-08-22-studio-gen2-reading-model.md) | [sketch](https://github.com/annenpolka/lattice/blob/4de0f5801adbe629722d188775630f7eddab5031/docs/sketches/reading/index.html) | The UI speaks one answer — what / when / scope / target / absence — then verbs. |

The sibling model in the second note, Weighted Subject, is out of scope and is not evaluated here.
Where it is mentioned at all, it is only as context that its own note supplies.

## The three dispute less than their names suggest

Before the disagreement, the agreement. All three already hold, without arbitration:

- One committed `LocusId` is semantic "here". Focus, hover, playhead, and in-flight gesture are not a
  second selection and never become one.
- Scrub moves the playhead only. It does not re-point, does not rewrite VEL, and produces no Undo
  entry. The Reading is the note that names the exact call site this removes; the other two state the
  same contract.
- Locus change never auto-seeks. The only coupling is an explicit, unpersisted "seek to this range".
- Fail-closed against target fallback. A verb whose exact target and scope the Engine cannot name is
  absent with a reason, and a control that rewrites a target while the displayed locus stays put is a
  bug in all three readings.
- Freeze is a rate-zero `TimeMap` explanation on an existing source. It is never a selectable object,
  never a new `LocusKind`, and never a synthetic tree identity.
- Geometry outlives pixels. When normalized placement is known, affordances may persist while frame
  pixels are unavailable — and a locus with no visual fields still gets no overlay.
- Core gaps are named, not simulated. No time-scoped audio edit is invented for an Audio rail, and no
  `TimeMap` edit variant is invented for freeze.
- Review is ungated and revision-bound. Direct manipulation commits without it.
- One pointer-up commits at most one `SemanticEdit` → one rewrite → one compile → one Undo. Escape
  cancels with source unchanged.
- Navigate is optional and never a gate.
- Chrome weight is not a verb license. Compass rejects the weight/width probe from the interaction
  model, Projection-Local rejects it as constitutive, and the Reading keeps it as a harness. None of
  the three lets weight decide legality.
- No kind-driven property form survives. Compass refuses to populate a form by `LocusKind`,
  Projection-Local deletes the Inspector as a verb home, and the Reading deletes the pane outright.

What was in dispute is one question — what licenses a verb — plus one parameter, overlap, which the
locked constraint has closed.

| | What licenses a verb | Where verbs appear | Overlap UI, as its own note proposed it |
|---|---|---|---|
| Semantic Compass | Engine legality for the committed locus. Surfaces do not license. | one Engine-derived list of transitions from here | identity-first, then an ownerless coordinate probe holding pointing unresolved until a candidate is chosen or cancelled — **out under the lock** |
| Projection-Local Verbs | touched projection ∩ Engine legality | on the instrument you touched; no global verb home | candidate list on the touched projection, failed point stays visible, one shared `LocusId` after the pick — **this is the locked UI** |
| The Reading | Engine legality, spoken before it is offered, with ranking allowed only where a wrong guess costs a pointing | in the utterance, anchored at the witness that produced the pointing | rank, commit immediately, state the collapse, step with one key — **out under the lock** |

Note the shape of that table. Compass and the Reading are near-siblings on the license question and
were opponents on overlap. Projection-Local Verbs is the only genuine dissent on license and was the
closer of the two to Compass on overlap. No two of the three agreed on both, which is why this reads
as three models rather than two — and why closing the second axis does not close the first. The
license question stays open below; only the overlap UI is settled.

One consequence of the lock is worth pulling out before it recurs: the locked UI holds a point open
until a card is chosen, so **"here" may be temporarily unset after a pointer-up.** That is a
downstream fact the merged model has to carry, not a detail of one sketch.

## Question 1 — intuitive for whom, misread as what

This is not a ranking. "Intuitive" is a relation between a model and a prior, so the only honest form
of the answer is one row per prior.

The priors, named:

- **First-time editor.** Arrives with NLE habits: a clip is an object, the object has a property
  sheet, the timeline is the document. Does not know what a locus is and should not have to.
- **Lattice-native.** Learned the product from VEL, `lattice explain`, and the CLI. Thinks in
  `LocusId`, `SemanticEdit`, scope, provenance, and typed absence.
- **Power user.** High volume, keyboard-first, forms habits in the first hour and expects them to
  hold. Optimizes gestures per commit and will not tolerate a stateful decision in the fast path.
- **External coding agent.** Listed because `AGENTS.md` makes the CLI a first-class client: it
  receives `locus + instruction`, consumes `--json`, and has no pointer at all.

### Who each model is intuitive for

**Projection-Local Verbs is the most intuitive for the first-time editor.** It is the only one of the
three whose license is discoverable by touching rather than by reading. "Do the thing where the thing
is" is the single habit that transfers from every timeline editor, and it transfers without
vocabulary: you move a title by dragging its rectangle on the picture and lengthen it by dragging its
edge on the timeline, and you never learn the word *locus* to do either. Its sketch states the license
directly on the instrument being touched — "verbs licensed only here ∩ Engine" — so the rule is
legible from the first gesture. It is also the most forgiving of a wrong touch: touching the wrong
instrument yields a wrong-but-harmless verb set rather than a rewrite of a target the user was not
looking at. Worst fit for this reader: the Reading, which answers with a sentence where an NLE prior
expects a control, and supplies no prior that says the sentence is the product.

**Semantic Compass is the most intuitive for the Lattice-native.** It is the interaction transcription
of invariants this reader already holds. One center, a projection ledger, `(verb, target, scope,
effect)` disclosed before invocation, and a typed reason for every absence is `lattice explain` with a
pointer attached; nothing has to be relearned, and the ledger's three outcomes — present,
structurally absent, currently unavailable — are already the vocabulary this reader uses for partial
projection. Worst fit for this reader: Projection-Local Verbs, because they know `SetPosition` is
legal on that title, so a Timeline that does not offer it reads as a surface contradicting the
domain — precisely the class of bug they came to remove.

**The Reading is the most intuitive for the power user, with Compass a close second.** Two reasons,
both about the loop rather than the vocabulary. Ranked-then-stated pointing is the only one of the
three that never places a decision between pointer-up and the next gesture, so the fast path stays
one gesture deep. And target and scope are stated before commit, so a habitual gesture can be
verified without a round trip through a panel. The lock removes the first reason — under
Projection-Local's overlap UI a point into overlap does place a decision in the path — so the
power-user case for the Reading now rests entirely on the second, which is the stronger half anyway:
what a fast user needs most is to confirm the target of a habitual gesture without stopping, and that
survives the lock intact. Compass is a close second because one verb house means
"can I do X to this?" has an answer that does not depend on where the pointer currently is, which is
the property keyboard-first invocation actually needs. Worst fit for this reader: Projection-Local
Verbs — the same capability with different availability depending on the surface is the one thing
muscle memory cannot cache.

**The Reading is the most intuitive for the external agent, by construction.** It is the only one of
the three that names the payload as an Engine derivation consumed identically by Studio, the CLI under
`--json`, and an agent, so edit legality is not guessed twice in two clients. Compass supplies the
same facts but frames them as Studio's reading of the center. Projection-Local Verbs is weakest here:
"the touched projection" has no referent for a client with no pointer, so an agent would have to
invent a fictional witness in order to know which verbs it may propose.

That last row deserves stating plainly rather than leaving in a list. **The license that is most
intuitive for a first-time human is the least expressible for the repo's other first-class client.**
Any merge has to carry that cost explicitly instead of discovering it later in the CLI surface.

### What each will be misread as

These are misreads of the three notes as written, which is why two of them are about overlap
mechanics the lock has since removed. A misread of a retired mechanism still matters: it is part of
why the mechanism was worth retiring, and the same reflex will be aimed at whatever replaces it.

**Semantic Compass**

- *"A better Inspector."* "Verbs live in ONE Engine list" is a claim about authority, and it will be
  heard as a claim about furniture: one list, therefore one always-on rail, therefore the property
  grid returns with nicer labels. That rebuild restores exactly the permanence the other two notes
  remove.
- *"A compass widget."* The name invites a chrome object — a navigator, a minimap, an orientation
  gizmo in a corner. There is no compass in the model. The compass is the state.
- *"The app is refusing to work."* Fail-closed absence reads as a missing feature to anyone who does
  not read the reason, and a power user who knows the edit exists is the most likely to skip it.
- *"A modal picker."* The coordinate probe reads as a dialog to dismiss rather than as the disclosure
  that pointing did not name one thing.

**Projection-Local Verbs**

- *"Titles have no position from the timeline."* This is the costly one, because it converts a routing
  fact into a legality claim. `Timeline ∩ Title ≠ SetPosition` means *this gesture does not commit
  that edit*. It will be read as *that edit does not exist for this thing*, and the user's next move
  is to conclude the feature is missing rather than to go where it commits.
- *"Hunt for the right pane."* The first-hour experience of anyone who touches the wrong instrument
  twice. Capability that is routed but unstated is indistinguishable from capability that is absent.
- *"Each projection has its own selection."* Per-projection verb sets look exactly like per-view
  selection, which the shared-locus invariant forbids. The note is explicit that they are not; the
  surface will keep suggesting it.
- *"A picker pane."* The overlap candidate list reads as persistent chrome and therefore as a second
  subject — the failure its own sibling model names. Because the lock makes this list *the* overlap
  UI, this is the one misread on the page that has to be designed against rather than merely noted:
  the list has to read as a point that has not landed yet, not as a place where things live.

**The Reading**

- *"A status bar."* Text where a control was expected. This reader waits for the sentence to become
  actionable and then reports the verbs as missing.
- *"The panes are going away."* "Five panes are not the product" is a claim about which property is
  invariant, and it will be heard as a plan to remove the UI — hide-VEL inverted into hide-everything.
  The note protects VEL explicitly. The slogan does not.
- *"A context menu that follows the mouse."* Anchoring the utterance at the witness reads as a
  floating popup, which is also where the note's own unsolved occlusion problem lives.
- *"It moved my selection."* Rank-then-state means the sentence explaining a collapse arrives after
  the pointing it explains. The model most committed to not guessing behind the user's back is
  therefore the one most exposed to that accusation.

## Question 2 — yes, there is room, on one condition

The merge is smaller than the three names suggest, because the shared base above is already large.
What has to happen first is not a compromise between three licenses. It is a distinction none of the
three notes draws. All three use "license" for two different facts:

- **Legality** — whether the Engine can name a `SemanticEdit`, its exact target `LocusId`, and its
  scope for the committed locus.
- **Routing** — which gesture on which projection commits that edit directly.

Compass answers legality and treats routing as presentation. Projection-Local Verbs answers routing
and lets it decide legality. The Reading answers legality, adds the utterance, and leaves routing
implicit in the hit-region contracts it inherits. Once the two are separated, two of the three notes
turn out to describe different layers of one model, and the third contributes the sentence that makes
the difference between the layers observable.

### The spine

> **One locus, one legal set, one utterance. The Engine says what is legal for here. The gesture you
> made says what it commits. When those two sets differ, the difference is spoken — never implied.**

That is one rule, and the shared base is already its entailment rather than an addition to it:

- Nothing outside the Engine set is ever offered → Compass's fail-closed verbs, and the deletion of
  project-wide first-match target fallthrough.
- The gesture set is a subset, and the gap carries its route → Projection-Local's grammar without its
  worst misread. "Legal for this locus, committed on the Canvas" is a sentence. "Absent" was not.
- Stating the gap *is* an absence clause → the Reading's vocabulary, widened from missing pictures to
  missing routes.
- Nothing may hold a verb whose target the Engine cannot name → the always-on property grid has no
  subject to invent, in one line, for all three.

### KEEP / STEAL / DELETE

**Semantic Compass**

- **KEEP.** The center, and `(verb, target, scope, effect)` disclosed before invocation. The ledger's
  three outcomes as *the* absence vocabulary for the merged model, because they are typed reasons
  rather than prose. Identity versus relation: displaying a relation never silently moves the center.
  Freeze as a temporal reading rather than a target.
- **STEAL.** Projection-Local's routing table, so "one Engine list" can no longer be read as one
  Engine panel. And Projection-Local's overlap list as the surface Compass left unowned: Compass
  already has the right typed reason for an unresolved point and no place to put it.
- **DELETE.** The implication that a single legal set requires a single surface — it is the source of
  the better-Inspector misread and it is not load-bearing. And the coordinate probe as an ownerless,
  cross-surface candidate set. "No view owns the candidates" is the one Compass sentence the lock
  overrules, and it is worth being exact about how little else goes with it: the probe's *content* —
  each candidate's identity, relation, span, provenance, and legal edit scopes — is what the locked
  list shows. Only the ownership changes, from nobody to the projection that was touched. Compass's
  `unresolved-pointing` reason is untouched and is kept, because the locked UI is the thing that needs
  it.

**Projection-Local Verbs**

- **KEEP.** The hit grammar as the routing table: scrub / point / mutate, no region doing two jobs,
  rest state predicts the role, Canvas commits geometry, Timeline commits time, VEL commits definition
  text, overlays bound to `TimelineClip.id` / `LocusId` and never reverse-matched by visible text.
  "The missing instrument speaks." No toolbar second target. And — by the locked constraint — the
  overlap UI entire: the failed point stays visible on the projection that was touched, each candidate
  carries its identity, its scope, and the verbs that projection would license for it, and the pick
  commits one shared `LocusId` to Canvas, Timeline, Source, Review, and agent context. The list is not
  project state, not per-view selection, and not in Undo.
- **STEAL.** The global legal set. This is the fix for its own worst misread and it costs the model
  nothing it wanted: a locally uncommittable verb becomes "legal for this locus, committed on that
  projection" with a Navigate route, instead of a silence that reads as impossibility.
- **DELETE.** The claim that the touched projection *licenses*; demote it to *commits*. And
  "no global verb home" as an absolute. The defensible rule is narrower and stronger: **no verb home
  that can invent a target.** That version still kills the Inspector and the gain fallback without
  also forbidding a locus-complete statement of what is legal.

**The Reading**

- **KEEP.** The fail-closed half of the spine asymmetry: where the consequence is a source rewrite,
  derive the target from the locus's own fields or fail closed and say what is needed. That is the
  half that deletes the project-wide first-match target fallthrough, and the lock does not touch it.
  Also: verbs create their own inputs, so no draft outlives the locus it was copied from; selectable implies
  resolvable; geometry outlives pixels; scrub stops pointing; and the derivation living in Engine so
  Studio, `--json`, and an agent consume one answer. That last item is what keeps the merged model
  from being Studio-only.
- **STEAL.** Compass's typed reason names, so the clauses have one stable set behind them in `--json`
  instead of per-clause prose. And Projection-Local's routing table, so the utterance can say *where*
  a verb commits and not merely that it is unavailable here.
- **DELETE.** The permissive half of the asymmetry — "ranking is allowed where the consequence is a
  reversible pointing" — and with it rank-then-state-then-step as the overlap UI. The lock forecloses
  it. What is left is simpler than the rule it came from, and that is a gain rather than a loss: the
  merged model never ranks, on either side of the pointing/rewrite line. Also "Five panes are not the
  product" and "Space is not the editor's identity" as stated deletions — both are consequences the
  model does not need to claim, and both generate the hide-the-UI misread. The testable claim survives
  without either: one utterance stays complete when only one projection is visible, which is exactly
  what the sketch's
  [~800px screen](https://github.com/annenpolka/lattice/blob/4de0f5801adbe629722d188775630f7eddab5031/docs/sketches/reading/h.html)
  already exercises. And anchor-at-the-witness as a model-level commitment: the note concedes it does
  not solve occlusion, and a merged model should not inherit an unsolved layout problem as an
  invariant.

### Overlap is not a merge question: it is locked

Everything above is a union with demotions. Overlap is not, because it is not open. The overlap UI is
Projection-Local's, as drawn in scene D of its sketch: a point at 2.40s that names `title Hello`,
`scene demo`, and `source fight` does not collapse. The Timeline — the projection that was touched —
says the point named several loci and lists them, each card carrying its `LocusId`, its scope, and the
verbs *that* projection would license for it. Pointing stays unresolved until a card is chosen, and
the choice commits one `LocusId` to Canvas, Timeline, Source, Review, and agent context. No
cross-surface modal, no ownerless candidate set, no ranking, no click-to-commit.

The lock is an input to the integration rather than a constraint bolted on top of it, because three
things follow for the merged model:

1. **An unresolved "here" is a reading, not an error.** The locked UI holds pointing open, so the
   merged model must carry that state as a first-class disclosure. Compass already has the typed reason
   for it and no surface to show it on; Projection-Local has the surface. That pairing is the whole
   fix, and it retires the Reading's requirement that pointing never obstruct.
2. **The resolution policy becomes uniform.** The Reading's asymmetry permitted ranking on the
   pointing side because a wrong rank cost one keystroke. The lock withdraws that permission, so both
   sides of the pointing/rewrite line now derive or disclose and neither ranks. The merged model comes
   out simpler than the rule that motivated it: it does not guess, and the asymmetry survives only as
   the argument for why rewrites in particular must not be guessed.
3. **The candidate list is routing, not legality.** This is the spine doing its work on the locked UI.
   Each card advertises what *this* projection would license for that candidate, which is a set of
   routes — not a claim that the candidate's other legal verbs are absent. Without that reading the
   list would reproduce Projection-Local's worst misread three cards at a time, which is exactly the
   failure mode the spine exists to prevent.

What the lock costs, stated rather than hidden: Projection-Local's own falsifier asked whether
candidates on the touched projection add scope predictability *without* materially obstructing
ordinary pointing. A lock forecloses the comparison that would have answered that, so the risk is now
carried rather than tested away. It will show up first on dense ties — two invocations with identical
spans and distinct `LocusId`s, which `crates/lattice-studio/tests/layout.rs` already builds as
duplicate overlays. Under the locked UI that fixture stops being an arbiter between policies and
becomes the stress test of the chosen one: whether a list whose cards differ only by `LocusId` still
says anything a user can act on. That is a question about how the locked UI presents identity, and it
is not an invitation to reopen the choice.

Two smaller tensions do merge. They are recorded here so they are not rediscovered as blockers:

- **Definition versus instance emphasis.** Compass treats one-to-many as relation disclosure,
  Projection-Local makes the touched instance the subject, and the Reading absorbs multiplicity as a
  clause. Merged rule: pointing names the identity that was touched; the definition relation is
  disclosed with its count and a Navigate route; every verb carries its real scope, so
  instance-versus-definition is never something the user has to guess. Simultaneous highlighting is
  neither required nor forbidden — it stops being a semantic question at all.
- **Where the seek verb lives.** Compass puts explicit seek among the transitions from here,
  Projection-Local puts it on the projection that currently lacks a picture, and the Reading names it
  as one of exactly two couplings. All three agree it is explicit, temporary, unpersisted, and absent
  from Undo, so placement is routing under the spine and not a model disagreement.

## What this note does not do

- It does not select a shipping model. The spine above is a proposal for how three existing notes
  could be read as one, offered as discussion input.
- It does not implement anything, and nothing here should be read as a plan of record.
- It does not reopen the overlap UI. That is locked to Projection-Local's candidate list on the
  touched projection, and the note reads the lock as an integration input rather than as an option.
- It does not fill the Core gaps the three notes name — no time-scoped audio edit, no `TimeMap` edit
  variant, and the bundling of definition and placement fields inside one `SemanticEdit::Title`
  variant. Naming a gap is not license to invent a filler.
- It does not touch the invariants in `AGENTS.md`, `docs/principles.md`, or `docs/interaction.md`.
  Where this note and the spec disagree, the spec wins.
