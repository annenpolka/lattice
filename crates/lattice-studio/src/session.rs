//! GPUI-free Studio session. The window is a client of this type.

use std::path::{Path, PathBuf};

use lattice_engine::{
    Compilation, Diagnostic, EditProposal, Engine, EngineError, Locus, LocusId, LocusKind,
    PreviewFrameRequest, Provenance, RenderPlan, SemanticEdit, Span, Time, TimeSpan,
    plan_from_timeline, write_source_atomic,
};

use crate::layout::{self, StudioLayout};

/// Engine-backed Studio state. No GPUI types.
pub struct StudioSession {
    engine: Engine,
    path: PathBuf,
    compilation: Compilation,
    saved_source: String,
    current: Option<LocusId>,
    review: Option<EditProposal>,
    playhead: Time,
    playing: bool,
    undo_stack: Vec<String>,
    redo_stack: Vec<String>,
    preview_generation: u64,
    source_width: Option<u32>,
    source_height: Option<u32>,
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
            preview_generation: 0,
            source_width: None,
            source_height: None,
        };
        session.cache_source_size();
        session.rebind_current();
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
        self.current = Some(id);
        self.sync_playhead_to_current();
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
        if let Some(locus) = &found {
            self.point_at(locus.id.clone());
        }
        Ok(found)
    }

    /// Point from a byte offset in the VEL source.
    pub fn point_from_source_offset(&mut self, offset: u32) -> Result<Option<Locus>, EngineError> {
        let found = self.engine.locus_at_source(&self.compilation, offset)?;
        if let Some(locus) = &found {
            self.point_at(locus.id.clone());
        }
        Ok(found)
    }

    /// Point from a time on the flattened timeline.
    /// Playhead is the input; the locus follows. Do not seek again.
    pub fn point_from_timeline_time(&mut self, time: Time) -> Result<Option<Locus>, EngineError> {
        let found = self.engine.locus_at_timeline(&self.compilation, time)?;
        if let Some(locus) = &found {
            self.current = Some(locus.id.clone());
        }
        Ok(found)
    }

    pub fn current_locus(&self) -> Result<Option<Locus>, EngineError> {
        let Some(id) = &self.current else {
            return Ok(None);
        };
        Ok(self
            .engine
            .inspect(&self.compilation, id)
            .ok()
            .map(|projection| projection.locus))
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
        let locus = self
            .current_locus()?
            .ok_or_else(|| EngineError::Edit("no current locus".into()))?;
        let proposal = self.engine.propose(
            &self.compilation,
            &locus,
            SemanticEdit::Title {
                text: Some(text.into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )?;
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
        self.apply_edit(SemanticEdit::Title {
            text: Some(text.into()),
            at: None,
            duration: None,
            opacity: None,
        })
    }

    pub fn apply_edit(&mut self, edit: SemanticEdit) -> Result<(), EngineError> {
        let locus = self.target_locus_for(&edit)?;
        let proposal = self.engine.propose(&self.compilation, &locus, edit)?;
        self.push_undo();
        let new_source = self
            .engine
            .apply_proposal(&self.compilation.source, &proposal)?;
        self.replace_working(&new_source)
    }

    pub fn set_in_at_playhead(&mut self) -> Result<(), EngineError> {
        let at = self.playhead_source_time()?;
        self.apply_edit(SemanticEdit::Trim {
            in_point: Some(at),
            out_point: None,
        })
    }

    pub fn set_out_at_playhead(&mut self) -> Result<(), EngineError> {
        let at = self.playhead_source_time()?;
        self.apply_edit(SemanticEdit::Trim {
            in_point: None,
            out_point: Some(at),
        })
    }

    pub fn split_at_playhead(&mut self) -> Result<(), EngineError> {
        let at = self.playhead_source_time()?;
        let scene = self.target_scene_locus()?;
        self.current = Some(scene.id);
        self.apply_edit(SemanticEdit::Split { at })
    }

    pub fn delete_selected_clip(&mut self) -> Result<(), EngineError> {
        let scene = self.target_scene_locus()?;
        self.current = Some(scene.id);
        self.apply_edit(SemanticEdit::Delete)
    }

    pub fn set_gain(&mut self, db: i32) -> Result<(), EngineError> {
        self.apply_edit(SemanticEdit::SetGain { db })
    }

    pub fn set_fade(&mut self, fade_in: Time) -> Result<(), EngineError> {
        self.apply_edit(SemanticEdit::SetFade {
            fade_in: Some(fade_in),
        })
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
        self.replace_working(&previous)
    }

    pub fn redo(&mut self) -> Result<(), EngineError> {
        let Some(next) = self.redo_stack.pop() else {
            return Ok(());
        };
        self.undo_stack.push(self.compilation.source.clone());
        self.replace_working(&next)
    }

    pub fn play(&mut self) {
        self.playing = true;
    }

    pub fn pause(&mut self) {
        self.playing = false;
    }

    pub fn seek(&mut self, time: Time) {
        self.playhead = clamp_time(time, self.timeline_duration());
        self.playing = false;
    }

    pub fn scrub(&mut self, time: Time) {
        self.playhead = clamp_time(time, self.timeline_duration());
    }

    /// Map a pointer ratio `num/den` on the timeline rail to Time in `[0, duration]`.
    #[must_use]
    pub fn time_at_timeline_ratio(&self, num: u32, den: u32) -> Time {
        time_at_ratio(self.timeline_duration(), num, den)
    }

    /// Scrub playhead from a timeline rail ratio. Does not change playing.
    pub fn scrub_timeline_ratio(&mut self, num: u32, den: u32) {
        self.playhead = self.time_at_timeline_ratio(num, den);
    }

    /// Timeline click from a rail ratio: scrub and point the locus at that time.
    pub fn click_timeline_ratio(
        &mut self,
        num: u32,
        den: u32,
    ) -> Result<Option<Locus>, EngineError> {
        self.click_timeline(self.time_at_timeline_ratio(num, den))
    }

    /// Timeline click: update playhead and the current locus.
    pub fn click_timeline(&mut self, time: Time) -> Result<Option<Locus>, EngineError> {
        self.playhead = clamp_time(time, self.timeline_duration());
        self.point_from_timeline_time(self.playhead)
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
        } else {
            self.playhead = next;
        }
    }

    pub fn request_preview_frame(&self, output: &Path) -> Result<PathBuf, EngineError> {
        let media_root = self.path.parent().unwrap_or_else(|| Path::new("."));
        let (width, height) = self.preview_pixel_size();
        self.engine.preview_frame(
            &self.compilation.project,
            &PreviewFrameRequest {
                timeline_time: self.playhead,
                width,
                height,
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
        let name = format!(
            "frame-{}-{}-{}-{width}x{height}.png",
            self.preview_generation,
            self.playhead.num(),
            self.playhead.den()
        );
        dir.join(name)
    }

    /// Existing cache only. Layout / GPUI paint must not spawn `FFmpeg`.
    #[must_use]
    pub fn peek_preview_frame(&self) -> Option<PathBuf> {
        let path = self.preview_cache_path();
        path.is_file().then_some(path)
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
        self.push_undo();
        self.replace_working(&source.into())
    }

    /// Flattened preview via Engine render (`FFmpeg`). Writes beside the open VEL.
    pub fn render_preview(&self) -> Result<std::path::PathBuf, EngineError> {
        let dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        let output = dir.join("studio-preview.mp4");
        self.engine
            .render(&self.compilation.project, &output, dir)?;
        Ok(output)
    }

    pub fn uses_engine_not_own_compiler(&self) -> bool {
        true
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.compilation.source.clone());
        self.redo_stack.clear();
    }

    fn replace_working(&mut self, source: &str) -> Result<(), EngineError> {
        let origin = Some(self.path.display().to_string());
        self.compilation = self.engine.compile_origin(source, origin)?;
        self.preview_generation = self.preview_generation.saturating_add(1);
        self.rebind_current();
        Ok(())
    }

    fn rebind_current(&mut self) {
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

    fn target_locus_for(&self, edit: &SemanticEdit) -> Result<Locus, EngineError> {
        match edit {
            SemanticEdit::Title { .. } => {
                let Some(locus) = self.current_locus()? else {
                    return Err(EngineError::Edit(
                        "title edit needs a current title, scene, or source locus".into(),
                    ));
                };
                if matches!(
                    locus.kind,
                    LocusKind::Title | LocusKind::Scene | LocusKind::Source
                ) {
                    return Ok(locus);
                }
                if locus.scene_id.is_some() {
                    return self.target_scene_locus();
                }
                Err(EngineError::Edit(
                    "title edit needs a title, scene, or source locus".into(),
                ))
            }
            SemanticEdit::Trim { .. } => self.target_source_locus(),
            SemanticEdit::Split { .. } | SemanticEdit::Delete => self.target_scene_locus(),
            SemanticEdit::SetGain { .. } | SemanticEdit::SetFade { .. } => self
                .target_source_locus()
                .or_else(|_| self.target_scene_locus()),
        }
    }

    fn target_scene_locus(&self) -> Result<Locus, EngineError> {
        let loci = self.loci()?;
        if let Some(cur) = self.current_locus()? {
            if cur.kind == LocusKind::Scene {
                return Ok(cur);
            }
            if let Some(scene_id) = &cur.scene_id
                && let Some(scene) = loci
                    .iter()
                    .find(|locus| locus.kind == LocusKind::Scene && locus.node_id == *scene_id)
            {
                return Ok(scene.clone());
            }
        }
        if let Some(at) = self
            .engine
            .locus_at_timeline(&self.compilation, self.playhead)?
        {
            if at.kind == LocusKind::Scene {
                return Ok(at);
            }
            if let Some(scene_id) = &at.scene_id
                && let Some(scene) = loci
                    .iter()
                    .find(|locus| locus.kind == LocusKind::Scene && locus.node_id == *scene_id)
            {
                return Ok(scene.clone());
            }
        }
        loci.into_iter()
            .find(|locus| locus.kind == LocusKind::Scene)
            .ok_or_else(|| EngineError::Edit("no scene locus".into()))
    }

    fn target_source_locus(&self) -> Result<Locus, EngineError> {
        let loci = self.loci()?;
        if let Some(cur) = self.current_locus()? {
            if cur.kind == LocusKind::Source {
                return Ok(cur);
            }
            if let Some(scene_id) = &cur.scene_id
                && let Some(source) = loci.iter().find(|locus| {
                    locus.kind == LocusKind::Source && locus.scene_id.as_deref() == Some(scene_id)
                })
            {
                return Ok(source.clone());
            }
        }
        loci.into_iter()
            .find(|locus| locus.kind == LocusKind::Source)
            .ok_or_else(|| EngineError::Edit("no source locus".into()))
    }

    fn playhead_source_time(&self) -> Result<Time, EngineError> {
        let timeline = Engine::timeline(&self.compilation.project)?;
        let (locator, content) = lattice_engine::map_timeline_to_source(&timeline, self.playhead)?;
        let _ = locator;
        Ok(content)
    }

    fn timeline_duration(&self) -> Time {
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
        }
    }
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
