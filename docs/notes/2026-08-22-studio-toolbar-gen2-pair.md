# Studio toolbar gen2 — two replacement models (Grok pair)

Date: 2026-08-22

Status: two isolated replacement models for the deleted locus-taking toolbar
bank. Not a selected design. Not a shipping decision. Not an implementation
plan. This note does not implement Studio, does not change Core or Engine, and
does not pick a winner.

Scope: where `set-gain`, `set-fade`, `split`, and `delete` commit after the
always-visible locus-taking verb bank is gone, and what the one utterance is
allowed to do. No other gen2 note, observer pack, or chair reading is used
here. The shared premise below is pasted, not browsed.

No look is claimed. There is no committed PNG and no sketch URL. Do not infer
chrome, panel weight, or a product skin from the names.

## Shared premise (pasted; not browsed)

- Unanimous DELETE: fixed always-visible bank of locus-taking SemanticEdits
  (Set In/Out, Split at Playhead, Delete Selected Clip, Gain -3dB, Fade).
- A non-verb session/application strip MAY remain global. Membership is NOT
  settled.
- Locks: overlap Projection-Local (failed point → candidates on THAT
  projection; one `LocusId` after pick); video click = source clip, no scene
  promotion; one locus / one Engine legal set / one utterance; gesture is
  routing; speak when legality ≠ routing; no silent `target_*_locus`; Title
  fields only on Title; no per-view selection; no Core Freeze; no GPUI in
  Core.
- Each model MUST name a route (or explicit unrouted) for set-gain, set-fade,
  split, delete. MUST say whether utterance is disclosure-only or a commit
  surface. MUST not reopen locks.

## What is being replaced

Today those four verbs (and Set In / Set Out, which is `trim` at the playhead)
are committed from a global, always-visible bank that takes the committed
locus as its implicit target. That bank is deleted in both models below.

The deletion is of **locus-taking SemanticEdit chrome**, not of the verbs.
Engine `legal_edits_for` is unchanged: `set-gain` / `set-fade` name a Source;
`split` / `delete` name a Scene. Studio routing is what must be rewritten.

A Play / Pause / Seek / renderer / Open strip is a different object. Both
models allow such a strip to remain global. Neither model names its members.

## Locks this note will not reopen

Pointing, legality, and Core boundaries are closed. Both models treat them as
invariants.

| Lock | Held as |
|---|---|
| Overlap | A failed point lists candidates on the touched projection only. One shared `LocusId` after the pick. No ownerless probe, no rank-then-step, no click-to-commit-as-overlap-UI. |
| Video click | Clicking a video clip points the source clip and keeps that identity. Here is not promoted to the containing Scene. |
| One locus | One committed `LocusId`. Focus, hover, playhead, and in-flight gesture are not a second selection. |
| One Engine legal set | `legal_edits_for(here)` is the only legal set. Surfaces do not invent verbs. |
| One utterance | One spoken answer for here. Not a per-view caption and not a second legal list. |
| Gesture is routing | A pointer gesture says which projection can commit. It does not license a verb the Engine cannot name. |
| Speak the gap | When Engine legality ≠ what the gesture can commit, the difference is spoken. Absence is not silence. |
| No silent retarget | No `target_source_locus` / `target_scene_locus` fallthrough. A control that rewrites the target while the displayed locus stays put is a bug. |
| Title fields | Title-shaped Inspector fields only when here is Title. |
| No per-view selection | No Canvas-selection vs Timeline-selection. Navigate is optional and never a gate. |
| No Core Freeze | Freeze remains a `TimeMap` reading, not a `LocusKind` and not a selectable row. |
| No GPUI in Core | Studio projects; Core does not take GPUI types. |

Playhead remains the session A/V clock. Scrub does not re-point, does not
rewrite VEL, and produces no Undo. Locus change never auto-seeks.

The axis that is open — and the only axis this pair answers — is **where a
legal verb commits after the global bank is gone**, and **whether the
utterance may accept that commit**.

## The cut that forces the two models apart

The two models are not skins of one idea. They disagree on the commit home.

