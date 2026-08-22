# Studio verb-license spine (what shipped)

Date: 2026-08-22

This is the implementation record for the INTEGRATED spine read in
`docs/notes/2026-08-22-studio-verb-license-intuition-integrate.md` (PR #22).
It is not Compass chrome, not the Reading five-pane product, and not
Projection-Local as a second selection.

Shipped in Studio/GPUI:

- One locus, one Engine legal set, one utterance.
- Touched projection is routing only.
- When legality ≠ what the gesture can commit, the difference is spoken.
- No `target_source_locus` / `target_scene_locus` fallthrough.
- Video clip click points the source clip.
- Overlap candidates stay on the touched Timeline projection.
- Scrub / playhead do not re-point.
- Title Inspector fields only when here is Title.
- No selectable freeze tree row.

Leftovers closed against that note, without a gen2 skin:

- Spoken clauses disclose `(verb, target, scope, effect)` before invocation.
- A pointed source keeps the scene as a spoken relation (Navigate), never as
  an adopted center and never as "split is absent here".
- A pointed scene speaks the source-binding affordance (`Point the video clip`)
  instead of inventing a first-match target.
- Overlap cards advertise Timeline routes; the duplicate-overlay fixture is
  the stress test that same-label / same-span cards stay distinct `LocusId`s.
- Lock tests go through the Timeline hit path / UiDriver click, not injected
  `point_at` / `point_video_clip` / `point_from_timeline_time`.
- `inspect --json` and Studio `semantic_state` carry the same Engine legal set.
- `routed_verbs` names only gesture paths the UI can commit: Timeline trim /
  overlay time / scene reorder; Canvas geometry; Toolbar trim/gain/fade/split/delete;
  Inspector title text. A missing route is spoken, not claimed as present.
- A source relation resolves the related Scene and speaks Engine
  `legal_edits_for` (verb, target, scope, effect). Studio does not hardcode
  "split/delete/reorder-scene are legal there", including when `scene_id` is
  missing or stale.
- `pick_point_candidate` is fail-closed: an active unresolved point is required,
  and only that projection's candidate list is accepted.
