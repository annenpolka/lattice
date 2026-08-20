//! Propose → inspect → Apply / Reject against the shipped VEL, via Engine.

use lattice_core::{SemanticEdit, Time, source_revision};
use lattice_engine::{Engine, write_source_atomic, write_source_atomic_no_commit};

const VEL: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

#[test]
fn propose_inspect_reject_leaves_source_bytes_unchanged() {
    let dir = std::env::temp_dir().join("lattice-edit-reject");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.vel");
    std::fs::write(&path, VEL).unwrap();
    let original = std::fs::read(&path).unwrap();

    let engine = Engine::default();
    let compilation = engine.compile_path(&path).unwrap();
    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::Title {
                text: Some("World".into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )
        .unwrap();
    assert!(proposal.vel_diff.contains("World"), "{}", proposal.vel_diff);
    assert!(proposal.new_source.contains("World"));
    assert_eq!(proposal.base_revision, source_revision(VEL));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "proposal must not write VEL"
    );

    let rejected = engine.reject_proposal(&compilation.source, &proposal);
    assert_eq!(rejected.as_bytes(), original);
    std::fs::write(&path, rejected).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn propose_apply_rewrites_source_and_recompile_reflects_title() {
    let dir = std::env::temp_dir().join("lattice-edit-apply");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.vel");
    std::fs::write(&path, VEL).unwrap();

    let engine = Engine::default();
    let compilation = engine.compile_path(&path).unwrap();
    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::Title {
                text: Some("World".into()),
                at: Some(Time::seconds(1)),
                duration: Some(Time::seconds(4)),
                opacity: Some(80),
            },
        )
        .unwrap();
    assert!(proposal.description.contains("World"));

    let applied = engine
        .apply_proposal(&compilation.source, &proposal)
        .unwrap();
    engine.write_source_atomic(&path, &applied).unwrap();
    assert_ne!(applied, VEL);
    assert!(applied.contains("World"));
    assert!(!applied.contains("title \"Hello\""));

    let recompiled = engine.compile_path(&path).unwrap();
    assert!(!recompiled.has_errors(), "{:?}", recompiled.diagnostics);
    let timeline = Engine::timeline(&recompiled.project).unwrap();
    let title = timeline.title_clips().next().expect("title");
    assert_eq!(title.text.as_deref(), Some("World"));
    assert_eq!(title.span.start, Time::seconds(1));
    assert_eq!(title.span.duration, Time::seconds(4));
    assert_eq!(title.opacity, Some(80));
}

#[test]
fn stale_proposal_is_rejected_and_does_not_overwrite() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).unwrap();
    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::Title {
                text: Some("World".into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )
        .unwrap();
    let changed = VEL.replace("title \"Hello\"", "title \"Later\"");
    let err = engine
        .apply_proposal(&changed, &proposal)
        .expect_err("stale");
    assert!(
        err.stale_revisions().is_some(),
        "expected structured stale error, got {err}"
    );
    assert!(changed.contains("Later"));
    assert!(!changed.contains("World"));
}

#[test]
fn missing_title_locus_does_not_edit_first_title() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).unwrap();
    let mut bogus = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    bogus.source_span = Some(lattice_core::Span::new(0, 1, 1, 1));
    bogus.id = lattice_core::LocusId::new("missing-title");
    let err = engine
        .propose(
            &compilation,
            &bogus,
            SemanticEdit::Title {
                text: Some("Nope".into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )
        .expect_err("stale locus");
    assert!(
        err.to_string().contains("title locus did not match"),
        "{err}"
    );
}

#[test]
fn title_edit_rebinds_same_locus_across_source_core_timeline() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).unwrap();
    let title = engine
        .locus_for_node(&compilation, "demo:title:1")
        .unwrap()
        .unwrap();
    let original_id = title.id.clone();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::Title {
                text: Some("World".into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )
        .unwrap();
    let applied = engine
        .apply_proposal(&compilation.source, &proposal)
        .unwrap();
    let recompiled = engine.compile(&applied).unwrap();
    let again = engine
        .locus_for_node(&recompiled, "demo:title:1")
        .unwrap()
        .expect("same node");
    assert_eq!(again.id, original_id);
    let span = again.source_span.expect("span");
    let mid = span.start + (span.end - span.start) / 2;
    let from_source = engine
        .locus_at_source(&recompiled, mid)
        .unwrap()
        .expect("source");
    assert_eq!(from_source.id, original_id);
    let from_timeline = engine
        .locus_at_timeline(&recompiled, Time::seconds(3))
        .unwrap()
        .expect("timeline");
    assert_eq!(from_timeline.id, original_id);
    let projection = engine.inspect(&recompiled, &original_id).unwrap();
    assert_eq!(projection.core.node_id, "demo:title:1");
    assert_eq!(
        projection.timeline.expect("range").clip_id,
        projection.core.node_id
    );
}

