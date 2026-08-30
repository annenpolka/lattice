//! Layout, locus pointing, and Review against the real Engine.

use std::path::PathBuf;

use lattice_engine::{Engine, LocusKind, Origin, SemanticEdit, plan_from_timeline};
use lattice_studio::StudioSession;

fn demo_vel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/gameplay-commentary/main.vel")
        .canonicalize()
        .expect("demo vel")
}

fn temp_copy() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-layout-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("main.vel");
    std::fs::copy(demo_vel(), &dest).unwrap();
    dest
}

fn temp_render_fixture() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-layout-render-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("main.vel");
    std::fs::write(
        &dest,
        r#"project "layout-render"

convention commentary

media game "capture.mp4"

sequence main {
  demo
}

scene demo {
  game[0s..2s] as clip

  title "Hello" {
    at 0s for 1s
  }
}
"#,
    )
    .unwrap();
    dest
}

fn temp_duplicate_overlay_fixture() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-duplicate-overlay-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    let dest = dir.join("main.vel");
    std::fs::write(
        &dest,
        r#"project "duplicate-overlays"
convention commentary
media game "capture.mp4"
sequence main {
  demo
}
scene demo {
  game[0s..2s] as clip
  title "Same" {
    at 0s for 2s
  }
  title "Same" {
    at 0s for 2s
  }
  callout "Same" {
    at 0s for 2s
  }
}
"#,
    )
    .unwrap();
    dest
}

#[test]
fn duplicate_overlay_text_and_span_keep_distinct_locus_ids() {
    let mut session = StudioSession::open(temp_duplicate_overlay_fixture()).expect("open");
    let loci = session.loci().expect("loci");
    let mut expected: Vec<_> = loci
        .iter()
        .filter(|locus| matches!(locus.kind, LocusKind::Title | LocusKind::Callout))
        .map(|locus| locus.id.as_str().to_string())
        .collect();
    expected.sort();

    let layout = session.layout().expect("layout");
    let mut actual: Vec<_> = layout
        .canvas
        .overlays
        .iter()
        .map(|overlay| overlay.locus_id.clone())
        .collect();
    actual.sort();
    assert_eq!(actual, expected, "canvas must preserve placement identity");
    for overlay in &layout.canvas.overlays {
        let locus = loci
            .iter()
            .find(|locus| locus.id.as_str() == overlay.locus_id)
            .expect("overlay locus");
        assert_eq!(overlay.callout, locus.kind == LocusKind::Callout);
    }

    let target = loci
        .iter()
        .filter(|locus| locus.kind == LocusKind::Title)
        .nth(1)
        .expect("second title")
        .id
        .clone();
    session.point_at(target.clone());
    let selected: Vec<_> = session
        .layout()
        .expect("selected layout")
        .canvas
        .overlays
        .into_iter()
        .filter(|overlay| overlay.selected)
        .map(|overlay| overlay.locus_id)
        .collect();
    assert_eq!(selected, vec![target.as_str().to_string()]);
}

#[test]
fn layout_matches_engine_compile_plan_and_timeline() {
    let vel = demo_vel();
    let engine = Engine::default();
    let compilation = engine.compile_path(&vel).expect("engine compile");
    let session = StudioSession::open(&vel).expect("studio open");
    assert!(
        session.uses_engine_not_own_compiler(),
        "Studio must not own compile"
    );
    let layout = session.layout().expect("layout");

    assert_eq!(layout.project_name, compilation.project.name);
    let sequence = &compilation.project.sequences[0];
    assert!(
        layout.tree.iter().any(|node| node.kind == "sequence"
            && node.label == sequence.name
            && node.id == sequence.id),
        "tree must include the compiled sequence: {:?}",
        layout.tree
    );
    let scene = &compilation.project.scenes[0];
    let scene_node = layout.tree[0]
        .children
        .iter()
        .find(|node| node.id == scene.id)
        .expect("scene in tree");
    assert_eq!(scene_node.label, scene.name);

    let timeline = Engine::timeline(&compilation.project).expect("timeline");
    let plan = plan_from_timeline(&timeline).expect("plan");
    let overlay_texts: Vec<_> = layout
        .canvas
        .overlays
        .iter()
        .map(|overlay| overlay.text.as_str())
        .collect();
    for overlay in &plan.overlays {
        if let Some(text) = &overlay.text {
            if overlay.span.contains(layout.playhead) {
                assert!(
                    overlay_texts.contains(&text.as_str()),
                    "canvas missing on-playhead overlay {text:?} from plan"
                );
            } else {
                assert!(
                    !overlay_texts.contains(&text.as_str()),
                    "canvas must not show off-playhead overlay {text:?}"
                );
            }
        }
    }

    let layout_ids: Vec<_> = layout
        .timeline
        .tracks
        .iter()
        .flat_map(|track| {
            track
                .clips
                .iter()
                .map(|clip| (clip.id.as_str(), clip.start, clip.duration))
        })
        .collect();
    for clip in &timeline.clips {
        assert!(
            layout_ids.iter().any(|(id, start, duration)| *id == clip.id
                && *start == clip.span.start
                && *duration == clip.span.duration),
            "timeline missing engine clip {}",
            clip.id
        );
    }
    assert_eq!(layout.timeline.duration, timeline.duration);

    let current = session
        .current_locus()
        .unwrap()
        .expect("default title locus");
    assert_eq!(current.kind, lattice_engine::LocusKind::Title);
    assert!(layout.inspector.heading.contains(&current.label));
    assert_eq!(layout.inspector.go_to_definition, current.source_span);
    let inspected = engine.inspect(&compilation, &current.id).unwrap();
    assert_eq!(
        layout.inspector.go_to_definition,
        inspected.locus.source_span
    );
    assert!(
        layout.source.highlight == current.source_span,
        "VEL pane must highlight the current locus span"
    );
}

