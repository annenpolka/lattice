//! GPUI-free Studio session. The window is a client of this type.

use std::path::{Path, PathBuf};

use lattice_engine::{
    Canvas, Compilation, Diagnostic, EditProposal, Engine, EngineError, ExportReport,
    LocalToneProvider, Locus, LocusId, LocusKind, LocusProjection, NormalizedPosition,
    PreviewFrameRequest, PreviewOptions, Provenance, RenderPlan, RendererRequest, Resolution,
    ResolveOptions, SemanticEdit, Span, Time, TimeSpan, plan_from_timeline, source_revision,
    text_overlay_size, write_source_atomic,
};

use crate::canvas::{
    CanvasDrag, CanvasPoint, CanvasRect, CanvasResize, CanvasResizePreview, CanvasSize,
    ResizeCorner,
};
use crate::gesture::{GestureOutcome, TimelineGesture};
use crate::layout::{self, StudioLayout};
use crate::preview::{PreviewJob, PreviewMailbox, playback_frame_at_or_before};
use crate::verb::{self, InvokedRecord, Projection, UnresolvedPointing, Utterance, refuse_edit};
use crate::viewport::{TimelineViewport, clamp_interaction_time};

/// Engine-backed Studio state. No GPUI types.
pub struct StudioSession {
    pub(crate) engine: Engine,
    pub(crate) path: PathBuf,
    pub(crate) compilation: Compilation,
    saved_source: String,
    pub(crate) current: Option<LocusId>,
    review: Option<EditProposal>,
    pub(crate) playhead: Time,
    playing: bool,
    pub(crate) undo_stack: Vec<String>,
    redo_stack: Vec<String>,
    invoked: Vec<Option<InvokedRecord>>,
    invoked_redo: Vec<Option<InvokedRecord>>,
    preview_generation: u64,
    source_width: Option<u32>,
    source_height: Option<u32>,
    pub(crate) viewport: TimelineViewport,
    pub(crate) gesture: TimelineGesture,
    canvas_drag: Option<CanvasDrag>,
    canvas_resize: Option<CanvasResize>,
    pub(crate) preview: PreviewMailbox,
    pub(crate) last_gesture_error: Option<String>,
    pub(crate) frame_rate: Option<(i64, i64)>,
    pub(crate) snap_time: Option<Time>,
    pub(crate) unresolved: Option<UnresolvedPointing>,
    pub(crate) touched_projection: Projection,
    pub(crate) last_spoken: Option<String>,
}

/// Move one history slot, including a `None` VEL-text placeholder.
///
/// `Vec::pop` is `Option<slot>`. The slot itself is `Option<InvokedRecord>`.
/// Matching the inner `Some` would drop the placeholder and desynchronize
/// Invoked-this-session from Undo.
fn transfer_invoked_slot(
    from: &mut Vec<Option<InvokedRecord>>,
    to: &mut Vec<Option<InvokedRecord>>,
) {
    to.extend(from.pop());
}

impl StudioSession {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, EngineError> {
        let path = path.as_ref().to_path_buf();
        let engine = Engine::default();
        let compilation = engine.compile_path(&path)?;
        let saved_source = compilation.source.clone();
        let mut session = Self {
            engine,
            path,
            compilation,
            saved_source,
            current: None,
            review: None,
            playhead: Time::ZERO,
            playing: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            invoked: Vec::new(),
            invoked_redo: Vec::new(),
            preview_generation: 0,
            source_width: None,
            source_height: None,
            viewport: TimelineViewport::fit(Time::ZERO, TimelineViewport::DEFAULT_WIDTH),
            gesture: TimelineGesture::None,
            canvas_drag: None,
            canvas_resize: None,
            preview: PreviewMailbox::default(),
            last_gesture_error: None,
            frame_rate: None,
            snap_time: None,
            unresolved: None,
            touched_projection: Projection::Timeline,
            last_spoken: None,
        };
        session.cache_source_size();
        session.fit_viewport();
        session.rebind_current();
        session.preview.set_stamp(session.session_stamp());
        Ok(session)
    }

