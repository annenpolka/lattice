//! Drive shipped compile → flatten → evaluate(t). Not a reimplementation.

use lattice_core::{Canvas, RenderNode, Time, evaluate_at};
use lattice_engine::Engine;

const VEL: &str = include_str!("../../../examples/gameplay-commentary/main.vel");

fn title_text(scene: &lattice_core::RenderScene) -> Vec<String> {
    fn walk(nodes: &[RenderNode], out: &mut Vec<String>) {
        for node in nodes {
            match node {
                RenderNode::Text(text) => out.push(text.text.clone()),
                RenderNode::Group(group) => walk(&group.children, out),
                RenderNode::Mask(mask) => {
                    walk(std::slice::from_ref(mask.content.as_ref()), out);
                }
                RenderNode::Effect(effect) => {
                    walk(std::slice::from_ref(effect.child.as_ref()), out);
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&scene.nodes, &mut out);
    out
}

fn text_fonts(nodes: &[RenderNode]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        match node {
            RenderNode::Text(text) => {
                out.push(
                    text.resolved_font
                        .as_ref()
                        .map(|id| id.path.clone())
                        .unwrap_or_default(),
                );
            }
            RenderNode::Group(group) => out.extend(text_fonts(&group.children)),
            _ => {}
        }
    }
    out
}

#[test]
fn compile_then_evaluate_is_deterministic() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let timeline = Engine::timeline(&compilation.project).expect("flatten");
    let t = Time::seconds(3);
    let a = evaluate_at(&timeline, t, Canvas::PREVIEW).expect("eval a");
    let b = evaluate_at(&timeline, t, Canvas::PREVIEW).expect("eval b");
    assert_eq!(a, b);
    assert!(
        title_text(&a).iter().any(|text| text == "Hello"),
        "title at 3s: {:?}",
        title_text(&a)
    );
}

#[test]
fn engine_sample_session_preserves_required_gpu_selection() {
    let engine = Engine::default();
    let compilation = engine.compile(VEL).expect("compile");
    let root = std::env::temp_dir().join("lattice-engine-gpu-selection");
    let result = engine.sample_session(
        &compilation.project,
        &lattice_engine::PreviewFrameRequest {
            timeline_time: Time::ZERO,
            width: 320,
            height: 180,
            fps_num: 10,
            fps_den: 1,
        },
        &root,
        &root.join("unused-output"),
        None,
        lattice_engine::RendererRequest::RequireGpuDx12,
    );
    match result {
        Ok(session) => assert_eq!(
            session.selection().active,
            Some(lattice_engine::RendererBackend::GpuDx12),
        ),
        Err(lattice_engine::EngineError::Export(lattice_engine::ExportError::Renderer(error))) => {
            assert_eq!(
                error.selection().requested,
                lattice_engine::RendererRequest::RequireGpuDx12
            );
            assert!(error.selection().active.is_none());
        }
        Err(error) => panic!("unexpected error: {error}"),
    }
}

#[test]
fn preview_and_export_share_sample_api() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    let timeline = Engine::timeline(&compilation.project).expect("flatten");
    let dir = std::env::temp_dir().join("lattice-parity-sample");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 21).expect("fixture");
    let t = Time::seconds(3);
    let options = lattice_engine::PreviewOptions {
        output: dir.join("parity.ppm"),
        media_root: dir.clone(),
        lock: None,
        spec: lattice_engine::OutputSpec::preview(),
        renderer: lattice_engine::RendererRequest::RequireCpu,
        allow_fixtures: false,
        font: None,
    };
    let spec = lattice_media::OutputSpec::preview();
    let (scene_a, frame_a) =
        lattice_media::sample_frame(&timeline, t, spec, &options).expect("sample a");
    let (scene_b, frame_b) =
        lattice_media::sample_frame(&timeline, t, spec, &options).expect("sample b");
    assert_eq!(scene_a, scene_b);
    let mismatch = frame_a
        .rgba
        .iter()
        .zip(&frame_b.rgba)
        .filter(|(a, b)| a.abs_diff(**b) > 8)
        .count();
    assert!(
        mismatch < 64,
        "preview/export sample frames should match within GPU tolerance, mismatches={mismatch}"
    );
    assert!(
        frame_a.rgba.iter().any(|b| *b > 40),
        "composited frame must not be blank"
    );
}

#[test]
fn sample_frame_has_yellow_title_bar() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    let timeline = Engine::timeline(&compilation.project).expect("flatten");
    let dir = std::env::temp_dir().join("lattice-title-bar");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 21).expect("fixture");
    let options = lattice_engine::PreviewOptions {
        output: dir.join("still.ppm"),
        media_root: dir.clone(),
        lock: None,
        spec: lattice_engine::OutputSpec::preview(),
        renderer: lattice_engine::RendererRequest::RequireCpu,
        allow_fixtures: false,
        font: None,
    };
    let spec = lattice_media::OutputSpec::preview();
    let (scene, during) =
        lattice_media::sample_frame(&timeline, Time::seconds(3), spec, &options).expect("at 3s");
    let fonts = text_fonts(&scene.nodes);
    assert!(
        fonts.iter().all(|path| !path.is_empty()),
        "evaluate must stamp resolved_font: {fonts:?}"
    );
    let (_scene, before) =
        lattice_media::sample_frame(&timeline, Time::seconds(1), spec, &options).expect("at 1s");
    let y = during.height.saturating_sub(1);
    let mut yellow = 0u32;
    for x in 0..during.width {
        let px = during.pixel(x, y).unwrap();
        if px[0] > 180 && px[1] > 180 && px[2] < 80 {
            yellow += 1;
        }
    }
    assert!(
        yellow * 2 > during.width,
        "cpu title bar yellow={yellow} width={}",
        during.width
    );
    let y1 = before.height.saturating_sub(1);
    let mut yellow_before = 0u32;
    for x in 0..before.width {
        let px = before.pixel(x, y1).unwrap();
        if px[0] > 180 && px[1] > 180 && px[2] < 80 {
            yellow_before += 1;
        }
    }
    assert!(
        yellow_before * 2 <= before.width,
        "no title bar at 1s yellow={yellow_before}"
    );
}

#[test]
fn evaluate_hides_title_outside_window() {
    let compilation = Engine::default().compile(VEL).expect("compile");
    let timeline = Engine::timeline(&compilation.project).expect("flatten");
    let before = evaluate_at(&timeline, Time::seconds(1), Canvas::PREVIEW).unwrap();
    assert!(
        !title_text(&before).iter().any(|text| text == "Hello"),
        "no title at 1s: {:?}",
        title_text(&before)
    );
}