#[test]
fn canvas_source_and_timeline_point_at_the_same_title_locus() {
    let vel = demo_vel();
    let engine = Engine::default();
    let compilation = engine.compile_path(&vel).unwrap();
    let title = engine
        .loci(&compilation)
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
        .expect("title locus from engine");
    let from_node = engine
        .locus_for_node(&compilation, title.node_id.as_str())
        .unwrap()
        .expect("node");
    assert_eq!(from_node.id, title.id);

    let mut session = StudioSession::open(&vel).unwrap();
    session.point_at(title.id.clone());
    let layout = session.layout().unwrap();
    assert_eq!(
        layout.playhead,
        title.timeline_span.expect("title span").start
    );
    assert_eq!(layout.canvas.playhead, layout.playhead);
    let overlay = layout
        .canvas
        .overlays
        .iter()
        .find(|overlay| overlay.text == title.label)
        .expect("title overlay at title time");
    let from_canvas = session
        .point_from_canvas_overlay(&overlay.locus_id)
        .unwrap()
        .expect("canvas point");
    assert_eq!(from_canvas.id, title.id);

    let span = title.source_span.expect("title source span");
    let mid = span.start + (span.end - span.start) / 2;
    let from_source = session
        .point_from_source_offset(mid)
        .unwrap()
        .expect("source point");
    assert_eq!(from_source.id, title.id);
    let engine_source = engine
        .locus_at_source(&compilation, mid)
        .unwrap()
        .expect("engine source");
    assert_eq!(engine_source.id, title.id);

    let range = title.timeline_span.expect("title timeline span");
    let inside = range.start;
    let from_tl = session.point_from_timeline_time(inside).unwrap();
    if let Some(pointed) = from_tl {
        assert_eq!(pointed.id, title.id);
    } else {
        let unresolved = session
            .unresolved_pointing()
            .expect("coordinate overlap stays unresolved");
        assert!(
            unresolved
                .candidates
                .iter()
                .any(|locus| locus.id == title.id),
            "title must be a candidate on the Timeline"
        );
        let picked = session
            .pick_point_candidate(title.id.clone())
            .unwrap()
            .expect("pick title");
        assert_eq!(picked.id, title.id);
    }
    let engine_tl = engine
        .locus_at_timeline(&compilation, inside)
        .unwrap()
        .expect("engine timeline");
    assert_eq!(engine_tl.id, title.id);

    let def = session.go_to_definition().unwrap().expect("definition");
    assert_eq!(Some(def), title.source_span);
    let provenance = session.current_provenance().unwrap().expect("prov");
    assert!(
        matches!(provenance.origin, Origin::Invocation { ref command } if command == "title"),
        "{:?}",
        provenance.origin
    );

    let layout = session.layout().unwrap();
    assert!(
        layout
            .canvas
            .overlays
            .iter()
            .any(|overlay| overlay.selected && overlay.locus_id == title.id.as_str()),
        "canvas should highlight the shared title locus"
    );
    assert!(layout.inspector.heading.contains(&title.label));
}

