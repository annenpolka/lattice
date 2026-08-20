//! GPUI-free layout projection of a Studio session.
//! Every pane is derived from Engine compile / timeline / plan / locus.

use lattice_engine::{
    EditProposal, Engine, Locus, LocusId, LocusKind, Origin, Span, Time, plan_from_timeline,
};

use crate::session::StudioSession;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub selected: bool,
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasOverlay {
    pub locus_id: String,
    pub text: String,
    pub callout: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasView {
    pub overlays: Vec<CanvasOverlay>,
    pub preview_frame: Option<std::path::PathBuf>,
    pub playhead: Time,
    pub preview_width: u32,
    pub preview_height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceView {
    pub text: String,
    pub highlight: Option<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectorView {
    pub heading: String,
    pub origin: String,
    pub defined_in: String,
    pub locus_id: Option<String>,
    pub go_to_definition: Option<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineClipView {
    pub id: String,
    pub kind: String,
    pub track: String,
    pub label: String,
    pub start: Time,
    pub duration: Time,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineTrackView {
    pub name: String,
    pub clips: Vec<TimelineClipView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineView {
    pub duration: Time,
    pub tracks: Vec<TimelineTrackView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewView {
    pub description: String,
    pub vel_diff: String,
    pub locus_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudioLayout {
    pub project_name: String,
    pub file_label: String,
    pub tree: Vec<TreeNode>,
    pub canvas: CanvasView,
    pub source: SourceView,
    pub inspector: InspectorView,
    pub timeline: TimelineView,
    pub review: Option<ReviewView>,
    pub playhead: Time,
    pub dirty: bool,
    pub playing: bool,
}

pub fn from_session(session: &StudioSession) -> Result<StudioLayout, lattice_engine::EngineError> {
    let compilation = session.compilation();
    let loci = session.loci()?;
    let current = session.current_locus()?;
    let current_id = current.as_ref().map(|locus| locus.id.clone());
    let timeline = Engine::timeline(&compilation.project)?;
    let plan = plan_from_timeline(&timeline)?;

    Ok(StudioLayout {
        project_name: compilation.project.name.clone(),
        file_label: file_label(session.path()),
        tree: tree_from_compilation(compilation, &loci, current_id.as_ref()),
        canvas: canvas_from_plan(
            &plan,
            &loci,
            current_id.as_ref(),
            session.playhead(),
            session.peek_preview_frame(),
            session.preview_pixel_size(),
        ),
        source: SourceView {
            text: compilation.source.clone(),
            highlight: current.as_ref().and_then(|locus| locus.source_span),
        },
        inspector: inspector_from_locus(current.as_ref(), session.path()),
        timeline: timeline_view(&timeline, current_id.as_ref()),
        review: session.review_proposal().map(review_from_proposal),
        playhead: session.playhead(),
        dirty: session.is_dirty(),
        playing: session.is_playing(),
    })
}

fn file_label(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project.vel")
        .to_string()
}

fn tree_from_compilation(
    compilation: &lattice_engine::Compilation,
    loci: &[Locus],
    current: Option<&LocusId>,
) -> Vec<TreeNode> {
    let mut roots = Vec::new();
    for sequence in &compilation.project.sequences {
        let mut scenes = Vec::new();
        for scene_id in &sequence.scene_ids {
            let Some(scene) = compilation
                .project
                .scenes
                .iter()
                .find(|scene| scene.id == *scene_id)
            else {
                continue;
            };
            let mut children = Vec::new();
            for source in &scene.sources {
                if source.generated {
                    continue;
                }
                children.push(node_for(
                    &source.id,
                    "source",
                    &source.name,
                    current,
                    Vec::new(),
                ));
                if source
                    .time_map
                    .segments
                    .iter()
                    .any(|segment| segment.rate.num() == 0)
                {
                    children.push(TreeNode {
                        id: format!("freeze:{}", source.id),
                        kind: "freeze".into(),
                        label: "freeze".into(),
                        selected: false,
                        children: Vec::new(),
                    });
                }
            }
            for locus in loci {
                if locus.scene_id.as_deref() != Some(scene.id.as_str()) {
                    continue;
                }
                if !matches!(
                    locus.kind,
                    LocusKind::Title | LocusKind::Callout | LocusKind::Speech
                ) {
                    continue;
                }
                children.push(node_for(
                    locus.id.as_str(),
                    kind_label(locus.kind),
                    &locus.label,
                    current,
                    Vec::new(),
                ));
            }
            scenes.push(node_for(&scene.id, "scene", &scene.name, current, children));
        }
        roots.push(node_for(
            &sequence.id,
            "sequence",
            &sequence.name,
            current,
            scenes,
        ));
    }
    roots
}

fn node_for(
    id: &str,
    kind: &str,
    label: &str,
    current: Option<&LocusId>,
    children: Vec<TreeNode>,
) -> TreeNode {
    TreeNode {
        selected: current.is_some_and(|locus| locus.as_str() == id),
        id: id.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        children,
    }
}

fn canvas_from_plan(
    plan: &lattice_engine::RenderPlan,
    loci: &[Locus],
    current: Option<&LocusId>,
    playhead: Time,
    preview_frame: Option<std::path::PathBuf>,
    preview_size: (u32, u32),
) -> CanvasView {
    let overlays = plan
        .overlays
        .iter()
        .filter(|overlay| overlay.span.contains(playhead))
        .filter_map(|overlay| {
            let text = overlay.text.clone()?;
            let locus = loci.iter().find(|locus| {
                locus.label == text
                    && matches!(locus.kind, LocusKind::Title | LocusKind::Callout)
                    && locus.timeline_span.is_none_or(|span| span == overlay.span)
            })?;
            Some(CanvasOverlay {
                selected: current.is_some_and(|id| id == &locus.id),
                locus_id: locus.id.as_str().to_string(),
                text,
                callout: overlay.callout,
            })
        })
        .collect();
    CanvasView {
        overlays,
        preview_frame,
        playhead,
        preview_width: preview_size.0,
        preview_height: preview_size.1,
    }
}

fn inspector_from_locus(locus: Option<&Locus>, path: &std::path::Path) -> InspectorView {
    let Some(locus) = locus else {
        return InspectorView {
            heading: "(no locus)".into(),
            origin: String::new(),
            defined_in: String::new(),
            locus_id: None,
            go_to_definition: None,
        };
    };
    let file = file_label(path);
    let defined_in = locus.source_span.map_or_else(
        || "provenance always present".into(),
        |span| format!("{file}:{}", span.line),
    );
    let origin = match &locus.provenance.origin {
        Origin::Invocation { command } => format!("invocation `{command}`"),
        Origin::Convention { name } => format!("convention `{name}`"),
        Origin::Builtin { name } => format!("builtin `{name}`"),
        Origin::Source => "source".into(),
    };
    InspectorView {
        heading: format!("{} \"{}\"", kind_label(locus.kind), locus.label),
        origin,
        defined_in,
        locus_id: Some(locus.id.as_str().to_string()),
        go_to_definition: locus.source_span,
    }
}

fn timeline_view(timeline: &lattice_engine::Timeline, current: Option<&LocusId>) -> TimelineView {
    let clips: Vec<TimelineClipView> = timeline
        .clips
        .iter()
        .map(|clip| {
            let kind = format!("{:?}", clip.kind).to_ascii_lowercase();
            let track = match kind.as_str() {
                "title" | "callout" => "text",
                "audio" => "audio",
                _ => "video",
            };
            TimelineClipView {
                selected: current.is_some_and(|id| id.as_str() == clip.id),
                id: clip.id.clone(),
                kind,
                track: track.into(),
                label: clip.text.clone().unwrap_or_else(|| clip.id.clone()),
                start: clip.span.start,
                duration: clip.span.duration,
            }
        })
        .collect();
    TimelineView {
        duration: timeline.duration,
        tracks: vec![
            track_named("Video", "video", &clips),
            track_named("Audio", "audio", &clips),
            track_named("Text", "text", &clips),
        ],
    }
}

fn track_named(name: &str, track: &str, clips: &[TimelineClipView]) -> TimelineTrackView {
    TimelineTrackView {
        name: name.into(),
        clips: clips
            .iter()
            .filter(|clip| clip.track == track)
            .cloned()
            .collect(),
    }
}

fn review_from_proposal(proposal: &EditProposal) -> ReviewView {
    ReviewView {
        description: proposal.description.clone(),
        vel_diff: proposal.vel_diff.clone(),
        locus_id: proposal.locus_id.as_str().to_string(),
    }
}

fn kind_label(kind: LocusKind) -> &'static str {
    match kind {
        LocusKind::Title => "title",
        LocusKind::Callout => "callout",
        LocusKind::Source => "source",
        LocusKind::Placement => "placement",
        LocusKind::Scene => "scene",
        LocusKind::Sequence => "sequence",
        LocusKind::Media => "media",
        LocusKind::Speech => "speech",
    }
}
