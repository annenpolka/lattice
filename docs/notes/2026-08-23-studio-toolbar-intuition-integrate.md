# Toolbar chrome: what a standing control may promise, and to whom

Date: 2026-08-23

Status: a reading of four sketch objects and the six gen2 notes behind them. Not a selected design, not
a shipping decision, not an implementation plan, and not a merge of any branch. Nothing here was
implemented and nothing here should be read as a plan of record.

Scope: Studio's top chrome after the verb bank, and nothing else. No Core type, VEL word, Engine
signature, GPUI view, or `AGENTS.md` invariant is changed or proposed for change. Where this note and
`docs/principles.md`, `docs/architecture.md`, `docs/interaction.md`, or `AGENTS.md` disagree, the spec
wins.

This is the same job the
[verb-license integrate note](https://github.com/annenpolka/lattice/blob/85b589ec260554f851c214731e607c7727c7cae8/docs/notes/2026-08-22-studio-verb-license-intuition-integrate.md)
did for verb license: read a field of finished proposals, say who each is intuitive for and what each
will be misread as, and propose how they read as one. It is not that note's spine restated at a
different altitude. That note settled who may *name* a verb. This one is about what a control on
screen is allowed to *promise* — a different question with a different answer.

## The one 座組 lock, restated as an input

**Deleted, unanimously across the six observer votes and absorbed by the
[chair](https://github.com/annenpolka/lattice/blob/1bbb48d742c8e9cf1ae674c59f7cec7d71292ca6/docs/notes/2026-08-22-studio-toolbar-chair.md):**
the fixed, always-visible bank of locus-taking `SemanticEdit` buttons — Set In, Set Out, Split at
Playhead, Delete Selected Clip, Gain −3 dB, Fade. A non-verb session/application strip **may** remain
global; its membership was not settled by the vote.

The chair is precise about how narrow that is, and the precision is load-bearing below. The vote
rejected one object. It did not decide whether a control may stand in a fixed window position, whether
its contents may be conditioned on here, or where the four verbs the bank alone routed should land. It
also recorded that DELETE is an object judgment rather than a removal sequence: on `main`,
`routed_verbs(Projection::Toolbar, …)` is the only commit projection for `set-gain`, `set-fade`,
`split`, and `delete`, so the deletion has a bill attached.

The pointing locks stay closed and are treated as invariants, not options: overlap resolves through
the candidate list on the projection that was touched; a video click points the source clip and keeps
its identity; one locus, one Engine legal set, one utterance, with the gap between legality and
routing spoken rather than implied; Title-shaped Inspector fields only when here is Title; scrub and
playhead do not re-point; no GPUI in Core; no per-view selection.

## What is being read

Four sketch objects, each drawn from a gen2 note, each answering *where a locus-taking verb commits
once the bank is gone* differently.

| Sketch | Note behind it | Its rule, in one line |
|---|---|---|
| [#37 Ledger](https://github.com/annenpolka/lattice/blob/8caa838643b38c46baba9fbe0b3bf6205f0f7be6/docs/sketches/toolbar-ledger/index.html) | [#33 Model 2, Command Ledger](https://github.com/annenpolka/lattice/blob/4b286a44137df6b79eae8a6c89dfe4b18945c150/docs/notes/2026-08-22-studio-toolbar-gen2-pair.md) | The four verbs are named commands against here. The utterance row is the thing you invoke. |
| [#38 Control](https://github.com/annenpolka/lattice/blob/94db45e3399d554c40747267d07fd10e93cc1633/docs/sketches/toolbar-control/index.html) | [#35 CONTROL](https://github.com/annenpolka/lattice/blob/8f2ae3af3fa5d3fc6efd84c9e5ea5a0088b7c686/docs/notes/2026-08-22-studio-toolbar-gen2-control.md) | Current chrome minus the six buttons, and nothing put back. The four verbs are legal and unrouted. |
| [#39 Flash](https://github.com/annenpolka/lattice/blob/aad0878a3b68f216331ecc4d6ec6034fa52e3dc2/docs/sketches/toolbar-flash/index.html) | [#34 Flash](https://github.com/annenpolka/lattice/blob/710172216b1c3b8c710aac04667970d27c407ce3/docs/notes/2026-08-22-studio-toolbar-gen2-flash.md) | A four-cluster session strip, and the verbs land in Inspector property fields plus keys and context menus. |
| [#40 On-target](https://github.com/annenpolka/lattice/blob/2ae70447a288364d92af9f60f670b0a2e7d56f11/docs/sketches/toolbar-ontarget/index.html) | [#36 Opus](https://github.com/annenpolka/lattice/blob/337b2322d48954ec7e9146deaa4cd9f3638d848c/docs/notes/2026-08-22-studio-toolbar-gen2-opus.md) | A verb's control is drawn on its target, or it is not drawn. Undrawn is legal-and-unrouted, spoken with a reason. |

Two gen2 notes have no sketch of their own and are read as prose:
[#32 Sol, route-bearing instruments](https://github.com/annenpolka/lattice/blob/dcf3833e5645ec61d347034f8e56d206f238eb47/docs/notes/2026-08-22-studio-toolbar-gen2-sol.md),
and Model 1 of the
[#33 pair](https://github.com/annenpolka/lattice/blob/4b286a44137df6b79eae8a6c89dfe4b18945c150/docs/notes/2026-08-22-studio-toolbar-gen2-pair.md),
Local-Projection Commit Homes. Both belong to On-target's family: the verb commits on the instrument
that already owns the subject. They differ from #36 in strictness — Sol permits a Delete key route and
a transient scene overflow, Model 1 fixes the commit home without claiming a shape — and neither makes
drawing constitutive. Reading them alongside #40 is what shows the field is smaller than four.

## The four dispute less than the sketches suggest

Before the disagreement, the agreement — which is nearly everything.

All four hold, without arbitration: the Engine is the only legality authority and `legal_edits_for` is
unchanged; the bank does not come back under a new label, in a new dock, or as an "Actions" menu; a
non-verb session strip may stand; there is no silent retarget and no `target_source_locus` fallthrough;
a verb the surface cannot commit is spoken with a reason rather than dropped; overlap and video click
are untouched; Title fields stay Title-only; the playhead is a clock and not a locus; and one
pointer-up still commits at most one `SemanticEdit` → one rewrite → one compile → one Undo.

They also agree on something the chair had left open, which is worth extracting because it makes the
session strip a solved problem rather than a fifth argument. Every candidate admits the strip by a
predicate rather than by a list. #30's vote states it directly — membership is
`commits_semantic_edit == false`, grouped by authority. #36 §5 restates it as a two-clause test and
then sharpens it in the paragraph that argues with itself about Undo/Redo: the honest form is about
targets rather than mutation, so **no standing control may name a locus it did not draw**, and
Undo/Redo pass because they name none. #34 supplies the only actual grouping anyone drew — identity
and file lifecycle, transport clock, engine phase and history, runtime telemetry. #35 supplies the
inventory those groups are applied to. Nothing in the four contradicts anything else here. The strip
is not in dispute; it is in four pieces that fit.

What is in dispute is one axis, plus one question that looks settled by counting:

| | Where a locus-taking verb commits | May the utterance commit? |
|---|---|---|
| Ledger (#37) | in the utterance, as a named command against here | **yes** — this is the only yes in the field |
| Control (#38) | nowhere, for four of them | no |
| Flash (#39) | Inspector property fields, plus keys and context menus, with Timeline handles as a second route | no |
| On-target (#40) | on the drawn subject: gain line, fade wedge, cut lane, delete handle | no |

The second column is three-to-one, which makes it look settled. It is not settled by counting, and
Question 3 answers it on grounds that survive the count being different.

## Question 1 — intuitive for whom, misread as what

Not a ranking. "Intuitive" is a relation between an object and a prior, so the only honest form is one
row per prior. The priors are the same four the verb-license note used, for the same reason: they are
the readers this repo actually has.

- **First-time editor.** NLE habits. A clip is an object, the object has a property sheet, verbs live
  on a toolbar. Does not know what a locus is and should not have to.
- **Lattice-native.** Learned the product from VEL, `lattice explain`, and the CLI. Thinks in
  `LocusId`, `SemanticEdit`, scope, provenance, and typed absence.
- **Power user.** Keyboard-first, high volume, forms habits in the first hour. Optimizes gestures per
  commit and cares about exact values.
- **External coding agent.** First-class per `AGENTS.md`: receives `locus + instruction`, consumes
  `--json`, has no pointer at all.

### Who each is intuitive for

**Flash is the most intuitive for the first-time editor, and it is the only one that is close.** It is
the sole candidate whose first hour looks like an editor this reader has used: a top strip with
transport and Save, an Inspector with a gain slider reading `-3.0 dB` and fade duration fields,
`Backspace` to delete, `S` to split. Nothing has to be discovered by hovering, and every verb has a
name printed somewhere. On-target is the worst fit and says so itself — #36 §8 lists first-hour
discoverability as a killer, because a handle is invisible until here is already the thing it belongs
to, which is a chicken-and-egg problem for a reader who does not yet know that pointing precedes
verbs. Control is worse than either: the verb is simply absent and a sentence explains why, which this
reader parses as a broken build rather than as a design. Ledger sits in between and is stranger than
its reputation — a sentence you click is unfamiliar, but it is at least *visible*, which is the
property the first hour actually runs on.

The cost has to be stated in the same breath, because it is the mirror of the verb-license note's
finding that the most human-intuitive license was the least expressible for the agent. Here the
asymmetry runs inside one candidate: **the object that feels most like an editor is the one that most
often draws a control the Engine cannot back.** Flash's Inspector offers a `Fade Out (s)` field, but
`SemanticEdit::SetFade` carries `fade_in` only — `TimelineClip.fade_out` is read by `evaluate` and set
by nothing. Flash's summary matrix routes `delete` to `LocusKind::Title`, and `legal_edits_for` for a
Title names `title`, `set-position`, and `resize-overlay` and no `delete`. Both are the familiar shape
of an NLE filling in what an NLE would have, which is exactly the reflex the first-time reader brings
and exactly the reflex the product cannot afford. Familiarity is not free here; it is purchased with
invented legality, and the purchase is visible in the sketch.

**On-target is the most intuitive for the Lattice-native.** It is `legal_edits_for` rendered as
geometry. Every drawn thing corresponds to a verb the Engine named for that exact `LocusId`; the gain
line spans the whole audio block because the scope is `source-binding` and Core has no time-scoped
audio edit; there is no tail wedge because `SetFade` has no `fade_out`; the cut lane calls the same
`map_timeline_to_source` that `split_at_playhead` already calls, so the argument becomes visible
without becoming new. This reader already believes a verb whose target cannot be named should not be
clickable, and On-target is that belief drawn rather than argued. Worst fit for this reader: Flash,
for the two inventions above — a form field claiming an edit Core does not have is the precise class of
thing this reader came to remove.

**The power user's row splits, and the split is the useful part.** Ledger is the most intuitive for
*invocation*: a named command against here, keyboard-reachable, no pointer travel, and a typed
argument instead of a drag. On-target is weakest exactly there, and #36 §8 names it — dragging a line
to exactly −3 dB is slower and less exact than typing −3, and `set-gain` is a scalar rather than a
coordinate, so the drag is a coordinate simulation of something that was never a coordinate. But
Ledger is the *least* intuitive for verification, because a clause built from
`LegalEdit { verb, target, scope, effect }` has no field that holds the current value: the row can say
`set-gain → source:fight (source-binding: set gain on this source)` and cannot say `−3.0 dB now`.
Flash is the only candidate that wins the verification loop outright, because a field shows what it
holds. So the honest reading is that this reader wants Ledger's reach, Flash's field, and On-target's
refusal to lie. Flash is the only candidate that gets two, and it gets them at the price Question 1
already named. Control is unusable for this reader by construction.

**The agent is nearly indifferent, and that is the finding.** Studio routing is not something an agent
consumes; it commits against a `LocusId` through the CLI, and #36 §3 states plainly that unrouted is a
Studio fact rather than an Engine one. So this is the first of these questions where Question 2 barely
moves the agent at all. Two places where it does move, both of which matter more than the indifference:

1. **Question 3 is the agent's question.** If the utterance commits, Studio's utterance and the
   utterance under `--json` stop being the same kind of object even when their content matches — one is
   a reading, one is an API. That is the condition `AGENTS.md` avoids by making the CLI first-class
   rather than secondary.
2. **Control inverts the client relationship.** It leaves four Engine-legal verbs permanently unrouted
   in Studio while an agent can still commit all four, which makes the CLI strictly more capable than
   the GUI. The CLI is a first-class client, not a superset, so a permanent gap in that direction is a
   product statement rather than a neutral baseline — and it is the one thing about Control that its
   own note, which is careful to call itself a measurement, does not have to defend.

### What each will be misread as

**Ledger (#37)**

- *"The bank, in a sentence."* The most likely and the most damaging, because it is half true and the
  true half is not the interesting half. What makes the bank the bank is not that it was clickable; it
  is that it named a target it did not draw and a value it did not show. A ledger row draws its target
  in words. It still shows no value.
- *"Every row is a button."* Present rows invoke and absent rows refuse, and both are rows. A reader
  who learns "rows are pressable" will press `needs-source-binding` and read the refusal as a bug.
- *"Ledger" is one object.* It is two, and the collision will manufacture a false consensus in any
  later reading. #25's vote names **Route Ledger + Projection Commit Affordance**, where the ledger is
  the utterance and *the projections commit*. #33 Model 2 and #37 name **Command Ledger**, where the
  ledger *is* the commit. Those are opposite answers to Question 3 sharing a word.

**Control (#38)**

- *"The feature was removed."* Its own routing table says `set-gain`, `set-fade`, `split`, and `delete`
  are unrouted. To a user that is a regression with a paragraph attached.
- *"The conservative option."* It is the most invasive of the four for the person using the product,
  because it removes reachable capability and supplies nothing. Conservative about the code is not
  conservative about the user.
- *"A proposal."* Control's job is to be the null hypothesis — the arm that reveals whether the four
  verbs were used at all. It will be scored as a design against designs, which is the one reading that
  destroys its value.

**Flash (#39)**

- *"We kept the toolbar."* It has the most standing chrome and the most keyboard verbs of the four, so
  a reviewer skimming one still sees a normal NLE and concludes nothing changed. The deletion did
  happen; the visual evidence for it is the weakest in the field.
- *"An Inspector button is not a bank button."* The sketch's Scene Operations block contains
  `✂ Split Scene at Playhead (S)`. Moving a control from the top strip into a pane changes its
  neighbours and not its promise: the scene is here, but the *time* argument still arrives from the
  playhead with nothing on screen tying the two together. This is the deleted object with a new
  address, and it will be cited as proof that the address was the problem.
- *"Property fields are the general answer."* They are the general answer for property-shaped edits,
  which is #30's word for verbs whose parameter is not a coordinate. Generalized past that, they
  rebuild the kind-driven property grid the whole verb-license reading already deleted.

**On-target (#40)**

- *"You deleted gain, fade, split, and delete."* #36 §7 names this as the most expensive misread and
  it is right. Four verbs gained routes; none lost one. If users say this after shipping, the drawing
  was bad rather than the rule.
- *"Handles are a hover toolbar."* They accrete unless something forbids it. The governor is narrow —
  one region, one verb, arguments from that region — and a region that hosts a second verb is a bank
  with a smaller footprint.
- *"Drawn means it will work."* #36 §7 names this and §9 supplies the case, and it is worth pulling
  forward because it is the hazard specific to making drawing constitutive.
  `legal_edits_for(Scene)` names `split` for any scene, while `apply_split` additionally requires the
  scene's binding to be shaped `media[start..end]` and errors otherwise. A cut lane drawn from the
  band's own range makes an over-broad legal set look narrow. The rule exists to stop routing from
  being read as legality; this is the same confusion running the other way, and only On-target is
  exposed to it, because only On-target promises that drawing is meaningful.

**The 座組 lock itself**

- *"Nothing may stand."* The vote deleted locus-taking buttons. Every candidate keeps a strip, and
  under the predicate above so does the synthesis. "No global verb home" and "no global chrome" are
  different sentences and the second one was never carried.

## Question 2 — the synthesis, and the distinction it needs first

The merge is smaller than four names suggest, because the shared base above is already large and the
session strip is already assembled. What is missing is a distinction none of the four draws. All four
argue about **where** a control sits — top strip, pane, rail, sentence — and treat the bank's failure
as a failure of location. The receipts say otherwise.

Run the deleted buttons against the flagship fixture, as #36 did through `Engine::propose` /
`Engine::apply_proposal`. `Gain −3 dB` on `gain fight by -3` spliced `gain fight by --3`, which parses,
compiles with zero diagnostics, and evaluates six decibels the wrong way. `Fade` at the fixture's own
`0.5s` produced a byte-identical source, an empty diff, and one Undo entry. Neither is a legality
failure — the commit gate did its job, the Engine authored nothing it could not name, and no target was
searched for. Both are **display** failures. Nothing on screen held the value the button was about to
overwrite, so nothing on screen disagreed with it.

That splits the bank's single defect into two:

- **Identity.** The control named a locus it did not draw. Six buttons, six implicit arguments.
- **Value.** The control changed a number it never showed. A wrong result looked exactly like a right
  one.

The field has been solving the first and assuming the second follows. It does not follow, and every
disagreement in the table above is really an argument about which half a given candidate solved.
On-target solves identity by drawing and solves value where the drawing happens to carry it — the gain
line displays its dB, which is why that one control is the strongest object anyone drew. Flash solves
value everywhere and states its identity condition at `LocusKind` granularity, one level coarser than
the target the edit names. Ledger solves identity in words and value nowhere. Control solves both by
having nothing to solve.

### The proposal

> **Only the session stands. A verb's control appears on the subject it changes and shows the value it
> will overwrite. Whatever the Engine allows and no control currently offers is spoken — by an
> utterance that says it and never does it.**

Three tiers, one admission test each. The names are already in the field, so this is not a new chrome
ontology: **session strip** is #25/#26/#27/#30/#35's word, **instrument** is #32's and
Projection-Local's, and **utterance** is locked.

| Tier | May stand? | Admission test | Assembled from |
|---|---|---|---|
| Session strip | permanently | takes no locus, and names no locus it did not draw | #34's four clusters over #35's inventory, admitted by #30's predicate and #36 §5 |
| Instrument | only while here is its subject | draws the subject it will change **and** shows the value it will overwrite | #40's handles, #32's route table, #39's fields for property-shaped verbs |
| Utterance | permanently | commits nothing; carries every legal verb no instrument currently offers, with a reason | locked, plus #37's record half |

The middle row's two clauses are the only thing this note adds, and they are the two halves of the
bank's failure turned into an admission test rather than a warning. Call it the **instrument test**,
because the synthesis needs a handle for it and the alternative is repeating the clause every time. It
is a reading, not a type: it should not become a struct, a `debug_selector`, or an `AbsenceReason` on
the authority of this note.

The test is what makes the field readable as layers instead of rivals, because it is indifferent to
furniture and decides every disputed control in the same voice:

- On-target's gain line draws the audio block and prints its dB. **Passes.** It is the reference
  instrument, and it is worth noticing that it passes on the strength of a property the note that drew
  it treated as a detail.
- Flash's gain slider shows `-3.0 dB` and appears only when a source is committed. **Passes**, if its
  condition is the exact `LocusId` rather than `LocusKind`. Both clauses matter and the second one is
  where Flash is loose.
- Flash's `✂ Split Scene at Playhead` button in the Inspector shows no time and draws no lane.
  **Fails.** It is the bank in a pane, and no amount of adjacency to here fixes it.
- Flash's `Fade Out (s)` field shows a value for an edit that does not exist. **Fails before the test
  applies** — there is no `SemanticEdit` for it to commit, so whether it draws and shows never comes
  up. That is the more serious way to fail, because the test would have passed it.
- A Ledger clause draws its target in words and shows no value. **Fails the value clause**, which is
  the substantive argument against invocable clauses and does not depend on how the row looks.
- The Title text field in the Inspector today draws the title's own text and holds the value it will
  overwrite. **Passes** — and this is the point of the test rather than a coincidence. The locked
  Title-fields rule is usually read as a restriction. It is also a precedent, and the test generalizes
  the reason that field is not the bank instead of generalizing the furniture it sits in.

The three tiers are also a routing order rather than a taste: a verb goes to an instrument if a
projection can draw its subject and show its value; a property-shaped verb goes to a field on the
locus that owns it, under the same two clauses; and anything left is spoken. Nothing is offered that
cannot be shown, and nothing legal is silent.

### What the synthesis costs, stated rather than buried

Two bills, and neither is paid here.

**The gestures-per-commit falsifier survives the merge.** #36 §8 asks whether trim, split, gain, and
fade cost more gestures than the bank did *for the verbs the bank committed correctly*, and the answer
for the synthesis is not automatically better than for On-target alone. Adding a typed field for
property-shaped verbs helps exactness and adds a focus step. This is testable on
`examples/gameplay-commentary/main.vel` and is not tested by anyone yet.

**A value display is an obligation with a refresh problem.** #32 is the only note in the field that
names it: a locus change must invalidate an in-flight parameter draft rather than apply it to the new
identity. The instrument test makes that failure mode reachable everywhere the test is passed, so the
rule stops being one model's detail and becomes a condition of the tier.

### KEEP / STEAL / DELETE

**#40 On-target, and #36 / #32 / #33 Model 1 behind it**

- **KEEP.** The identity clause: a verb's control appears on the subject it changes. C2 as #36 states
  it — identity is never implicit, other arguments must merely be drawn — which is what makes a
  keyboard accelerator admissible and a "current scene" button not. C3: an accelerator is live exactly
  while its handle is drawn, so deleting the bank cannot regrow it invisibly in the keymap. The gain
  line spanning the whole block because the scope is `source-binding`. No tail wedge, because
  `SetFade` has no `fade_out` and drawing one would invent a Core gap filler. The cut lane reusing
  `map_timeline_to_source`. The audio block and the video block resolving to one `LocusId`, since two
  loci for one binding is per-view selection wearing a rail. The §6 code consequences, all of them
  mechanical: `routed_verbs(Projection::Toolbar, _)` empty rather than smaller, no spoken clause
  containing `committed on Toolbar`, `commit_projection`'s `None` honoured instead of
  `.unwrap_or(Projection::Timeline)`, and an empty legal set spoken rather than rendered as a blank
  line. From #32: the required-routes table with its "if the requirement is not met" column, and the
  in-flight-draft invalidation rule. From #33 Model 1: the refusal list, which is the cleanest
  statement of what abandoning the model would look like.
- **STEAL.** Flash's typed entry for property-shaped verbs, because a drag is a coordinate simulation
  of a scalar and #36's own exactness falsifier is answered by a field rather than by a better line.
  Ledger's keyboard reach for the power path — admitted under C3 as an accelerator bound to a drawn
  handle, not as an independent route. And #30's vocabulary, coordinate-shaped versus property-shaped,
  which is the distinction that tells you which of the two On-target's rule handles well.
- **DELETE.** "Drawn or nothing" as it applies to *values*. As it applies to identity it is the
  load-bearing half and survives whole; as a law about parameters it forces every scalar into a
  coordinate and produces `too-small-to-draw` as an ordinary outcome, which #36 §8 already lists as a
  killer. Also delete "one region, one verb" read as a layout law rather than as a governor against
  accretion. It is right as a governor and it is the reason handle density is a real risk.

**#39 Flash, and #34 behind it**

- **KEEP.** The four-cluster session strip — identity and file lifecycle, transport clock, engine phase
  and history, runtime telemetry. It is the only grouping anyone drew, and chair test 6 asks for
  grouping by authority rather than by implementation convenience. The recognition that `set-gain` and
  `set-fade` are property-shaped and want a field that holds a value. Disclosure-only for the
  utterance, with the reasons #34 gives for it. The explicit fail-closed text for `[` / `]` with no
  source pointed, which is the right shape for an accelerator's refusal.
- **STEAL.** On-target's identity clause, at `LocusId` granularity. A field conditioned on `LocusKind`
  is a kind-driven property form, which the verb-license reading already deleted for all three models;
  a field conditioned on the exact locus is the Title field generalized by its reason.
- **DELETE.** `Fade Out (s)`: `TimelineClip.fade_out` is honoured by `evaluate` and no `SemanticEdit`
  sets it, so the field is a Core gap filled by furniture. `Backspace` deleting Title and Callout, and
  the summary matrix row that routes `delete` to `LocusKind::Title`: `legal_edits_for` for a Title
  names `title`, `set-position`, and `resize-overlay`, and a key that deletes it is invented legality
  in the most dangerous available direction. The Inspector `✂ Split Scene at Playhead` button, per the
  instrument test. And `In Point` / `Out Point` as fields — `trim` is coordinate-shaped and already
  routed on the Timeline edges, so these are a second and weaker route to a shipped one, which is the
  exact defect #36 §1 identified in the bank's own `Set In` / `Set Out`.

**#38 Control, and #35 behind it**

- **KEEP.** The measurement, which nothing else in the field supplies. The inventory table mapping each
  control to its `debug_selector`, its session call, its Engine verb, and its current `routed_verbs`
  home is the ground truth every other candidate is edited against, and the synthesis's session-strip
  tier is #34's grouping applied to exactly this list. The honest statement that `Toolbar` is today the
  only commit projection for four verbs. The refusal to fill the hole so the candidate looks complete.
  The observation that VEL text editing rewrites `gain` and scene structure without being a
  `routed_verbs` home for those verbs, which stops "you can always edit the text" from being an answer.
- **STEAL.** Nothing into Control; it is a baseline and a design stolen into it stops being one. What
  is stolen *from* it is the falsifier role: Control is the arm that answers whether the four verbs were
  reached often enough to deserve chrome, and that question is worth keeping alive after a spine is
  picked.
- **DELETE.** Control as a shipping destination — not because it is wrong, but because shipping a
  measurement means shipping four unrouted legal verbs, and per Question 1 that makes the CLI strictly
  more capable than the GUI. Also delete its framing as the low-risk option; the chair's tension 3 is
  precise that deleting the only route to legal work is a reachability cost, not a purity gain.

**#37 Ledger, and #33 Model 2 behind it**

- **KEEP.** The record half, which is the part of #37 nobody is arguing about and the part worth the
  most. The sketch's `Invoked this session` list of committed `(verb, target, scope, effect)` rows is
  not a verb bank; it is the after-the-fact display the deleted bank lacked, in Engine vocabulary, and
  it is the only place in the entire field where a user can read back what they just did. It answers
  the `--3` corruption from the other side of the commit: the button could not be contradicted before
  the click, and nothing contradicted it after either. This is a volatile working-session readout of
  Studio's existing source-backed Undo history, not a second persistent history store — persistent
  history stays Git.
- **STEAL.** On-target's value clause. If any part of the ledger keeps an invoke, the row must show the
  value it will overwrite and take the new one in place, because a row that discloses verb, target,
  scope, and effect and omits the current value is precisely the bank's missing display rendered as
  prose.
- **DELETE.** The utterance as the commit surface — Question 3. And the name: `Ledger` must denote one
  object. #25's Route Ledger discloses while projections commit; #33 Model 2 and #37's Command Ledger
  commits. Carrying both words forward guarantees that a later reading records agreement where there
  is none.

**#31 chair**

- **KEEP.** The six tests as the reading frame. The four-way disambiguation of "global" — position,
  membership, legality, target — which is what lets the synthesis keep a fixed strip without keeping a
  fixed verb set. And the statement that DELETE is an object judgment rather than a removal sequence.
- **STEAL into the synthesis.** Chair tension 3, semantic purity versus present reachability, is the
  bill the merge exists to pay: no single candidate both deletes the bank and leaves every legal verb
  reachable, and the tiered reading is what makes paying it possible rather than a matter of nerve.

## Question 3 — the utterance stays disclosure-only

**Answer: disclosure-only. Keep #37's record; drop #37's invoke.** Four reasons. The first three
belong to the field; the fourth is this note's, and it is the one that does not reduce to taste.

**A speaker that is also an instrument cannot report a missing instrument.** The utterance exists to
make the gap between the Engine legal set and what a gesture can commit observable — the locked rule
is that when legality differs from routing, the difference is spoken. If the sentence can close the gap
itself, every route hole becomes committable from the sentence, and the routing table stops being
falsifiable. #36 §4 states this and it is the strongest structural argument in the field.

**One utterance has to survive having no pointer.** The same derivation is consumed by Studio, by
`inspect --json`, and by an agent. A reading can be identical across three clients. A commit surface
cannot, and the moment they diverge there are two answers to "what may I do here" — the condition
`AGENTS.md` forecloses by making the CLI a first-class client.

**It keeps the one-pointer-up contract trivially true.** Every commit path is a gesture with a
lifecycle: pointer down begins, move updates ephemeral geometry, up commits exactly one
`SemanticEdit`, Escape cancels with source unchanged. A clause that commits needs its own answer to
what its ephemeral geometry is, what Escape means while it holds focus, and where its parameter is
entered. #33 Model 2 supplies an Escape rule and leaves parameter entry explicitly open, which is
honest and is also the shape of a second contract.

**A commit surface owes a value, and the utterance has no field to hold one.** This is the reason the
count in the table above does not settle anything. The utterance is built by iterating
`legal_edits_for`, whose `LegalEdit` carries `verb`, `target`, `scope`, and `effect` — four fields, none
of which is the number the edit would overwrite. To make a clause invocable under the instrument test
you must add a value display and a parameter input to every clause, and at that point the object is a
property form laid out as sentences and the honest thing is to call it one. So "let the utterance
commit" is not a cheap change of role. It is a different object wearing the utterance's name, and the
fixture receipts are what make that expensive rather than merely inelegant: the deleted `Gain −3 dB`
button did not fail because it was global.

**The carve-out, drawn tightly and taken from #36 §4.** The utterance may carry Navigate and explicit
seek. Both are non-edits: they change no source, push no Undo, and do not move here. The line is
mechanical rather than tasteful — *the utterance may move the eye; it may never move the source* — and
it is checkable by construction, since nothing reachable from the utterance may reach `apply_edit`.

**What disclosure-only costs, stated.** The power user's fastest conceivable path is here → named
command → typed value, and disclosure-only forbids the last two steps living in the sentence. They have
to be built somewhere else: an accelerator bound to a drawn handle under C3, plus a field that holds
the value. If that combination is slower than the bank was for the verbs the bank committed correctly,
disclosure-only bought honesty with throughput, and #36's falsifier applies to this note as much as to
the note it came from. The risk is carried, not argued away.

One further obligation follows for the agent, and it is not hypothetical. A routing reason is a Studio
fact. `unrouted`, and the `too-small-to-draw` / `no-drawn-block` reasons #36 names as reasons a design
would need, are conditions of a layout, not claims about legality. If one of them reaches `--json`
wearing an existing Engine reason such as `structurally-absent`, an agent reads a Studio layout
condition as an Engine legality claim, and the one-answer property that motivates disclosure-only is
lost inside the thing that was supposed to protect it. #36 is explicit that adding a typed routing
reason is a decision for whoever ships, with an explain event and an origin. This note agrees and adds
only that the alternative — borrowing an Engine reason — is not the cheap option it looks like.

## PR #41 is one candidate in the world

[#41](https://github.com/annenpolka/lattice/pull/41) is an in-flight implementation of the On-target
rule, started before any 座組 winner existed. This note reads it as one candidate and neither merges,
blocks, nor ratifies it. The synthesis above is not #41, and the difference is nameable rather than
atmospheric:

- **#41 is one tier of three.** It implements the identity clause thoroughly — Toolbar routes nothing,
  `commit_projection` no longer names it, the empty legal set is spoken, `commit_projection`'s `None`
  is honoured instead of falling back to Timeline — and it draws the value where the drawing carries
  it. It does not add typed entry for property-shaped verbs, which is the Flash steal, and it does not
  add the invoked-rows record, which is the Ledger steal.
- **Its unrouted clause reuses `structurally-absent`.** That is the borrowing named at the end of
  Question 3. It is a real decision with a real cost and it is exactly the kind of thing #36 said
  belongs to whoever ships, with an explain event and an origin.
- **It fixed two of #36's findings while implementing.** The `apply_gain` sign corruption and the
  empty-diff Undo are recorded there as fixed. Those are true defects independent of any 座組 outcome,
  and their fate should not be coupled to whether this spine is the one that ships.

If a later chair proposes this spine, #41 is the largest existing head start on its middle tier. That
is a fact about overlap, not a verdict, and this note does not cast one.

## What this note does not do

- It does not select a shipping model. The three tiers and the instrument test are a proposal for how
  four sketches and six notes read as one, offered as discussion input.
- It does not implement anything. No Studio, GPUI, Engine, Core, VEL, or CLI change is made or planned
  here, and no file outside `docs/notes/` is touched.
- It does not merge, rank, or bless any of #25–#41, and it does not edit their branches. They were read
  at their PR heads as sources.
- It does not reopen the 座組 lock or any pointing lock. The bank stays deleted, overlap stays
  Projection-Local on the touched projection, and a video click stays the source clip.
- It does not settle session-strip membership as a list. It admits the strip by the predicate the field
  already converged on and applies #34's grouping to #35's inventory; a hand-authored list is how six
  locus-taking buttons got there in the first place.
- It does not fill the Core gaps it names. No `fade_out` edit variant, no time-scoped audio edit, no
  legality for a `Placement`, and no new `AbsenceReason`. Naming a gap is not license to invent a
  filler.
- It does not ship the instrument test. It is a reading with a handle attached, not a type.
- It does not claim a look. This note adds no image and draws no sketch. The four sketches it reads
  stay where they are, on their own branches, at the heads named above.