#[test]
fn review_propose_reject_apply_uses_engine() {
    let vel = temp_copy();
    let engine = Engine::default();
    let mut session = StudioSession::open(&vel).unwrap();
    let original = session.source().to_string();
    let proposal = session.propose_title_text("World").expect("propose");
    assert!(
        proposal.description.contains("World"),
        "{}",
        proposal.description
    );
    assert!(
        proposal.vel_diff.contains("World"),
        "diff must come from Engine: {}",
        proposal.vel_diff
    );
    let engine_proposal = engine
        .propose(
            session.compilation(),
            &session.current_locus().unwrap().unwrap(),
            SemanticEdit::Title {
                text: Some("World".into()),
                at: None,
                duration: None,
                opacity: None,
            },
        )
        .unwrap();
    assert_eq!(proposal.vel_diff, engine_proposal.vel_diff);
    assert_eq!(session.source(), original);
    assert_eq!(session.reject_review(&proposal), original);
    assert_eq!(std::fs::read_to_string(&vel).unwrap(), original);

    let proposal = session.propose_title_text("World").unwrap();
    session.apply_review(&proposal).expect("apply");
    assert_ne!(session.source(), original);
    assert!(session.source().contains("World"));
    let recompiled = engine.compile_path(&vel).unwrap();
    let timeline = Engine::timeline(&recompiled.project).unwrap();
    let title = timeline.title_clips().next().expect("title");
    assert_eq!(title.text.as_deref(), Some("World"));
}

#[test]
fn cargo_toml_defaults_to_window_binary() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains("default = [\"window\"]"),
        "window must be the default feature so cargo test cannot emit a stub binary"
    );
    assert!(
        manifest.contains("required-features"),
        "the studio bin must require the window feature"
    );
    let main = include_str!("../src/main.rs");
    assert!(
        !main.contains("Rebuild with `--features window`"),
        "stub overwrite path must not exist"
    );
}

#[test]
fn apply_title_text_rewrites_vel_and_render_preview_writes_mp4() {
    let vel = temp_render_fixture();
    let mut session = StudioSession::open(&vel).unwrap();
    let original = session.source().to_string();
    session.apply_title_text("World").expect("apply title");
    assert_ne!(session.source(), original);
    assert!(session.source().contains("World"));
    session.save().expect("save working source");
    let engine = Engine::default();
    let compilation = engine.compile_path(&vel).unwrap();
    let timeline = Engine::timeline(&compilation.project).unwrap();
    let title = timeline.title_clips().next().expect("title");
    assert_eq!(title.text.as_deref(), Some("World"));
    lattice_media::generate_av_fixture(vel.parent().unwrap().join("capture.mp4"), 3).unwrap();
    let preview = session.render_preview().expect("render");
    assert!(
        preview.is_file(),
        "Engine render must write {}",
        preview.display()
    );
}

