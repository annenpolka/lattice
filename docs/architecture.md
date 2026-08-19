# Architecture

```text
VEL source
   ↓  lattice-vel (generic invocation AST)
Wasm lowering registry          (today: in-process builtins)
   ↓  lattice-wasm
Core IR                         lattice-core
   ↓
Validator
   ↓
Explain / Timeline flatten
   ↓
Render Plan
   ↓
FFmpeg                          lattice-media
```

CLI (`lattice-cli`) and Studio (`lattice-studio`) are clients of `lattice-engine`. They must not fork business logic.

```text
          Applications
 ┌────────────┴────────────┐
 │                         │
lattice-studio         lattice-cli
   GPUI                    CLI
 │                         │
 └────────────┬────────────┘
              │
         lattice-engine
              │
   ┌──────────┼──────────┐
   │          │          │
lattice-vel  lattice-wasm  lattice-media
   │          │          │
   └──────────┼──────────┘
              │
         lattice-core
```

## Dependency rules

1. `lattice-core` does not depend on any other Lattice crate.
2. `lattice-core` does not depend on GPUI / Wasmtime / FFmpeg.
3. `lattice-vel` does not know Wasm or FFmpeg.
4. `lattice-wasm` does not parse VEL.
5. Wasm components (and today's builtins) speak Core fragments, not VEL.
6. Render backends do not know surface VEL.
7. Studio and CLI own no compile semantics; they call the engine.
8. OS-specific code stays out of Core.

## Phases

| Phase | Input | Output | May be non-deterministic? |
|---|---|---|---|
| Compile | VEL | Core IR + diagnostics + explain | no |
| Resolve | Core IR + media/TTS/analysis | lockable artifacts | yes, then lock |
| Render | resolved IR | frames / file | no, given locks |

v0 implements Compile and a minimal Render (timeline flatten + FFmpeg preview). Resolve stays a named hole so it is not folded into parse.

## Walking skeleton builtins

These stdlib words are registered in `lattice-wasm` and will move to WIT components:

- `freeze` — TimeMap hold segment (temporal)
- `title` — generated visual placement (visual)
- `flow` — sequence body order is scene order (placement)
- `convention commentary` — default A/V placement, never invents cuts
