//! Shared sample/render-at-t path used by Studio preview and export.

use std::path::{Path, PathBuf};

use lattice_core::{
    Canvas, EvaluateOpts, FontIdentity, FontSpec, RenderScene, Time, Timeline, evaluate,
};
use thiserror::Error;

use crate::audio::mix_timeline_audio;
#[cfg(test)]
use crate::backend::RendererInitError;
use crate::backend::{
    Encoder, FrameRenderer, OutputSpec, RawFrame, RendererRequest, RendererSelection,
};
use crate::composite::CpuCompositor;
use crate::decode::FfmpegVideoDecoder;
use crate::encode::{FfmpegEncoder, write_frame_image};
use crate::export::{ExportError, ExportReport, PreviewOptions, validate_export_spec};
use crate::font::{FontResolution, resolve_font};
use crate::gpu::GpuCompositor;
use crate::mix::MixSpec;
use crate::plan::plan_from_timeline_with_spec;

fn map_eval(err: lattice_core::EvaluateError) -> ExportError {
    match err {
        lattice_core::EvaluateError::TimeOutOfRange(_) => ExportError::TimeOutOfRange,
        other => ExportError::Map(other.to_string()),
    }
}

fn resolve_scene_font(options: &PreviewOptions) -> Result<Option<FontResolution>, ExportError> {
    match resolve_font(
        &FontSpec::preview_sans(18),
        &options.media_root,
        options.lock.as_ref(),
        options.font.as_deref(),
    ) {
        Ok(font) => Ok(Some(font)),
        Err(ExportError::MissingFont) => Ok(None),
        Err(err) => Err(err),
    }
}

fn evaluate_scene(
    timeline: &Timeline,
    time: Time,
    canvas: Canvas,
    font: Option<&FontResolution>,
) -> Result<RenderScene, ExportError> {
    evaluate(
        timeline,
        time,
        canvas,
        EvaluateOpts {
            style: None,
            font: font.map(|font| &font.identity),
        },
    )
    .map_err(map_eval)
}

fn timeline_has_overlay_text(timeline: &Timeline) -> bool {
    timeline
        .title_clips()
        .chain(timeline.callout_clips())
        .any(|clip| clip.text.as_ref().is_some_and(|text| !text.is_empty()))
}

fn compositor_for_timeline(
    timeline: &Timeline,
    font: Option<FontResolution>,
) -> Result<CpuCompositor, ExportError> {
    if timeline_has_overlay_text(timeline) {
        let font = font.ok_or(ExportError::MissingFont)?;
        return Ok(CpuCompositor::new(Some(font)));
    }
    Ok(CpuCompositor::new(font))
}

/// Session inputs whose change requires rebuilding decoder or renderer state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionRebindRequirement {
    Renderer,
    DecoderMediaRoot,
    DecoderOutputHint,
    DecoderFixturePolicy,
    DecoderFrameRate,
    Font,
}

/// A timeline rebind is deliberately all-or-nothing.
#[derive(Debug, Error)]
pub enum SessionRebindError {
    #[error("sample session recreation required; changed requirements: {changed:?}")]
    RecreateRequired {
        changed: Vec<SessionRebindRequirement>,
    },
    #[error("replacement font requirements could not be resolved: {source}")]
    FontResolution {
        #[source]
        source: Box<ExportError>,
    },
}