#[test]
fn apply_title_and_callout_text_rewrites_vel_and_evaluate() {
    let vel = temp_copy();
    let mut session = StudioSession::open(&vel).unwrap();
    let original = session.source().to_string();

    session.point_at_title().unwrap();
    let layout = session.layout().unwrap();
    assert!(layout.inspector.title_fields);
    assert!(!layout.inspector.callout_fields);
    session.apply_title_text("World").expect("apply title");
    assert!(session.source().contains("title \"World\""));

    let callout = session
        .loci()
        .unwrap()
        .into_iter()
        .find(|locus| locus.kind == LocusKind::Callout)
        .expect("callout");
    session.point_at(callout.id);
    let layout = session.layout().unwrap();
    assert!(!layout.inspector.title_fields, "callout is not Title");
    assert!(layout.inspector.callout_fields);
    session
        .apply_callout_text("Release")
        .expect("apply callout");
    assert_ne!(session.source(), original);
    assert!(session.source().contains("callout \"Release\""));
    assert!(!session.source().contains("callout \"Hold\""));

    let engine = Engine::default();
    let compilation = engine.compile(session.source()).unwrap();
    let timeline = Engine::timeline(&compilation.project).unwrap();
    assert_eq!(
        timeline
            .title_clips()
            .next()
            .and_then(|clip| clip.text.clone()),
        Some("World".into())
    );
    assert_eq!(
        timeline
            .callout_clips()
            .next()
            .and_then(|clip| clip.text.clone()),
        Some("Release".into())
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn window_source_composes_documented_panes() {
    let main = include_str!("../src/main.rs");
    for pane in ["SEQUENCE", "Canvas", "VEL", "Inspector", "Timeline"] {
        assert!(
            main.contains(pane),
            "window source must compose documented pane {pane}"
        );
    }
    assert!(
        main.contains("Go to definition"),
        "Navigate Go to definition must be in the window"
    );
    assert!(
        main.contains("Apply") && main.contains("Reject"),
        "Review Apply/Reject must be in the window"
    );
    for action in [
        "Open Video…",
        "Play",
        "Pause",
        "Seek",
        "Scrub",
        "Undo",
        "Redo",
        "Resolve",
    ] {
        assert!(main.contains(action), "window source must expose {action}");
    }
    for evicted in [
        "Set In",
        "Set Out",
        "Split at Playhead",
        "Delete Selected Clip",
        "Gain -3 dB",
        "Fade",
        "Fade Out",
        "Split Scene",
        "In Point",
        "Out Point",
    ] {
        assert!(
            !main.contains(&format!("\"{evicted}\"")),
            "session strip must not keep locus-taking button {evicted}"
        );
    }
    for cluster in [
        "toolbar.cluster.file",
        "toolbar.cluster.clock",
        "toolbar.cluster.engine",
        "toolbar.cluster.telemetry",
    ] {
        assert!(
            main.contains(cluster),
            "session strip must group existing actions as {cluster}"
        );
    }
    assert!(
        main.contains("inspector.gain") && main.contains("inspector.fade"),
        "Inspector must draw typed gain/fade fields"
    );
    assert!(
        main.contains("inspector.invoked") && main.contains("Invoked this session"),
        "Inspector must read back invoked rows"
    );
    assert!(
        main.contains("utterance.eye"),
        "utterance may navigate by eye"
    );
    assert!(
        !main.contains("inspector.split")
            && !main.contains("inspector.fade-out")
            && !main.contains("inspector.in-point")
            && !main.contains("inspector.out-point"),
        "deleted Flash furniture must stay gone"
    );
    assert!(
        main.contains("timeline.fade.") && main.contains("timeline.gain."),
        "drawn gain/fade routes must exist"
    );
    assert!(
        main.contains("timeline.cut.") && main.contains("timeline.delete."),
        "drawn split/delete routes must exist"
    );
    assert!(
        main.contains("StudioSession::open_video"),
        "Open Video… must call StudioSession::open_video"
    );
    assert!(
        main.contains("prompt_for_paths") && main.contains("PathPromptOptions"),
        "Open Video… must use GPUI's native platform file picker"
    );
    assert!(
        !main.contains("System.Windows.Forms.OpenFileDialog"),
        "Open Video… must not fork a Windows-only picker"
    );
    assert!(
        main.contains("img("),
        "Canvas must draw the preview frame image"
    );
    assert!(
        main.contains("refresh_preview(\"timeline-clip\")"),
        "timeline clip click must refresh the playhead preview"
    );
    assert!(
        main.contains("object_fit(ObjectFit::Contain)"),
        "canvas still must preserve aspect instead of stretching"
    );
    assert!(
        main.contains("id(\"canvas-frame\")"),
        "canvas still id must remain"
    );
    assert!(
        !main.contains(".id(\"canvas-frame\").absolute().size_full()"),
        "canvas still must not fill the pane without object-fit"
    );
    assert!(
        main.contains("on_mouse_down")
            && main.contains("on_mouse_move")
            && main.contains("on_mouse_up")
            && main.contains("begin_timeline_pointer")
            && main.contains("update_timeline_pointer")
            && main.contains("commit_timeline_pointer")
            && !main.contains("TIMELINE_RATIO_DEN")
            && !main.contains("scrub_timeline_ratio"),
        "timeline pointer-down/move/up must use continuous viewport x, not 101 hit cells"
    );
    assert!(
        main.contains("spawn_play_clock") && main.contains("step_clock"),
        "Play must tick the session clock off the paint path"
    );
    assert!(
        !main.contains("gap.min(4.0)"),
        "timeline clips must sit at their start time, not pack with a 4px gap"
    );
    assert!(
        main.contains("id(\"playhead\")"),
        "timeline must draw a playhead aligned to Time"
    );
    assert!(
        main.contains("capture_any_mouse_down"),
        "clip children must not steal timeline pointer-down from the rail"
    );
    assert!(
        !main.contains(".text_xl().text_color(rgb(0xffffff)).child(text)"),
        "GPUI overlay must be selection chrome, not a second title compositor"
    );
    assert!(
        !main.contains("eprintln!") && !main.contains("println!"),
        "eprintln!/println! panic when stderr is a closed Windows pipe (0x800700e8); use trace::log"
    );
    assert!(
        main.contains("trace::install"),
        "the window binary must install the durable log before GPUI starts"
    );
}

#[test]
fn session_and_layout_have_no_gpui() {
    let session = include_str!("../src/session.rs");
    let layout = include_str!("../src/layout.rs");
    let gesture = include_str!("../src/gesture.rs");
    let viewport = include_str!("../src/viewport.rs");
    let preview = include_str!("../src/preview.rs");
    let interaction = include_str!("../src/interaction.rs");
    let verb = include_str!("../src/verb.rs");
    assert!(!session.contains("gpui"), "session must stay GPUI-free");
    assert!(!layout.contains("gpui"), "layout must stay GPUI-free");
    assert!(!gesture.contains("gpui"), "gesture must stay GPUI-free");
    assert!(!viewport.contains("gpui"), "viewport must stay GPUI-free");
    assert!(!preview.contains("gpui"), "preview must stay GPUI-free");
    assert!(
        !interaction.contains("gpui"),
        "interaction must stay GPUI-free"
    );
    assert!(!verb.contains("gpui"), "verb spine must stay GPUI-free");
    let core = include_str!("../../../crates/lattice-core/Cargo.toml");
    assert!(!core.contains("gpui"));
}
