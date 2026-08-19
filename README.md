# Lattice

Lattice is a text-first video editing system. You describe a cut in **VEL** (a small declarative DSL), compile it to a typed project graph, and render through a backend. Studio is a GPUI IDE over the same graph. Coding agents talk to Lattice through the CLI.

Lattice Alpha is a dogfoodable slice, not a NLE:

```text
.vel → parse → WIT stdlib lowering → Core IR → locus → review → resolve → render plan → FFmpeg
```

## Status

| Layer | Alpha |
|---|---|
| Core IR (time, TimeMap, scene, provenance, locus) | yes |
| VEL parser (generic invocation DSL) | yes |
| Wasmtime WIT stdlib: `freeze`, `title` | yes |
| Host builtins: `callout`, `fade`, `gain`, `speech`, commentary | yes |
| Shared locus (VEL ↔ Core ↔ timeline) | yes |
| Review: propose / inspect / Apply / Reject | yes |
| Resolve + lockable generated media | yes (local tone; TTS when a provider exists) |
| `lattice check` / `compile --emit-ir` / `explain` / `render` | yes |
| Agent JSON: `locus` / `inspect` / `propose` / `apply` / `reject` / `resolve` | yes |
| GPUI Studio (Windows) | vertical slice over the Engine |

Platform: developed and dogfooded on **Windows 11 x64**. Core, VEL, and the compiler stay platform-neutral and run on Linux CI. Other Studio platforms are deferred. WSL is not a supported runtime path.

## Quick start

Requires Rust `1.97.1` (see `rust-toolchain.toml`) and `ffmpeg` / `ffprobe` on PATH for render.

```powershell
cargo test --workspace
cargo run -p lattice-cli -- check examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- compile examples/gameplay-commentary/main.vel --emit-ir
cargo run -p lattice-cli -- explain examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- render examples/gameplay-commentary/main.vel -o preview.mp4
```

Add `--json` for agent-friendly output.

```powershell
cargo run -p lattice-cli -- --json inspect examples/gameplay-commentary/main.vel --locus demo:title:1
cargo run -p lattice-cli -- --json propose examples/gameplay-commentary/main.vel --title-text World
cargo run -p lattice-studio -- examples/gameplay-commentary/main.vel
```

## Layout

```text
crates/lattice-core      semantic IR (no GPUI / FFmpeg / Wasmtime / VEL)
crates/lattice-vel       lexer + generic invocation parser
crates/lattice-wasm      lowering registry (in-process builtins until Wasm)
crates/lattice-engine    compile / validate / explain orchestration
crates/lattice-cli       check / compile / explain / render / locus / inspect / propose / apply / reject / resolve
crates/lattice-media     FFmpeg preview/export adapter
crates/lattice-studio    GPUI Studio (Engine client; `--features window`)
crates/lattice-stdlib-guest  WIT guest for freeze/title (wasm32-wasip2)
stdlib/lattice-stdlib.wasm   vendored stdlib component
docs/                    principles, architecture, glossary, interaction
docs/mockups/studio/     interaction boards (locus, three capabilities)
docs/notes/              design conversation logs
examples/gameplay-commentary
```

## Design notes

The VEL / Lattice constitution was written in the design conversations under [`docs/notes/`](docs/notes/). Start with:

- [docs/principles.md](docs/principles.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/glossary.md](docs/glossary.md)
- [docs/interaction.md](docs/interaction.md) — locus, Manipulate / Navigate / Review
- [AGENTS.md](AGENTS.md) — rules for coding agents working in this repo