impl SessionRebindError {
    #[must_use]
    pub fn changed_requirements(&self) -> &[SessionRebindRequirement] {
        match self {
            Self::RecreateRequired { changed } => changed,
            Self::FontResolution { .. } => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SessionBinding {
    renderer: RendererRequest,
    media_root: PathBuf,
    output_hint: PathBuf,
    allow_fixtures: bool,
    fps_num: i64,
    fps_den: i64,
    font: Option<FontIdentity>,
}

impl SessionBinding {
    fn new(spec: OutputSpec, options: &PreviewOptions, font: Option<&FontResolution>) -> Self {
        Self {
            renderer: options.renderer,
            media_root: options.media_root.clone(),
            output_hint: options.output.clone(),
            allow_fixtures: options.allow_fixtures,
            fps_num: spec.fps_num,
            fps_den: spec.fps_den,
            font: font.map(|font| font.identity.clone()),
        }
    }

    fn non_font_changes(
        &self,
        spec: OutputSpec,
        options: &PreviewOptions,
    ) -> Vec<SessionRebindRequirement> {
        let mut changed = Vec::new();
        if self.renderer != options.renderer {
            changed.push(SessionRebindRequirement::Renderer);
        }
        if self.media_root != options.media_root {
            changed.push(SessionRebindRequirement::DecoderMediaRoot);
        }
        if self.output_hint != options.output {
            changed.push(SessionRebindRequirement::DecoderOutputHint);
        }
        if self.allow_fixtures != options.allow_fixtures {
            changed.push(SessionRebindRequirement::DecoderFixturePolicy);
        }
        if self.fps_num != spec.fps_num || self.fps_den != spec.fps_den {
            changed.push(SessionRebindRequirement::DecoderFrameRate);
        }
        changed
    }
}

enum ActiveRenderer {
    Cpu(Box<CpuCompositor>),
    GpuDx12(Box<GpuCompositor>),
}

impl FrameRenderer for ActiveRenderer {
    fn render(
        &mut self,
        scene: &RenderScene,
        sampler: &mut dyn crate::backend::VideoDecoder,
    ) -> Result<RawFrame, ExportError> {
        match self {
            Self::Cpu(renderer) => renderer.render(scene, sampler),
            Self::GpuDx12(renderer) => renderer.render(scene, sampler),
        }
    }
}

/// Reusable preview/export sample-at-t session.
///
/// The decoder and compositor are created once, while each call remains
/// `evaluate(t)` + Lattice compositor rather than a realtime player.
pub struct SampleSession {
    timeline: Timeline,
    canvas: Canvas,
    font: Option<FontResolution>,
    decoder: FfmpegVideoDecoder,
    renderer: ActiveRenderer,
    selection: RendererSelection,
    binding: SessionBinding,
}

impl SampleSession {
    pub fn open(
        timeline: Timeline,
        spec: OutputSpec,
        options: &PreviewOptions,
    ) -> Result<Self, ExportError> {
        let font = resolve_scene_font(options)?;
        let binding = SessionBinding::new(spec, options, font.as_ref());
        let (renderer, selection) = match options.renderer {
            RendererRequest::RequireCpu => (
                ActiveRenderer::Cpu(Box::new(compositor_for_timeline(&timeline, font.clone())?)),
                RendererSelection::cpu(),
            ),
            RendererRequest::RequireGpuDx12 => {
                if timeline_has_overlay_text(&timeline) && font.is_none() {
                    return Err(ExportError::MissingFont);
                }
                let compositor = GpuCompositor::new_dx12(font.clone())?;
                let selection = compositor.selection();
                (ActiveRenderer::GpuDx12(Box::new(compositor)), selection)
            }
        };
        Ok(Self {
            canvas: Canvas {
                width: spec.width,
                height: spec.height,
            },
            decoder: FfmpegVideoDecoder::with_frame_rate(
                options.media_root.clone(),
                options.output.clone(),
                options.allow_fixtures,
                spec.fps_num,
                spec.fps_den,
            ),
            timeline,
            font,
            renderer,
            selection,
            binding,
        })
    }

    #[must_use]
    pub fn selection(&self) -> &RendererSelection {
        &self.selection
    }

    /// Replaces timeline/canvas state without rebuilding the warm decoder or renderer.
    ///
    /// Renderer, decoder configuration, frame rate, and resolved font are session
    /// invariants. If any changes, the operation leaves this session untouched and
    /// returns [`SessionRebindError::RecreateRequired`]. Canvas size may change;
    /// the persistent GPU runtime resizes only its frame targets when next sampled.
    pub fn rebind_timeline(
        &mut self,
        timeline: Timeline,
        spec: OutputSpec,
        options: &PreviewOptions,
    ) -> Result<(), SessionRebindError> {
        let changed = self.binding.non_font_changes(spec, options);
        if !changed.is_empty() {
            return Err(SessionRebindError::RecreateRequired { changed });
        }

        let font =
            resolve_scene_font(options).map_err(|source| SessionRebindError::FontResolution {
                source: Box::new(source),
            })?;
        let incoming_font = font.as_ref().map(|font| font.identity.clone());
        if self.binding.font != incoming_font {
            return Err(SessionRebindError::RecreateRequired {
                changed: vec![SessionRebindRequirement::Font],
            });
        }
        if timeline_has_overlay_text(&timeline) && font.is_none() {
            return Err(SessionRebindError::FontResolution {
                source: Box::new(ExportError::MissingFont),
            });
        }

        self.timeline = timeline;
        self.canvas = Canvas {
            width: spec.width,
            height: spec.height,
        };
        Ok(())
    }

    pub fn sample(&mut self, time: Time) -> Result<(RenderScene, RawFrame), ExportError> {
        let scene = evaluate_scene(&self.timeline, time, self.canvas, self.font.as_ref())?;
        let frame = self.renderer.render(&scene, &mut self.decoder)?;
        Ok((scene, frame))
    }

    pub fn sample_to_path(
        &mut self,
        time: Time,
        output: &Path,
    ) -> Result<std::path::PathBuf, ExportError> {
        if output.is_file() {
            return Ok(output.to_path_buf());
        }
        let (_scene, frame) = self.sample(time)?;
        write_frame_image(&frame, output)?;
        Ok(output.to_path_buf())
    }
}

/// Compatibility name for callers that only use the warm still-preview API.
pub type PreviewSampler = SampleSession;

pub fn sample_frame(
    timeline: &Timeline,
    time: Time,
    spec: OutputSpec,
    options: &PreviewOptions,
) -> Result<(RenderScene, RawFrame), ExportError> {
    SampleSession::open(timeline.clone(), spec, options)?.sample(time)
}

pub fn render_still(
    timeline: &Timeline,
    time: Time,
    spec: OutputSpec,
    options: &PreviewOptions,
    output: &Path,
) -> Result<std::path::PathBuf, ExportError> {
    SampleSession::open(timeline.clone(), spec, options)?.sample_to_path(time, output)
}

pub fn render_timeline(
    timeline: &Timeline,
    options: &PreviewOptions,
) -> Result<ExportReport, ExportError> {
    let spec = options.spec;
    validate_export_spec(spec)?;
    let plan = plan_from_timeline_with_spec(timeline, spec)?;
    if plan.segments.is_empty() {
        return Err(lattice_core::TimelineError::NoVideo.into());
    }
    let mix_spec = MixSpec {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    };
    let prepared_audio =
        mix_timeline_audio(timeline, options, mix_spec).map_err(ExportError::from)?;
    // Preserve the existing export container contract: timelines without audio
    // windows still receive a duration-matched silent PCM stream. Studio can
    // distinguish that case because `mix_timeline_audio` returns `None`.
    let silent_pcm;
    let pcm = if let Some(prepared) = prepared_audio.as_ref() {
        prepared.pcm()
    } else {
        silent_pcm = crate::backend::PcmBuffer::silence(
            spec.sample_rate,
            spec.channels,
            crate::mix::time_to_frames(timeline.duration, spec.sample_rate).max(1),
        );
        &silent_pcm
    };
    let frame_count = timeline
        .duration
        .frame_count_ceil(spec.fps_num, spec.fps_den)
        .unwrap_or(1)
        .max(1);
    let mut session = SampleSession::open(timeline.clone(), spec, options)?;
    let renderer = session.selection().clone();
    let audio = if pcm.frame_count() == 0 {
        None
    } else {
        Some(pcm)
    };
    let mut encoder = FfmpegEncoder::start(&options.output, spec, timeline.duration, audio)?;
    for index in 0..frame_count {
        let time = Time::from_frames(index.cast_signed(), spec.fps_num, spec.fps_den)
            .unwrap_or(Time::ZERO);
        let time = if time > timeline.duration {
            timeline.duration
        } else {
            time
        };
        let (_scene, frame) = session.sample(time)?;
        encoder.push_frame(&frame)?;
    }
    encoder.finish()?;
    let duration = crate::probe::probe_duration(&options.output)?;
    Ok(ExportReport {
        output: options.output.clone(),
        duration,
        spec: spec.into(),
        plan: crate::export::PlanSummary {
            hold_segments: plan.segments.iter().filter(|segment| segment.hold).count(),
            overlays: plan.overlays.len(),
        },
        renderer,
    })
}

/// GPU adapter probe. Returns Ok(non-blank) or Err with a reason string.
pub fn try_gpu_sample(scene: &RenderScene, video: &RawFrame) -> Result<RawFrame, String> {
    crate::gpu::render_offscreen(scene, video)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{PlacementKind, TimeSpan, Timeline, TimelineClip};

    #[test]
    fn sample_and_export_share_evaluate() {
        let src = include_str!("sample.rs");
        assert!(src.contains("evaluate("));
        assert!(src.contains("CpuCompositor"));
        assert!(src.contains("PreviewSampler"));
        let export_loop = src
            .split("pub fn render_timeline")
            .nth(1)
            .and_then(|body| body.split("/// GPU adapter probe").next())
            .expect("render_timeline body");
        assert!(export_loop.contains("SampleSession::open"));
        assert!(export_loop.contains("session.sample(time)"));
        assert!(!export_loop.contains("FfmpegVideoDecoder::new"));
        assert!(!export_loop.contains("compositor.render"));
        let still = include_str!("preview.rs");
        assert!(still.contains("render_still") || still.contains("sample_frame"));
        assert!(!still.contains("drawtext"));
        assert!(!still.contains("filter_complex"));
    }

    fn empty_timeline() -> Timeline {
        Timeline {
            duration: Time::seconds(1),
            clips: Vec::new(),
        }
    }

    fn sample_options(renderer: RendererRequest) -> PreviewOptions {
        let root = std::env::temp_dir().join("lattice-sample-session-selection");
        let mut options = PreviewOptions::new(root.join("out.mp4"), root);
        options.renderer = renderer;
        options
    }

    #[test]
    fn cpu_session_reports_explicit_selection_and_preview_alias_works() {
        let options = sample_options(RendererRequest::RequireCpu);
        let mut session: PreviewSampler =
            SampleSession::open(empty_timeline(), OutputSpec::preview(), &options)
                .expect("CPU sample session");
        assert_eq!(
            session.selection(),
            &RendererSelection {
                requested: RendererRequest::RequireCpu,
                active: Some(crate::RendererBackend::Cpu),
                adapter: None,
                reason: "explicit CPU renderer request".into(),
            }
        );
        let (scene, frame) = session.sample(Time::ZERO).expect("CPU sample");
        assert_eq!(scene.canvas, Canvas::PREVIEW);
        assert_eq!((frame.width, frame.height), (320, 180));
    }

    #[test]
    fn required_gpu_dx12_never_falls_back_to_cpu() {
        let options = sample_options(RendererRequest::RequireGpuDx12);
        match SampleSession::open(empty_timeline(), OutputSpec::preview(), &options) {
            Ok(mut session) => {
                assert_eq!(
                    session.selection().active,
                    Some(crate::RendererBackend::GpuDx12)
                );
                assert!(
                    session
                        .selection()
                        .adapter
                        .as_deref()
                        .is_some_and(|adapter| !adapter.is_empty())
                );
                let (_scene, frame) = session.sample(Time::ZERO).expect("DX12 sample");
                assert_eq!((frame.width, frame.height), (320, 180));
            }
            Err(ExportError::Renderer(
                error @ (RendererInitError::Unavailable { .. }
                | RendererInitError::Initialization { .. }),
            )) => {
                let selection = error.selection();
                assert_eq!(selection.requested, RendererRequest::RequireGpuDx12);
                assert_eq!(selection.active, None);
                assert_eq!(selection.adapter, None);
                assert!(selection.reason.contains("DX12"), "{}", selection.reason);
            }
            Err(error) => panic!("unexpected DX12 session error: {error}"),
        }
    }

    #[test]
    fn rebind_replaces_timeline_and_canvas_without_recreating_session_state() {
        let options = sample_options(RendererRequest::RequireCpu);
        let mut session =
            SampleSession::open(empty_timeline(), OutputSpec::preview(), &options).unwrap();
        let selection = session.selection().clone();
        let mut spec = OutputSpec::preview();
        spec.width = 640;
        spec.height = 360;
        let replacement = Timeline {
            duration: Time::seconds(2),
            clips: Vec::new(),
        };

        session
            .rebind_timeline(replacement, spec, &options)
            .expect("compatible rebind");
        let (scene, frame) = session.sample(Time::seconds(1)).expect("rebound sample");
        assert_eq!(scene.canvas.width, 640);
        assert_eq!(scene.canvas.height, 360);
        assert_eq!((frame.width, frame.height), (640, 360));
        assert_eq!(session.selection(), &selection);
    }

    fn assert_recreate(
        result: Result<(), SessionRebindError>,
        expected: &[SessionRebindRequirement],
    ) {
        let Err(error @ SessionRebindError::RecreateRequired { .. }) = result else {
            panic!("expected typed recreation requirement");
        };
        assert_eq!(error.changed_requirements(), expected);
    }

    #[test]
    fn rebind_reports_every_warm_state_change_and_is_atomic() {
        let options = sample_options(RendererRequest::RequireCpu);
        let mut session =
            SampleSession::open(empty_timeline(), OutputSpec::preview(), &options).unwrap();
        let replacement = Timeline {
            duration: Time::seconds(2),
            clips: Vec::new(),
        };

        let mut changed = options.clone();
        changed.renderer = RendererRequest::RequireGpuDx12;
        assert_recreate(
            session.rebind_timeline(replacement.clone(), OutputSpec::preview(), &changed),
            &[SessionRebindRequirement::Renderer],
        );

        let mut changed = options.clone();
        changed.media_root = changed.media_root.join("other-root");
        assert_recreate(
            session.rebind_timeline(replacement.clone(), OutputSpec::preview(), &changed),
            &[SessionRebindRequirement::DecoderMediaRoot],
        );

        let mut changed = options.clone();
        changed.output = changed.output.with_extension("other");
        assert_recreate(
            session.rebind_timeline(replacement.clone(), OutputSpec::preview(), &changed),
            &[SessionRebindRequirement::DecoderOutputHint],
        );

        let mut changed = options.clone();
        changed.allow_fixtures = !changed.allow_fixtures;
        assert_recreate(
            session.rebind_timeline(replacement.clone(), OutputSpec::preview(), &changed),
            &[SessionRebindRequirement::DecoderFixturePolicy],
        );

        let mut changed_spec = OutputSpec::preview();
        changed_spec.fps_num += 1;
        assert_recreate(
            session.rebind_timeline(replacement.clone(), changed_spec, &options),
            &[SessionRebindRequirement::DecoderFrameRate],
        );

        std::fs::create_dir_all(&options.media_root).unwrap();
        let other_font = options.media_root.join("changed-font.ttf");
        std::fs::write(&other_font, b"different font identity").unwrap();
        let mut changed = options.clone();
        changed.font = Some(other_font);
        assert_recreate(
            session.rebind_timeline(replacement, OutputSpec::preview(), &changed),
            &[SessionRebindRequirement::Font],
        );

        assert_eq!(session.timeline.duration, Time::seconds(1));
        assert_eq!(session.canvas, Canvas::PREVIEW);
    }

    #[cfg(windows)]
    #[test]
    fn gpu_rebind_keeps_runtime_and_rendered_frame_counter() {
        let options = sample_options(RendererRequest::RequireGpuDx12);
        let mut session =
            match SampleSession::open(empty_timeline(), OutputSpec::preview(), &options) {
                Ok(session) => session,
                Err(ExportError::Renderer(error)) => {
                    eprintln!("DX12 unavailable in test environment: {error}");
                    return;
                }
                Err(error) => panic!("unexpected GPU session error: {error}"),
            };
        let gpu_frames = |session: &SampleSession| match &session.renderer {
            ActiveRenderer::GpuDx12(renderer) => renderer.rendered_frames(),
            ActiveRenderer::Cpu(_) => panic!("GPU request selected CPU"),
        };

        session.sample(Time::ZERO).expect("first GPU frame");
        assert_eq!(gpu_frames(&session), 1);
        let selection = session.selection().clone();
        let mut resized = OutputSpec::preview();
        resized.width = 64;
        resized.height = 36;
        session
            .rebind_timeline(
                Timeline {
                    duration: Time::seconds(2),
                    clips: Vec::new(),
                },
                resized,
                &options,
            )
            .expect("same-runtime GPU rebind");
        let (_scene, frame) = session.sample(Time::seconds(1)).expect("second GPU frame");

        assert_eq!((frame.width, frame.height), (64, 36));
        assert_eq!(gpu_frames(&session), 2, "DX12 runtime must be retained");
        assert_eq!(session.selection(), &selection);
    }

    #[test]
    fn empty_timeline_has_canvas() {
        let tl = Timeline {
            duration: Time::seconds(1),
            clips: vec![TimelineClip {
                id: "v".into(),
                kind: PlacementKind::Video,
                span: TimeSpan::new(Time::ZERO, Time::seconds(1)),
                source: None,
                text: None,
                opacity: None,
                fade_in: None,
                fade_out: None,
                position: None,
                scale: None,
                style: None,
                gain_db: None,
            }],
        };
        let scene = evaluate(&tl, Time::ZERO, Canvas::PREVIEW, EvaluateOpts::default()).unwrap();
        assert_eq!(scene.canvas, Canvas::PREVIEW);
    }
}
