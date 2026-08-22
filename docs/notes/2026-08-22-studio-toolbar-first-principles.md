# Is a global verb button row the right object at all?

Date: 2026-08-22
Read at: `85b589ec260554f851c214731e607c7727c7cae8` (main, after the INTEGRATED
verb-license spine and its leftover fix).

## The assignment, and the lens

Not "how should the current buttons be grouped." The question is whether a
**global top-of-window verb button row** is the right object to have, given a
spine that is already locked:

> one locus, one Engine legal set, one utterance; gesture is routing; speak when
> legality ≠ routing.

One lens only: **Engine / `SemanticEdit` / legal set.** Studio is a client of the
Engine. The Engine names what is legal for `here`. Everything below is derived
from that relation, checked against the running window.

This is a docs-only rethink. Nothing here is implemented, and §7 deliberately
stops before choosing a widget.

Held fixed, not reopened: overlap is Projection-Local with one `LocusId` after
the pick; a video click points the source clip; no silent retarget; no per-view
selection; no Core Freeze; no GPUI in Core.

**Answer up front: no.** Three independent reasons, each sufficient on its own
(§3), and a fourth result that is more interesting than the refutation — the
fixed point of *"keep the row but make it correct"* is the Inspector (§6). The
global position was never load-bearing.

## 1. What a button row is, structurally

Strip the pixels off and a row of buttons is exactly three commitments:

1. **Fixed membership.** Which affordances exist is authored once, at compile
   time. The set has a cardinality before the program runs.
2. **Fixed global position.** It sits outside every projection. That is what
   "global top-of-window" means — it is present regardless of what you are
   looking at.
3. **Standing invitation.** Each member renders as invocable whenever the row
   renders. A rendered, cursor-pointer button is a claim that the verb exists and
   can be committed now.

## 2. What a verb is, under the locked spine

1. **Membership is a function of `here`.** `legal_edits_for(locus)` is computed,
   not authored. It returns `Vec<LegalEdit>`, each element
   `(verb, target: LocusId, scope, effect)`.
2. **A verb has a place.** `LegalEdit.target` is a `LocusId`. Loci are rendered
   by projections. So every legal edit has somewhere on screen where its subject
   is visible.
3. **Invocability is conditional on the gesture.** `routed_verbs(projection,
   kind)` — note the *first* parameter. The signature already asserts that verbs
   are local. Were verbs global, the function would be `routed_verbs(kind)`.

The button row and the verb disagree on all three axes. §3 turns each
disagreement into a result.

## 3. Three disqualifications

### 3.1 Impossibility — a fixed global set matches almost no legal set

The Engine names eight locus kinds. Their legal sets:

| `LocusKind` | `legal_edits_for` |
|---|---|
| Title | `title`, `set-position`, `resize-overlay` |
| Callout | `callout`, `set-position`, `resize-overlay` |
| Source | `trim`, `set-gain`, `set-fade` |
| Scene | `split`, `delete`, `reorder-scene` |
| Placement, Sequence, Media, Speech | *(empty)* |

The union has ten verbs; no single set has more than three; Title and Source
share nothing. So for a fixed global set `G`:

- `G` = the union → wrong for all eight kinds (each offers ≥7 verbs too many).
- `G` = some kind's set → correct for that one kind, wrong for the other seven.

**A fixed global verb set can be correct for at most one of eight locus kinds.**
That is a property of the object, not of this implementation. Cosmetic regrouping
cannot touch it.

What the shipped row actually achieves is zero of eight. Its six verb buttons
cover `trim` (twice), `split`, `delete`, `set-gain`, `set-fade` — Source's whole
set plus two of Scene's three, which is no kind's legal set:

| `here` kind | of 6 verb buttons, live | legal verbs the row cannot offer |
|---|---|---|
| Source | 4 | 0 |
| Scene | 2 | 1 (`reorder-scene`) |
| Title | 0 | 3 |
| Callout | 0 | 3 |
| Placement, Sequence, Media, Speech | 0 | 0 |

Of the 48 (button, locus-kind) pairs the six verb buttons span, **6 can commit**.
The other 42 render live, identically styled, and refuse. The row's best case,
reached on exactly one kind, is four of six.

### 3.2 Redundancy — both of the row's jobs already shipped, done better

