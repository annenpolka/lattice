# Lattice

Lattice is a text-first video editing system. A cut is described in **VEL**, a small declarative DSL, compiled into a typed Core graph, and rendered by Lattice-owned visual and audio paths. Studio is a GPUI editor over the same Engine API, while coding agents use the CLI and shared locus model.

Lattice Alpha is a dogfoodable vertical slice, not a general-purpose NLE:

```text
VEL → generic parse → WIT/registry lowering → Core IR + locus
                                             ├─ optional Review → Apply / Reject
                                             ├─ Resolve → lockable assets
                                             └─ flatten → Timeline
                                                            ↓
                           Timeline + lock + t → evaluate(t) → RenderScene + AudioPlan
                                      ┌─────────────────┴─────────────────┐
                           FFmpeg-decode frames                 FFmpeg-decode audio
                                      ↓                                   ↓
                          CPU or DX12 compositor                      PCM mixer
                                      └────────── RGBA + PCM ────────────┘
                                                            ↓
                                              FFmpeg encode / mux → MP4
```

FFmpeg handles media I/O and codecs. It is not the scene or audio semantic model, and Lattice does not build Core behavior out of filtergraphs.

## Current status

| Area | Alpha implementation |
|---|---|
| VEL | Generic invocation parser; no stdlib meaning in the parser |
| Stdlib lowering | `freeze`, `title`, `caption`, `callout`, `fade`, `gain`, `speech`, and sequence `gap` through a Wasmtime-hosted WIT component behind one registry |
| Core | Time/TimeMap, explicit sequence offsets, normalized space, properties, `RenderScene`, `AudioPlan`, provenance, diagnostics, locus |
| Editing | Source-backed semantic edits, volatile Studio Undo/Redo, stale-proposal protection |
| Review and agents | Locus projection, inspect, propose, Apply/Reject, JSON CLI surface |
| Resolve | Lockable file/font/generated assets; deterministic LocalTone speech provider |
| Visual render | Shared `SampleSession`, CPU reference compositor, explicit Windows DX12 compositor |
| Audio render | Shared AudioPlan decode/mix for export and Studio; gain, commentary duck, holds, generated speech |
| Studio preview | Source-FPS sampling, warm renderer/decoder, in-memory frames, bounded latest-wins playback |
| Studio interaction | Editable/navigable VEL projection; Timeline scrub/trim/reorder/zoom/snap; Canvas move and aspect-preserving four-corner resize |
| Studio audio | Windows/macOS default-output monitoring (WASAPI/CoreAudio), playhead synchronization, drift reporting/correction |
| Export | Lattice RGBA + PCM rendered to MP4 through FFmpeg encode/mux |

The repository and CI pin Rust **1.97.1**; Cargo's declared MSRV is 1.97. CI runs the workspace's headless checks on Ubuntu, Windows, and macOS. Native Studio development/dogfood paths are maintained for Windows 11 x64 and macOS on Apple Silicon. CPU is the default renderer on both. DX12 remains Windows-only: an explicit `gpu-dx12` request on macOS returns a typed unavailable-platform failure and never falls back to CPU. Audio monitoring requires a usable default output device and uses CoreAudio on macOS. This macOS support is for source builds and local dogfooding; the repository does not yet provide a signed or notarized `.app` distribution. WSL is not supported.

## Reproducible quick start

Install Rust 1.97.1, then put `ffmpeg` plus `ffprobe` on `PATH`. FFmpeg/ffprobe are needed for fixture generation, media probe/import, preview, audio preparation, and export; `check`, `compile`, and `explain` alone do not need media codecs.

The commands below assume PowerShell 7 (`pwsh`), which is also used by the Windows detached Studio launcher and fixture-preparation script. On macOS, install the Xcode SDK/developer tools and FFmpeg. A Homebrew rustup setup is:

```bash
brew install rustup ffmpeg
export PATH="$(brew --prefix rustup)/bin:$PATH"
```

Keep that rustup path ahead of Homebrew's standalone Rust binaries (including `cargo-clippy`) so the repository selects its pinned compiler and components. Install `pwsh` as well when running the checked-in `.ps1` helpers. Studio enables GPUI's runtime shaders, so a separate Metal compiler/Metal Toolchain download is not required, but the Xcode SDK itself is still required to build the native application.

The checked-in Alpha VEL refers to a gitignored local video. Generate the deterministic 21-second, 320×180, 10 fps A/V fixture first:

