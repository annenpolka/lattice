# Lattice repository guidance

## Product invariants

- Magic is allowed. Hidden behavior is not. Every expansion must be explainable via `lattice explain`.
- VEL is a DSL, not a general-purpose language. No `for` / `while` / functions / generics. If you want computation, it belongs in a Wasm component (MoonBit recommended, not required).
- The VEL parser must not know the meaning of `freeze`, `title`, `callout`, or any other stdlib word. It parses generic invocations only.
- `lattice-core` owns semantic primitives: time and TimeMap, normalized space, media locators, placement, properties, provenance, locus, diagnostics, semantic edits, resolve locks, `RenderScene`, and `AudioPlan`. It depends on none of the other Lattice crates.
- `lattice-core` must not depend on GPUI, Wasmtime, FFmpeg, MoonBit, Git, or the VEL parser.
- Wasm components and the temporary in-process builtins behind the same registry produce Core IR fragments. Neither is on the render hot path.
- Compile, Resolve, Evaluate, Render, and Encode are separate phases. Parse/Compile must never call a generated-media provider. An application command may compose Resolve then Render only through the Engine phase boundary and must persist the provider result in the lock; never hide provider I/O inside parse, compile, or evaluate.
- Project state is text-first and Git-friendly. Do not introduce a project database.
- Persistent history belongs to Git. Studio Undo/Redo is a volatile, source-backed working-session history; do not turn it into a second persistent history store.
- `evaluate_at` is the semantic source of truth for preview and export. Both consume a flattened `Timeline`, never VEL, and produce backend-neutral scene/audio plans.
- FFmpeg is a decode, probe, encode, and mux backend, not the semantic model or compositor. Do not put filtergraphs or FFmpeg argv in Core IR or Engine semantics.
- Renderer requests are explicit. `RequireGpuDx12` either selects DX12 or returns a typed error; it never silently falls back to CPU.
- Once an `AudioPlan` contains a window, a missing source, stale generated asset, decode failure, or device failure is observable. Do not convert it to implicit silence.
- External coding agents interact primarily through the CLI. Every subcommand must retain the global `--json` option; preserve exit codes and stderr as well as JSON payloads.
- Do not add an in-process LLM SDK, OpenAI/Anthropic/xAI client, or "agent runtime" to this repo. Agents stay external.
- Do not leak GPUI types into Core. Studio is a client of the engine.
- A locus is the shared "here" across VEL, canvas, timeline, and agent context. Do not invent a per-view selection model. Agent context is `locus + instruction`, not a prompt alone.
- Provenance is always present and must not obstruct. Navigate (`Go to definition` and other projections) is optional, not a gate.

## Crate boundaries

The dependency column below lists Lattice-to-Lattice dependencies; ordinary external libraries are omitted.

| Crate | Lattice dependencies | Owns | Must not know |
|---|---|---|---|
| `lattice-core` | none | semantic IR, time/space, scene/audio evaluation, locus, provenance, edits, locks | other Lattice crates, GPUI, CPAL, FFmpeg, wgpu, Wasmtime, VEL syntax |
| `lattice-vel` | core | lexer and generic invocation AST/parser | stdlib word meaning, Wasm, render/audio backends, GPUI |
| `lattice-wasm` | core | Wasmtime WIT host and lowering registry | VEL parser internals, FFmpeg, wgpu, GPUI |
| `lattice-media` | core | CPU/DX12 compositors, PCM mix, FFmpeg I/O, reusable sample sessions | VEL surface syntax, GPUI, application workflow |
| `lattice-engine` | core, vel, wasm, media | compile/validate/explain/locus/rewrite/resolve/render orchestration | GPUI, CPAL, FFmpeg argv |
| `lattice-cli` | core, engine | CLI/JSON surface and explicit phase composition | duplicated compile, edit, resolve, or render semantics |
| `lattice-studio` | engine | GPUI view/input, volatile session/Undo, viewport/playhead, preview scheduling, Windows/macOS audio-device sync | forked compiler/evaluator/rewriter, a parallel selection model, FFmpeg argv |

## Current Alpha vertical slice

Keep changes inside this implemented product shape unless the task explicitly expands it:

