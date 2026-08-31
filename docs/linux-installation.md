# Linux installation dependencies and licenses

This page inventories the Linux dependencies that the current source tree and
agent smoke path use. It does not declare product Linux support. The supported
distributions, X11/Wayland envelope, audio strategy, hardware matrix, and
packaging format remain separate product decisions.

## Build dependencies

The repository pins Rust 1.97.1. The root workspace needs a C/C++ toolchain for
GPUI's native dependencies and the following development packages on the
Ubuntu CI/agent path:

| Capability | Ubuntu package or tool | Why Lattice needs it |
|---|---|---|
| Rust build | Rust 1.97.1 with `rustfmt` and `clippy` | Workspace compiler and CI gates |
| Native link | `gcc`, `g++` | GPUI and C++ runtime linkage |
| X11 client | `libxcb1-dev` | GPUI Linux link dependency |
| Keyboard input | `libxkbcommon-dev`, `libxkbcommon-x11-dev` | GPUI X11 keyboard handling |
| Specs | Node.js 22 | Pinned Quint 0.32.0 checks |

The repository's Ubuntu-oriented agent environment installs the build and smoke
packages with:

```bash
sudo apt-get update
sudo apt-get install -y \
  gcc g++ libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev \
  mesa-vulkan-drivers ffmpeg python3 xdotool x11-utils xvfb
```

This package line documents the demonstrated Ubuntu agent path. It is not a
compatibility promise for Ubuntu or other distributions. When `cc` is Clang on
the current cloud image, `scripts/studio-linux-smoke.sh` sets
`RUSTFLAGS=-C linker=gcc`; this workaround is intentionally not stored in Cargo
configuration.

## Studio window and Vulkan

The CPU compositor does not remove the Studio window's GPU requirement. GPUI
still initializes its Blade/wgpu window renderer, so a Vulkan loader, a usable
ICD, a device, and matching window-system integration must all be present.

The existing X11 agent smoke accepts Mesa lavapipe from
`mesa-vulkan-drivers`. It checks for an ICD manifest under
`/usr/share/vulkan/icd.d/` and does not set `VK_ICD_FILENAMES`. Hardware Vulkan
drivers may provide a different ICD. A successful headless build or
`vulkaninfo` result alone does not prove that GPUI can create and present a
window.

Only the X11 agent path is currently demonstrated. Native Wayland, X11 under a
Wayland session, hardware adapters, and software Vulkan as a product fallback
are not declared supported here. See [studio-linux-smoke.md](studio-linux-smoke.md)
for the bounded agent harness.

Linux audio monitoring is also not supplied by the current Studio build:
`cpal` is enabled only on Windows and macOS. Installing ALSA, PulseAudio, or
PipeWire development packages does not enable a Linux audio path by itself.

## FFmpeg and ffprobe contract

`ffmpeg` and `ffprobe` are runtime executables, not Rust link dependencies.
Lattice resolves them from `PATH`, or from `LATTICE_FFMPEG` and
`LATTICE_FFPROBE` when those variables are set.

The current media path requires:

- `ffprobe` JSON probing for container duration and video/audio stream metadata;
- input demuxers and decoders for every media format used by a project;
- FFmpeg `rawvideo` output in RGBA for frame decode;
- FFmpeg `s16le` output for PCM extraction;
- the `libx264` video encoder and `aac` audio encoder for MP4 export; and
- `yuv420p` output support.

Check an installation before using it:

```bash
ffmpeg -hide_banner -version
ffprobe -hide_banner -version
ffmpeg -hide_banner -encoders | grep -E 'libx264|aac'
```

CI currently uses BtbN's `ffmpeg-n9.0-latest-linux64-gpl-9.0` archive. That is
a reproducible CI choice, not yet the minimum supported product version. The
Linux CI workspace tests exercise probing, raw-frame decode, preview, PCM audio
preparation, and MP4 export against that build.

If a tool cannot start, the typed error names the resolved executable, the
operation, and the corresponding override (`LATTICE_FFMPEG` or
`LATTICE_FFPROBE`). A tool that starts but cannot demux, decode, or encode
reports the operation, exit status, and captured stderr. File-system failures
are reported separately as media I/O errors instead of being mislabeled as a
missing FFmpeg executable. Contract tests run missing-tool and invalid-container
cases in isolated processes so environment overrides cannot leak into parallel
media tests.

## Fonts

Text rendering needs a readable TTF or OTF file with the required glyphs.
Resolution prefers a valid font recorded in `lattice.lock.json`, then an
explicit render override, the project's `fonts/` or `assets/` directory, the
repository fixture, and finally a small list of known system font paths.
`LATTICE_FONT` can point at a specific TTF for development and CI.

The checked-in `fixtures/fonts/MPLUS1p-Regular.ttf` is deterministic test and CI
data. It is M PLUS 1p Regular, copyright the M+ Project Authors, distributed
under the SIL Open Font License 1.1; its notice is stored in
`fixtures/fonts/OFL.txt`. A product package that redistributes this font must
keep that notice and the font under the OFL. A project's own fonts retain their
own licenses, and the project author is responsible for embedding and
redistribution rights.

## Redistribution inventory

Lattice source is MIT licensed; retain the repository `LICENSE` notice when
redistributing Lattice. Rust dependencies keep their own licenses and must be
audited from the locked dependency graph for a release artifact.

FFmpeg licensing depends on the exact binary configuration and enabled codecs.
The CI archive is explicitly GPL-labeled and uses `libx264`; do not assume it is
suitable for redistribution merely because CI downloads it. A Linux package
must either depend on a distribution-provided FFmpeg or ship a reviewed build,
and must preserve all license notices and source/code-offer obligations that
apply to that build and its codec libraries.

Mesa/Vulkan, X11, xkbcommon, system audio libraries, and system fonts are
currently expected from the operating system rather than bundled by Lattice.
If packaging later vendors any of them, their licenses and notices become part
of the release review.
