//! `lattice render` of a SemanticEdit-built multi-cut project, twice.

use std::path::{Path, PathBuf};
use std::process::Command;

use lattice_core::{SemanticEdit, Time};
use lattice_engine::Engine;
use lattice_media::{
    content_pixels, extract_frame, extract_pcm_s16le_span, generate_av_fixture, has_audio_stream,
    mean_abs_diff, near_white_pixels, pcm_rms, probe_duration,
};

fn lattice_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lattice"))
}

fn render_ok(vel: &Path, out: &Path) -> String {
    let output = Command::new(lattice_bin())
        .args([
            "--json",
            "render",
            vel.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("spawn lattice render");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "lattice render failed: {stderr}\n{stdout}"
    );
    stdout
}

#[allow(clippy::too_many_lines)]
fn edit_imported_project(dir: &Path) -> PathBuf {
    let media = dir.join("gameplay.mp4");
    generate_av_fixture(&media, 8).unwrap();
    let engine = Engine::default();
    let imported = engine
        .import_media(&media, Some(&dir.join("proj")))
        .unwrap();
    assert!(
        imported.locator == "../gameplay.mp4" || imported.locator.ends_with("gameplay.mp4"),
        "unexpected locator {} vel {}",
        imported.locator,
        imported.vel_path.display()
    );
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
                            .any(|source| source.source_range.start == Time::seconds(2))
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
    engine
        .write_source_atomic(&imported.vel_path, &compilation.source)
        .unwrap();
    if let Some(parent) = imported.vel_path.parent() {
        let beside = parent.join("gameplay.mp4");
        if !beside.is_file() {
            let _ = std::fs::copy(&media, beside);
        }
    }
    imported.vel_path
}

#[test]
#[allow(clippy::too_many_lines)]
fn lattice_render_edited_cuts_twice() {
    let dir = std::env::temp_dir().join("lattice-cli-export-edited");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = edit_imported_project(&dir);
    let out1 = dir.join("export-1.mp4");
    let out2 = dir.join("export-2.mp4");
    let log1 = render_ok(&vel, &out1);
    let log2 = render_ok(&vel, &out2);
    assert!(out1.is_file());
    assert!(out2.is_file());
    let d1 = probe_duration(&out1).unwrap();
    let d2 = probe_duration(&out2).unwrap();
    assert_eq!(d1, d2);
    let expected = Time::seconds(6);
    let delta = if d1 > expected {
        d1 - expected
    } else {
        expected - d1
    };
    assert!(
        delta < Time::milliseconds(150),
        "duration {d1} vs remaining cuts {expected}"
    );
    assert!(has_audio_stream(&out1).unwrap());

    let media = dir.join("gameplay.mp4");
    let out_at_3 = extract_frame(&out1, Time::seconds(3), &dir.join("out3.ppm")).unwrap();
    let src_at_3 = extract_frame(&media, Time::seconds(3), &dir.join("src3.ppm")).unwrap();
    let src_at_5 = extract_frame(&media, Time::seconds(5), &dir.join("src5.ppm")).unwrap();
    let out_px = content_pixels(&out_at_3).unwrap();
    let src3 = content_pixels(&src_at_3).unwrap();
    let src5 = content_pixels(&src_at_5).unwrap();
    assert!(
        mean_abs_diff(&out_px, &src5) + 4 < mean_abs_diff(&out_px, &src3)
            || mean_abs_diff(&out_px, &src5) < 12,
        "timeline 3s should show source 5s, not deleted 3s"
    );
    let title_frame = extract_frame(&out1, Time::seconds(1), &dir.join("hello.ppm")).unwrap();
    let whites = near_white_pixels(&title_frame, 0.65, 1.0).unwrap();
    assert!(whites > 10, "Hello glyphs missing ({whites} white pixels)");

    let gained = pcm_rms(&extract_pcm_s16le_span(&out1, None).unwrap());
    let control_vel = std::fs::read_to_string(&vel)
        .unwrap()
        .replace("gain video by -12", "gain video by 0");
    let control_path = vel.parent().unwrap().join("control.vel");
    std::fs::write(&control_path, control_vel).unwrap();
    let control_out = dir.join("control.mp4");
    render_ok(&control_path, &control_out);
    let control = pcm_rms(&extract_pcm_s16le_span(&control_out, None).unwrap());
    assert!(
        gained + 50 < control,
        "gain -12dB should lower level ({gained} vs {control})"
    );

    if let Some(evidence) = std::env::var_os("LATTICE_EVIDENCE_DIR") {
        let evidence = PathBuf::from(evidence);
        if evidence.is_dir() {
            std::fs::copy(&out1, evidence.join("export-1.mp4")).unwrap();
            std::fs::copy(&out2, evidence.join("export-2.mp4")).unwrap();
            std::fs::write(evidence.join("render-1.log"), &log1).unwrap();
            std::fs::write(evidence.join("render-2.log"), &log2).unwrap();
        }
    }
}
