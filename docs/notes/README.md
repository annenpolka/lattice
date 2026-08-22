# Design notes

These files are the design conversations that produced Lattice. They are source material, not spec. When they disagree with `docs/principles.md`, `docs/architecture.md`, or `AGENTS.md`, the latter win.

| File | Date | What it is |
|---|---|---|
| [2026-08-12-ai-driven-video-editor.md](2026-08-12-ai-driven-video-editor.md) | 2026-08-12 | Earlier product sketch: script-canonical, utterance-first editor (the TakeGraph line of thought). |
| [2026-08-18-video-editor-cost-and-vel-design.md](2026-08-18-video-editor-cost-and-vel-design.md) | 2026-08-18–19 | Costing, VEL language design, Lattice constitution, crate layout, and the initialization plan this repo follows. |
| [2026-08-22-studio-verb-license-intuition-integrate.md](2026-08-22-studio-verb-license-intuition-integrate.md) | 2026-08-22 | Reads three gen2 verb-license models (Semantic Compass, Projection-Local Verbs, the Reading): who each is intuitive for, what each is misread as, and the proposed integration, with pointing fixed by the overlap and video-click locks. |
| [2026-08-22-studio-verb-license-spine.md](2026-08-22-studio-verb-license-spine.md) | 2026-08-22 | What shipped: the INTEGRATED verb-license spine in Studio (not Compass / Reading / Projection-Local as a product skin), plus leftovers closed against the integrate note. |
| [2026-08-22-studio-toolbar-interaction.md](2026-08-22-studio-toolbar-interaction.md) | 2026-08-22 | Interaction lens on the current top chrome after that spine: a global verb BUTTON ROW is the wrong object (gesture is routing; verbs belong where the projection commits, or stay spoken). |

Do not "implement the whole chat log." Milestone 0 is the walking skeleton described at the end of the 08-18 log: parse a small `.vel`, lower through a registry, emit IR, explain magic.

Studio interaction (locus, Manipulate / Navigate / Review) is specified in [`docs/interaction.md`](../interaction.md), not in these logs. The boards that freeze that model live in [`docs/mockups/studio/`](../mockups/studio/). When a log disagrees with that spec, the spec wins.
