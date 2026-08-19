# Lattice principles

- Magic is allowed, hidden behavior is not.
- Every magic expansion must be explainable.
- VEL is a DSL, not a general-purpose language.
- Extension execution boundary is Wasm Component / WIT.
- MoonBit is recommended, not required. Lattice depends on WIT, not on MoonBit.
- Rust Core owns semantic primitives (nouns). Wasm owns verbs.
- Wasm produces plans / IR; it is not in the render hot path.
- Compile, Resolve, and Render are separate phases.
- Project state is text-first and Git-friendly.
- Persistent history belongs to Git.
- FFmpeg is a backend, not the semantic model.
- External coding agents interact primarily through the CLI.
- GPUI types never leak into Core.
- Provenance is always present. It must not obstruct the edit.
- A locus survives projections (VEL, canvas, timeline, agent). Do not invent a per-view selection model.