    /// Import a real local video through the shared Engine API, then open it.
    pub fn open_video(media: impl AsRef<Path>) -> Result<Self, EngineError> {
        let media = media.as_ref();
        let dest = media
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(media.file_stem().unwrap_or_default());
        let imported = Engine::default().import_media(media, Some(&dest))?;
        Self::open(imported.vel_path)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn source(&self) -> &str {
        &self.compilation.source
    }

    pub fn saved_source(&self) -> &str {
        &self.saved_source
    }

    pub fn is_dirty(&self) -> bool {
        self.compilation.source != self.saved_source
    }

    pub fn playhead(&self) -> Time {
        self.playhead
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    #[must_use]
    pub fn duration(&self) -> Time {
        self.timeline_duration()
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.compilation.diagnostics
    }

    pub fn compilation(&self) -> &Compilation {
        &self.compilation
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn review_proposal(&self) -> Option<&EditProposal> {
        self.review.as_ref()
    }

    pub fn loci(&self) -> Result<Vec<Locus>, EngineError> {
        self.engine.loci(&self.compilation)
    }

    pub fn layout(&self) -> Result<StudioLayout, EngineError> {
        layout::from_session(self)
    }

    pub fn point_at(&mut self, id: LocusId) {
        self.unresolved = None;
        self.last_spoken = None;
        self.current = Some(id);
        self.sync_playhead_to_current();
    }

    /// Whether the selected locus is a timeline clip the toolbar Delete can name.
    #[must_use]
    pub fn toolbar_shows_delete(&self) -> bool {
        matches!(
            self.current_locus().ok().flatten().map(|locus| locus.kind),
            Some(LocusKind::Source | LocusKind::Title | LocusKind::Callout | LocusKind::Scene)
        )
    }

    pub fn point_at_title(&mut self) -> Result<Option<Locus>, EngineError> {
        let title = self
            .loci()?
            .into_iter()
            .find(|locus| locus.kind == LocusKind::Title);
        if let Some(locus) = &title {
            self.point_at(locus.id.clone());
        }
        Ok(title)
    }

    /// Point from a canvas overlay (locus id of that overlay).
    pub fn point_from_canvas_overlay(
        &mut self,
        locus_id: &str,
    ) -> Result<Option<Locus>, EngineError> {
        let found = self
            .loci()?
            .into_iter()
            .find(|locus| locus.id.as_str() == locus_id);
        self.touched_projection = Projection::Canvas;
        if let Some(locus) = &found {
            self.point_at(locus.id.clone());
        }
        Ok(found)
    }

    /// Begin a spatial overlay gesture. Pointer pixels stay ephemeral; commit
    /// rewrites normalized Canvas Space into VEL through the Engine.
    pub fn begin_canvas_overlay_drag(
        &mut self,
        locus_id: &str,
        overlay: CanvasRect,
        canvas: CanvasSize,
        pointer: CanvasPoint,
    ) -> Result<(), EngineError> {
        self.stop_transport_for_position_change();
        self.last_gesture_error = None;
        self.canvas_resize = None;
        let locus = self
            .point_from_canvas_overlay(locus_id)?
            .ok_or_else(|| EngineError::Edit(format!("unknown canvas locus `{locus_id}`")))?;
        if !matches!(locus.kind, LocusKind::Title | LocusKind::Callout) {
            return Err(EngineError::Edit(
                "only title/callout overlays can move in Canvas Space".into(),
            ));
        }
        self.canvas_drag = Some(
            CanvasDrag::begin(locus.id, overlay, canvas, pointer)
                .map_err(|err| EngineError::Edit(format!("canvas drag: {err}")))?,
        );
        Ok(())
    }

    /// Update only ephemeral geometry. This does not rewrite VEL or push Undo.
    pub fn update_canvas_overlay_drag(
        &mut self,
        pointer: CanvasPoint,
    ) -> Result<NormalizedPosition, EngineError> {
        let drag = self
            .canvas_drag
            .as_mut()
            .ok_or_else(|| EngineError::Edit("no active canvas drag".into()))?;
        drag.update(pointer)
            .map_err(|err| EngineError::Edit(format!("canvas drag: {err}")))
    }

    /// Commit exactly one source-backed edit, so the existing source Undo/Redo
    /// remains the only session history.
    pub fn commit_canvas_overlay_drag(
        &mut self,
        pointer: CanvasPoint,
    ) -> Result<GestureOutcome, EngineError> {
        self.update_canvas_overlay_drag(pointer)?;
        let drag = self
            .canvas_drag
            .take()
            .ok_or_else(|| EngineError::Edit("no active canvas drag".into()))?;
        let Some(patch) = drag.commit() else {
            return Ok(GestureOutcome::Clicked);
        };
        if self.current.as_ref() != Some(&patch.locus_id) {
            return Err(EngineError::Edit(
                "canvas locus changed before drag commit".into(),
            ));
        }
        if let Err(err) = self.apply_edit(SemanticEdit::SetPosition {
            position: patch.after,
        }) {
            self.last_gesture_error = Some(err.to_string());
            return Err(err);
        }
        Ok(GestureOutcome::Applied)
    }

    pub fn cancel_canvas_overlay_drag(&mut self) -> GestureOutcome {
        let Some(drag) = self.canvas_drag.take() else {
            return GestureOutcome::Idle;
        };
        let _ = drag.cancel();
        GestureOutcome::Cancelled
    }

    #[must_use]
    pub fn canvas_overlay_drag_active(&self) -> bool {
        self.canvas_drag.is_some()
    }

    pub(crate) fn canvas_overlay_drag_position(
        &self,
        locus_id: &LocusId,
    ) -> Option<NormalizedPosition> {
        self.canvas_drag
            .as_ref()
            .and_then(|drag| (drag.locus_id() == locus_id).then_some(drag.preview_position()))
    }

    /// Begin an aspect-preserving corner resize. The supplied corner moves;
    /// its opposite corner remains fixed for the full gesture.
    pub fn begin_canvas_overlay_resize(
        &mut self,
        locus_id: &str,
        corner: ResizeCorner,
        overlay: CanvasRect,
        canvas: CanvasSize,
        pointer: CanvasPoint,
    ) -> Result<(), EngineError> {
        self.stop_transport_for_position_change();
        self.last_gesture_error = None;
        self.canvas_drag = None;
        let locus = self
            .point_from_canvas_overlay(locus_id)?
            .ok_or_else(|| EngineError::Edit(format!("unknown canvas locus `{locus_id}`")))?;
        if !matches!(locus.kind, LocusKind::Title | LocusKind::Callout) {
            return Err(EngineError::Edit(
                "only title/callout overlays can resize in Canvas Space".into(),
            ));
        }
        let requested = locus
            .visual
            .as_ref()
            .and_then(|visual| visual.scale)
            .unwrap_or_default();
        let (canvas_width, canvas_height) = canvas_pixel_dimensions(canvas)?;
        let (base_width, base_height) = text_overlay_size(Canvas {
            width: canvas_width,
            height: canvas_height,
        });
        let scale = requested.fit_within(base_width, base_height, canvas_width, canvas_height);
        self.canvas_resize = Some(
            CanvasResize::begin(locus.id, corner, overlay, canvas, pointer, scale)
                .map_err(|err| EngineError::Edit(format!("canvas resize: {err}")))?,
        );
        Ok(())
    }

    pub fn update_canvas_overlay_resize(
        &mut self,
        pointer: CanvasPoint,
    ) -> Result<CanvasResizePreview, EngineError> {
        let resize = self
            .canvas_resize
            .as_mut()
            .ok_or_else(|| EngineError::Edit("no active canvas resize".into()))?;
        resize
            .update(pointer)
            .map_err(|err| EngineError::Edit(format!("canvas resize: {err}")))
    }

    /// Commit position and scale as one semantic/source patch and one Undo entry.
    pub fn commit_canvas_overlay_resize(
        &mut self,
        pointer: CanvasPoint,
    ) -> Result<GestureOutcome, EngineError> {
        self.update_canvas_overlay_resize(pointer)?;
        let resize = self
            .canvas_resize
            .take()
            .ok_or_else(|| EngineError::Edit("no active canvas resize".into()))?;
        let Some(patch) = resize.commit() else {
            return Ok(GestureOutcome::Clicked);
        };
        if self.current.as_ref() != Some(&patch.locus_id) {
            return Err(EngineError::Edit(
                "canvas locus changed before resize commit".into(),
            ));
        }
        if let Err(err) = self.apply_edit(SemanticEdit::ResizeOverlay {
            position: patch.after.position,
            scale: patch.after.scale,
        }) {
            self.last_gesture_error = Some(err.to_string());
            return Err(err);
        }
        Ok(GestureOutcome::Applied)
    }

    pub fn cancel_canvas_overlay_resize(&mut self) -> GestureOutcome {
        let Some(resize) = self.canvas_resize.take() else {
            return GestureOutcome::Idle;
        };
        let _ = resize.cancel();
        GestureOutcome::Cancelled
    }

    #[must_use]
    pub fn canvas_overlay_resize_active(&self) -> bool {
        self.canvas_resize.is_some()
    }

    pub(crate) fn canvas_overlay_resize_preview(
        &self,
        locus_id: &LocusId,
    ) -> Option<CanvasResizePreview> {
        self.canvas_resize
            .as_ref()
            .and_then(|resize| (resize.locus_id() == locus_id).then_some(resize.preview()))
    }

    /// Point from a byte offset in the VEL source.
    pub fn point_from_source_offset(&mut self, offset: u32) -> Result<Option<Locus>, EngineError> {
        self.touched_projection = Projection::Source;
        let found = self.engine.locus_at_source(&self.compilation, offset)?;
        if let Some(locus) = &found {
            self.point_at(locus.id.clone());
        }
        Ok(found)
    }

    /// Coordinate point on the Timeline. Playhead is not here.
    ///
    /// One covering locus commits that identity. Several covering loci hold
    /// pointing unresolved and list candidates on this projection only.
    pub fn point_from_timeline_time(&mut self, time: Time) -> Result<Option<Locus>, EngineError> {
        self.touched_projection = Projection::Timeline;
        self.last_spoken = None;
        let covering = self
            .engine
            .loci_covering_timeline(&self.compilation, time)?;
        let covering = covering
            .into_iter()
            .filter(|locus| {
                matches!(
                    locus.kind,
                    LocusKind::Title
                        | LocusKind::Callout
                        | LocusKind::Source
                        | LocusKind::Scene
                        | LocusKind::Speech
                )
            })
            .collect::<Vec<_>>();
        match covering.len() {
            0 => Ok(None),
            1 => {
                let locus = covering.into_iter().next().expect("one covering locus");
                self.unresolved = None;
                self.current = Some(locus.id.clone());
                Ok(Some(locus))
            }
            _ => {
                self.current = None;
                self.unresolved = Some(UnresolvedPointing {
                    projection: Projection::Timeline,
                    time: Some(time),
                    candidates: covering,
                });
                self.last_spoken = Some(self.utterance().spoken_text());
                Ok(None)
            }
        }
    }

    /// Identity-bearing video clip click: keep the source clip, never promote to Scene.
    pub fn point_video_clip(&mut self, clip_id: &str) -> Result<Option<Locus>, EngineError> {
        self.touched_projection = Projection::Timeline;
        self.point_source_for_clip(clip_id)
    }

    pub fn pick_point_candidate(&mut self, id: LocusId) -> Result<Option<Locus>, EngineError> {
        let Some(point) = self.unresolved.as_ref() else {
            return Err(EngineError::Edit("no unresolved pointing".into()));
        };
        if !point.candidates.iter().any(|locus| locus.id == id) {
            return Err(EngineError::Edit(
                "candidate is not on the touched projection".into(),
            ));
        }
        self.point_at(id);
        self.current_locus()
    }

    #[must_use]
    pub fn unresolved_pointing(&self) -> Option<&UnresolvedPointing> {
        self.unresolved.as_ref()
    }

    #[must_use]
    pub fn touched_projection(&self) -> Projection {
        self.touched_projection
    }

    #[must_use]
    pub fn last_spoken(&self) -> Option<&str> {
        self.last_spoken.as_deref()
    }

    #[must_use]
    pub fn utterance(&self) -> Utterance {
        let here = self.current_locus().ok().flatten();
        let loci = self.loci().unwrap_or_default();
        verb::utterance(
            here.as_ref(),
            self.unresolved.as_ref(),
            self.touched_projection,
            &loci,
        )
    }

    pub(crate) fn point_source_for_clip(
        &mut self,
        clip_id: &str,
    ) -> Result<Option<Locus>, EngineError> {
        self.unresolved = None;
        self.last_spoken = None;
        let Some(source_id) = source_id_for_clip(self, clip_id) else {
            return Err(EngineError::Edit(format!(
                "video clip `{clip_id}` has no source binding"
            )));
        };
        let found = self.loci()?.into_iter().find(|locus| {
            locus.kind == LocusKind::Source
                && (locus.node_id == source_id || locus.id.as_str() == source_id)
        });
        if let Some(locus) = &found {
            self.current = Some(locus.id.clone());
        }
        Ok(found)
    }

    pub fn touch_projection(&mut self, projection: Projection) {
        self.touched_projection = projection;
    }

    pub fn current_locus(&self) -> Result<Option<Locus>, EngineError> {
        Ok(self
            .current_projection()?
            .map(|projection| projection.locus))
    }

    /// Agent-facing projection for the shared locus. Studio does not invent a separate
    /// selection or prompt-only context.
    pub fn current_projection(&self) -> Result<Option<LocusProjection>, EngineError> {
        let Some(id) = &self.current else {
            return Ok(None);
        };
        self.engine.inspect(&self.compilation, id).map(Some)
    }

    pub fn current_projection_json(&self) -> Result<Option<String>, EngineError> {
        self.current_projection()?
            .map(|projection| {
                serde_json::to_string_pretty(&projection)
                    .map_err(|err| EngineError::Edit(format!("serialize locus: {err}")))
            })
            .transpose()
    }

    pub fn current_provenance(&self) -> Result<Option<Provenance>, EngineError> {
        Ok(self.current_locus()?.map(|locus| locus.provenance))
    }

    /// Optional Navigate: the current locus's source span.
    pub fn go_to_definition(&self) -> Result<Option<Span>, EngineError> {
        Ok(self.current_locus()?.and_then(|locus| locus.source_span))
    }

    pub fn preview_plan(&self) -> Result<RenderPlan, EngineError> {
        let timeline = Engine::timeline(&self.compilation.project)?;
        Ok(plan_from_timeline(&timeline)?)
    }

    pub fn propose_title_text(
        &mut self,
        text: impl Into<String>,
    ) -> Result<EditProposal, EngineError> {
        self.propose_overlay_edit(SemanticEdit::Title {
            text: Some(text.into()),
            at: None,
            duration: None,
            opacity: None,
        })
    }

    pub fn propose_callout_text(
        &mut self,
        text: impl Into<String>,
    ) -> Result<EditProposal, EngineError> {
        self.propose_overlay_edit(SemanticEdit::Callout {
            text: Some(text.into()),
            at: None,
            duration: None,
        })
    }

    /// Inspector body rewrite for the current title or callout locus.
    pub fn propose_overlay_text(
        &mut self,
        text: impl Into<String>,
    ) -> Result<EditProposal, EngineError> {
        let here = self.current_locus()?;
        let edit = overlay_body_edit(here.as_ref(), text.into())?;
        self.propose_overlay_edit(edit)
    }

    fn propose_overlay_edit(&mut self, edit: SemanticEdit) -> Result<EditProposal, EngineError> {
        self.touched_projection = Projection::Inspector;
        let locus = self.target_locus_for(&edit)?;
        let proposal = self.engine.propose(&self.compilation, &locus, edit)?;
        self.review = Some(proposal.clone());
        Ok(proposal)
    }

    pub fn apply_review(&mut self, proposal: &EditProposal) -> Result<(), EngineError> {
        let new_source = self
            .engine
            .apply_proposal(&self.compilation.source, proposal)?;
        write_source_atomic(&self.path, &new_source)?;
        self.saved_source.clone_from(&new_source);
        self.replace_working(&new_source)?;
        self.review = None;
        Ok(())
    }

    pub fn reject_review(&mut self, proposal: &EditProposal) -> String {
        let original = self
            .engine
            .reject_proposal(&self.compilation.source, proposal);
        self.review = None;
        original
    }

    /// Everyday Manipulate: rewrite through Engine into the working source.
    pub fn apply_title_text(&mut self, text: &str) -> Result<(), EngineError> {
        self.touched_projection = Projection::Inspector;
        self.apply_edit(SemanticEdit::Title {
            text: Some(text.into()),
            at: None,
            duration: None,
            opacity: None,
        })
    }

    pub fn apply_callout_text(&mut self, text: &str) -> Result<(), EngineError> {
        self.touched_projection = Projection::Inspector;
        self.apply_edit(SemanticEdit::Callout {
            text: Some(text.into()),
            at: None,
            duration: None,
        })
    }

    /// Inspector body rewrite for the current title or callout locus.
    pub fn apply_overlay_text(&mut self, text: &str) -> Result<(), EngineError> {
        self.touched_projection = Projection::Inspector;
        let here = self.current_locus()?;
        let edit = overlay_body_edit(here.as_ref(), text.to_string())?;
        self.apply_edit(edit)
    }

    pub fn apply_edit(&mut self, edit: SemanticEdit) -> Result<(), EngineError> {
        let locus = match self.target_locus_for(&edit) {
            Ok(locus) => locus,
            Err(err) => {
                self.last_spoken = Some(err.to_string());
                return Err(err);
            }
        };
        let invoked = InvokedRecord::from_edit(&edit, &locus);
        let proposal = self.engine.propose(&self.compilation, &locus, edit)?;
        self.last_spoken = None;
        if proposal.new_source == self.compilation.source {
            return Ok(());
        }
        self.push_undo(Some(invoked));
        let new_source = self
            .engine
            .apply_proposal(&self.compilation.source, &proposal)?;
        self.replace_working(&new_source)
    }

    pub fn set_in_at_playhead(&mut self) -> Result<(), EngineError> {
        self.touched_projection = Projection::Toolbar;
        let at = self.playhead_source_time()?;
        self.apply_edit(SemanticEdit::Trim {
            in_point: Some(at),
            out_point: None,
        })
    }

    pub fn set_out_at_playhead(&mut self) -> Result<(), EngineError> {
        self.touched_projection = Projection::Toolbar;
        let at = self.playhead_source_time()?;
        self.apply_edit(SemanticEdit::Trim {
            in_point: None,
            out_point: Some(at),
        })
    }

    pub fn split_at_playhead(&mut self) -> Result<(), EngineError> {
        self.touched_projection = Projection::Toolbar;
        let at = self.playhead_source_time()?;
        self.apply_edit(SemanticEdit::Split { at })
    }

    pub fn delete_selected_clip(&mut self) -> Result<(), EngineError> {
        self.touched_projection = Projection::Toolbar;
        self.apply_edit(SemanticEdit::Delete)
    }

    pub fn set_gain(&mut self, db: i32) -> Result<(), EngineError> {
        self.touched_projection = Projection::Toolbar;
        self.apply_edit(SemanticEdit::SetGain { db })
    }

    pub fn set_fade(&mut self, fade_in: Time) -> Result<(), EngineError> {
        self.touched_projection = Projection::Toolbar;
        self.apply_edit(SemanticEdit::SetFade {
            fade_in: Some(fade_in),
        })
    }

    pub fn apply_inspector_gain(&mut self, db: i32) -> Result<(), EngineError> {
        self.touched_projection = Projection::Inspector;
        self.apply_edit(SemanticEdit::SetGain { db })
    }

    pub fn apply_inspector_fade(&mut self, fade_in: Time) -> Result<(), EngineError> {
        self.touched_projection = Projection::Inspector;
        self.apply_edit(SemanticEdit::SetFade {
            fade_in: Some(fade_in),
        })
    }

    /// Seek the playhead to a named locus. Does not point, apply, or push Undo.
    pub fn seek_eye(&mut self, id: &str) -> Result<bool, EngineError> {
        let loci = self.loci()?;
        let Some(locus) = loci
            .iter()
            .find(|locus| locus.id.as_str() == id || locus.node_id == id)
            .cloned()
        else {
            return Ok(false);
        };
        if let Some(span) = self.preview_span_for(&locus) {
            self.seek(span.start);
            return Ok(true);
        }
        let timeline = Engine::timeline(&self.compilation.project)?;
        if let Some(span) = self
            .clip_span_for_scene(&timeline, id)
            .or_else(|| self.clip_span_for_scene(&timeline, locus.id.as_str()))
            .or_else(|| self.clip_span_for_scene(&timeline, &locus.node_id))
            .or_else(|| self.clip_span_for_source(&timeline, &locus.node_id))
            .or_else(|| self.clip_span_for_source(&timeline, locus.id.as_str()))
        {
            self.seek(span.start);
            return Ok(true);
        }
        Ok(false)
    }

    #[must_use]
    pub fn invoked_this_session(&self) -> Vec<InvokedRecord> {
        self.invoked.iter().filter_map(Clone::clone).collect()
    }

    pub fn save(&mut self) -> Result<(), EngineError> {
        write_source_atomic(&self.path, &self.compilation.source)?;
        self.saved_source.clone_from(&self.compilation.source);
        Ok(())
    }

    pub fn undo(&mut self) -> Result<(), EngineError> {
        let Some(previous) = self.undo_stack.pop() else {
            return Ok(());
        };
        self.redo_stack.push(self.compilation.source.clone());
        transfer_invoked_slot(&mut self.invoked, &mut self.invoked_redo);
        self.replace_working(&previous)
    }

    pub fn redo(&mut self) -> Result<(), EngineError> {
        let Some(next) = self.redo_stack.pop() else {
            return Ok(());
        };
        self.undo_stack.push(self.compilation.source.clone());
        transfer_invoked_slot(&mut self.invoked_redo, &mut self.invoked);
        self.replace_working(&next)
    }

    pub fn play(&mut self) {
        // A fresh transport epoch prevents an older paused/scrub decode from flashing after Play.
        self.preview.invalidate();
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.stop_transport_for_position_change();
    }

    pub fn seek(&mut self, time: Time) {
        self.stop_transport_for_position_change();
        self.playhead = clamp_interaction_time(time, self.timeline_duration());
        self.ensure_playhead_visible();
    }

    pub fn scrub(&mut self, time: Time) {
        self.stop_transport_for_position_change();
        self.playhead = clamp_interaction_time(time, self.timeline_duration());
    }

    #[must_use]
    pub fn viewport(&self) -> TimelineViewport {
        self.viewport
    }

    #[must_use]
    pub fn gesture(&self) -> &TimelineGesture {
        &self.gesture
    }

    #[must_use]
    pub fn preview_mailbox(&self) -> &PreviewMailbox {
        &self.preview
    }

    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    #[must_use]
    pub fn last_gesture_error(&self) -> Option<&str> {
        self.last_gesture_error.as_deref()
    }

    /// Agent/log snapshot of the shared locus, playhead, and in-flight drag.
    #[must_use]
    pub fn semantic_state(&self) -> serde_json::Value {
        crate::semantic_state::snapshot(self)
    }

    pub(crate) fn canvas_drag(&self) -> Option<&crate::canvas::CanvasDrag> {
        self.canvas_drag.as_ref()
    }

    pub(crate) fn canvas_resize(&self) -> Option<&crate::canvas::CanvasResize> {
        self.canvas_resize.as_ref()
    }

    #[must_use]
    pub fn snap_indicator(&self) -> Option<Time> {
        self.snap_time
    }

    #[must_use]
    pub fn frame_rate(&self) -> Option<(i64, i64)> {
        self.frame_rate
    }

    fn preview_frame_rate(&self) -> (i64, i64) {
        self.frame_rate.unwrap_or((10, 1))
    }

    pub fn set_rail_width(&mut self, width: f64) {
        self.viewport.set_width(width);
    }

    #[must_use]
    pub fn time_at_x(&self, x: f64) -> Time {
        self.viewport.time_at_x(x)
    }

    #[must_use]
    pub fn x_at_time(&self, time: Time) -> f64 {
        self.viewport.x_at_time(time)
    }

    /// Continuous scrub from a rail-local pointer x. No VEL rewrite, no Undo.
    pub fn scrub_at_x(&mut self, x: f64) {
        self.stop_transport_for_position_change();
        let time = clamp_interaction_time(self.viewport.time_at_x(x), self.timeline_duration());
        self.playhead = time;
    }

    /// Start a scrub gesture even when a clip fills the rail.
    pub fn begin_timeline_scrub(&mut self, x: f64, snap_off: bool) {
        self.stop_transport_for_position_change();
        self.gesture = crate::gesture::TimelineGesture::Scrub {
            start_playhead: self.playhead,
            start_x: x,
        };
        self.playhead =
            clamp_interaction_time(self.viewport.time_at_x(x), self.timeline_duration());
        if !snap_off {
            let _ = self.update_timeline_pointer(x, snap_off);
        }
    }

    pub fn zoom_around(&mut self, anchor: Time, factor: f64) {
        self.viewport.zoom_around(anchor, factor);
        self.viewport.clamp_to_project(self.timeline_duration());
    }

    pub fn scroll_pixels(&mut self, delta_x: f64) {
        self.viewport.scroll_by_pixels(delta_x);
        self.viewport.clamp_to_project(self.timeline_duration());
    }

    pub fn fit_viewport(&mut self) {
        self.viewport.fit_project(self.timeline_duration());
    }

    /// Map a pointer ratio `num/den` on the timeline rail to Time in `[0, duration]`.
    #[must_use]
    pub fn time_at_timeline_ratio(&self, num: u32, den: u32) -> Time {
        time_at_ratio(self.timeline_duration(), num, den)
    }

    /// Scrub playhead from a timeline rail ratio and stop the transport at that exact position.
    pub fn scrub_timeline_ratio(&mut self, num: u32, den: u32) {
        self.stop_transport_for_position_change();
        self.playhead = self.time_at_timeline_ratio(num, den);
    }

    /// Timeline click from a rail ratio: move the playhead only. Playhead is not here.
    pub fn click_timeline_ratio(
        &mut self,
        num: u32,
        den: u32,
    ) -> Result<Option<Locus>, EngineError> {
        self.click_timeline(self.time_at_timeline_ratio(num, den))
    }

    /// Move the playhead. Does not re-point. Scrub is not here.
    pub fn click_timeline(&mut self, time: Time) -> Result<Option<Locus>, EngineError> {
        self.stop_transport_for_position_change();
        self.playhead = clamp_interaction_time(time, self.timeline_duration());
        self.current_locus()
    }

    pub fn step_clock(&mut self, dt: Time) {
        if !self.playing {
            return;
        }
        let next = self.playhead.checked_add(dt).unwrap_or(self.playhead);
        let duration = self.timeline_duration();
        if next >= duration {
            self.playhead = duration;
            self.playing = false;
            self.preview.invalidate();
        } else {
            self.playhead = next;
        }
        self.ensure_playhead_visible();
    }

    fn ensure_playhead_visible(&mut self) {
        let start = self.viewport.visible_start();
        let end = self.viewport.visible_end();
        if self.playhead >= start && self.playhead <= end {
            return;
        }
        if self.playhead < start {
            let delta = start.checked_sub(self.playhead).unwrap_or(Time::ZERO);
            let next = self
                .viewport
                .visible_start()
                .checked_sub(delta)
                .unwrap_or(Time::ZERO);
            self.viewport.set_visible_start(next);
        } else {
            let delta = self.playhead.checked_sub(end).unwrap_or(Time::ZERO);
            let next = self
                .viewport
                .visible_start()
                .checked_add(delta)
                .unwrap_or(self.viewport.visible_start());
            self.viewport.set_visible_start(next);
        }
        self.viewport.clamp_to_project(self.timeline_duration());
    }

    pub fn request_preview_frame(&self, output: &Path) -> Result<PathBuf, EngineError> {
        let media_root = self.path.parent().unwrap_or_else(|| Path::new("."));
        let (width, height) = self.preview_pixel_size();
        let (fps_num, fps_den) = self.preview_frame_rate();
        self.engine.preview_frame(
            &self.compilation.project,
            &PreviewFrameRequest {
                timeline_time: self.playhead,
                width,
                height,
                fps_num,
                fps_den,
            },
            media_root,
            output,
        )
    }

    /// Still size that preserves the probed source aspect inside a max box.
    #[must_use]
    pub fn preview_pixel_size(&self) -> (u32, u32) {
        fit_preview_size(
            self.source_width.unwrap_or(PREVIEW_MAX_WIDTH),
            self.source_height.unwrap_or(PREVIEW_MAX_HEIGHT),
            PREVIEW_MAX_WIDTH,
            PREVIEW_MAX_HEIGHT,
        )
    }

    fn preview_cache_path(&self) -> PathBuf {
        let dir = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".lattice-cache");
        let (width, height) = self.preview_pixel_size();
        let (fps_num, fps_den) = self.preview_frame_rate();
        let rev = source_revision(&self.compilation.source);
        let lock = self.lock_stamp();
        let name = format!(
            "frame-{rev}-{lock}-{}-{}-{}-{width}x{height}-{fps_num}x{fps_den}.png",
            self.preview_generation,
            self.playhead.num(),
            self.playhead.den()
        );
        dir.join(name)
    }

    /// Existing cache only. Layout / GPUI paint must not spawn `FFmpeg`.
    /// A newer in-flight request may leave the last published frame visible.
    #[must_use]
    pub fn peek_preview_frame(&self) -> Option<PathBuf> {
        if let Some(path) = self.preview.published_path()
            && path.is_file()
        {
            return Some(path.to_path_buf());
        }
        let path = self.preview_cache_path();
        path.is_file().then_some(path)
    }

    /// Open a preview generation. Does not extract a frame.
    pub fn request_preview_job(&mut self) -> PreviewJob {
        self.request_preview_job_with_renderer(RendererRequest::RequireCpu)
    }

    /// Open a preview generation with an explicit renderer requirement.
    /// A required backend either initializes or reports an error; it never silently falls back.
    pub fn request_preview_job_with_renderer(&mut self, renderer: RendererRequest) -> PreviewJob {
        let generation = self.preview.request();
        let (width, height) = self.preview_pixel_size();
        let (fps_num, fps_den) = self.preview_frame_rate();
        let timeline_time = self.snapped_preview_time();
        let output = self.preview_cache_path_for(timeline_time, generation);
        PreviewJob {
            generation,
            timeline_time,
            width,
            height,
            fps_num,
            fps_den,
            renderer,
            output,
            media_root: self
                .path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            compilation: self.compilation.clone(),
            source_revision: source_revision(&self.compilation.source),
            stamp: self.session_stamp(),
            lock_stamp: self.lock_stamp(),
        }
    }

    fn session_stamp(&self) -> String {
        format!(
            "{}:{}:{}",
            self.path.display(),
            source_revision(&self.compilation.source),
            self.lock_stamp()
        )
    }

    fn lock_stamp(&self) -> String {
        let path = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("lattice.lock.json");
        std::fs::read(&path).ok().map_or_else(
            || "nolock".into(),
            |bytes| source_revision(&String::from_utf8_lossy(&bytes)),
        )
    }

    /// Preview time snaps to probed fps (or 10/1). Play uses the frame at-or-before the wall-clock
    /// playhead; paused interaction uses nearest-frame snapping for precise scrubbing.
    #[must_use]
    pub fn snapped_preview_time(&self) -> Time {
        let (num, den) = self.preview_frame_rate();
        if self.playing {
            playback_frame_at_or_before(self.playhead, num, den)
        } else {
            crate::gesture::nearest_frame(self.playhead, num, den).unwrap_or(self.playhead)
        }
    }

    /// Worker result. Ignores stale generations.
    pub fn accept_preview_result(&mut self, generation: u64, path: PathBuf, time: Time) -> bool {
        self.accept_preview_result_stamped(generation, path, time, "")
    }

    pub fn accept_preview_result_stamped(
        &mut self,
        generation: u64,
        path: PathBuf,
        time: Time,
        stamp: &str,
    ) -> bool {
        self.preview.accept_stamped(generation, path, time, stamp)
    }

    /// Accept a worker-produced frame without touching the filesystem.
    ///
    /// During Play, a completed frame may be older than the newest queued request. Publishing it
    /// is preferable to starving the canvas while `FFmpeg` catches up. Paused/scrub requests remain
    /// latest-only.
    pub fn accept_preview_frame_stamped(
        &mut self,
        generation: u64,
        frame: std::sync::Arc<lattice_engine::RawFrame>,
        time: Time,
        stamp: &str,
    ) -> bool {
        self.preview
            .accept_frame_stamped(generation, frame, time, stamp, self.playing)
    }

    #[must_use]
    pub fn has_memory_preview(&self) -> bool {
        self.preview.published_frame().is_some()
    }

    fn preview_cache_path_for(&self, playhead: Time, _generation: u64) -> PathBuf {
        let dir = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(".lattice-cache");
        let (width, height) = self.preview_pixel_size();
        let (fps_num, fps_den) = self.preview_frame_rate();
        let rev = source_revision(&self.compilation.source);
        let lock = self.lock_stamp();
        let name = format!(
            "frame-{rev}-{lock}-{}-{}-{}-{width}x{height}-{fps_num}x{fps_den}.png",
            self.preview_generation,
            playhead.num(),
            playhead.den()
        );
        dir.join(name)
    }

    /// Extract a frame if the cache miss. Call off the GPUI paint path.
    pub fn cached_preview_frame(&self) -> Result<PathBuf, EngineError> {
        let path = self.preview_cache_path();
        if path.is_file() {
            return Ok(path);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        self.request_preview_frame(&path)
    }

    /// Apply a working-source text edit (VEL pane) and recompile.
    pub fn set_working_source(&mut self, source: impl Into<String>) -> Result<(), EngineError> {
        let source = source.into();
        if source == self.compilation.source {
            return Ok(());
        }
        let previous = self.compilation.source.clone();
        self.replace_working(&source)?;
        self.undo_stack.push(previous);
        self.redo_stack.clear();
        self.invoked.push(None);
        self.invoked_redo.clear();
        Ok(())
    }

    /// Flattened preview via Engine render (`FFmpeg`). Writes beside the open VEL.
    pub fn render_preview(&self) -> Result<std::path::PathBuf, EngineError> {
        Ok(self
            .render_preview_with_renderer(RendererRequest::RequireCpu)?
            .output)
    }

    /// Flattened preview with the same explicit renderer contract used by the live canvas.
    pub fn render_preview_with_renderer(
        &self,
        renderer: RendererRequest,
    ) -> Result<ExportReport, EngineError> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let output = dir.join("studio-preview.mp4");
        let mut options = PreviewOptions::new(output, dir.to_path_buf());
        options.lock = Engine::load_lock(dir);
        options.renderer = renderer;
        self.engine
            .render_with_options(&self.compilation.project, &options)
    }

    /// Resolve generated media and persist the same project-local lock consumed by CLI render.
    /// Compile remains side-effect free; Studio only calls this from an explicit user action.
    pub fn resolve_media(&mut self) -> Result<Resolution, EngineError> {
        let media_root = self.path.parent().unwrap_or_else(|| Path::new("."));
        let artifact_dir = media_root.join(".lattice");
        let lock_path = media_root.join("lattice.lock.json");
        let existing = Engine::load_lock(media_root);
        let mut provider = LocalToneProvider;
        let resolution = self.engine.resolve(
            &self.compilation.project,
            &ResolveOptions {
                media_root,
                artifact_dir: &artifact_dir,
                lock: existing.as_ref(),
            },
            &mut provider,
        )?;
        let lock_json = serde_json::to_string_pretty(&resolution.lock)
            .map_err(|err| EngineError::Edit(format!("serialize resolve lock: {err}")))?;
        write_source_atomic(&lock_path, &lock_json)?;
        self.preview.set_stamp(self.session_stamp());
        self.preview.invalidate();
        Ok(resolution)
    }

    pub fn uses_engine_not_own_compiler(&self) -> bool {
        true
    }

    pub(crate) fn push_undo(&mut self, invoked: Option<InvokedRecord>) {
        self.undo_stack.push(self.compilation.source.clone());
        self.redo_stack.clear();
        self.invoked.push(invoked);
        self.invoked_redo.clear();
    }

    pub(crate) fn replace_working(&mut self, source: &str) -> Result<(), EngineError> {
        self.canvas_drag = None;
        self.canvas_resize = None;
        let origin = Some(self.path.display().to_string());
        self.compilation = self.engine.compile_origin(source, origin)?;
        self.preview_generation = self.preview_generation.saturating_add(1);
        self.preview.set_stamp(self.session_stamp());
        self.preview.invalidate();
        self.viewport.clamp_to_project(self.timeline_duration());
        self.playhead = clamp_interaction_time(self.playhead, self.timeline_duration());
        self.rebind_current();
        Ok(())
    }

    fn rebind_current(&mut self) {
        if self.unresolved.is_some() {
            self.current = None;
            return;
        }
        if let Some(id) = &self.current
            && self.engine.inspect(&self.compilation, id).is_ok()
        {
            return;
        }
        let Ok(loci) = self.loci() else {
            self.current = None;
            return;
        };
        let next = loci
            .iter()
            .find(|locus| locus.kind == LocusKind::Title)
            .or_else(|| loci.iter().find(|locus| locus.kind == LocusKind::Scene))
            .or_else(|| loci.first());
        self.current = next.map(|locus| locus.id.clone());
    }

    fn sync_playhead_to_current(&mut self) {
        let Ok(Some(locus)) = self.current_locus() else {
            return;
        };
        let Some(span) = self.preview_span_for(&locus) else {
            return;
        };
        if !span.contains(self.playhead) {
            self.playhead = clamp_time(span.start, self.timeline_duration());
        }
    }

    /// Timeline span the canvas should show for this locus.
    fn preview_span_for(&self, locus: &Locus) -> Option<TimeSpan> {
        if let Some(span) = locus.timeline_span {
            return Some(span);
        }
        let timeline = Engine::timeline(&self.compilation.project).ok()?;
        match locus.kind {
            LocusKind::Source => self.clip_span_for_source(&timeline, &locus.node_id),
            LocusKind::Scene => self.clip_span_for_scene(&timeline, &locus.node_id),
            _ => None,
        }
    }

    fn clip_span_for_source(
        &self,
        timeline: &lattice_engine::Timeline,
        source_id: &str,
    ) -> Option<TimeSpan> {
        timeline.clips.iter().find_map(|clip| {
            let uses = self.compilation.project.scenes.iter().any(|scene| {
                scene.placements.iter().any(|placement| {
                    placement.id == clip.id && placement.source_id.as_deref() == Some(source_id)
                })
            });
            uses.then_some(clip.span)
        })
    }

    fn clip_span_for_scene(
        &self,
        timeline: &lattice_engine::Timeline,
        scene_id: &str,
    ) -> Option<TimeSpan> {
        let scene = self
            .compilation
            .project
            .scenes
            .iter()
            .find(|scene| scene.id == scene_id)?;
        let mut start = None;
        let mut end = None;
        for placement in &scene.placements {
            let Some(clip) = timeline.clips.iter().find(|clip| clip.id == placement.id) else {
                continue;
            };
            start = Some(start.map_or(clip.span.start, |time: Time| time.min(clip.span.start)));
            end = Some(end.map_or(clip.span.end(), |time: Time| time.max(clip.span.end())));
        }
        let start = start?;
        let end = end?;
        Some(TimeSpan::new(start, end.checked_sub(start).ok()?))
    }

    pub(crate) fn target_locus_for(&self, edit: &SemanticEdit) -> Result<Locus, EngineError> {
        let loci = self.loci().unwrap_or_default();
        let here = self.current_locus()?;
        let Some(locus) = here else {
            return Err(EngineError::Edit(refuse_edit(None, edit, &loci)));
        };
        if lattice_engine::is_legal_verb(&locus, verb::verb_for_edit(edit)) {
            return Ok(locus);
        }
        Err(EngineError::Edit(refuse_edit(Some(&locus), edit, &loci)))
    }

    fn playhead_source_time(&self) -> Result<Time, EngineError> {
        let timeline = Engine::timeline(&self.compilation.project)?;
        let (locator, content) = lattice_engine::map_timeline_to_source(&timeline, self.playhead)?;
        let _ = locator;
        Ok(content)
    }

    pub(crate) fn timeline_duration(&self) -> Time {
        Engine::timeline(&self.compilation.project).map_or(Time::ZERO, |timeline| timeline.duration)
    }

    fn cache_source_size(&mut self) {
        let root = self.path.parent().unwrap_or_else(|| Path::new("."));
        if let Some(info) = self
            .engine
            .project_media_info(&self.compilation.project, root)
        {
            self.source_width = info.width;
            self.source_height = info.height;
            self.frame_rate = match (info.frame_rate_num, info.frame_rate_den) {
                (Some(num), Some(den)) if num > 0 && den > 0 => Some((num, den)),
                _ => None,
            };
        }
    }

    pub fn begin_timeline_pointer(&mut self, x: f64, snap_off: bool) -> Result<(), EngineError> {
        self.stop_transport_for_position_change();
        crate::interaction::begin(self, x, snap_off, None)
    }

    pub fn begin_timeline_pointer_on(
        &mut self,
        x: f64,
        snap_off: bool,
        track: &str,
    ) -> Result<(), EngineError> {
        self.stop_transport_for_position_change();
        crate::interaction::begin(self, x, snap_off, Some(track))
    }

    pub fn begin_timeline_pointer_on_xy(
        &mut self,
        x: f64,
        y: f64,
        snap_off: bool,
        track: &str,
    ) -> Result<(), EngineError> {
        self.stop_transport_for_position_change();
        crate::interaction::begin_xy(self, x, y, snap_off, Some(track))
    }

    pub fn update_timeline_pointer(&mut self, x: f64, snap_off: bool) -> Result<(), EngineError> {
        crate::interaction::update(self, x, snap_off)
    }

    pub fn update_timeline_pointer_xy(
        &mut self,
        x: f64,
        y: f64,
        snap_off: bool,
    ) -> Result<(), EngineError> {
        crate::interaction::update_xy(self, x, y, snap_off)
    }

    pub fn commit_timeline_pointer(&mut self, x: f64) -> Result<GestureOutcome, EngineError> {
        crate::interaction::commit(self, x, false)
    }

    pub fn commit_timeline_pointer_xy(
        &mut self,
        x: f64,
        y: f64,
    ) -> Result<GestureOutcome, EngineError> {
        crate::interaction::commit_xy(self, x, y, false)
    }

    pub fn commit_timeline_pointer_xy_snap(
        &mut self,
        x: f64,
        y: f64,
        snap_off: bool,
    ) -> Result<GestureOutcome, EngineError> {
        crate::interaction::commit_xy(self, x, y, snap_off)
    }

    pub fn commit_timeline_pointer_snap(
        &mut self,
        x: f64,
        snap_off: bool,
    ) -> Result<GestureOutcome, EngineError> {
        crate::interaction::commit(self, x, snap_off)
    }

    pub fn cancel_timeline_pointer(&mut self) -> GestureOutcome {
        crate::interaction::cancel(self)
    }

    /// Shared commit path: on failure discard ephemeral geometry and record the error.
    pub fn apply_committed_edit(&mut self, edit: SemanticEdit) -> Result<(), EngineError> {
        crate::interaction::apply_committed(self, edit)
    }

    #[must_use]
    pub fn cursor_at(&self, x: f64) -> crate::gesture::CursorKind {
        crate::interaction::cursor_at(self, x)
    }

    #[must_use]
    pub fn cursor_at_on(&self, x: f64, track: Option<&str>) -> crate::gesture::CursorKind {
        crate::interaction::cursor_at_on_track(self, x, track)
    }

    fn stop_transport_for_position_change(&mut self) {
        self.playing = false;
        // Keep the displayed frame while rejecting every result from the previous transport epoch.
        self.preview.invalidate();
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn canvas_pixel_dimensions(canvas: CanvasSize) -> Result<(u32, u32), EngineError> {
    if !canvas.width.is_finite()
        || !canvas.height.is_finite()
        || canvas.width <= 0.0
        || canvas.height <= 0.0
        || canvas.width > f64::from(u32::MAX)
        || canvas.height > f64::from(u32::MAX)
    {
        return Err(EngineError::Edit(
            "canvas resize needs finite positive dimensions".into(),
        ));
    }
    Ok((canvas.width.round() as u32, canvas.height.round() as u32))
}

/// Fit `src` inside `max` without stretching. Integer rounding is allowed.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn fit_preview_size(src_w: u32, src_h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let src_w = src_w.max(1);
    let src_h = src_h.max(1);
    let max_w = max_w.max(1);
    let max_h = max_h.max(1);
    let wide = u64::from(src_w).saturating_mul(u64::from(max_h));
    let tall = u64::from(src_h).saturating_mul(u64::from(max_w));
    if wide >= tall {
        let height = (u64::from(src_h).saturating_mul(u64::from(max_w)) / u64::from(src_w)).max(1);
        (max_w, height.min(u64::from(max_h)) as u32)
    } else {
        let width = (u64::from(src_w).saturating_mul(u64::from(max_h)) / u64::from(src_h)).max(1);
        (width.min(u64::from(max_w)) as u32, max_h)
    }
}

const PREVIEW_MAX_WIDTH: u32 = 640;
const PREVIEW_MAX_HEIGHT: u32 = 360;

fn time_at_ratio(duration: Time, num: u32, den: u32) -> Time {
    if den == 0 || num == 0 || duration.is_zero() {
        return Time::ZERO;
    }
    if num >= den {
        return duration;
    }
    let Ok(fraction) = Time::new(i64::from(num), i64::from(den)) else {
        return Time::ZERO;
    };
    duration
        .checked_mul(fraction)
        .map_or(Time::ZERO, |time| clamp_time(time, duration))
}

fn clamp_time(time: Time, max: Time) -> Time {
    if time < Time::ZERO {
        Time::ZERO
    } else if time > max {
        max
    } else {
        time
    }
}

fn source_id_for_clip(session: &StudioSession, clip_id: &str) -> Option<String> {
    for scene in &session.compilation.project.scenes {
        if let Some(placement) = scene
            .placements
            .iter()
            .find(|placement| placement.id == clip_id)
        {
            return placement.source_id.clone();
        }
    }
    None
}

fn overlay_body_edit(here: Option<&Locus>, text: String) -> Result<SemanticEdit, EngineError> {
    match here.map(|locus| locus.kind) {
        Some(LocusKind::Title) => Ok(SemanticEdit::Title {
            text: Some(text),
            at: None,
            duration: None,
            opacity: None,
        }),
        Some(LocusKind::Callout) => Ok(SemanticEdit::Callout {
            text: Some(text),
            at: None,
            duration: None,
        }),
        _ => Err(EngineError::Edit(
            "overlay body needs a title or callout locus".into(),
        )),
    }
}
