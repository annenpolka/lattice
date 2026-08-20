# warframe-cut

Playable Alpha sample. `warframe-cut.mp4` sits next to `main.vel` and is gitignored (`*.mp4`). `examples/gameplay-commentary` stays the golden VEL.

## What's in `main.vel`

| Surface | Where |
|---|---|
| `convention commentary` | default A/V placement |
| four `scene`s in `sequence` | Studio body-drag reorders these |
| `game[start..end]` | trim / Set In / Set Out / Split |
| `fade` / `gain` | envelope + level (buttons or VEL) |
| `title` + `opacity` | overlay track / canvas; two titles in `hook` |
| `callout` | overlay track drag |
| `freeze` | TimeMap hold on `hold` |
| `speech` | Compile records intent; `resolve` / `render` materializes a local tone |
| `scene hold over line` | commentary ducks game audio (−15 dB) |

## CLI

```powershell
cargo run -p lattice-cli -- check examples/warframe-cut/main.vel
cargo run -p lattice-cli -- compile examples/warframe-cut/main.vel --emit-ir
cargo run -p lattice-cli -- explain examples/warframe-cut/main.vel
cargo run -p lattice-cli -- resolve examples/warframe-cut/main.vel
cargo run -p lattice-cli -- render examples/warframe-cut/main.vel -o examples/warframe-cut/preview.mp4
```

`render` resolves generated speech if needed (writes `.lattice/` and `lattice.lock.json`, both gitignored). Add `--json` for agents.

```powershell
cargo run -p lattice-cli -- --json inspect examples/warframe-cut/main.vel --locus hook:title:1
cargo run -p lattice-cli -- --json propose examples/warframe-cut/main.vel --locus hook:title:1 --title-text Hello
```

## Studio

```powershell
./scripts/studio-debug.ps1 examples/warframe-cut/main.vel
```

Try:

- scrub the ruler (playhead is not a locus)
- trim a clip edge, undo once
- drag a video body to reorder hook / fight / hold / outro
- drag a title or callout on the text track (`hook` has two titles)
- zoom / scroll / Alt to disable snap
- Split at Playhead, Set In / Set Out, Gain, Fade, Delete
- point the same locus on canvas overlay, timeline, and inspector
- Play / Pause (preview is generation-ordered, not a live decoder)