| | **Local-Projection Commit Homes** | **Command Ledger** |
|---|---|---|
| Commit home | The projection that already owns that instrument. No global verb home. | A named-command ledger. Projections do not grow verb buttons for these four. |
| How a verb fires | Pointer-up on that projection's instrument, one `SemanticEdit`. | Invoking a named command against the one committed locus, one `SemanticEdit`. |
| Utterance | **Disclosure-only.** Speaks `(verb, target, scope, effect)` and the routing gap. Never commits. | **Commit surface.** A present clause may be invoked. Disclosure and commit share the same spoken row. |
| What is unrouted | A verb with no instrument on the touched projection. Spoken, not relocated to a bank. | A verb the ledger will not accept for here. Spoken, not relocated onto a projection. |
| What would collapse them | Putting the four verbs back on a global always-visible bank, or letting the utterance commit in this model. | Growing Timeline/Canvas verb chrome for the four, or making the utterance disclosure-only in this model. |

They share the deleted bank, the locks, and the unsettled session strip. They
do not share a commit path.

---

## Model 1 — Local-Projection Commit Homes

One-line rule: **a legal verb commits only on the projection that already
owns that instrument. The utterance tells you why a surface is silent. It
never fires the verb.**

This is the projection-local replacement. The deleted bank is not moved to
another global strip. The verbs go onto the surfaces that already route
related gestures (Timeline timing / source-binding; Canvas geometry stays
geometry). If a projection has no instrument for a verb, that verb is
**unrouted on that projection**, even when the Engine names it for here.

### Utterance

**Disclosure-only.** The utterance is the spoken difference between Engine
legality and projection routing. It names present clauses and absences. It
is not a button row, not a command palette, and not an Undo surface.
Clicking, confirming, or "applying" a spoken clause commits nothing.

### Routes for the four verbs

