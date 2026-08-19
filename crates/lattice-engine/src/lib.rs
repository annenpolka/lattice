//! Compile / validate / explain. CLI and Studio call this crate.

mod compile;
mod lower;
mod time_eval;

pub use compile::{Compilation, Engine, EngineError, ExplainEvent};
pub use lattice_core::{Timeline, flatten_project};
pub use lattice_media::{ExportReport, RenderPlan, plan_from_timeline};
