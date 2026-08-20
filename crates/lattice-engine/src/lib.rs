//! Compile / validate / explain / locus / review / resolve. CLI and Studio call this crate.

mod atomic;
mod compile;
mod edit;
mod import;
mod locus;
mod lower;
mod resolve;
mod time_eval;

pub use atomic::{write_source_atomic, write_source_atomic_no_commit};
pub use compile::{Compilation, Engine, EngineError, ExplainEvent};
pub use import::ImportResult;
pub use lattice_core::{
    Diagnostic, EditProposal, Locus, LocusId, LocusKind, LocusProjection, Origin, Provenance,
    SemanticEdit, Span, Time, TimeSpan, Timeline, flatten_project, source_revision,
};
pub use lattice_media::{
    ExportReport, MediaInfo, PreviewFrameRequest, PreviewOptions, RenderPlan, generate_av_fixture,
    map_timeline_to_source, plan_from_timeline, preview_frame,
};
pub use resolve::{
    CountingProvider, GenerateRequest, GeneratedMediaProvider, LocalToneProvider, Resolution,
    ResolveOptions, generated_request_key,
};
