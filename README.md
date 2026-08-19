# Lattice

Lattice is a text-first video editing system. You describe a cut in **VEL** (a small declarative DSL), compile it to a typed project graph, and render through a backend. Studio is a GPUI IDE over the same graph. Coding agents talk to Lattice through the CLI.

This repository is a walking skeleton, not a NLE. Milestone 0 is:

```text
.vel → parse → invocation AST → lowering → Core IR → validate → explain
```

FFmpeg render, Wasm stdlib execution, and the GPUI Studio shell come next. They are stubbed, not wired.

## Status

| Layer | v0 |
|---|---|
| Core IR (time, TimeMap, scene, provenance) | yes |
| VEL parser (generic invocation DSL) | yes |
| Host builtins: `freeze`, `title`, commentary convention | yes |
| `lattice check` / `compile --emit-ir` / `explain` | yes |
| Wasmtime / WIT components | contract only |
| FFmpeg render | not yet |
| GPUI Studio | not yet |

Platform: developed and dogfooded on **Windows 11 x64**. Core, VEL, and the compiler stay platform-neutral and run on Linux CI. Other Studio platforms are deferred. WSL is not a supported runtime path.

## Quick start

Requires Rust `1.97.1` (see `rust-toolchain.toml`).

```powershell
cargo test --workspace
cargo run -p lattice-cli -- check examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- compile examples/gameplay-commentary/main.vel --emit-ir
cargo run -p lattice-cli -- explain examples/gameplay-commentary/main.vel
```

Add `--json` for agent-friendly output.

## Layout

```text
crates/lattice-core      semantic IR (no GPUI / FFmpeg / Wasmtime / VEL)
crates/lattice-vel       lexer + generic invocation parser
crates/lattice-wasm      lowering registry (in-process builtins until Wasm)
crates/lattice-engine    compile / validate / explain orchestration
crates/lattice-cli       check / compile / explain / render
crates/lattice-media     FFmpeg backend (stub)
crates/lattice-studio    GPUI Studio (stub)
docs/                    principles, architecture, glossary
docs/notes/              design conversation logs
examples/gameplay-commentary
```

## Design notes

The VEL / Lattice constitution was written in the design conversations under [`docs/notes/`](docs/notes/). Start with:

- [docs/principles.md](docs/principles.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/glossary.md](docs/glossary.md)
- [AGENTS.md](AGENTS.md) — rules for coding agents working in this repo