A verb row can only be for two things.

**Discovery** — "what can I do here?" That is the utterance, verbatim. One locus,
one legal set, one utterance. The utterance names every legal verb *with* target,
scope, and effect, plus the surface that commits it, plus a typed `AbsenceReason`
when it is absent. A button carries a label. The utterance is strictly richer, and
it is the thing the lock already put in charge of this question.

The measurement matters more than the argument. The row contributes **zero**
discovery today: across three loci with three different legal sets, the entire top
chrome is byte-identical (§5.1), and the row reads no legal set at render time.
Removing it removes no discovery, because it never supplied any.

**Invocation** — "do it now." For a parameterised edit a button can only offer a
frozen constant, and the shipped row freezes all of them: `-3` dB, `500ms`, the
playhead, `Time::ZERO`. For a non-parameterised edit the gesture is already
available and already carries the target. Neither case wants a button.

Discovery is the utterance's. Invocation is the gesture's. There is no residue for
a global verb row to hold.

### 3.3 Inversion — "gesture is routing" means the act carries the target

This is the load-bearing one.

"Gesture is routing" says the act you perform *selects* the verb. Dragging a clip
edge **is** trim; there is no step where trim is chosen and then a target sought.
The target is in the gesture, because the gesture happened on the thing.

A button inverts the order: the verb is chosen while no target exists, so a
target must be found afterwards. On a global surface the only thing available to
find is the ambient `here`. That is precisely the mechanism visible in the code:
`target_locus_for` exists *because* buttons arrive without targets, and its
fail-closed refusal is the cost of the inversion.

The consequence is not a lock violation — reading the one shared locus is exactly
right, and it is not a second selection. The consequence is that a global verb
button turns `here` into an **implicit argument, invisible at the point of
commitment**. "Magic is allowed. Hidden behavior is not." An unnamed target at the
moment a verb commits is hidden behavior in the one place it matters most.

A gesture cannot have this defect. Its target is the thing under the cursor.

## 4. The criterion that falls out

The row is not uniformly wrong, and saying so would be sloppy. Twenty buttons
sit in it; only six are verbs. So the useful question is a test:

> **Does this control commit a `SemanticEdit`?**
> Yes → it takes a locus → it belongs where that locus is rendered.
> No → it has no locus → a global control is correct for it.

That partitions the shipped row exactly:

| | Controls |
|---|---|
| **Commits a `SemanticEdit`** → must be local | Set In, Set Out, Split at Playhead, Delete Selected Clip, Gain -3 dB, Fade |
| **Takes no locus** → legitimately global | Open Video…, Save, Undo, Redo, Resolve, CPU, GPU DX12, Play, Pause, Seek, Scrub, Zoom In, Zoom Out, `Renderer · …`, `Audio · …` |
| **Reads `here`, commits nothing** → global is harmless | Copy locus JSON |

Undo is the sharpest check on the criterion: it is deeply semantic — it pops the
source stack and recompiles — yet it takes no locus, because it acts on the whole
source. It stays global, correctly. "Semantic" is not the test; "takes a locus"
is.

So the object at the top of the window is right about fourteen of its twenty
buttons. What it is wrong about is the six that make it a *verb* row.

**There is no global verb button row here.** There is a session / transport /
phase bar that has been carrying six verbs it cannot type-check. Remove those six
and the remaining object is coherent and needs no defence.

## 5. What the running window shows

Read live, not from source alone: `lattice-studio` built with `--features window`,
run on the agent X11 path (`scripts/studio-linux-smoke.sh`, `DISPLAY=:1`,
lavapipe, preview and audio detached) against the `timeline-basic` fixture at the
commit above; unique viewable client identified by `_NET_WM_PID`, captured with
`ffmpeg -window_id`. `LINUX SMOKE OK`. A second run pointed three loci in turn and
clicked two top-row verb buttons at bounds measured from the capture itself.

### 5.1 One row, three legal sets, zero pixels of difference

Pointing `title "Hello"`, then `source "clip"`, then `scene "demo"` — three
disjoint or near-disjoint legal sets — leaves the whole top chrome (rows 0–109:
`header_bar` plus both wrapped lines of `actions_bar`) byte-identical:

