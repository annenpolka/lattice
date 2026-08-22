# Studio interaction gen2: Weighted Subject vs Projection-Local Verbs

Date: 2026-08-22

Status: discussion input; two second-generation models, not a selected design

Scope: interaction models only. No Studio UI, GPUI, or HTML sketch.

Source of truth: the chair note at
[`docs/notes/2026-08-22-studio-interaction-chair-note.md`](https://github.com/annenpolka/lattice/blob/3fc72b7eaaf3c036ec3900510edf97ebdb330c1d/docs/notes/2026-08-22-studio-interaction-chair-note.md)
(PR #12 head `3fc72b7`). This reply does not restate Candidates A–F and does not treat the
mutation ledger as a menu to assemble. Each model is a policy for what licenses a verb.

## Frame

The chair note extracts independently variable claims. The useful split for a second generation is
not “more Inspector” versus “more Timeline.” It is:

- **What licenses a legal verb?**
- **What happens when one pixel names several loci?**
- **How does absence speak without becoming another selection?**

Both models keep the fixed boundary:

- One shared locus is semantic “here” across VEL, Canvas, Timeline, Review, and agent context.
  Focus, hover, playhead, and ephemeral gesture are not a second selection.
- Studio is an Engine client. Commits are `SemanticEdit`s and source-backed rewrites.
- No GPUI in Core. FFmpeg is a backend. Project state stays text-first and Git-friendly.
- No in-process agent runtime or LLM SDK. Agents receive `locus + instruction`.
- Magic stays explainable. Domain nouns are not warped for widget convenience.
- Existing five-pane chrome is not sacred.

Both models refuse the already-dropped mutations: hide VEL; readout-only Timeline; color or
desaturation as a standalone semantic change; new Placement/TimeMap panes or `LocusKind`s; a second
playhead locus; inventing a time-scoped audio `SemanticEdit`; a new handle-local trim callback as
the proof of interactivity; automatic seek on locus change.

They refuse toolbar fallback that mutates a target other than the displayed locus without moving
that locus. They refuse title-shaped editing for scene, sequence, source, or media loci.

The models are not two skins of one policy. They disagree on the license for a verb, on overlap,
and on whether chrome weight is constitutive.

| Question | Weighted Subject | Projection-Local Verbs |
|---|---|---|
| What licenses a verb? | The named subject plus Engine legality. Chrome roles make the naming legible. | The touched projection plus Engine legality. A projection that cannot commit the verb does not offer it. |
| Where do verbs appear? | One Engine-derived subject surface, labelled by scope, independent of which projection named the locus. | On the projection being touched. Timeline does not offer `SetPosition`. Canvas does not offer `ReorderScene`. |
| Overlap / M10 | Disclosed ranking plus cycle. No candidate list as the ordinary pointing UI. | Candidate list on the touched projection. Specificity is not allowed to manufacture the subject. |
| Playhead coupling | Never automatic. Exclusive roles keep both visible. | Never automatic. Playhead is not a verb subject. |
| Hierarchy | Constitutive: pointing, weight, and disclosure are one function. | Not constitutive: weight cannot grant a verb the touched projection does not own. |
| Inspector / toolbar as verb home | Replaced by the subject surface. | Deleted. No global verb home. |
| Simultaneous highlight | Not required. Compact projection count on the subject. | Not required. Quiet identity on other projections; Navigate to reach them. |

Evidence that the current product fails both licenses is already committed: title-shaped Inspector
on a scene locus, Gain fallback that can leave the displayed locus unmoved, Video click collapsing
to scene, Audio scrub re-pointing “here” through specificity, and Canvas emptiness that does not
name its cause. Those shots diagnose; they do not choose a skin.

## Shared edit-scope vocabulary

M3 and M5 require named scopes. Both models use this Engine-facing vocabulary and no UI-only
categories:

| Scope label | Maps to | Does not map to |
|---|---|---|
| **definition** | `SemanticEdit::Title { text }` on the title definition; other definition-owned fields when Engine exposes them | “whatever the Inspector currently shows” |
| **this placement** | `SetPosition`, `ResizeOverlay`, `Title { at, duration, opacity }`, `Callout { at, duration }`, `SetFade` | every rendered rectangle that shares a label |
| **this clip / time range** | `Trim`, `Split` | a TimeMap rate change |
| **scene order** | `ReorderScene` | dragging a title overlay |
| **source** | `SetGain` (no time scope today) | a time-scoped audio envelope that does not exist |
| **TimeMap** | rate / freeze of an existing source TimeMap, when Engine exposes a legal edit | a synthetic `freeze:source:clip` tree row or a new `Core Freeze` kind |

`Delete` takes the scope of the named locus: a title placement is not a scene, and a scene is not
the sequence. `SetGain` remains source-scoped until Core grows a time-scoped audio verb; neither
model invents one for Review or for an Audio rail.

A locus surviving several projections does not make every legal edit hit every instance. That is
chair contradiction 4, and both models keep it.

---

## Model 1 — Weighted Subject

Interaction plus hierarchy as one system: pointing, chrome weight, and disclosure are the same
function. They assign exclusive roles so that “what I named,” “when I am looking,” and “what I can
commit” cannot occupy one signal.

### What “here” is

“Here” is one session `LocusId`. It is the **weighted subject**: the only semantic target that may
carry persistent subject weight.

A view may have keyboard focus, hover, an insertion marker, or an in-flight gesture. Those are
ephemeral roles. They never receive subject weight and they never become a second locus. Agent
context is this same `LocusId` plus an instruction.

“Here” is not “the pane that is highlighted.” Shared identity does not require five equal
highlights. The subject is named once; other projections are discoverable from a compact
projection inventory on that subject (M1), including when only one surface is on screen.

### Playhead versus locus

Playhead and locus are independent session facts (M2 taken).

- The playhead is transient time. Scrub moves only the playhead. It does not rewrite VEL, does not
  push Undo, and does not call `point_from_timeline_time`.
- Pointing a projected item names the locus and leaves the playhead where it is.
- Canvas remains `evaluate_at(playhead)`. A locus whose range does not contain the playhead is a
  live subject with no current picture, not a broken projection.

The mix is what makes the two facts co-visible. Locus gets persistent subject weight. Playhead
gets a unique transient time signal. Those roles are exclusive: the current teal collision among
pane label, playhead, Text clip, and tree selection is not a palette note; it is the model failing
to assign roles.

No user action silently couples them. The only legal coupling is an explicit subject-surface verb,
“seek playhead to this locus range.” It is temporary session state, not persisted project state,
and not applied on point.

### How legal verbs appear

Pointing names the subject. It does not open a verb kit on the pixel that was hit.

Legal verbs appear on **one Engine-derived subject surface**, labelled with the scope table above.
The surface is reconstructed from Engine/Core for the current locus: identity, provenance when
present, timeline range when present, visual fields when present, and the legal `SemanticEdit`
set. Kind-switching Inspector forms are rejected. Title fallback is rejected.

Because the license is the subject, a title locus named from the Timeline still offers
definition-text and this-placement position verbs on the subject surface. The user does not have
to find the “right pane” to be allowed to edit. That is the opposite of Projection-Local Verbs.

Chrome weight is how those verbs stay honest:

- Definition-scope commits sit with definition identity, not with a title-shaped text box borrowed
  for a scene.
- This-placement commits sit with the visual/time fields that placement actually has.
- Unsupported loci show why no mutation is legal, at the same disclosure weight as an available
  verb. Emptiness is a first-class role, not a missing form.
- Transport, renderer/audio status, and destructive actions are separate roles. They do not share
  the subject’s signal and they do not invent a second target.

The current toolbar/pane stack is accidental hierarchy. This model replaces it with exclusive
roles. It does not preserve the five-pane composition in order to recolor it (M8 is taken as
constitutive, not as a skin harness).

Review is not the everyday path (M7 taken, gating rejected). Direct manipulation still commits on
pointer-up: one `SemanticEdit`, one rewrite, one compile, one Undo. When an `EditProposal` already
exists from `apply_committed`, Review may show current picture/time, semantic effect and scope,
and source diff. Picture is evidence, not a checkpoint.

### Overlap / M10 policy

Weighted Subject **rejects M10’s candidate list as ordinary pointing UI**.

`Locus::specificity` is already a kind-ranked order in Core (title/callout/speech > source >
placement > scene > sequence/media), and `locus_at_timeline` / `locus_at_source` already collapse
with `max_by_key`. That is manufactured singularity, not a discovered unique noun. The model
accepts manufactured singularity as **session policy**, provided the manufacture is disclosed.

Ordinary pointing therefore:

1. Commits exactly one shared `LocusId` using the existing ranking.
2. Discloses the reason on the subject surface: which candidates existed at that time or offset,
   which one won, and on what key (kind rank, then shorter span).
3. Offers a cycle-to-next-candidate verb that rebinds the same shared locus. Cycle is not
   per-view selection; every projection receives the new `LocusId`.

A persistent candidate list would be a second weighted subject. It would compete with the exclusive
role grammar and turn overlap into a picker pane. That is a different model (it is Model 2).

Evidence that ranking is policy, not domain necessity: the same timeline time can contain Title,
Source, and Scene at once; Core still stores one `LocusId` in Session after collapse. The invariant
requires clients to share the result. It does not require the result to be silent.

### How absence is disclosed

Absence is a **weight drop plus a named cause**, using the M9 taxonomy. Black Canvas is never a
single state.

Distinct disclosures, not one empty pane:

- locus outside playhead — subject remains; range is stated; explicit seek is offered
- no visual projection — Media / Sequence / some Source loci may legally lack picture
- renderer initializing
- layout failure
- preview disabled
- media unavailable (typed, observable; not implicit success)
- compile diagnostics present but not displayed

The cause is reconstructed from Engine/session facts already available. If Engine cannot tell
those states apart, the model does not invent a UI-only cause, and it does not auto-seek to paper
over the ambiguity.

Overlay affordances that bind to `TimelineClip.id` / `LocusId` may remain when geometry is known
and frame pixels are temporarily missing. They do not remain when the locus has no visual
projection.

VEL may be off-screen under width pressure. Navigate stays optional. Source and provenance remain
reachable from the subject inventory. Hiding the only mutable source string as a default is
refused.

Freeze has no tree weight unless the row resolves through an existing Core locus with source span,
timeline range, provenance, and explain output (M6). The synthetic `freeze:source:clip` identity
is not a subject. Freeze remains a TimeMap rate-0 explanation on the source.

### M1–M10

| Probe | Stance | Why |
|---|---|---|
| M1 | **Take** | The subject surface *is* the inventory. Facet disclosure beats `LocusKind` forms. No new panes or kinds. Canvas stays `evaluate_at`, not a locus field. |
| M2 | **Take** | Two named facts, exclusive roles, no hidden seek, no second locus. |
| M3 | **Take** | Legal verbs, scope-labelled, fail-closed. This is the verb half of the mix. |
| M4 | **Take** | Exclusive hit roles are the pointing half of the same function as exclusive chrome roles. Ruler/rail background scrubs; projected item points; handle or named affordance begins a semantic gesture. Rest-state pixels must predict the role. Existing rail-level capture is enough; no handle-local callback. |
| M5 | **Take, definition-emphasis** | The heavy subject is the definition. Verified instances are lighter echoes with a count, not a second selection. Simultaneous highlight is not required to teach 1→many. Fixtures must be semantically identified, not inferred from a rail crop filename. |
| M6 | **Take, prefer unexplained-row removal** | Keep a row only if it is a real locus. Do not add `Core Freeze`. |
| M7 | **Take, no gating** | Reuse `EditProposal`. Do not manufacture Gain/TimeMap rows without a legal edit of that scope. |
| M8 | **Take as constitutive** | Upgraded from harness. Role exclusivity *is* the hierarchy half. The probe still selects no palette and no pane arrangement; it forbids one signal from meaning locus, playhead, and Text clip at once. Width captures remain constraints, not a 1024→800 story. |
| M9 | **Take** | Taxonomy feeds the weight-drop copy. Empty Canvas is not one problem. |
| M10 | **Reject as ordinary UI** | Ranking plus disclose plus cycle. A candidate list is a second subject. |

Delete first among the live probes: **M10 as a list**. It violates exclusive subject weight and
treats manufactured singularity as a picker rather than as disclosed policy. The observation it
would “fix” (silent collapse) is already fixed by disclosure and cycle.

### One falsifier

If, after exclusive roles and a scope-labelled subject surface are in place, participants still
cannot predict before pointer-down whether that pixel will seek, name the locus, or rewrite — or
if they treat the subject surface as a second selection and edit a target other than the displayed
`LocusId` — Weighted Subject is false.

A weaker falsifier, sufficient on its own: if scope labels cannot map one-to-one onto existing
`SemanticEdit`s without inventing UI-only categories, the verb half has failed M3’s own falsifier.

### Implementability notes

No Core type changes. Session already stores one `LocusId`. `Locus::specificity` and the current
`max_by_key` lookups can stay as the ranking function if their result is shown and cyclable.
Legal verbs are an Engine query over the current locus, not a GPUI-owned form model. Review keeps
the `EditProposal` already built during commit. Gesture lifecycle is unchanged: pointer-up
commits; Escape cancels; scrub has no Undo. Role exclusivity is a Studio presentation contract
over existing session facts (locus, playhead, insertion, transport, renderer status). It does not
leak GPUI types into Core.

---

## Model 2 — Projection-Local Verbs

Locus plus direct manipulation: the shared locus is the subject, but **legal verbs live on the
projection you are touching**. Chrome weight cannot grant a verb that this projection cannot
commit.

### What “here” is

“Here” is one session `LocusId`. It is the **verb subject**, not a weighted chrome object.

A definition may project to source span, timeline clip, canvas overlay, Review proposal, and agent
context. Those are instruments, not extra selections. Touching an instrument names the shared
locus *and* restricts the legal verb set to what that instrument can commit.

Focus, hover, and playhead still do not own a locus. Agent context remains `locus + instruction`.
The instruction is about the named locus; the verbs an agent proposes still have scopes, and
Review is the projection that applies or rejects them.

“Here” is not “whatever rectangle is teal.” Shared identity is a relation among projections. It
does not require them to highlight together, and it does not require them to offer the same verbs.

### Playhead versus locus

Playhead and locus stay independent (M2 taken). The playhead is not a verb subject.

- Scrub is a time gesture on ruler or rail background. It moves the playhead only.
- It does not rewrite VEL, does not Undo, and does not re-point the locus.
- Pointing a projected item names the locus. It does not seek.

Canvas is still `evaluate_at(playhead)`, which is why a title active at 1s–4s can be the locus
while the picture at 0s is empty. That is not a broken Canvas projection. The Canvas projection
discloses the mismatch and may offer an explicit local verb, “seek playhead to this locus range.”
That verb lives on the projection that lacks the picture, is temporary, and is never applied by
pointing alone.

If a user wants the picture and the subject to coincide, they use that verb. Coupling is never
persisted and never inferred from emptiness.

### How legal verbs appear

The license is **touched projection ∩ Engine-legal edits for the named locus**.

| Touched projection | Verbs that may appear | Verbs that must not appear here |
|---|---|---|
| Canvas overlay bound to `TimelineClip.id` / `LocusId` | `SetPosition`, `ResizeOverlay`, placement opacity / fade when Engine-legal | `ReorderScene`, `SetGain`, TimeMap, definition-text unless the overlay is itself the text-bearing visual and Engine treats that gesture as `Title { text }` |
| Timeline clip body | point (name locus), `ReorderScene` on a scene body | `SetPosition` |
| Timeline trim / timing affordance | `Trim`, `Title { at, duration }`, `Callout { at, duration }`, `Split` | definition text, Gain |
| Timeline ruler or rail background | scrub playhead only | point, trim, reorder |
| VEL / source span | definition-owned fields, Navigate to provenance | spatial placement as if pixels were in Core |
| Review (`EditProposal`) | Apply / Reject of that proposal | beginning a new direct-manipulation gesture that bypasses the proposal’s `locus_id` |
| Subject inventory (Navigate, not a pane requirement) | discover absent projections; optional “go to source / range / picture” | a second locus |

There is no Inspector-as-verb-home and no toolbar-as-second-target. If the touched projection has
no legal verb, the projection says so. A Gain control that targets a source while a title locus
stays displayed is a category error: chrome granted a verb the touched projection did not own.

Direct-manipulation lifecycle is the existing contract, not a new mode:

```text
pointer down  → begin a named role (scrub | point | mutate)
pointer move  → ephemeral geometry only
pointer up    → at most one SemanticEdit → one rewrite → one compile → one Undo
Escape        → cancel; VEL unchanged
```

Scrub is not mutate. Point is not mutate. A rest-state pixel that looks like a handle must be a
mutate affordance; a rail that looks empty must scrub. No region silently does two of those.

Review remains ungated (M7). Everyday canvas move and timeline trim commit without it. When a
proposal is kept, Review is another projection of that `EditProposal`, showing picture, scope, and
diff. It is not a required checkpoint and not a title-only text pane.

### Overlap / M10 policy

Projection-Local Verbs **takes M10**.

If the pixel that was touched maps to several loci, pointing failed to name a unique verb subject.
Silent `max_by_key(specificity)` is rejected as ordinary policy. The touched projection presents
the candidate list with each candidate’s reason and scope. Choosing one commits the one shared
session `LocusId`. After that choice, every projection — including agent context — receives the
same id. The list is not project state and not per-view selection.

Specificity is current interaction policy, not domain ordering. Evidence:

- Core already admits several loci at one time (`contains_timeline_time`).
- Collapse is `max_by_key((specificity, shorter span))`, which is an editor convenience.
- Video click today can drop clip identity and leave a scene locus, which then offers the wrong
  local verbs (or a title fallback). That is the failure M10 is for.
- Kind rank would be domain ordering only if overlapping Title and Scene were the same noun. They
  are not: their legal edits and scopes differ.

If candidates are later shown to be semantically equivalent for every legal edit, M10’s own
falsifier applies and this model must drop the list. Until then, manufacturing a subject before
the instrument has spoken is hidden behavior.

Cycle-without-list is not enough here. Cycle still picks on the user’s behalf after a failed
point. This model requires the failed point to remain visible.

### How absence is disclosed

The missing projection speaks. Absence is “this instrument has no verbs / no picture / no span
because [M9 cause],” not a global weight drop on leftover chrome.

- locus outside playhead — Canvas (or the time projection) states the active range and offers
  local seek; other projections of the same locus remain usable
- no visual projection — Canvas says the locus kind has no picture; it does not pretend to be
  initializing
- renderer initializing, layout failure, preview disabled, media unavailable, compile
  diagnostics — each is a distinct local disclosure. Overlay chrome bound to a known
  `LocusId` may survive missing pixels; it must not survive a locus with no visual fields
- VEL off-screen — Navigate from the touched projection to source remains available; the mutable
  source is never inexplicable
- freeze — not a verb surface. TimeMap rate 0 is explained on the source projection. A
  synthetic tree row that does not resolve is not selectable (M6)

Toolbar and status are not the absence channel. Mixing renderer failure into the command strip
conceals the instrument that failed.

### M1–M10

| Probe | Stance | Why |
|---|---|---|
| M1 | **Take** | Inventory is the set of available projections and the verbs each can commit. Absence is explained. Canvas is still `evaluate_at`. |
| M2 | **Take** | Playhead is time, not a subject. Explicit seek is a local verb on the projection that lacks the picture. |
| M3 | **Take, relocated** | Scope labels sit on the local affordance, not on a global strip. Fail-closed: no title fallback, no hidden second target. |
| M4 | **Take as the spine** | Hit-region split *is* verb appearance. Three non-overlapping contracts, rest-state predictability, existing rail capture. |
| M5 | **Take, instance-emphasis** | The instance you touched is the verb subject. Definition relationship is Navigate, not automatic multi-instance mutate. Highlighting every echo would overstate edit scope. Use semantically verified fixtures. |
| M6 | **Take** | No verb surface, no `Core Freeze`. Reconstruct from source TimeMap or drop the row. |
| M7 | **Take, no gating** | Review is a proposal projection. Breadth follows real legal edits. |
| M8 | **Reject as constitutive** | Keep it as a measurement harness. Role and width observations constrain visibility; they do not license verbs. Treating weight as the model would move legality off the touched projection and collapse this into Weighted Subject. |
| M9 | **Take** | Each projection discloses its own cause. |
| M10 | **Take** | Failed point stays visible. One committed `LocusId` afterward. |

Delete first among the live probes: **M8 as a design mutation**. Using chrome weight to decide
what is legal would violate “verbs live on the projection you are touching” and would hide the
toolbar-fallback class of bugs behind a prettier strip. The observation it encodes (accidental
chroma and width loss) remains a harness, not a model.

### One falsifier

If participants who touch a timeline span then change definition text, or who touch one placement
then believe every instance will move — or if local verb labels require UI-only semantic
categories that do not map onto existing `SemanticEdit`s — Projection-Local Verbs is false.

A second, independent falsifier: if presenting overlap candidates on the touched projection adds
no scope predictability and materially obstructs ordinary pointing (M10’s own falsifier), the
overlap half is false and this model may not keep the list.

### Implementability notes

No new `LocusKind`s and no GPUI types in Core. Filter Engine-legal edits by the projection that
began the gesture; that filter is a Studio policy over existing edits, not a new Core noun.
`locus_at_timeline` / `locus_at_source` grow a candidate-returning path for the pointing surface
only; Session still stores one `LocusId` after choice. Overlay and clip hit testing already bind
to stable ids; do not reverse-match by visible text. Gesture lifecycle, Escape, and “scrub has no
Undo” stay as they are. `SetGain` offered only from a source projection, with the missing time
scope disclosed — do not mint an audio trim to make the Audio rail feel complete.

---

## Chair hooks, answered per model

These are independent answers, not a merged vote.

### 1. Delete first

- **Weighted Subject:** delete M10-as-list. It creates a second subject and fights exclusive
  weight. Silent collapse is already answered by disclose-plus-cycle.
- **Projection-Local Verbs:** delete M8-as-design. Weight is not a verb license. Keep M8 as
  measurement only.

### 2. Predicted scopes (M3 / M5)

For a verified title definition with several placements, before any apply:

- change the words — **definition** (`Title { text }`)
- drag the canvas rectangle — **this placement** (`SetPosition` / `ResizeOverlay`)
- drag a timeline edge on that placement — **this clip / time range** or **this placement**
  timing (`Trim` or `Title { at, duration }`, whichever Engine actually commits for that handle)
- drag a scene body — **scene order** (`ReorderScene`)
- Gain — **source**, and only if a source projection is the instrument; no time scope
- freeze — **TimeMap** of the source, never a freeze locus

Weighted Subject shows all of those on the subject surface once the definition is named, each
labelled, with instances as lighter echoes. Projection-Local Verbs shows only the row that the
touched projection can commit; the others are Navigate, not implied selection.

### 3. Locus / playhead coupling (M2)

Neither model couples on point, scrub, or locus change. The only coupling is an explicit, temporary
“seek playhead to this locus range.” Weighted Subject places that verb on the subject surface.
Projection-Local Verbs places it on the projection that currently has no picture. Neither persists
the coupling in VEL or the lock.

### 4. Pixel regions (M4)

Both models use the same exclusive map. No region may silently do two jobs.

- Timeline ruler, unmarked rail background, Audio rail background: **scrub**
- Timeline clip / scene body interior: **point** (Projection-Local: scene body may also begin
  `ReorderScene` only through a distinct rest-state affordance or a named drag threshold that is
  not scrub and not trim)
- Timeline in/out handle or equivalently disclosed timing affordance: **mutate** (`Trim` /
  placement timing)
- Canvas empty field: **not point at a locus**; may scrub only if this model later grants Canvas
  a time role — neither model does that today
- Canvas overlay body: **mutate** position, after point-or-prior locus bind to that overlay’s id
- Canvas overlay corner: **mutate** `ResizeOverlay`
- VEL text: **source edit** of the named locus, through the existing source handler
- Toolbar transport: **playhead / preview**, never locus
- Toolbar status: **not a hit target for edits**

Weighted Subject adds no extra hit roles; it assigns exclusive chrome to the same map.
Projection-Local Verbs treats the map as the entire verb grammar.

### 5. Freeze (M6)

Neither model makes freeze directly selectable unless the row is already a real Core locus.
Preferred reconstruction: TimeMap rate 0 on the existing source, with explain output. Do not add
`Core Freeze`. Weighted Subject gives the synthetic row no weight. Projection-Local Verbs gives it
no verb surface.

### 6. Specificity (M10)

- **Weighted Subject:** specificity is interaction policy that happens to be implemented on
  `Locus` in Core. That does not make it domain necessity. It is usable if disclosed and cyclable.
  Evidence it is policy: several loci satisfy `contains_timeline_time`; one id is stored afterward.
- **Projection-Local Verbs:** the same evidence, opposite product rule. Policy that manufactures
  the subject before the instrument has spoken is hidden behavior. Show candidates on the touched
  projection.

### 7. Falsifiers

Stated under each model. Neither falsifier is “the panes look wrong.”

---

## Why these are not Candidates A–F

A–F were first-generation lenses (structure, semantic, hierarchy, affordance, first principles,
interaction). This pair is not those lenses renamed.

- Weighted Subject is not Candidate C. C critiqued teal and wrapping while leaving the five-pane
  stack implicit. This model makes role exclusivity the interaction function itself, relocates
  verbs onto a subject surface, and refuses M10-as-list.
- Weighted Subject is not Candidate A. A asked for actions-by-locus and cheap forks. This model
  specifies how pointing, weight, and disclosure are one function, and it keeps manufactured
  singularity as disclosed policy.
- Projection-Local Verbs is not Candidate E. E inverted 1→many and said verbs come from the locus.
  This model says verbs come from **locus ∩ touched projection**, so the same locus offers
  different instruments, and it takes M10 as ordinary pointing.
- Projection-Local Verbs is not Candidate F. F asked for explicit hit contracts. This model uses
  those contracts as the *only* verb grammar and rejects chrome weight as a license.

If a later packet can implement both models by changing only color, pane count, or Inspector
layout, the pair has failed its own contrast and should be thrown out.