| Verb | Engine names it on | Commit route | Explicitly unrouted |
|---|---|---|---|
| `set-gain` | Source | Timeline source-binding instrument on the pointed source clip (gain handle / level on that clip). Pointer-up commits `SemanticEdit::SetGain`. | Canvas, Inspector, Review, Tree, Source pane, and any leftover global verb bank. Scene here: unrouted; speak `needs-source-binding` / point the video clip. Do not promote, do not retarget. |
| `set-fade` | Source | Timeline source-binding instrument on the pointed source clip (fade handle on that clip). Pointer-up commits `SemanticEdit::SetFade`. | Same surfaces as `set-gain`. Scene here: unrouted; speak the source-binding gap. |
| `split` | Scene | Timeline scene instrument on the pointed scene span (razor / split at the playhead's time on **that** scene). Pointer-up commits `SemanticEdit::Split { at }`. Playhead supplies the time argument; it does not become here. | Canvas, Inspector, and every global bank. Source here (including video click): unrouted; speak that `split` is legal on the related scene and that here is still the source clip. No scene promotion. No silent `target_scene_locus`. |
| `delete` | Scene | Timeline scene instrument on the pointed scene span (delete on **that** scene). Pointer-up commits `SemanticEdit::Delete`. | Same as `split`. Source here: unrouted; speak the related-scene legality as Navigate, never as an adopted center. |

Trim / Set In / Set Out are outside the required four. They are named only
to keep the deletion honest: the always-visible Set In / Set Out buttons die
with the bank. Existing Timeline edge-trim remains a Timeline instrument
under this model. This note does not invent a second trim home.

### Session strip

A non-verb session/application strip MAY remain global (transport, renderer
request, Open). Membership is not named. No locus-taking SemanticEdit may
join it. If a control on that strip starts taking `here` as a silent target,
this model has been abandoned.

### What this model refuses

- Rebuilding the deleted bank under a new label ("Actions", "Edit", "Clip").
- Committing `set-gain` / `set-fade` / `split` / `delete` from the utterance.
- A Canvas widget that commits any of the four. Canvas remains placement
  (`SetPosition` / `ResizeOverlay`) for Title and Callout.
- Promoting a video-click Source to Scene so that `split` / `delete` become
  local. The instrument is absent; the utterance speaks.
- Per-view "Timeline selection" vs "Canvas selection". The instrument reads
  the one committed `LocusId`.
- Overlay candidate lists on a projection that was not touched.

### Falsifier

If, after the bank is gone, any of the four verbs is committed from a global
always-visible control, or from a spoken clause, this is not Local-Projection
Commit Homes. If `split` or `delete` fires while here is the video-click
source clip, the video-click lock has been broken and this model is void.

---

## Model 2 — Command Ledger

One-line rule: **the four verbs are named commands against the one committed
locus. They do not grow projection instruments. The utterance is the ledger
row you may invoke.**

This is the command/ledger replacement. Timeline and Canvas keep the
gestures they already own (trim edges, overlay time, scene reorder, Canvas
geometry). They do **not** receive gain handles, fade handles, razors, or
delete hits for the deleted bank. Those verbs live as named commands whose
target is always `here`, recorded as a session ledger of invoked
`(verb, target, scope, effect)` rows.

The ledger is not the deleted bank. The deleted bank was a fixed,
always-visible row of locus-taking buttons. The ledger is the utterance
plus an invoke step: a present clause can be committed; an absent clause
cannot. There is no second legal set and no second target.

### Utterance

**Commit surface.** The one utterance *is* the command ledger for these
four verbs (and any other Engine-named verb that has no projection
instrument). A present clause may be invoked. Invocation commits exactly
one `SemanticEdit` against the named target, which must be the committed
`LocusId`. An absent clause (`needs-source-binding`, `needs-scene`,
`unresolved-pointing`, `structurally-absent`) is not invocable. Escape
cancels an in-flight invoke; source is unchanged.

Disclosure still happens: every clause still speaks verb, target, scope,
and effect before invoke. The difference from Model 1 is that a present
clause is a commit surface, not a caption.

### Routes for the four verbs

| Verb | Engine names it on | Commit route | Explicitly unrouted |
|---|---|---|---|
| `set-gain` | Source | Command Ledger invoke of the present `set-gain` clause, only when here is Source. Arguments (`db`) are part of the invoke, not a Timeline handle. | Timeline, Canvas, Inspector, and any global always-visible verb button. Scene here: clause absent (`needs-source-binding`); unrouted; speak "Point the video clip". No retarget. |
| `set-fade` | Source | Command Ledger invoke of the present `set-fade` clause, only when here is Source. | Same as `set-gain`. Scene here: unrouted; spoken, not retargeted. |
| `split` | Scene | Command Ledger invoke of the present `split` clause, only when here is Scene. The playhead supplies `at` as a session clock argument. Playhead is not here. | Timeline razor, Canvas, Inspector, and any global "Split at Playhead" button. Source here (including video click): clause absent for `split` on this target; unrouted; speak related-scene legality as Navigate. No scene promotion. No silent `target_scene_locus`. |
| `delete` | Scene | Command Ledger invoke of the present `delete` clause, only when here is Scene. | Timeline delete hit, Canvas, Inspector, and any global "Delete Selected Clip" button. Source here: unrouted; spoken as related-scene Navigate, never as an adopted center. |

How the invoke is triggered (palette, key chord, click on the spoken row)
is a membership question for the ledger chrome, not a second model. What is
fixed: the commit home is the ledger/utterance, and the target is the one
`LocusId`.

Trim / Set In / Set Out buttons die with the bank. Existing Timeline
edge-trim may remain a Timeline gesture; this model does not move trim into
the ledger and does not put Set In / Set Out back on a strip. That
membership is unsettled and is not used to pick a winner.

### Session strip

A non-verb session/application strip MAY remain global. Membership is not
named. The ledger is not that strip. If the strip grows `Split`, `Delete`,
`Gain`, or `Fade` as always-visible locus-taking buttons, this model has
been abandoned and the unanimous delete has been reversed.

### What this model refuses

- Rebuilding the deleted bank as "the ledger, but always visible and
  locus-taking without an utterance clause".
- Growing a Timeline gain/fade handle or a Timeline razor/delete hit as the
  commit path for these four. Those would be Model 1.
- Treating the utterance as disclosure-only. If a present clause cannot be
  invoked, this is not Command Ledger.
- Invoking a clause whose target is not here. Related-scene or
  related-source speech is Navigate, not a hidden retarget.
- A per-view command list. One Engine legal set, one utterance.
- Resolving overlap in the ledger. Overlap stays on the touched projection.

### Falsifier

If, after the bank is gone, `set-gain` or `set-fade` is committed by a
Timeline handle, or `split` / `delete` is committed by a Timeline hit, this
is not Command Ledger. If a spoken clause whose target is not the committed
`LocusId` can be invoked, the no-silent-retarget lock has been broken and
this model is void.

---

## Worked cases (same locks, opposite commit homes)

These cases do not pick a winner. They show the cut.

### 1. Video click, then split

Here is the source clip. Engine legal set is `trim`, `set-gain`, `set-fade`.
`split` is legal on the related scene and is not legal here.

- **Local-Projection Commit Homes.** The Timeline source clip has no split
  instrument. The related scene's split instrument is not armed, because
  here is not that scene. Utterance discloses the related-scene legality and
  the missing route. Nothing commits. Pointing the scene span (not a
  promotion of the video click) is a new point; then the scene instrument
  can commit.
- **Command Ledger.** The `split` clause is absent for this target. Invoke
  is refused. Utterance speaks the related scene as Navigate. Pointing the
  scene makes `split` present; invoke on that clause commits.

Neither model splits the source clip. Neither model silently becomes the
scene.

### 2. Scene pointed on Timeline, then set-gain

Here is the scene. Engine legal set is `split`, `delete`, `reorder-scene`.
`set-gain` is legal on the related source and is not legal here.

- **Local-Projection Commit Homes.** No gain instrument on the scene span.
  Utterance discloses `needs-source-binding`. Point the video clip; then the
  source-binding instrument can commit `set-gain`.
- **Command Ledger.** The `set-gain` clause is absent. Invoke is refused.
  Speak "Point the video clip". After that point, the clause is present and
  invocable.

Neither model writes gain onto the scene. Neither model retargets through
`target_source_locus`.

### 3. Overlap on Timeline, then delete

A coordinate names several loci. Candidates appear on the Timeline
projection that was touched. Until a card is picked, pointing is unresolved
and the Engine legal set is empty.

- **Local-Projection Commit Homes.** No scene delete instrument is armed.
  Utterance speaks `unresolved-pointing`. Pick a card on **that** Timeline;
  one `LocusId`. If it is a Scene, the scene instrument may then commit
  `delete`. If it is a Source, `delete` stays unrouted.
- **Command Ledger.** No clause is invocable. Utterance speaks
  `unresolved-pointing`. Pick stays on the touched Timeline. After the pick,
  `delete` is present only when here is Scene; invoke commits only then.

Neither model moves the candidate list to the ledger, the utterance, or
another projection.

### 4. Title on Canvas, then fade

Here is Title. Engine legal set is `title`, `set-position`, `resize-overlay`.
Canvas routes placement only.

- Both models: `set-fade` is structurally absent for this target. Unrouted.
  Spoken. No Title-shaped fade, no Inspector fade, no bank button.

---

## What is not settled (on purpose)

These are left open so the pair can be judged without smuggling a winner:

- Membership of the non-verb session/application strip.
- Whether Timeline edge-trim stays as the only `trim` route, and how Set In /
  Set Out (playhead trim) is replaced, if at all. The required four do not
  include `trim`.
- Invoke chrome for Command Ledger (palette vs spoken-row click vs key). The
  commit home is fixed; the widget is not.
- Exact Timeline instrument shapes for Local-Projection gain/fade/split/
  delete. The commit home is fixed; the pixels are not. No look is claimed.
- Whether a command ledger row after invoke is also the Studio Undo entry
  (volatile, source-backed) or only the pre-invoke utterance. Persistent
  history remains Git in both models.

## What this note does not do

- It does not implement Studio, GPUI, Core, Engine, CLI, or Wasm.
- It does not choose Local-Projection Commit Homes or Command Ledger.
- It does not reopen overlap, video click, per-view selection, Core Freeze,
  or GPUI-in-Core.
- It does not restore the deleted always-visible locus-taking bank.
- It does not add an in-process agent runtime or a project database.
- It does not claim a look. There is no PNG, no `blob/<sha>` image, and no
  mp4.

A later chair may pick, merge, or reject. This pair only forces the commit
home and the utterance role apart.