```
here = demo:title:1   rows 0..109 sha256 836321c98ef4869f0550c18408972ec547e1b2eaa86aec303f7eedfc9cdc6d7b
here = source:clip    rows 0..109 sha256 836321c98ef4869f0550c18408972ec547e1b2eaa86aec303f7eedfc9cdc6d7b
here = scene:demo     rows 0..109 sha256 836321c98ef4869f0550c18408972ec547e1b2eaa86aec303f7eedfc9cdc6d7b
pairwise differing subpixels in rows 0..109: 0
```

The same hash appears in a separate earlier process run, so it is stable across
launches, not an artefact of one frame. Everything that does react sits below:
the first differing row anywhere in the window is 151, in the VEL pane's locus
hint line, and the Inspector — where the utterance lives — first differs at row
157. This is §3.2 measured rather than argued.

### 5.2 The Engine says "committed on Toolbar", which is not a place

At `here = source:clip` the utterance reads
`set-gain → source:clip (source-binding: set gain on this source) is legal for
this source, committed on Toolbar — not implied absent here.` At
`here = scene:demo`, `"routed":[]`, with both `split` and `delete` "committed on
Toolbar".

"Committed on Canvas" is an instruction: go there, the thing is there, point it.
"Committed on Toolbar" is not — the toolbar is always present and contains no
locus. `Projection::Toolbar` is a constant sitting in a function whose first
parameter exists to express variation, and `commit_projection` can return it, so
one sentence form is doing two different jobs with no way for the reader to tell
which.

`set-gain`, `set-fade`, `split`, and `delete` route *only* through Toolbar. Those
four legal edits therefore have no surface anywhere where their target is visible
at the moment of invocation. `delete` is legal on a Scene, targets that Scene, and
its sole path is a global button labelled for a clip.

### 5.3 The refusal is not readable

`speak_toolbar` writes the refusal into `last_render`, which the Inspector
renders as `format!("wrote {path}")`. With `EngineError::Edit`'s
`#[error("edit: {0}")]`, the sentence the window actually composes for a refused
verb is `wrote edit: set-gain is not legal for title "Hello"
(needs-source-binding).` — a legality refusal wearing a file-write prefix.

Live, clicking `Gain -3 dB` at `here = title "Hello"` logged exactly that
refusal and changed **three rows of pixels** — 677–679, inside the Inspector pane
(x 1169–1331): glyph tops at its clip boundary. Every other differing pixel in
the frame is the mouse cursor. `Split at Playhead` behaved the same way. The
Engine spoke correctly and fail-closed; `here` did not move; and the screen did
not say so.

### 5.4 Smaller facts the argument leans on

- The row is `.flex().flex_wrap()`. At 1400px it wraps to two lines and splits
  the six verb buttons across both. Positions move with window width, and
  `studio-linux-smoke.sh` has to wait out the reflow before trusting
  `smoke_geom`. Whatever stability a global row is supposed to buy, this one does
  not have.
- `header_bar` renders `format!("{file} · Scene demo")` — a hardcoded literal. It
  read `main.vel · Scene demo` directly above six verb buttons while `here` was
  `title "Hello"`. The one locus-shaped string in the top chrome is wrong by
  construction.
- Labels keep at most the verb and drop `target`, `scope`, `effect`.
  **Delete Selected Clip** additionally names a *selection*, which the model does
  not have, and the wrong kind: `Delete` is legal on a Scene and targets a Scene.
- `action_selector` is a `match` on the visible label and `action_button` takes
  its GPUI `id` from the label, though `docs/interaction.md` states selectors "do
  not depend on visible labels." Unknown labels collapse to a shared
  `toolbar.unknown`.
- `Copy locus JSON` is the counterexample already present: it reads `here`, calls
  `Engine::inspect`, and reports `"no current locus"` *before* acting. The row can
  be locus-aware. It just is not, for the verbs.

## 6. The interesting result: correcting the row converges on the Inspector

Suppose you keep a global verb surface and only insist it stop lying. Then, from
§2, it must render `legal_edits_for(here)`; its membership must vary with `here`;
its labels must carry the Engine's `target`, `scope`, and `effect`; and it must
distinguish invocable-here from routed-elsewhere from absent-with-a-reason.

