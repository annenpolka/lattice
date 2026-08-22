//! Multi-cut export from imported media through `SemanticEdit`, not a hand-built filtergraph.

use lattice_core::{SemanticEdit, Time};
use lattice_engine::Engine;
use lattice_media::{
    content_pixels, extract_frame, extract_pcm_s16le_span, generate_av_fixture, has_audio_stream,
    mean_abs_diff, near_white_pixels, pcm_rms, probe_duration,
};

#[test]
#[allow(clippy::too_many_lines)]
fn export_matches_cuts_title_gain_and_source() {
    let dir = std::env::temp_dir().join("lattice-export-cuts");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    generate_av_fixture(&media, 8).unwrap();
    let engine = Engine::default();
    let imported = engine
        .import_media(&media, Some(&dir.join("proj")))
        .unwrap();
    let mut compilation = engine.compile_path(&imported.vel_path).unwrap();

    let scene = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Scene)
        .unwrap();
    let split_a = engine
        .propose(
            &compilation,
            &scene,
            SemanticEdit::Split {
                at: Time::seconds(2),
            },
        )
        .unwrap();
    compilation = engine
        .compile(
            &engine
                .apply_proposal(&compilation.source, &split_a)
                .unwrap(),
        )
        .unwrap();
    let second = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Scene && locus.label.contains('_'))
        .expect("second");
    let split_b = engine
        .propose(
            &compilation,
            &second,
            SemanticEdit::Split {
                at: Time::seconds(4),
            },
        )
        .unwrap();
    compilation = engine
        .compile(
            &engine
                .apply_proposal(&compilation.source, &split_b)
                .unwrap(),
        )
        .unwrap();
    let middle = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| {
            locus.kind == lattice_engine::LocusKind::Scene
                && compilation.project.scenes.iter().any(|scene| {
                    scene.id == locus.node_id
                        && scene
                            .sources
                            .iter()
                            .any(|s| s.source_range.start == Time::seconds(2))
                })
        })
        .expect("middle");
    let deleted = engine
        .propose(&compilation, &middle, SemanticEdit::Delete)
        .unwrap();
    compilation = engine
        .compile(
            &engine
                .apply_proposal(&compilation.source, &deleted)
                .unwrap(),
        )
        .unwrap();

    let first = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Scene)
        .unwrap();
    let titled = engine
        .propose(
            &compilation,
            &first,
            SemanticEdit::Title {
                text: Some("Hello".into()),
                at: Some(Time::ZERO),
                duration: Some(Time::seconds(2)),
                opacity: None,
            },
        )
        .unwrap();
    compilation = engine
        .compile(&engine.apply_proposal(&compilation.source, &titled).unwrap())
        .unwrap();
    let scene_names: Vec<String> = compilation
        .project
        .scenes
        .iter()
        .map(|scene| scene.name.clone())
        .collect();
    for name in scene_names {
        let scene = engine
            .loci(&compilation)
            .unwrap()
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Scene && locus.label == name)
            .expect("scene");
        let gained = engine
            .propose(&compilation, &scene, SemanticEdit::SetGain { db: -12 })
            .unwrap();
        compilation = engine
            .compile(&engine.apply_proposal(&compilation.source, &gained).unwrap())
            .unwrap();
    }
    assert!(
        compilation.source.matches("gain video by -12").count() >= 2,
        "gain should be on remaining clips:\n{}",
        compilation.source
    );

    let media_root = imported.vel_path.parent().unwrap();
    let out = dir.join("export.mp4");
    let report = engine
        .render(&compilation.project, &out, media_root)
        .expect("render edited project");
    assert!(out.is_file());
    let expected = Time::seconds(6);
    let probed = probe_duration(&out).unwrap();
    let delta = if probed > expected {
        probed - expected
    } else {
        expected - probed
    };
    assert!(
        delta < Time::milliseconds(150),
        "duration {} vs remaining cuts {}, report {}",
        probed,
        expected,
        report.duration
    );
    assert!(has_audio_stream(&out).unwrap());

    let out_at_3 = extract_frame(&out, Time::seconds(3), &dir.join("out3.ppm")).unwrap();
    let src_at_3 = extract_frame(&media, Time::seconds(3), &dir.join("src3.ppm")).unwrap();
    let src_at_5 = extract_frame(&media, Time::seconds(5), &dir.join("src5.ppm")).unwrap();
    let out_px = content_pixels(&out_at_3).unwrap();
    let src3 = content_pixels(&src_at_3).unwrap();
    let src5 = content_pixels(&src_at_5).unwrap();
    assert!(
        mean_abs_diff(&out_px, &src5) + 4 < mean_abs_diff(&out_px, &src3)
            || mean_abs_diff(&out_px, &src5) < 12,
        "timeline 3s should show source 5s (deleted 2s-4s), not source 3s"
    );

    let title_frame = extract_frame(&out, Time::seconds(1), &dir.join("hello.ppm")).unwrap();
    let whites = near_white_pixels(&title_frame, 0.65, 1.0).unwrap();
    assert!(
        whites > 10,
        "title Hello should paint light glyphs, got {whites} near-white pixels"
    );

    let gained_pcm = extract_pcm_s16le_span(&out, None).unwrap();
    let control = dir.join("ungained.mp4");
    let control_vel = compilation
        .source
        .replace("gain video by -12", "gain video by 0");
    assert!(
        !control_vel.contains("gain video by -12"),
        "control still has -12:\n{control_vel}"
    );
    let control_c = engine.compile(&control_vel).unwrap();
    engine
        .render(&control_c.project, &control, media_root)
        .expect("control render");
    let control_pcm = extract_pcm_s16le_span(&control, None).unwrap();
    let gained_level = pcm_rms(&gained_pcm);
    let control_level = pcm_rms(&control_pcm);
    assert!(
        gained_level + 50 < control_level,
        "gain -12dB should lower level ({gained_level} vs {control_level})"
    );
}
