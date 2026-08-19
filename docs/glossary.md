# Glossary

Use these names. Do not invent aliases for the same concept.

| Term | Meaning |
|---|---|
| **Lattice** | The ecosystem: language, core, engine, CLI, Studio, stdlib. |
| **VEL** | Surface DSL. Declarative edit intent. Not a general-purpose language. |
| **Project** | A named edit, stored as text (`*.vel`) plus optional lock files. |
| **Workspace** | An ordinary directory / git repo Lattice understands. |
| **Sequence** | Ordered list of scenes (v0: flow). |
| **Scene** | A bounded editorial unit with sources and placements. |
| **Source** | A named binding of a media slice plus a TimeMap. |
| **Placement** | A concrete A/V object on a scene's local timeline. |
| **Composition** | Spatial arrangement of visuals (later: canvas). |
| **TimeMap** | Mapping from local time to content time via rate segments. Freeze is rate 0. |
| **Locus** | The semantic "here": the editing target shared across VEL, canvas, timeline, and agent context. Not a rendered instance. One source definition may project to many instances; the locus is what is being pointed at. |
| **Manipulate** | Directly change what is visible (select, drag). Everyday editing may stop here. |
| **Navigate** | Follow the same locus in another representation (Canvas ↔ VEL, timeline ↔ source, proposal ↔ affected target). `Go to definition` is one Navigate, not the whole capability. |
| **Review** | Inspect a proposed change as meaning, picture, and source, then Apply or Reject. |
| **Convention** | Named defaults that fill placement, not editorial intent. Must not invent cuts. |
| **Compile** | VEL → Core IR. Deterministic. |
| **Resolve** | Materialize TTS, analysis, and other non-deterministic inputs. |
| **Render** | Deterministic output from a resolved IR. |
| **Core IR** | Typed semantic graph. JSON-dumpable. |
| **Render Plan** | Backend-facing operations (later). Not FFmpeg argv in Core. |
| **Invocation** | Generic VEL command: name, args, modifiers, optional block. |
| **Provenance** | Why a Core node exists (source span + origin). |
| **Sugar** | Deterministic magic (e.g. `title`, `scene over speech`). |
| **Anchor** | A named temporal reference (later). |
