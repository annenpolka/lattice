# Lattice repository guidance

## Product invariants

- Magic is allowed. Hidden behavior is not. Every expansion must be explainable via `lattice explain`.
- VEL is a DSL, not a general-purpose language. No `for` / `while` / functions / generics. If you want computation, it belongs in a Wasm component (MoonBit recommended, not required).
- The VEL parser must not know the meaning of `freeze`, `title`, `callout`, or any other stdlib word. It parses generic invocations only.
- `lattice-core` owns semantic primitives (time, space, media locators, placement, TimeMap, provenance, diagnostics). It depends on none of the other Lattice crates.
- `lattice-core` must not depend on GPUI, Wasmtime, FFmpeg, MoonBit, Git, or the VEL parser.
- Wasm (today: in-process builtins behind the same registry) produces Core IR fragments. It is not on the render hot path.
- Compile, Resolve, and Render are separate phases. Do not resolve TTS or network data during parse or compile.
- Project state is text-first and Git-friendly. Do not introduce a project database.
- Persistent history belongs to Git. Session Undo/Redo may exist later as EditPatch; do not invent a parallel history store.
- FFmpeg is a backend, not the semantic model. Do not put filtergraphs in Core IR.
- External coding agents interact primarily through the CLI (`check`, `compile`, `explain`, later `render`). All of these must keep `--json`.
- Do not add an in-process LLM SDK, OpenAI/Anthropic/xAI client, or "agent runtime" to this repo. Agents stay external.
- Do not leak GPUI types into Core. Studio is a client of the engine.

## Crate boundaries

| Crate | May depend on | Must not know |
|---|---|---|
| `lattice-core` | std, serde, thiserror | other Lattice crates, GPUI, FFmpeg, Wasmtime |
| `lattice-vel` | `lattice-core` | Wasm, FFmpeg, GPUI, command meaning |
| `lattice-wasm` | `lattice-core` | VEL parser internals, FFmpeg, GPUI |
| `lattice-engine` | core, vel, wasm | GPUI, FFmpeg CLI flags |
| `lattice-cli` / `lattice-studio` | engine | business logic of their own |
| `lattice-media` | core | VEL surface syntax |
| `lattice-studio` | engine | Core internals beyond the engine API |

## Walking skeleton scope

Keep Milestone 0 small:

1. Parse `examples/gameplay-commentary/main.vel`
2. Lower `freeze` / `title` / commentary convention through the registry
3. Emit IR and explain text
4. Do not start GPUI Timeline, OTIO, TTS providers, or a GPU renderer

The in-process builtins in `lattice-wasm` are a stand-in for WIT components. When you add Wasmtime, replace the builtin bodies, not the Core types.

## Verification

- Change Core time/TimeMap algebra → add a unit test (and a proptest if you touch algebraic laws).
- Change VEL syntax → add a parser test and update golden IR/explain if surface semantics change.
- Do not hand-wave diagnostics. New magic needs an explain event with origin.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
