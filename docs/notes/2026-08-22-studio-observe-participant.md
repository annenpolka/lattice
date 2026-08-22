# Participant turn — cheap-mutation lens

Not the chair. Not a pack author. No Studio UI implementation. No winning skin.

Lens: which proposed mutations are cheap on existing `SemanticEdit` / `StudioSession` / `UiDriver`. Captures reread for this turn only:

- title locus (header stuck; Review button, no surface)
- scene locus (Inspector Title text = `demo`)
- freeze click (`layout failed`)
- title-only Review (`set title text "Hello"`, `@@ no line changes @@`, no picture)

---

## A — Implementation-grounded

**KEEP.** One `StudioSession.current: Option<LocusId>`. Cheap forks named here already have session/Engine verbs: hide VEL in `StudioView::body`; gate Inspector on `LocusKind`; `engine.propose` for `SemanticEdit::Trim` / `SetPosition` into `session.review`; Audio rail `ClipKind::Other` can commit `SetGain` / `SetFade` instead of `TimelineGesture::Scrub`; freeze row `id: format!("freeze:{}", source.id)` is not in `Engine::inspect`.

**CHANGE.** Freeze click is inspect-fail now (`unknown locus freeze:source:clip`), not an immediate `rebind_current`. Rebound is `replace_working` only.

**STEAL** the freeze-id deletion. Cheap: stop emitting the synthetic `TreeNode`, or ignore clicks whose id fails `inspect`. Do not mint a Core freeze locus to save the row.

---

## B — Semantic

**KEEP** the sentence “one locus, many projections.” Matches `Locus::project` → `LocusProjection { source, core, timeline }` (`lattice-core/src/locus.rs`). Freeze tree node is not a Core locus — same fact as A, confirmed by the freeze capture.

**CHANGE** “missing perceptual projection of Source / Placement / TimeMap / playhead-vs-locus” from a new-pane request into two cheap vs not-cheap piles:

- Cheap: playhead-vs-locus is already visible in the title capture (playhead `0.00s`, title clip starts at 1s). Do not make playhead a second `LocusId`.
- Not cheap: a TimeMap / Placement “look” that invents a Core kind. `LocusKind` has no Freeze. `LocusProjection` has no canvas facet; canvas geometry sits on `Locus.visual`.

**DELETE** any mutation that adds a per-view selection to “show” those looks.

**UNSTATED ASSUMPTION.** That every Core noun needs a dedicated pane. Source is already a SEQUENCE row. Placement is not a user word in Alpha.

---

## C — Visual hierarchy

**KEEP** as paint notes, not domain notes. Captures: Play / Save / Resolve / CPU are teal while idle; Canvas is empty-black; toolbar wraps to two rows.

**CHANGE** “1024 wraps toolbar.” This window was 1400×840 and still wraps (`actions_bar` is `flex_wrap`). The wrap is real; the breakpoint number is not evidenced here.

**CHANGE** “VEL looks static.” `StudioSourceInputHandler` edits. The chrome looks like a readout. Cheap: keep the editor, stop painting it like a frozen listing. Not cheap: replacing it with a second selection.

**STEAL** idle-teal demotion (chrome only). Leave `TEAL` for current locus + in-flight Review Apply.

**DELETE** as a reason to pick a Premiere/Figma/IDE skin.

---

## D — Affordance

**KEEP.** Playhead and Text clips share `TEAL` (`main.rs` timeline rail). Ruler has no thumb (2px line only). Status strings sit in the button row. Title capture shows SEQUENCE / VEL / Inspector / Timeline sharing one title; Canvas has no overlay chrome.

**UNSTATED ASSUMPTION.** “800px drops panes.” Not in these shots. `body` is four flex children with fixed SEQUENCE/Inspector widths; squeeze is unmeasured.

**STEAL** playhead color ≠ text-clip color. Cheap paint. Do not invent a playhead locus to “fix” the collision.

**CHANGE** “5-pane selection sync is the strong signal.” The strong signal in the title capture is four panes. Canvas does not show the locus when `preview_image` is `None`.

---

## E — First principles

