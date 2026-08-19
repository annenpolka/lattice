//! Compile / validate / explain. CLI and Studio call this crate.

mod compile;
mod lower;
mod time_eval;

pub use compile::{Compilation, Engine, EngineError, ExplainEvent};