At that point it is no longer a button row. Its membership, text, and cardinality
are all functions of `here` — it *is* the utterance, drawn with commit
affordances. And a rendering of the utterance for `here` is what the Inspector
already is.

So the corrected object is locus-scoped, and being global was doing no work in
it. Worse, keeping it global costs the one property a global surface is supposed
to provide — a fixed place — because its contents now change on every locus
change, while the Inspector renders the same answer already adjacent to `here`.

**"Make the global verb row correct" has the Inspector as its fixed point.** That
is a stronger statement than "the row is wrong": there is no correct global verb
row to build, because every step toward correctness moves it somewhere else.

## 7. Where the verbs live, without inventing a surface

Falls out of §2.2 — a verb goes where its target is rendered — plus one
observation: a verb's *parameter* also has a home, and it is the projection where
that parameter is a coordinate. The six the row currently holds are marked ★; the
rest of the union is included so the assignment can be checked for completeness.

| Verb | Target is rendered by | Parameter | Natural surface |
|---|---|---|---|
| ★ `trim` | Timeline (clip has a span) | in/out times | Timeline — already routes it |
| ★ `split` | Timeline, SEQUENCE (scene) | a time | Timeline — time is a coordinate there |
| ★ `delete` | Timeline, SEQUENCE (scene) | none | the projection that renders the scene |
| ★ `set-gain` | Timeline Audio track (binding) | a scalar | a continuous control on the binding |
| ★ `set-fade` | Timeline Audio track (binding) | a duration | Timeline — duration is a span there |
| `reorder-scene` | Timeline, SEQUENCE | an index | Timeline — already routes it |
| `callout` | Timeline | times only | Timeline — already routes it |
| `title` | Timeline (span), Inspector (text) | text and times | Inspector for text — already routes it; Timeline for its span |
| `set-position`, `resize-overlay` | Canvas | normalized space | Canvas — already routes them |

(`trim` is ★ twice over: Set In and Set Out are both `Trim`.)

Two things to notice.

**Nothing new is required.** Timeline, Canvas, SEQUENCE, and Inspector are all
existing `Projection` variants. Three of the four already commit verbs today.
SEQUENCE renders scenes and commits nothing — it is a rendered-target surface
sitting idle while `delete` lives on a global button.

**The Inspector is the general answer for verbs with no coordinate.** `title`
text is not geometric; there is no coordinate to point at. `routed_verbs(Inspector,
Title) = ["title"]` and the Title text field is already there, appearing only when
`here` is Title. That is the pattern for any property-shaped edit — and it is
locus-scoped, not global, which is exactly the distinction §3.3 turns on.

So the assignment is: **coordinate-shaped edits to the projection that renders the
coordinate; property-shaped edits to the Inspector; structural edits to the
projection that renders the object.** Nothing is left over, and no global verb
home is needed.

### The four "homeless" verbs are a debt, not a counterexample

The strongest objection to "no global verb home" is that `set-gain`, `set-fade`,
`split`, and `delete` have no local route, so removing the row strands them.

True today, and the reason matters: they are homeless because **no gesture was
built for them**, not because they resist being local. Each has a projection that
already renders both its target and its parameter's coordinate (table above).
`split` in particular is *more* natural on the Timeline than on a button — it
takes a time, and the Timeline is where time is a coordinate; the button has to
borrow the playhead as an unnamed second argument, which is why it is labelled
"Split at Playhead."

That reframes the row: it is not an architecture holding verbs that cannot be
local. It is a **placeholder standing in for four unbuilt local affordances**, and
naming it as debt is more honest than defending it as a design.

## 8. What has to be true before the row's verbs can leave

Not optional, and not cosmetic:

1. **The utterance must be legible.** Observed, it is clipped mid-sentence in a
   240px pane, and the refusal lands below the fold (§5.3). If discovery is the
   utterance's job (§3.2), it cannot be below the fold. This is a prerequisite,
   not a polish item.
2. **Every verb in the union needs one named local route, or a spoken reason it
   has none.** Four do not have one (§5.2). Until they do, deleting the row
   deletes reachability, and "unreachable" must be spoken, never implied absent.
3. **`refuse_edit` becomes an invariant check, not a channel.** If verbs appear
   only where they commit, a refusal means Studio rendered something it should
   not have. The fail-closed gate stays; it stops being the primary way the user
   learns anything.
