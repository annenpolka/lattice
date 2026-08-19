# Interaction

Studio, the CLI, and external agents are clients of the same Core graph. They do not each invent a selection model. They share a **locus**.

Boards that freeze this model: [`mockups/studio/model.png`](mockups/studio/model.png), [`mockups/studio/scenario.png`](mockups/studio/scenario.png).

## Two principles

- **Provenance is always present. It must not obstruct.** Why a node exists (`Origin`, source span) is visible. Everyday edits do not stop for a review of that fact.
- **A locus survives projections.** The same semantic "here" is what Canvas, VEL, Timeline, and an agent are pointing at. One source definition may project to many rendered instances; the locus is the meaning, not a particular rectangle.

## Locus

A locus is Lattice's **here**: the editing target held across representations.

It is not an "object" in the GUI sense. A `title` written once in VEL may appear as a scene instance, a timeline span, and a canvas rectangle. Those are projections. The locus is the shared pointing.

When the type lands in `lattice-core` (it is not in Milestone 0), it may carry:

```text
Locus
- semantic identity
- source span
- timeline range
- visual target / bounds
- scene / sequence context
- derived-from / provenance
```

GPUI types stay out of it. Studio projects a locus; it does not own it.

This is what an external agent should receive as context. Instruction alone is not enough:

```text
Agent receives: locus(title Hello · main.vel:16) + "これ変えて"
```

The same shape covers Canvas click, Timeline click, VEL cursor, "point the agent here", and Review's affected target.

## Three unordered capabilities

These are abilities over a locus, not a pipeline. There is no required `Navigate → Manipulate → Review`.

| Capability | Meaning | Everyday |
|---|---|---|
| **Manipulate** | Touch what is visible. Select, drag, type. | Select → move → done. |
| **Navigate** | Follow the same semantic target in another representation. Canvas ↔ VEL, timeline ↔ source, proposal ↔ affected locus. `Go to definition` is one Navigate, not the capability itself. | Optional. |
| **Review** | Inspect a proposed change as meaning, picture, and source, then Apply or Reject. The current VEL does not change until Apply. | Agent path. |

Navigate is never a gate. Daily editing may never open VEL. Agent editing may never drag. Writing VEL may never open Review.

## One scenario (not the only path)

Main line:

```text
point at title Hello on the canvas
  → provenance visible (Defined in main.vel:16), not a checkpoint
  → "change this" to an agent, with that locus
  → Review: picture + semantic proposal + VEL diff
  → Apply / Reject
```

Optional branch off the canvas: `Go to definition` into VEL, then back. Not step two of the main line.

Other legal paths over the same locus:

- Manipulate only: select → move → done
- Agent: point → instruction + locus → Review
- VEL: Navigate to definition → edit source → confirm on canvas