#[test]
fn atomic_write_failure_does_not_truncate_live_vel() {
    let dir = std::env::temp_dir().join("lattice-atomic-vel");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.vel");
    std::fs::write(&path, "source B\n").unwrap();
    let err = write_source_atomic_no_commit(&path, "truncated-new-source").unwrap_err();
    assert!(err.to_string().contains("simulated"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "source B\n");
    write_source_atomic(&path, "source C\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "source C\n");
}

#[test]
#[allow(clippy::too_many_lines)]
fn trim_split_delete_gain_fade_round_trip() {
    let vel = r#"project "cut"

convention commentary

media game "capture.mp4"

sequence main {
  clip
}

scene clip {
  game[0s..20s] as video
}
"#;
    let engine = Engine::default();
    let compilation = engine.compile(vel).unwrap();
    let source = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_core::LocusKind::Source)
        .expect("source");

    let trimmed = engine
        .propose(
            &compilation,
            &source,
            SemanticEdit::Trim {
                in_point: Some(Time::seconds(0)),
                out_point: Some(Time::seconds(16)),
            },
        )
        .unwrap();
    let compilation = engine
        .compile(&engine.apply_proposal(vel, &trimmed).unwrap())
        .unwrap();
    assert!(compilation.source.contains("game[0s..16s]"));

    let split = engine
        .propose(
            &compilation,
            &engine
                .loci(&compilation)
                .unwrap()
                .into_iter()
                .find(|locus| locus.kind == lattice_core::LocusKind::Scene)
                .unwrap(),
            SemanticEdit::Split {
                at: Time::seconds(8),
            },
        )
        .unwrap();
    let compilation = engine
        .compile(&engine.apply_proposal(&compilation.source, &split).unwrap())
        .unwrap();
    assert!(compilation.source.contains("game[0s..8s]"));
    assert!(compilation.source.contains("game[8s..16s]"));
    assert_eq!(compilation.project.scenes.len(), 2);
    assert_eq!(compilation.project.sequences[0].scene_ids.len(), 2);

    let second = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| {
            locus.kind == lattice_core::LocusKind::Scene && locus.label.contains("clip_2")
        })
        .expect("second scene");
    let deleted = engine
        .propose(&compilation, &second, SemanticEdit::Delete)
        .unwrap();
    let compilation = engine
        .compile(
            &engine
                .apply_proposal(&compilation.source, &deleted)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(compilation.project.scenes.len(), 1);
    assert!(!compilation.source.contains("clip_2"));

    let scene = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_core::LocusKind::Scene)
        .unwrap();
    let titled = engine
        .propose(
            &compilation,
            &scene,
            SemanticEdit::Title {
                text: Some("Hello".into()),
                at: Some(Time::seconds(0)),
                duration: Some(Time::seconds(2)),
                opacity: None,
            },
        )
        .unwrap();
    let compilation = engine
        .compile(&engine.apply_proposal(&compilation.source, &titled).unwrap())
        .unwrap();
    assert!(compilation.source.contains("title \"Hello\""));

    let scene = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_core::LocusKind::Scene)
        .unwrap();
    let gained = engine
        .propose(&compilation, &scene, SemanticEdit::SetGain { db: -6 })
        .unwrap();
    let compilation = engine
        .compile(&engine.apply_proposal(&compilation.source, &gained).unwrap())
        .unwrap();
    assert!(compilation.source.contains("gain video by -6"));
    let timeline = Engine::timeline(&compilation.project).unwrap();
    assert!(timeline.audio_clips().any(|clip| clip.gain_db == Some(-6)));

    let scene = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_core::LocusKind::Scene)
        .unwrap();
    let faded = engine
        .propose(
            &compilation,
            &scene,
            SemanticEdit::SetFade {
                fade_in: Some(Time::from_decimal_seconds(0, 5, 1).unwrap()),
            },
        )
        .unwrap();
    let compilation = engine
        .compile(&engine.apply_proposal(&compilation.source, &faded).unwrap())
        .unwrap();
    assert!(compilation.source.contains("fade video"));
    let timeline = Engine::timeline(&compilation.project).unwrap();
    assert!(timeline.video_clips().any(|clip| clip.fade_in.is_some()));
}
