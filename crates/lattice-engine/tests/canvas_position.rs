use lattice_engine::{
    Canvas, Engine, LocusKind, NormalizedPosition, NormalizedScale, SemanticEdit, evaluate_at,
};

const SOURCE: &str = r#"project "canvas-position"
media game "capture.mp4"
sequence main { intro }
scene intro {
  game[0s..4s] as clip
  title "Hello" {
    at 0s for 3s
  }
  callout "Look" {
    at 0s for 3s
    position (100%, 0%)
  }
}
"#;

#[test]
fn position_edit_roundtrips_source_core_locus_timeline_and_evaluate() {
    let engine = Engine::default();
    let compilation = engine
        .compile_origin(SOURCE, Some("main.vel".into()))
        .unwrap();
    let title = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .expect("title locus");
    let wanted = NormalizedPosition::new(2_500, 1_000).unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::SetPosition { position: wanted },
        )
        .unwrap();
    assert!(proposal.new_source.contains("position (25%, 10%)"));

    let next = engine
        .compile_origin(&proposal.new_source, Some("main.vel".into()))
        .unwrap();
    let next_title = engine
        .loci(&next)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .expect("recompiled title locus");
    assert_eq!(next_title.id, title.id, "shared locus must survive rewrite");
    assert_eq!(
        next_title
            .visual
            .as_ref()
            .and_then(|visual| visual.position),
        Some(wanted)
    );
    assert!(
        next.explain
            .iter()
            .any(|event| event.message.contains("position (25.00%, 10.00%)")),
        "spatial magic must be explainable: {:?}",
        next.explain
    );

    let timeline = Engine::timeline(&next.project).unwrap();
    let title_clip = timeline.title_clips().next().unwrap();
    assert_eq!(title_clip.position, Some(wanted));
    let scene = evaluate_at(&timeline, title_clip.span.start, Canvas::PREVIEW).unwrap();
    let title_transform = scene
        .nodes
        .iter()
        .find(|node| node.z() == 10)
        .map(|node| node.props().transform);
    assert!(title_transform.is_some());
}

#[test]
fn existing_generic_position_tuple_is_replaced_without_parser_vocabulary() {
    let engine = Engine::default();
    let compilation = engine.compile_origin(SOURCE, None).unwrap();
    let callout = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Callout)
        .unwrap();
    assert_eq!(
        callout.visual.as_ref().and_then(|visual| visual.position),
        NormalizedPosition::new(10_000, 0)
    );
    let proposal = engine
        .propose(
            &compilation,
            &callout,
            SemanticEdit::SetPosition {
                position: NormalizedPosition::new(1_250, 8_750).unwrap(),
            },
        )
        .unwrap();
    assert!(proposal.new_source.contains("position (12.5%, 87.5%)"));
    assert_eq!(proposal.new_source.matches("position ").count(), 1);
}

#[test]
fn resize_edit_atomically_roundtrips_position_and_generic_scale() {
    let engine = Engine::default();
    let compilation = engine
        .compile_origin(SOURCE, Some("main.vel".into()))
        .unwrap();
    let title = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .unwrap();
    let position = NormalizedPosition::new(1_250, 2_500).unwrap();
    let scale = NormalizedScale::new(750).unwrap();
    let proposal = engine
        .propose(
            &compilation,
            &title,
            SemanticEdit::ResizeOverlay { position, scale },
        )
        .unwrap();
    assert!(proposal.new_source.contains("position (12.5%, 25%)"));
    assert!(proposal.new_source.contains("scale 75%"));
    let next = engine
        .compile_origin(&proposal.new_source, Some("main.vel".into()))
        .unwrap();
    let rebound = engine
        .loci(&next)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Title)
        .unwrap();
    assert_eq!(rebound.id, title.id);
    let visual = rebound.visual.unwrap();
    assert_eq!(visual.position, Some(position));
    assert_eq!(visual.scale, Some(scale));
    let timeline = Engine::timeline(&next.project).unwrap();
    let clip = timeline.title_clips().next().unwrap();
    assert_eq!((clip.position, clip.scale), (Some(position), Some(scale)));
    assert!(
        next.explain
            .iter()
            .any(|event| event.message.contains("scale 75%"))
    );

    let callout = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Callout)
        .unwrap();
    let callout_resize = engine
        .propose(
            &compilation,
            &callout,
            SemanticEdit::ResizeOverlay {
                position: NormalizedPosition::new(5_000, 5_000).unwrap(),
                scale: NormalizedScale::new(1_250).unwrap(),
            },
        )
        .unwrap();
    assert_eq!(callout_resize.new_source.matches("position ").count(), 1);
    assert_eq!(callout_resize.new_source.matches("scale ").count(), 1);
    let callout_next = engine.compile(&callout_resize.new_source).unwrap();
    let callout_visual = engine
        .loci(&callout_next)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Callout)
        .unwrap()
        .visual
        .unwrap();
    assert_eq!(callout_visual.scale, NormalizedScale::new(1_250));
}

#[test]
fn normalized_x_move_changes_composited_pixels_end_to_end() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("lattice-canvas-position-{nonce}"));
    std::fs::create_dir_all(&root).unwrap();
    lattice_engine::generate_av_fixture(root.join("capture.mp4"), 2).unwrap();
    let source_at = |x: u16| {
        format!(
            r#"project "pixel-position"
media game "capture.mp4"
sequence main {{ intro }}
scene intro {{
  game[0s..2s] as clip
  title "X" {{
    at 0s for 2s
    position ({x}%, 100%)
  }}
}}
"#
        )
    };
    let engine = Engine::default();
    let left = engine.compile(&source_at(0)).unwrap();
    let right = engine.compile(&source_at(100)).unwrap();
    let scaled = engine
        .compile(&source_at(0).replace("position (0%, 100%)", "position (0%, 100%)\n    scale 50%"))
        .unwrap();
    let request = lattice_engine::PreviewFrameRequest {
        timeline_time: lattice_engine::Time::seconds(1),
        width: 320,
        height: 180,
        fps_num: 10,
        fps_den: 1,
    };
    let (_, left_frame) = engine
        .sample_frame(&left.project, &request, &root, None)
        .unwrap();
    let (_, right_frame) = engine
        .sample_frame(&right.project, &request, &root, None)
        .unwrap();
    let (_, scaled_frame) = engine
        .sample_frame(&scaled.project, &request, &root, None)
        .unwrap();
    let y = 176;
    let yellow = |pixel: [u8; 4]| pixel[0] > 200 && pixel[1] > 180 && pixel[2] < 80;
    assert!(yellow(left_frame.pixel(10, y).unwrap()));
    assert!(!yellow(right_frame.pixel(10, y).unwrap()));
    assert!(!yellow(left_frame.pixel(310, y).unwrap()));
    assert!(yellow(right_frame.pixel(310, y).unwrap()));
    assert!(yellow(scaled_frame.pixel(100, y).unwrap()));
    assert!(!yellow(scaled_frame.pixel(200, y).unwrap()));
    let _ = std::fs::remove_dir_all(root);
}