4. **`Projection::Toolbar` becomes dead and observable as dead.**
   `routed_verbs(Toolbar, k)` empty for all `k`, and `commit_projection` unable to
   name it — so "committed on X" always names somewhere you can point.

## 9. The steelman, and where its residue actually lands

The honest case for a global verb row is expert speed: a fixed place and muscle
memory beat hunting for the right projection.

Three answers. It does not have a fixed place (§5.4, `flex_wrap`). Speed on a
parameterised verb is illusory when the parameter is frozen at `-3` dB. And the
real accelerator for a locus-scoped verb is an **input binding**, which is global
in *input* while staying local in *semantics*: it acts on `here`, can be gated on
`legal_edits_for`, and — the part that matters for §3.3 — makes no standing visual
claim, so an illegal binding can decline and speak without ever having lied.

So the legitimate residue of the steelman lands on bindings, not on a rendered
global surface. Whether Studio grows them is not decided here.

## 10. What this does not touch

- **Not "hide what is illegal."** The row must be able to *disclose* a verb it
  cannot commit; the defect is that it *offers* it. Showing only routable verbs is
  implied absence, which the spine forbids outright.
- **Not a second legal set.** Studio keeps reading `legal_edits_for`. Nothing here
  asks Studio to compute legality.
- **Not a second utterance.** `session.utterance()` is computed once; a second
  reader of the *same* `Utterance` value is not a second utterance. The lock
  forbids a second legal set and a second selection, not a second renderer.
- **Not a second selection, and not per-view selection.** No surface acquires a
  locus of its own. Everything reads the one `here`.
- **Not the Inspector swallowing direct manipulation.** §7 sends the Inspector
  only what has no coordinate. Geometric and temporal verbs stay on Canvas and
  Timeline gestures, because gesture is routing.
- **Not a relaxation of the commit gate.** §8.3 keeps `target_locus_for`.
- Overlap Projection-Local, video-click-points-the-source-clip, silent retarget,
  Core Freeze, and GPUI in Core are untouched.

## 11. Not decided here

Which projection takes `delete` when both Timeline and SEQUENCE render the scene.
What a continuous control for `set-gain` is. Whether `Projection::Toolbar` is
deleted or kept as an explicitly empty variant. Whether the session bar keeps
`Seek` and `Scrub` — both are transport at verb rank, and `Seek` freezes
`Time::ZERO` behind a verb label, so they are instances of §4, not a separate
question. Whether input bindings exist. All design; this note fixes only what any
answer must satisfy.

## Evidence

Pane bounds, for reading the crops: SEQUENCE `x 0–199`, Canvas `200–879`, VEL
`880–1159`, Inspector `1160–1400`.

- `docs/screenshots/toolbar-rethink-one-row-three-legal-sets.png` — the argument
  in one frame: the identical top chrome above the VEL and Inspector panes at
  `here = title "Hello"`, `source "clip"`, `scene "demo"`. Three legal sets, one
  row, zero differing pixels in the row.
- `docs/screenshots/toolbar-observe-top-row.png` — the object itself, rows 0–109.
- `docs/screenshots/toolbar-observe-here-title.png`,
  `docs/screenshots/toolbar-observe-here-scene.png` — full window at two loci.
- `docs/screenshots/toolbar-observe-refusal-invisible.png` — the VEL and Inspector
  panes before (left) and after (right) a refused top-row verb click; the red
  gutter marks rows 677–679, the entire visible response.
- `docs/screenshots/toolbar-vote-labels-vs-engine-fields.png` — §12 test 2: the
  row's labels above the Engine's own words for the same `here`.

---

## 12. Phase III vote

**Concurrence with the narrow claim.** The current fixed, always-visible bank of
locus-taking `SemanticEdit` buttons is not a coherent global verb surface; a
session strip may remain global. §4 reached the same split independently, from
the `SemanticEdit` criterion rather than from the frame: six buttons commit a
`SemanticEdit` and must be local, fourteen take no locus and are legitimately
global.

### Scoring the bank against the six tests

