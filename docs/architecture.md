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

Alpha implements Compile, an explicit Resolve (paths, generated media, locks), and Render (timeline flatten + FFmpeg). Compile never calls a generated-media provider.

## Locus

Studio, CLI, and external agents point at the same Core graph. The shared "here" is a **locus**, not a GPUI selection and not a chat utterance.

A locus is a semantic target. Canvas, VEL cursor, timeline range, and agent context are projections of it. Navigate follows those projections; Manipulate edits what is visible; Review inspects a proposed change as meaning, picture, and source. Those three are unordered capabilities, not a pipeline. `Go to definition` is one Navigate, not a required step.

Agent input is `locus + instruction`. Natural language without a locus is not the product contract.

`lattice-core::Locus` is the serializable noun. Engine projects it among source span, Core node id, and timeline range. Studio is an Engine client and must not own a parallel selection model.

## Review

A `SemanticEdit` is named before any rewrite. `EditProposal` carries a description, a VEL diff, `new_source`, and a `base_revision` fingerprint of the source it was built from. Apply writes that source (atomically) and recompiles, and rejects the proposal if the current source no longer matches. Reject leaves current VEL bytes unchanged. CLI: `propose` / `inspect` / `apply` / `reject` / `import` (all `--json`).

## Resolve

Generated locators (`speech`) are intent at Compile. Resolve materializes them into lockable artifacts (`lattice.lock.json`). A second resolve against a valid lock does not call the provider. The default provider is a local tone; a remote TTS may be swapped onto the same path when credentials exist.

## Wasm / WIT

`freeze` and `title` lower through a Wasmtime-hosted `lattice:stdlib` component (`stdlib/lattice-stdlib.wasm`). The component emits TimeMap / placement fragments, never FFmpeg or GPUI. Ambient net/fs/random are not granted. Other stdlib words (`callout`, `fade`, `gain`, `speech`) remain in-process builtins behind the same registry.

See [interaction.md](interaction.md). Boards: [mockups/studio/](mockups/studio/).

## Walking skeleton builtins

These stdlib words are registered in `lattice-wasm`. `freeze` and `title` are hosted on Wasmtime; the rest are in-process builtins behind the same registry:

- `freeze` — TimeMap hold segment (temporal, WIT)
- `title` — generated visual placement (visual, WIT)
- `callout` — second overlay (visual)
- `fade` — opacity envelope on a source's video placement (visual)
- `gain` — audio gain in dB (audio)
- `speech` — generated-media intent; Resolve materializes (audio)
- `flow` — sequence body order is scene order (placement)
- `convention commentary` — default A/V placement, never invents cuts
