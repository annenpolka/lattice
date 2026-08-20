# gameplay-commentary

Intentionally edited short used as the Alpha product fixture.

The clip is a media slice plus a small coherent vocabulary:

- `freeze` (TimeMap hold)
- `fade` (opacity envelope on the video placement)
- `title` with `opacity`
- `callout`
- `gain`
- `speech` (generated media; Compile records intent, Resolve materializes)
- commentary convention (default A/V placement)
- sequence flow

## Prepare the local media

`main.vel` references `capture.mp4` in this directory. Generate the explicit
21-second audio+video fixture from the repository root:

```powershell
./scripts/prepare-gameplay-commentary.ps1
```

The script requires `ffmpeg` and `ffprobe` on `PATH`, overwrites the ignored
local `capture.mp4`, and verifies its duration, frame size, frame rate, and A/V
streams. It does not install a production fallback: render still reports an
error when referenced user media is missing.

The complete local walking slice can then be exercised explicitly:

```powershell
cargo run -p lattice-cli -- --json check examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- --json resolve examples/gameplay-commentary/main.vel
cargo run -p lattice-cli -- --json render examples/gameplay-commentary/main.vel -o examples/gameplay-commentary/preview.mp4
```
