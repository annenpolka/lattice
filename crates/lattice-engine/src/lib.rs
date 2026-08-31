//! Compile / validate / explain / locus / review / resolve. CLI and Studio call this crate.

mod atomic;
mod compile;
mod edit;
mod highlight;
mod import;
mod legal;
mod locus;
mod lower;
mod resolve;
mod time_eval;

pub use atomic::{write_source_atomic, write_source_atomic_no_commit};
pub use compile::{Compilation, Engine, EngineError, ExplainEvent};
pub use highlight::{VelHighlight, VelHighlightClass};
pub use import::ImportResult;
pub use lattice_core::{
    AudioPlan, Canvas, Diagnostic, EditProposal, Locus, LocusId, LocusKind, LocusProjection,
    NormalizedPosition, NormalizedScale, OVERLAY_SCALE_MAX, OVERLAY_SCALE_MIN, OVERLAY_SCALE_ONE,
    Origin, Project, Provenance, RenderScene, ResolveLock, SemanticEdit, Span, Time, TimeSpan,
    Timeline, audio_plan_from_timeline, evaluate, evaluate_at, flatten_project, source_revision,
    text_overlay_size,
};
pub use lattice_media::{
    AudioMixError, AudioMixReport, ExportError, ExportReport, GpuCompositor, GpuRendererRuntime,
    MediaInfo, MixSpec, OutputSpec, OutputSpecReport, PcmBuffer, PreparedAudio,
    PreviewFrameRequest, PreviewOptions, PreviewSampler, RawFrame, RenderPlan, RendererBackend,
    RendererInitError, RendererInitStage, RendererRenderError, RendererRequest, RendererSelection,
    SampleSession, generate_av_fixture, map_timeline_to_source, mix_timeline_audio,
    plan_from_timeline, plan_from_timeline_with_spec, preview_frame, sample_frame,
};
pub use lattice_wasm::{LoweringRegistry, OverlayPresetSource};
pub use legal::{AbsenceReason, LegalEdit, is_legal_verb, legal_edits_for};
pub use resolve::{
    CountingProvider, GenerateRequest, GeneratedMediaProvider, LocalToneProvider, Resolution,
    ResolveOptions, generated_request_key,
};