```powershell
./scripts/prepare-gameplay-commentary.ps1
```

Then exercise the phases explicitly:

```powershell
cargo run -p lattice-cli -- --json check examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- --json compile examples/gameplay-commentary/main.vel --emit-ir
cargo run -p lattice-cli -- --json explain examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- --json resolve examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- --json render examples/gameplay-commentary/main.vel -o examples/gameplay-commentary/preview.mp4 --renderer cpu
```

Export dimensions and frame rate are explicit when needed; the backward-compatible default remains 320×180 at 10 fps:

```powershell
cargo run -p lattice-cli -- --json render examples/gameplay-commentary/main.vel -o examples/gameplay-commentary/preview-1080p.mp4 --renderer cpu --width 1920 --height 1080 --fps 30
```

`--fps` accepts an integer or a rational such as `30000/1001`. Width and height must be positive even values for the yuv420p encoder path. The JSON/text report records the effective width, height, rate, sample rate, and channel count.

`resolve` writes `lattice.lock.json`, generated media under `.lattice/`, and may copy a face under the project's `fonts/` directory. The lock, generated media, rendered MP4s, and copied project fonts are ignored. The licensed fixture under `fixtures/fonts/` stays in the repository. The Alpha provider produces a deterministic tone for `speech`; a production TTS provider is not included. CLI `render` also composes Resolve automatically when generated media is present, but the explicit step above makes the phase visible and prepares the same lock for Studio. Resolve reports missing user media as a warning and never substitutes a test source; Render and preview treat it as an error.

On Windows with a working DX12 adapter, the same export can require the GPU backend:

```powershell
cargo run -p lattice-cli -- --json render examples/gameplay-commentary/main.vel -o examples/gameplay-commentary/preview-gpu.mp4 --renderer gpu-dx12
```

When more than one DX12 adapter is present, `LATTICE_DX12_ADAPTER` selects by a case-insensitive name substring. With no filter, Lattice requests the high-performance adapter:

```powershell
$env:LATTICE_DX12_ADAPTER = "NVIDIA"
cargo run -p lattice-cli -- --json render examples/gameplay-commentary/main.vel -o examples/gameplay-commentary/preview-gpu.mp4 --renderer gpu-dx12
Remove-Item Env:LATTICE_DX12_ADAPTER
```

Renderer selection has no Auto mode. The CLI and Studio default to CPU; a `gpu-dx12` request either activates DX12 or returns a typed failure without falling back.

To create a VEL project around your own media without copying the source file:

```powershell
cargo run -p lattice-cli -- import C:\path\to\gameplay.mp4 -o C:\path\to\my-cut
# Equivalent directory-first form:
cargo run -p lattice-cli -- new C:\path\to\my-cut --media C:\path\to\gameplay.mp4
```

## Studio

### Windows

Resolve generated media first, then use the detached launcher so a closed agent pipe cannot terminate the GPUI process:

```powershell
$env:LATTICE_STUDIO_RENDERER = "cpu"
./scripts/studio-debug.ps1 examples/gameplay-commentary/main.vel
```

Set the renderer to `gpu-dx12` to dogfood DX12. The launcher builds Studio, stops existing `lattice-studio` processes, starts a visible detached window, waits briefly, and prints the durable log at `%LOCALAPPDATA%\lattice\studio.log`. `LATTICE_STUDIO_LOG` overrides that path. `-NoPreview` disables live video-frame extraction only; project probing and AudioPlan preparation may still use FFmpeg.

### macOS

Resolve the project first, then launch Studio directly from the repository. The process remains attached to the terminal, which is useful for development logs:

```bash
export LATTICE_STUDIO_RENDERER=cpu
cargo run -p lattice-studio -- examples/gameplay-commentary/main.vel
```

CPU is the macOS renderer. The Open Video button uses the native file picker, and audio monitoring sends the same prepared PCM mix used by export to the default CoreAudio output. Set `LATTICE_STUDIO_AUDIO_MONITOR=0` only when deliberately isolating the GPUI window from device I/O. Requesting `gpu-dx12` on macOS reports the explicit typed failure described above.

Studio currently provides:

