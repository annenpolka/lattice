//! Compile / validate / explain / locus / review / resolve. CLI and Studio call this crate.

mod compile;
mod edit;
mod locus;
mod lower;
mod resolve;
mod time_eval;

pub use compile::{Compilation, Engine, EngineError, ExplainEvent};
pub use lattice_core::{
    Diagnostic, EditProposal, Locus, LocusId, LocusKind, LocusProjection, Origin, Provenance,
    SemanticEdit, Span, Time, Timeline, flatten_project,
};
pub use lattice_media::{ExportReport, RenderPlan, plan_from_timeline};
pub use resolve::{
    CountingProvider, GenerateRequest, GeneratedMediaProvider, LocalToneProvider, Resolution,
    ResolveOptions,
};
