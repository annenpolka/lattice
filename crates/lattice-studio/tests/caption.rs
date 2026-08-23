//! CHI-84: caption stays `LocusKind::Placement` so Title inspector does not fire.

use lattice_engine::LocusKind;
use lattice_studio::StudioSession;

fn unique_dir(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-caption-{tag}-{nanos}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn caption_locus_does_not_open_title_inspector() {
    let dir = unique_dir("inspector");
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 4).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "caption-inspector"
convention commentary
media game "capture.mp4"
sequence main {
  demo
}
scene demo {
  game[0s..4s] as clip
  caption "cue" at 0s for 2s
  title "Hello" {
    at 2s for 2s
  }
}
"#,
    )
    .unwrap();
    let mut session = StudioSession::open(&vel).expect("open");
    let caption = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| {
            matches!(
                locus.provenance.origin,
                lattice_engine::Origin::Invocation { ref command } if command == "caption"
            )
        })
        .expect("caption locus");
    assert_eq!(caption.kind, LocusKind::Placement);
    session.point_at(caption.id);
    let layout = session.layout().unwrap();
    assert!(
        !layout.inspector.title_fields,
        "Title inspector keys on LocusKind::Title"
    );
    assert!(
        layout.inspector.heading.starts_with("placement "),
        "heading: {}",
        layout.inspector.heading
    );
}