- One locus across VEL, Canvas, Timeline, Inspector, Review, and copied agent context. The VEL pane is a real editor: line clicks project source offsets into the shared locus, Go to definition focuses/selects/scrolls to the Core span, and valid edits recompile immediately without letting invalid drafts corrupt the compiled session or Undo history.
- In-memory video frames at the probed source frame rate, with one active job plus one latest pending request and stale-result rejection.
- Windows/macOS AudioPlan monitoring from the same PCM mix as export. Video waits until required PCM/device state is ready, and audio errors block A/V play instead of becoming silence.
- Play, Pause, Seek, Scrub, timeline zoom/scroll/snap, clip trim, scene reorder, and on-target gain / fade / split / delete handles. The session strip keeps non-verb controls only.
- Direct title/callout movement and four-corner uniform resize. Pointer movement is ephemeral; mouse-up writes normalized `position`/`scale` through one semantic source patch and one Undo entry, while Escape cancels.
- Review Apply/Reject, direct Inspector Apply, Save, Undo/Redo, Resolve, CPU/DX12 switching, and MP4 export. The Studio export is written beside the VEL as `studio-preview.mp4`.

The preview is a bounded sample-at-time pipeline rather than a continuous streaming decoder. Audio preparation runs on a worker; Studio MP4 export is currently synchronous and can block the UI while encoding.

Linux is not a Studio dogfood or product target. Ubuntu agents (including Cursor Cloud) can use the UI-only smoke path in [docs/studio-linux-smoke.md](docs/studio-linux-smoke.md) to build, launch, screenshot, and click/drag a `--ui-fixture` window without preview or audio-device I/O. That path does not change the Windows 11 x64 or macOS dogfood commands above.

The current Linux native-library, FFmpeg codec, font, and redistribution
inventory is documented in
[docs/linux-installation.md](docs/linux-installation.md). It records the source
and agent requirements without declaring product Linux support.

## CLI for agents

The `lattice` executable exposes `check`, `fmt`, `compile`, `explain`, `render`/`preview`, `locus`, `inspect`, `propose`, `apply`, `reject`, `resolve`, `import`, and `new`. The global `--json` flag is available on every subcommand.

```powershell
cargo run -p lattice-cli -- --json inspect examples/gameplay-commentary/main.vel --locus demo:title:1

$proposal = Join-Path $env:TEMP "lattice-title-proposal.json"
cargo run -q -p lattice-cli -- --json propose examples/gameplay-commentary/main.vel --locus demo:title:1 --title-text "Agent cut" | Set-Content -LiteralPath $proposal -Encoding utf8
cargo run -p lattice-cli -- --json reject examples/gameplay-commentary/main.vel --proposal $proposal
# Use `apply` instead of `reject` to atomically write the proposed VEL source.

# Canonicalize generic VEL syntax, or only check it in CI.
cargo run -p lattice-cli -- fmt examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- --json fmt examples/gameplay-commentary/main.vel --check
```

`fmt` validates the file before writing, preserves comments and string lexemes, and
formats generic invocations without knowing stdlib word meanings. `fmt --check`
never writes and exits 1 when the file would change.

`propose --edit` covers the complete Engine `SemanticEdit` set: `title`, `callout`,
`trim`, `split`, `delete`, `set-gain`, `set-fade`, `reorder-scene`, `set-position`,
and `resize-overlay`. Non-title/callout edits require `--locus`. Canvas position
flags use `0..=100` percent and overlay scale uses `25..=200` percent; the CLI
converts them to Core normalized space before calling `Engine::propose`.

```powershell
cargo run -q -p lattice-cli -- --json propose main.vel --edit trim --locus source:fight --trim-in 11s --trim-out 18s
cargo run -q -p lattice-cli -- --json propose main.vel --edit resize-overlay --locus demo:title:1 --position-x 12.5 --position-y 75 --scale 125
```

JSON covers successful results, diagnostics, proposal workflows, and renderer initialization/render failures. A successful renderer report includes the selected adapter when applicable. A failed explicit GPU request returns `ok=false` with requested/active renderer, phase/kind/stage, and reason, then exits 2 without falling back. Callers must still check the process exit code and stderr: generic top-level filesystem and similar runtime failures may still use plain stderr.

## Verification

The CI-equivalent local gate is:

