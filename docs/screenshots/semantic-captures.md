# Studio semantic captures

Observation-only captures from `main` at `60ce42f51ce64c3db05ce693b1605b249828038e`.

1. [`semantic-shared-locus.png`](semantic-shared-locus.png) — title `Hello` is projected simultaneously into Canvas selection chrome, the Text timeline clip, the VEL source highlight, and Inspector.
2. [`semantic-playhead-outside-locus.png`](semantic-playhead-outside-locus.png) — the title locus remains selected while the playhead is at `0s`, outside its `1s..4s` span, so the title is absent from the evaluated Canvas.
3. [`semantic-freeze-node-before.png`](semantic-freeze-node-before.png) and [`semantic-freeze-node-after.png`](semantic-freeze-node-after.png) — the synthetic `freeze freeze` tree row is visible, then clicking its non-Core ID collapses projection with `layout failed` / `no timeline`.
4. [`semantic-review-diff-current-canvas.png`](semantic-review-diff-current-canvas.png) — Review shows the proposed title description and VEL diff while Canvas and Inspector still show current `Hello`; no proposed `new_source` Canvas is projected.
