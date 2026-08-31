# Architecture

```text
VEL source
   ↓  lattice-vel (generic invocation AST)
Wasmtime-hosted WIT stdlib registry
   ↓  lattice-wasm
Core IR                         lattice-core
   ↓
Validator
   ↓
Explain / Timeline flatten
   ↓
Resolve (lockable assets, fonts)
   ↓
evaluate(t) → RenderScene + AudioPlan     lattice-core
   ↓
Lattice compositor + PCM mixer            lattice-media
   ↓  raw RGBA frames + PCM
Encoder (mux)                             lattice-media
   ↓
FFmpeg as codec/I/O backend only
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
5. Wasm components speak Core fragments, not VEL.
6. Render backends do not know surface VEL.
7. Studio and CLI own no compile semantics; they call the engine.
8. OS-specific code stays out of Core.

## Phases

These names match the Quint specs in `spec/*.qnt` (`build_protocol`, `resolution`, `rendering`) and the Rust entry points.

| Phase | Quint | Rust | Input | Output | May be non-deterministic? |
|---|---|---|---|---|---|
| Compile | `compile` | `Engine::compile` | VEL | Core IR + diagnostics + explain | no |
| Resolve | `resolveOpen` / `resolveLocked` | `Engine::resolve` | Core IR + media/TTS/fonts | `lattice.lock.json` | yes, then lock |
| Evaluate | `evaluateScene` | `evaluate_at` | resolved timeline + `t` | `RenderScene` + `AudioPlan` | no, given locks |
| Render | `startRender` / `finishFrames` | `sample_frame` / compositor | `RenderScene` | raw RGBA + mixed PCM | no, given locks |
| Encode | `startEncode` / `finishEncode` | `Encoder` | frames + PCM | container (mp4, …) | no (codec choice is not scene state) |

**Render ≠ Encode.** Preview and export call the same `evaluate_at` / `sample_frame` path. FFmpeg may decode, probe, encode, and mux; it is not the visual or audio compositor. A valid lock then Render/Encode must not increment provider calls. Stale or missing assets (including fonts) block render start. Renderer/backend failure must not mutate VEL source or the lock.

Compile never calls a generated-media provider. Preview and export share `evaluate_at` / `sample_frame`. The compositor is Lattice-owned (CPU reference path, optional wgpu offscreen). `FFmpeg` is decode/encode/mux only.

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

`freeze`, `title`, `caption`, `callout`, `fade`, `gain`, `speech`, and sequence `gap` lower through a Wasmtime-hosted `lattice:stdlib` component (`stdlib/lattice-stdlib.wasm`). The component emits TimeMap, placement, sequence-offset, envelope, gain, and generated-media intent fragments, never FFmpeg or GPUI. Ambient net/fs/random are not granted. Host code converts the generic invocation view and assembles the returned fragments into Core IR; provider I/O remains in Resolve.

See [interaction.md](interaction.md). Boards: [mockups/studio/](mockups/studio/).

## Walking skeleton builtins

These stdlib words are registered in `lattice-wasm` and hosted on Wasmtime:

- `freeze` — TimeMap hold segment (temporal, WIT)
- `title` — generated visual placement (visual, WIT)
- `caption` — timed cue on the title overlay (visual, WIT)
- `callout` — second overlay (visual, WIT)
- `fade` — opacity envelope on a source's video placement (visual, WIT)
- `gain` — audio gain in dB (audio, WIT)
- `speech` — generated-media intent; Resolve materializes (audio, WIT)
- `gap` — explicit empty time before the next scene, lowered to a Core sequence offset (placement, WIT)
- `flow` — sequence body order is scene order (placement)
- `convention commentary` — default A/V placement, never invents cuts
