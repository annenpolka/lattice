# Does Studio need a global verb button row?

Date: 2026-08-22
Reference base: `85b589ec260554f851c214731e607c7727c7cae8`

This is a docs-only domain-semantics review. It asks whether a global
top-of-window home for verbs is the right object at all. It does not propose
polish for the current row and does not specify Studio implementation.

The concrete object under review is visible in this immutable capture of the
whole Studio client:
[current top-of-window row](https://github.com/annenpolka/lattice/blob/77894c052f8d5b63783df875a9de6880dad4c991/docs/artifacts/2026-08-22-studio-toolbar-semantics.png?raw=true).

## Answer

No. A global verb button row is the wrong semantic object.

The spine has three authorities with deliberately different jobs:

- **Legality:** Engine names the exact legal edits for the one committed
  `LocusId`, including verb, target, scope, and effect.
- **Routing:** a projection says what a gesture on that projection can commit.
- **Utterance:** one account of here discloses legality and says where a legal
  edit commits when the present projection does not route it.

A global row can only fit this model by becoming another commit projection.
Once it does, it stops being neutral chrome: it needs target acquisition rules
for verbs whose natural target is currently represented in Timeline, Canvas,
or Inspector. That is precisely where a convenient global action starts to
look like global legality or permission to retarget.

The alternative is not a better global row. It is **no global verb home**.

## Why the global row sends the wrong message

### Presence looks like legality

A persistent edit button remains visible across Title, Source, Scene, and
unresolved pointing. Its stable presence visually claims broader availability
than Engine's locus-dependent legal set. Disabled states do not repair the
model: they still make the row the apparent authority and reduce typed absence
to button state.

For example, with Title `demo:title:1` as here in the capture, Engine names
`title`, `set-position`, and `resize-overlay`. A persistent split, delete,
trim, gain, or fade control is not evidence that any of those verbs is legal
for Title.

### A global click has no intrinsic route

Timeline, Canvas, and Inspector each have a semantic relationship to what they
show. A click in a global row has no such relationship. To commit, it must
either:

1. use the one here and refuse when the verb is illegal; or
2. search another represented object and silently substitute it.

Only the first is lawful, but repeated lawful refusal exposes the deeper
problem: the action is located away from the projection that can commit it.
The second is forbidden silent retargeting, including source-to-scene
promotion or scene-to-source lookup.

### The row collapses unlike command classes

The observed row mixes semantic edits with transport, viewport, renderer,
persistence, history, Resolve, status, and context-copy commands. Those
commands do not share Engine legality:

| Command class | Domain authority |
|---|---|
| Semantic edit | Engine legal set for the one here; committed by a projection |
| Transport / viewport | Session playhead or projection-local view state |
| Save / Undo / Redo | Source-backed working-session state |
| Resolve | Explicit application phase and lock persistence |
| Renderer / audio | Runtime request and observable status |
| Copy locus | Projection of the same locus into agent context |

Putting them in one row encourages “everything clickable is a verb” and then
forces non-edit commands into a license vocabulary they do not belong to.

## Replacement semantic shape

There are only three commit surfaces:

- **Timeline** commits time-, clip-, and sequence-shaped edits against the
  represented locus.
- **Canvas** commits normalized placement and aspect-preserving resize against
  the represented overlay locus.
- **Inspector** commits definition/property edits for the one here.

This is a statement about routing ownership, not a new selection model. Every
surface reads the same here. Touching a projection does not create another
locus or legal set.

The global affordance, if any, is the spoken/legal account rather than a bank
of verb buttons:

```text
one LocusId
    ↓
Engine legal edits: verb + target + scope + effect
    ↓
one utterance: where each edit commits, or why no route exists
```

The account may say that a legal edit commits on Timeline, Canvas, or
Inspector. It must not perform the edit itself, choose a hidden target, or
turn an unavailable route into implied absence. A projection exposes a verb
only where that projection can actually commit it.

## Consequences

- Toolbar is removed from the semantic routing vocabulary; it is not a fourth
  commit surface.
- A legal verb without a current route remains legal and is spoken as such.
  The UI must not invent a global fallback route.
- A projection-local verb that is illegal for here is refused with the typed
  reason. The projection does not search for a more convenient locus.
- Transport, viewport, project, phase, and runtime controls may have shell
  affordances, but they are not a global **verb** home and do not participate
  in `legal_edits_for`.
- The one utterance can expose the complete legal set without rendering every
  legal edit as a persistent button.

## Locks preserved

This conclusion does not reopen overlap handling: overlap stays local to the
touched projection until one candidate becomes the shared `LocusId`. Video
click still points the source clip, never promotes to Scene. There is no
cross-surface modal, silent retarget, per-view selection, Core Freeze, or GPUI
type in Core.

## Phase III vote addendum — domain semantics

This is a lens vote, not a shipping-winner selection.

- **DELETE** the fixed, always-visible bank of locus-taking `SemanticEdit`
  buttons. Its standing invitations imply availability independently of here,
  and a global click has no intrinsic committing projection.
- **KEEP** a global **session strip** for transport, viewport, project,
  persistence/history, Resolve, renderer/audio, status, and context-copy
  controls, grouped by the authority that owns each command.
- **STEAL** the useful disclosure into a named replacement object:
  **Route Ledger + Projection Commit Affordance**. The one utterance is the
  ledger; Timeline, Canvas, and Inspector alone expose affordances they can
  commit.

The replacement object votes against the six tests as follows:

| Test | Vote |
|---|---|
| 1. Standing invitation for a locus-taking edit? | **PASS — no.** An edit affordance exists only on its committing projection for the one here. |
| 2. Full disclosure before commit? | **PASS — required.** Verb, target, scope, effect, concrete or gesture-bound parameter, and committing projection are disclosed before commit. |
| 3. Engine only legality authority? | **PASS.** The ledger projects Engine's legal set; projections add routes, never legality. |
| 4. One here, fail-closed, no search/promotion? | **PASS.** An illegal or unresolved edit refuses against the same here; it never searches or promotes a target. |
| 5. Every legal edit routed or spoken unrouted? | **PASS.** The ledger names Timeline, Canvas, or Inspector, otherwise explicitly says `unrouted`. |
| 6. Non-verb globals grouped by actual authority? | **PASS.** The session strip groups session, projection-view, phase, runtime, and context commands without calling them Engine-licensed verbs. |

The unchanged locks remain constraints on this vote, not tradeable properties
of the replacement object.
