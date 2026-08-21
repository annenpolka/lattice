//! Deterministic Studio UI fixtures for agent smoke.
//!
//! These are ordinary VEL documents opened through [`crate::StudioSession`].
//! They are not a second project model.

use std::path::{Path, PathBuf};

/// Named UI fixture. CLI shape is `--ui-fixture <name>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiFixture {
    TimelineBasic,
    DragValid,
    DragInvalid,
    DenseProject,
}

impl UiFixture {
    pub const ALL: [Self; 4] = [
        Self::TimelineBasic,
        Self::DragValid,
        Self::DragInvalid,
        Self::DenseProject,
    ];

    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name.trim().to_ascii_lowercase().as_str() {
            "timeline-basic" | "timeline_basic" => Self::TimelineBasic,
            "drag-valid" | "drag_valid" => Self::DragValid,
            "drag-invalid" | "drag_invalid" => Self::DragInvalid,
            "dense-project" | "dense_project" => Self::DenseProject,
            _ => return None,
        })
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TimelineBasic => "timeline-basic",
            Self::DragValid => "drag-valid",
            Self::DragInvalid => "drag-invalid",
            Self::DenseProject => "dense-project",
        }
    }

    #[must_use]
    pub fn source(self) -> &'static str {
        match self {
            Self::TimelineBasic => {
                include_str!("../../../fixtures/studio-ui/timeline-basic/main.vel")
            }
            Self::DragValid => include_str!("../../../fixtures/studio-ui/drag-valid/main.vel"),
            Self::DragInvalid => include_str!("../../../fixtures/studio-ui/drag-invalid/main.vel"),
            Self::DenseProject => {
                include_str!("../../../fixtures/studio-ui/dense-project/main.vel")
            }
        }
    }

    /// Write the fixture VEL to a stable directory and return `main.vel`.
    pub fn materialize(self) -> std::io::Result<PathBuf> {
        let root = std::env::var_os("LATTICE_STUDIO_FIXTURE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("lattice-studio-ui-fixtures"));
        self.materialize_in(&root)
    }

    pub fn materialize_in(self, root: &Path) -> std::io::Result<PathBuf> {
        let dir = root.join(self.as_str());
        std::fs::create_dir_all(&dir)?;
        let vel = dir.join("main.vel");
        std::fs::write(&vel, self.source())?;
        Ok(vel)
    }
}

impl std::fmt::Display for UiFixture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::UiFixture;
    use crate::StudioSession;

    #[test]
    fn parse_accepts_documented_names() {
        assert_eq!(
            UiFixture::parse("timeline-basic"),
            Some(UiFixture::TimelineBasic)
        );
        assert_eq!(UiFixture::parse("DRAG_VALID"), Some(UiFixture::DragValid));
        assert!(UiFixture::parse("windows-dogfood").is_none());
    }

    #[test]
    fn fixtures_open_to_the_same_initial_semantic_state() {
        for fixture in UiFixture::ALL {
            let first = open_fixture(fixture, "a");
            let second = open_fixture(fixture, "b");
            let left = first.semantic_state();
            let right = second.semantic_state();
            assert_eq!(
                left["playhead"], right["playhead"],
                "{fixture} playhead must be stable"
            );
            assert_eq!(
                left["locus"], right["locus"],
                "{fixture} locus must be stable"
            );
            assert_eq!(
                left["interaction"], "idle",
                "{fixture} must start idle: {left}"
            );
            assert_eq!(left["playing"], false);
            assert!(left["drag"].is_null(), "{fixture} must start with no drag");
            assert!(
                !first.compilation().has_errors(),
                "{fixture} diagnostics: {:?}",
                first.diagnostics()
            );
        }
    }

    fn open_fixture(fixture: UiFixture, tag: &str) -> StudioSession {
        let root = std::env::temp_dir().join(format!(
            "lattice-ui-fixture-{}-{tag}-{}",
            fixture.as_str(),
            std::process::id()
        ));
        let vel = fixture.materialize_in(&root).expect("materialize");
        StudioSession::open(vel).expect("open fixture")
    }
}