```powershell
./scripts/check-quint.ps1
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The Quint script uses the CI-pinned 0.32.0 release and requires Node.js. The root workspace does not include the nested `crates/lattice-stdlib-guest` workspace; root tests do check that the committed stdlib Wasm matches its source stamp.

Native Studio process smoke is a local desktop gate, not a GitHub-hosted CI gate:

```powershell
./scripts/studio-smoke.ps1 -Renderer cpu
./scripts/studio-smoke.ps1 -Renderer gpu-dx12
./scripts/studio-smoke.ps1 -Renderer gpu-dx12 -Adapter "NVIDIA"
# Bounded-memory DX12 soak; writes per-sample WS/private/VRAM telemetry CSV.
./scripts/studio-smoke.ps1 -Renderer gpu-dx12 -SoakMinutes 30
# Release 1080p CPU/GPU export comparison with ffprobe verification.
./scripts/renderer-benchmark.ps1 -Adapter "NVIDIA" -Width 1920 -Height 1080 -Fps 30
```

On macOS, run the deterministic CPU/GPUI process smoke:

```bash
./scripts/studio-smoke-macos.sh
```

The macOS smoke builds Studio, launches the `timeline-basic` UI fixture with preview and device audio disabled, checks bounded startup/paint/state logs, and exits automatically. Use the direct VEL launch in the Studio section for the separate FFmpeg/CoreAudio integration check on a logged-in Mac with a default output device.

On Windows, with no VEL argument the short smoke creates and resolves a temporary A/V project. It verifies window startup, AudioPlan PCM readiness before A/V play, audio-stream start on the shared playhead, explicit renderer selection, at least three distinct in-memory preview times, no hot-path PNG cache or GPU runtime recreation, timed shutdown, and absence of panic/native abort. Soak mode instead creates a timeline longer than the requested run, explicitly disables the AudioPlan monitor, requires preview timestamps to keep advancing, records process working set/private bytes/dedicated GPU memory after a warm-up, and fails on configured growth limits. The short A/V smoke requires a usable default audio device; the GPU forms require DX12. Passing a VEL path may write its adjacent lock and generated artifacts.

See [docs/renderer-dogfood.md](docs/renderer-dogfood.md) for the debug/release commands, 30-minute thresholds, and vendor evidence matrix.

Ubuntu agent verification of the GPUI window (not CI, not product Linux support):

```bash
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture timeline-basic
DISPLAY=:1 ./scripts/studio-linux-smoke.sh --fixture drag-valid
```

The Linux script forces `LATTICE_STUDIO_PREVIEW=0` and `LATTICE_STUDIO_AUDIO_MONITOR=0`, captures the identified Studio window (not the whole DISPLAY), writes `semantic_state` begin/update/commit lines, and is documented in [docs/studio-linux-smoke.md](docs/studio-linux-smoke.md). `mesa-vulkan-drivers` is enough for lavapipe; the script does not set `VK_ICD_FILENAMES` and does not gate on `vulkaninfo`. When `cc` is clang it sets `RUSTFLAGS=-C linker=gcc` (not Cargo.toml). Missing `timeline-pointer-commit` fails. Xvfb is `--allow-xvfb` only. Windows `studio-smoke.ps1` / `studio-debug.ps1` and the direct macOS launch still take a single VEL path.

## Repository layout

```text
crates/lattice-core          backend-neutral semantic IR and evaluation
crates/lattice-vel           lexer + generic invocation parser
crates/lattice-wasm          Wasmtime WIT host + lowering registry
crates/lattice-media         CPU/DX12 compositor, PCM mix, FFmpeg I/O
crates/lattice-engine        compile / explain / locus / edit / resolve / render orchestration
crates/lattice-cli           CLI and JSON surface
crates/lattice-studio        GPUI Studio, session transport, Windows/macOS audio output
crates/lattice-stdlib-guest  separate wasm32-wasip2 WIT guest workspace
stdlib/lattice-stdlib.wasm   vendored stdlib component
fixtures/fonts               licensed M PLUS 1p fixture (OFL)
fixtures/studio-ui           deterministic agent UI fixtures (no generated media)
examples/gameplay-commentary reproducible Alpha VEL (generated MP4 is ignored)
examples/warframe-cut        optional local dogfood cut; supply the ignored source MP4
spec                         Quint build/resolve/review/rendering models
docs                         principles, architecture, glossary, interaction, mockups, notes
```

## Design notes

The VEL/Lattice constitution lives in the design conversations under [`docs/notes/`](docs/notes/). Start with:

- [docs/principles.md](docs/principles.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/glossary.md](docs/glossary.md)
- [docs/interaction.md](docs/interaction.md) — locus, Manipulate / Navigate / Review, and UI test layers
- [AGENTS.md](AGENTS.md) — repository rules for coding agents
