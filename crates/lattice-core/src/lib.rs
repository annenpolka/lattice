//! Portable Lattice semantic model.
//!
//! This crate is the only place Core IR types live. It has no dependency on
//! VEL syntax, GPUI, Wasmtime, or `FFmpeg`.

mod diagnostic;
mod ir;
mod locator;
mod provenance;
mod span;
mod time;
mod time_map;
mod timeline;

pub use diagnostic::{Diagnostic, Severity};
pub use ir::{
    Audio, Media, Placement, PlacementKind, Project, Scene, Sequence, Source, TimeSpan, Visual,
};
pub use locator::MediaLocator;
pub use provenance::{Origin, Provenance};
pub use span::Span;
pub use time::{Time, TimeError};
pub use time_map::{TimeMap, TimeMapError, TimeMapSegment};
pub use timeline::{Timeline, TimelineClip, TimelineError, TimelineSource, flatten_project};