| # | Test | Bank | Why |
|---|---|---|---|
| 1 | standing invitation for a locus-taking edit? | **fail** | Six buttons render live and `cursor_pointer` regardless of `here`. 42 of 48 (button, locus-kind) pairs are false invitations (§3.1). |
| 2 | target / scope / effect / parameter / committing projection disclosed before commit? | **fail ×5** | Labels keep at most the verb. `LegalEdit` supplies target, scope, effect; all three are dropped. Parameters are not disclosed but decided (`-3` dB, `500ms`, playhead, `Time::ZERO`). The route is never shown. |
| 3 | Engine only legality authority? | **pass** | `target_locus_for` calls `is_legal_verb`; `refuse_edit` reads `legal_edits_for`. Studio computes no second legal set. The bank consults the authority too *late*, which is test 1's failure, not this one's. |
| 4 | uses the one `here`, fail-closed, no target search / promotion? | **pass** | Verified live: refused clicks left `here` unchanged, and there is no `target_source_locus` / `target_scene_locus` fallthrough. |
| 5 | every legal edit has a named route or is spoken unrouted? | **fail (qualified)** | Nothing is silently absent — the utterance speaks all ten union verbs. But four verbs' named route is `Toolbar`, which is not a surface you can point in, and `trim` is double-routed with only Timeline nameable, so the bank's own Set In / Set Out are unspeakable (§5.2, §3.1). |
| 6 | non-verb global controls grouped by actual authority? | **fail** | Five control layers plus a display channel at one visual rank; teal means selected-mode, emphasis, and transport; the renderer/audio typed-error channel is a label between two buttons in a wrapping row. |

**The scoring partitions cleanly, and that is the finding.** The bank passes
exactly the two tests the spine already disciplined — Engine authority and
fail-closed pointing — and fails exactly the four that are properties of *being a
fixed global bank*: standing invitation, non-disclosure, unnameable routes, and
undifferentiated rank. No further spine work reaches tests 1, 2, or 6, because
they are consequences of fixedness, globality, and rank rather than of legality
plumbing.

One caveat on the test-4 pass, since it is the regression surface: the bank
satisfies "no target search" only because search was explicitly forbidden. A
button arrives without a target, so the *pressure* toward searching for one is
intrinsic to the object. Any future change that makes these buttons "work more
often" reintroduces it.

### Vote: DELETE the object, STEAL the verbs

One motion seen from two ends. The bank ceases to exist as a verb surface
(DELETE), and each verb lands on the projection that already renders its target
(STEAL).

- **KEEP is unreachable, not merely unattractive.** Repairing tests 1 and 2
  forces the surface to render `legal_edits_for(here)` with the Engine's target,
  scope, and effect; membership then varies with `here` and it is the utterance,
  whose fixed point is the Inspector (§6). Every step toward passing the tests
  moves the object somewhere else.
- **DELETE alone is admissible but leaves debt.** It clears tests 1 and 2 at a
  stroke and does not fail test 5, since "spoken unrouted" is permitted. It does
  cost reachability for the four verbs whose only route is `Toolbar` (§7).
- **STEAL dominates DELETE** on tests 5 and 6: routes become nameable places, and
  what remains is non-verb, so it *can* be grouped by authority.

### The object I name for the residue

Not "the current row minus six buttons." **A session strip whose membership is a
predicate, not a list** — `commits_semantic_edit == false`. Test 6 asks for
grouping by actual authority, and that is only checkable if membership is derived
from authority. A hand-authored list is exactly how six locus-taking verbs came
to live in a global bank in the first place, and it is how they would return.
Under the predicate the strip's groups are Engine phase and project I/O, session
history, transport, renderer request plus its typed error channel, viewport, and
read-only locus export (§4).

### Sequencing, not a shipping choice

Two prerequisites, from §8, that make the difference between DELETE and dropping
things on the floor: the utterance must stop being clipped before it inherits
discovery, and each of the four `Toolbar`-only verbs must gain a named local route
or be spoken unrouted. A third consequence is the end-state check:
`routed_verbs(Toolbar, k)` empty for every `k`, and `commit_projection` unable to
name a non-place.

**Not decided by this vote**, and deliberately: which projection takes each verb
where more than one renders the target, what a continuous control for `set-gain`
is, and whether `Projection::Toolbar` is deleted or kept as an explicitly empty
variant. §11 stands. No shipping winner is picked here.
