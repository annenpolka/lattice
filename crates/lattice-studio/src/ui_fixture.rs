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
        let root = std::env::var_os("LATTICE_STUDIO_FIXTURE_DIR").map_or_else(
            || std::env::temp_dir().join("lattice-studio-ui-fixtures"),
            PathBuf::from,
        );
        self.materialize_in(&root)
    }

    pub fn materialize_in(self, root: &Path) -> std::io::Result<PathBuf> {
        let dir = root.join(self.as_str());
        std::fs::create_dir_all(&dir)?;
        let vel = dir.join("main.vel");
        std::fs::write(&vel, self.source())?;
        Ok(vel)
    }

    /// Pinned initial semantic/layout facts. CHI-65: not only two-run equality.
    #[must_use]
    pub fn expected_initial(self) -> ExpectedInitial {
        match self {
            Self::TimelineBasic => ExpectedInitial {
                project: "timeline-basic",
                playhead: "0s",
                duration: "4s",
                locus_id: "demo:title:1",
                locus_kind: "title",
                clip_ids: &[
                    "demo:video:3",
                    "demo:audio:4",
                    "demo:title:1",
                    "demo:callout:2",
                ],
                track_count: 4,
            },
            Self::DragValid => ExpectedInitial {
                project: "drag-valid",
                playhead: "0s",
                duration: "4s",
                locus_id: "left:title:1",
                locus_kind: "title",
                clip_ids: &[
                    "left:video:2",
                    "left:audio:3",
                    "left:title:1",
                    "right:video:2",
                    "right:audio:3",
                    "right:title:1",
                ],
                track_count: 4,
            },
            Self::DragInvalid => ExpectedInitial {
                project: "drag-invalid",
                playhead: "0s",
                duration: "0.5s",
                locus_id: "pinned:title:1",
                locus_kind: "title",
                clip_ids: &["pinned:video:2", "pinned:audio:3", "pinned:title:1"],
                track_count: 4,
            },
            Self::DenseProject => ExpectedInitial {
                project: "dense-project",
                playhead: "0s",
                duration: "4s",
                locus_id: "one:title:1",
                locus_kind: "title",
                clip_ids: &[
                    "one:title:1",
                    "one:callout:2",
                    "two:title:1",
                    "three:title:1",
                    "four:title:1",
                ],
                track_count: 4,
            },
        }
    }
}

/// Expected initial fixture facts for agent/oracle pins.
#[derive(Clone, Copy, Debug)]
pub struct ExpectedInitial {
    pub project: &'static str,
    pub playhead: &'static str,
    pub duration: &'static str,
    pub locus_id: &'static str,
    pub locus_kind: &'static str,
    pub clip_ids: &'static [&'static str],
    pub track_count: usize,
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

    #[test]
    fn fixtures_pin_expected_ids_durations_and_layout() {
        for fixture in UiFixture::ALL {
            let session = open_fixture(fixture, "pin");
            let expected = fixture.expected_initial();
            let state = session.semantic_state();
            let layout = session.layout().expect("layout");
            assert_eq!(layout.project_name, expected.project, "{fixture} project");
            assert_eq!(
                state["playhead"], expected.playhead,
                "{fixture} playhead: {state}"
            );
            assert_eq!(
                state["duration"], expected.duration,
                "{fixture} duration: {state}"
            );
            assert_eq!(
                state["locus"]["id"], expected.locus_id,
                "{fixture} locus id: {state}"
            );
            assert_eq!(
                state["locus"]["kind"], expected.locus_kind,
                "{fixture} locus kind: {state}"
            );
            assert_eq!(
                layout.timeline.duration.to_string(),
                expected.duration,
                "{fixture} layout duration"
            );
            let clip_ids: Vec<String> = layout
                .timeline
                .tracks
                .iter()
                .flat_map(|track| track.clips.iter().map(|clip| clip.id.clone()))
                .collect();
            for id in expected.clip_ids {
                assert!(
                    clip_ids.iter().any(|clip| clip == id),
                    "{fixture} missing clip {id} in {clip_ids:?}"
                );
            }
            assert_eq!(
                layout.timeline.tracks.len(),
                expected.track_count,
                "{fixture} track count"
            );
        }
    }

    #[test]
    fn inflight_scrub_exposes_gesture_before_commit_resets() {
        let mut session = open_fixture(UiFixture::TimelineBasic, "inflight");
        session.set_rail_width(640.0);
        session.begin_timeline_scrub(80.0, true);
        let inflight = session.semantic_state();
        assert_eq!(inflight["interaction"], "scrub", "{inflight}");
        assert_eq!(inflight["gesture"]["kind"], "scrub", "{inflight}");
        assert_ne!(
            inflight["playhead"], "0s",
            "scrub must move playhead: {inflight}"
        );
        session
            .update_timeline_pointer(400.0, true)
            .expect("update");
        let updated = session.semantic_state();
        assert_eq!(updated["interaction"], "scrub", "{updated}");
        assert_eq!(updated["gesture"]["kind"], "scrub", "{updated}");
        session
            .commit_timeline_pointer_snap(400.0, true)
            .expect("commit");
        let committed = session.semantic_state();
        assert_eq!(committed["interaction"], "idle", "{committed}");
        assert_eq!(committed["gesture"]["kind"], "none", "{committed}");
        assert!(committed["drag"].is_null(), "{committed}");
    }

    #[test]
    fn inflight_overlay_drag_exposes_source_target_validity() {
        let mut session = open_fixture(UiFixture::TimelineBasic, "drag");
        session.set_rail_width(640.0);
        let layout = session.layout().expect("layout");
        let title = layout
            .timeline
            .tracks
            .iter()
            .flat_map(|track| track.clips.iter())
            .find(|clip| clip.id == "demo:title:1")
            .expect("title clip");
        let x = session.x_at_time(title.start) + 24.0;
        session
            .begin_timeline_pointer_on(x, true, "text")
            .expect("begin overlay");
        let inflight = session.semantic_state();
        assert_ne!(inflight["gesture"]["kind"], "none", "{inflight}");
        assert_ne!(inflight["interaction"], "idle", "{inflight}");
        if !inflight["drag"].is_null() {
            assert!(inflight["drag"]["source"].is_string(), "{inflight}");
            assert!(inflight["drag"].get("target").is_some(), "{inflight}");
            assert!(inflight["drag"]["valid"].is_boolean(), "{inflight}");
        }
        session
            .update_timeline_pointer(x + 80.0, true)
            .expect("update");
        let updated = session.semantic_state();
        assert_ne!(updated["gesture"]["kind"], "none", "{updated}");
        session
            .commit_timeline_pointer_snap(x + 80.0, true)
            .expect("commit");
        let committed = session.semantic_state();
        assert_eq!(committed["gesture"]["kind"], "none", "{committed}");
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