**KEEP** “one locus / many looks” and “natural verb is locus → legal `SemanticEdit`.” That is `StudioSession::target_locus_for` + `apply_edit` / `propose`.

**CHANGE** “toolbar ignores locus and falls back.” Toolbar buttons call session methods. Those methods **do** start from `current`, then `target_source_locus` / `target_scene_locus` walk to a neighbor or the first source/scene. That is silent fallback, not ignore. Cheap mutation: fail closed when `current.kind` is illegal for that verb (Gain on a Title should error, not punch the clip).

**CHANGE** “timeline is a readout.” Timeline is a `LocusProjection.timeline` **and** a gesture surface (`interaction::{begin,update,commit}` → `Trim` / `ReorderScene` / overlay timing). Making it readout-only would delete Manipulate.

**CHANGE** “panes should follow `LocusProjection` facets.” Facets are source / core / timeline, plus `Locus.visual`. SEQUENCE and Inspector are not extra facets; they are indexes over the same id. Do not add a pane per field.

**STEAL** fail-closed toolbar. Not cheap: rebuilding the window as “many clips / one look” NLE.

**UNSTATED ASSUMPTION.** That “1 Title → 3 clips” must be taught by extra chrome. The title capture already shows one title locus and three track rows (video/audio/text). The cheap lesson is overlay-on-canvas when preview is off, not a new model.

---

## F — Interaction

**KEEP**

- One `LocusId` projected (title capture).
- Video body commit points the scene: `commit` on `Reorder` calls `point_scene` even when `!moved` (`interaction.rs`).
- Audio rail is always `TimelineGesture::Scrub` (`ClipKind::Other` in `begin`).
- Inspector Apply/Review always `apply_title_text` / `propose_title_text` (scene capture: Title text `demo` on a scene).
- Space unbound (`handle_key` is Escape / Ctrl-Z/Y / `+/-` only).
- Trim handles are drawn with `debug_selector` and **no** `on_mouse_down`; the rail’s `capture_any_mouse_down` → `begin_timeline_pointer_on` + `hit_test` owns the edge (`main.rs`).
- Preview-off removes overlay chrome: overlays are children of `if let Some(preview_image)` else empty `div` (`canvas_pane`). Title capture: title selected, Canvas black, no handles.

**MISREAD** “Audio rail … also rewrites here.” Scrub commit is `point_from_timeline_time` + `GestureOutcome::Scrubbed`. No VEL. Rewrite on audio is the Gain/Fade **toolbar**, via `target_source_locus` fallback (E), not the rail.

**CHANGE** “open title×0s leaves Canvas empty.” In these shots Canvas is empty because `LATTICE_STUDIO_PREVIEW=0` (no still at all). Separately, `overlay_playhead_visible` would hide a title that starts at 1s when `t=0` **if** a still existed. Do not treat the black canvas as proof of the second rule.

**STEAL** (cheap, no new skin)

- Space → `StudioSession::play` / `pause` (already on the session).
- Draw overlay hit-rects even when `preview_image` is `None` (same `CanvasOverlay` from `layout::from_session`).
- Inspector Apply/Review dispatch `target_locus_for` + the matching `SemanticEdit`, not always Title.

**DELETE** treating trim-handle missing `on_mouse_down` as “trim is fake.” `UiDriver` already drags `timeline.trim.<id>.in` through the rail path.

---

## Cheap mutations (not a winner)

Independent, existing API, `UiDriver` can lock them:

1. Drop or ignore `freeze:{source.id}` tree rows. Freeze capture is the oracle.
2. Inspector widgets follow `LocusKind`; Title Apply/Review only on `LocusKind::Title`.
3. Toolbar verbs fail closed when `current` is the wrong kind.
4. Audio rail body/edge: one `SemanticEdit::SetGain` / `SetFade` **or** remain scrub — pick one; do not do both.
5. Overlay chrome without a still; Space play/pause; playhead color ≠ text clip.

Not cheap (do not steal as “just UI”):

- Playhead as locus
- TimeMap/Placement as new Core kinds for a pane
- Timeline as readout-only
- Per-view selection
- Agent/chat runtime
- Product Linux A/V

End of turn.
