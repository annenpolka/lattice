//! Portable Lattice semantic model.
//!
//! This crate is the only place Core IR types live. It has no dependency on
//! VEL syntax, GPUI, Wasmtime, or `FFmpeg`.

mod diagnostic;
mod edit;
mod ir;
mod locator;
mod locus;
mod provenance;
mod resolve;
mod span;
mod time;
mod time_map;
mod timeline;

pub use diagnostic::{Diagnostic, Severity};
pub use edit::{EditProposal, SemanticEdit};
pub use ir::{
    Audio, Media, Placement, PlacementKind, Project, Scene, Sequence, Source, TimeSpan, Visual,
};
pub use locator::MediaLocator;
pub use locus::{
    CoreProjection, Locus, LocusId, LocusKind, LocusProjection, SourceProjection,
    TimelineProjection, VisualProjection,
};
pub use provenance::{Origin, Provenance};
pub use resolve::{AssetIdentity, LockedAsset, ResolveLock, ResolvedAsset};
pub use span::Span;
pub use time::{Time, TimeError};
pub use time_map::{TimeMap, TimeMapError, TimeMapSegment};
pub use timeline::{Timeline, TimelineClip, TimelineError, TimelineSource, flatten_project};

#[cfg(test)]
mod crate_purity {
    #[test]
    fn cargo_toml_has_no_forbidden_runtime_deps() {
        let manifest = include_str!("../Cargo.toml");
        for forbidden in [
            "gpui",
            "wasmtime",
            "ffmpeg-next",
            "git2",
            "async-openai",
            "anthropic",
            "async_openai",
        ] {
            assert!(
                !manifest.contains(forbidden),
                "lattice-core must not depend on {forbidden}"
            );
        }
        assert!(
            !manifest.contains("lattice-vel"),
            "lattice-core must not depend on the VEL parser"
        );
    }
}