1. Parse the generic VEL DSL; use `examples/gameplay-commentary/main.vel` as the reproducible end-to-end fixture.
2. Lower `freeze` and `title` through the Wasmtime-hosted WIT component in `stdlib/lattice-stdlib.wasm`; lower the remaining stdlib words through the same registry.
3. Emit Core IR, explain events, provenance, and one shared locus across source, Canvas, Timeline, Review, and agents.
4. Resolve generated `speech` and fonts into `lattice.lock.json`. Compile records intent only and never performs provider I/O.
5. Flatten to a `Timeline`, then call Core evaluation to obtain `RenderScene` and `AudioPlan` at time `t`.
6. Preview and export share `SampleSession`: the CPU reference compositor and the explicit Windows DX12 compositor consume the same scene; export and Studio monitoring share the same PCM mix.
7. Studio keeps preview frames in memory, coalesces playback work, and exposes source-backed timeline edits, Canvas move/four-corner resize, Review Apply/Reject, Resolve, renderer selection, and Windows/macOS audio monitoring.

`callout`, `fade`, `gain`, `speech`, and the commentary convention remain in-process host lowerings behind the WIT-compatible registry. Sequence flow is currently Engine-owned ordering/explain logic, not a registry builtin. Replace the remaining registry lowering bodies with components without changing Core types or surface syntax. OTIO, a project database, an automatic renderer fallback, and an in-process agent runtime remain outside this slice.

## Implementation contracts

### Render and audio

- Keep the decoder and renderer warm inside `SampleSession`; compatible timeline/canvas edits should use its typed rebind path. Recreate the session when renderer, media root, output hint, fixture policy, frame rate, or font identity changes.
- CPU is the reference renderer. DX12 owns platform device/pipeline state in `lattice-media`; Core remains backend-neutral. Add CPU/DX12 parity coverage for shared scene semantics.
- Export and Studio audio must both go through `mix_timeline_audio` / `Engine::prepare_audio`. Core owns timing/gain intent, media owns decode/mix, and Studio owns only output-device transport and drift reconciliation.
- The Studio session playhead is the A/V clock. Play waits for required PCM; Pause/Seek/Scrub invalidate stale work and synchronize both sides. CPAL/device types must not escape Studio, and Studio must never monitor audio by decoding an exported MP4.

### Direct manipulation

- GPUI translates pointer input into a gesture lifecycle; it does not rewrite VEL directly.
- Pointer down begins, pointer move updates ephemeral geometry, pointer up commits exactly one `SemanticEdit`/source rewrite/compile/Undo entry, and Escape cancels without changing source.
- Canvas geometry stored in Core/VEL is normalized `position` plus uniform `scale`; GPUI pixels remain view-local. Four-corner resize preserves aspect ratio and fixes the opposite corner.
- Bind rendered overlays to their `TimelineClip.id`/`LocusId`. Do not reverse-match overlays by visible text or source span; duplicate labels are legal.

## Verification

- Change Core time/TimeMap algebra → add a unit test (and a proptest if you touch algebraic laws).
- Change VEL syntax → add a parser test and update golden IR/explain if surface semantics change.
- Change renderer/evaluation semantics → add CPU reference coverage and DX12 parity where the hardware path applies.
- Change `AudioPlan` or mixing semantics → cover both export and Studio preparation; never test missing generated speech as accepted silence.
- Do not hand-wave diagnostics. New magic needs an explain event with origin.

The repository/CI toolchain is Rust 1.97.1. Full local verification also needs FFmpeg/ffprobe, the checked-in fixture font, and Node.js for the pinned Quint 0.32.0 specs:

```powershell
./scripts/check-quint.ps1
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The root workspace does not include the nested `crates/lattice-stdlib-guest` workspace. Its component build is a separate operation; root tests do verify the committed Wasm artifact freshness.

Local Studio process smoke (Windows/macOS desktop; not CI). GPUI windowing on GitHub-hosted runners is not a reliable gate:

```powershell
./scripts/studio-smoke.ps1 -Renderer cpu
./scripts/studio-smoke.ps1 -Renderer gpu-dx12
```

```bash
./scripts/studio-smoke-macos.sh
```

The Windows smoke creates/resolves an A/V fixture when no VEL path is supplied and requires a usable default Windows audio output. The GPU form additionally requires a working DX12 adapter. The macOS smoke is a bounded CPU/GPUI fixture smoke with preview and device audio disabled; separately launch a resolved VEL on a logged-in Mac to exercise FFmpeg and CoreAudio. Use `./scripts/prepare-gameplay-commentary.ps1` when exercising the checked-in Alpha VEL directly; production paths must never synthesize missing user media.

Studio UI behavior belongs in `#[gpui::test]` tests backed by `VisualTestContext` and stable
`debug_selector` names. Resolve selectors to current rendered bounds and dispatch real GPUI input;
do not put absolute screen coordinates in Rust tests. Assert Session/Engine/source state after the
interaction. Keep Computer Use and the process smoke for OS integration, native window/GPU/audio,
DPI, and final visual checks rather than ordinary button, drag, or shortcut correctness.
