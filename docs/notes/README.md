# Design notes

These files are the design conversations that produced Lattice. They are source material, not spec. When they disagree with `docs/principles.md`, `docs/architecture.md`, or `AGENTS.md`, the latter win.

| File | Date | What it is |
|---|---|---|
| [2026-08-12-ai-driven-video-editor.md](2026-08-12-ai-driven-video-editor.md) | 2026-08-12 | Earlier product sketch: script-canonical, utterance-first editor (the TakeGraph line of thought). |
| [2026-08-18-video-editor-cost-and-vel-design.md](2026-08-18-video-editor-cost-and-vel-design.md) | 2026-08-18–19 | Costing, VEL language design, Lattice constitution, crate layout, and the initialization plan this repo follows. |
| [2026-08-22-studio-interaction-control.md](2026-08-22-studio-interaction-control.md) | 2026-08-22 | Phase IV CONTROL model: honesty and contract cleanup on current Studio chrome (locus ⊥ playhead, Inspector verbs, freeze row dropped, Review reuses `EditProposal`). Proposal, not spec. |

Do not "implement the whole chat log." Milestone 0 is the walking skeleton described at the end of the 08-18 log: parse a small `.vel`, lower through a registry, emit IR, explain magic.

Studio interaction (locus, Manipulate / Navigate / Review) is specified in [`docs/interaction.md`](../interaction.md), not in these logs. The boards that freeze that model live in [`docs/mockups/studio/`](../mockups/studio/). When a log disagrees with that spec, the spec wins.
