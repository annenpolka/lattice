//! GPUI-free layout projection of a Studio session.
//! Every pane is derived from Engine compile / timeline / plan / locus.

use lattice_engine::{
    Canvas, EditProposal, Engine, Locus, LocusId, LocusKind, NormalizedScale, Origin, Span, Time,
    plan_from_timeline, text_overlay_size,
};

use crate::session::StudioSession;
use crate::verb::{self, InvokedRecord, PointCandidate, Utterance};

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
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: NormalizedScale,
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
    /// Title-shaped fields exist only when here is Title.
    pub title_fields: bool,
    /// Property fields bound to this exact source `LocusId`, not `LocusKind`.
    pub gain_db: Option<i32>,
    pub fade_in: Option<Time>,
    pub invoked: Vec<InvokedRecord>,
    pub utterance: UtteranceView,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct TimelineClipView {
    pub id: String,
    pub kind: String,
    pub track: String,
    pub label: String,
    pub start: Time,
    pub duration: Time,
    pub selected: bool,
    pub scene_id: String,
    pub handles: bool,
    pub fade_handle: bool,
    pub gain_handle: bool,
    pub cut_lane: bool,
    pub delete_handle: bool,
    pub fade_in: Option<Time>,
    pub gain_db: Option<i32>,
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
    pub snap_indicator: Option<Time>,
    pub insertion_marker: Option<Time>,
    pub viewport_start: Time,
    pub viewport_duration: Time,
    /// Overlap cards on this projection only. Not a cross-surface modal.
    pub candidates: Vec<PointCandidate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UtteranceView {
    pub here: String,
    pub pointing: String,
    pub legal: Vec<String>,
    pub routed: Vec<String>,
    pub spoken: Vec<SpokenLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenLine {
    pub text: String,
    pub eye_target: Option<String>,
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
            session,
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
        inspector: inspector_from_session(session, current.as_ref(), &session.utterance()),
        timeline: timeline_view(session, &timeline, current.as_ref()),
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
    session: &StudioSession,
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
        .filter(|overlay| {
            let locus = loci.iter().find(|locus| locus.id == overlay.locus_id);
            match locus {
                Some(locus) => crate::interaction::overlay_playhead_visible(
                    session,
                    locus.id.as_str(),
                    overlay.span,
                ),
                None => overlay.span.contains(playhead),
            }
        })
        .filter_map(|overlay| {
            let text = overlay.text.clone()?;
            let locus = loci.iter().find(|locus| locus.id == overlay.locus_id)?;
            let (mut x, mut y, base_width, base_height) =
                overlay_bounds(overlay.callout, preview_size.0, preview_size.1);
            let resize = session.canvas_overlay_resize_preview(&locus.id);
            let requested_scale = resize
                .map(|preview| preview.scale)
                .or_else(|| locus.visual.as_ref().and_then(|visual| visual.scale))
                .unwrap_or_default();
            let scale =
                requested_scale.fit_within(base_width, base_height, preview_size.0, preview_size.1);
            let width = scale.scaled_extent(base_width);
            let height = scale.scaled_extent(base_height);
            let position = resize
                .map(|preview| preview.position)
                .or_else(|| session.canvas_overlay_drag_position(&locus.id))
                .or_else(|| locus.visual.as_ref().and_then(|visual| visual.position));
            if let Some(position) = position {
                (x, y) = position.pixel_origin(preview_size.0, preview_size.1, width, height);
            } else if !overlay.callout {
                y = i32::try_from(preview_size.1.saturating_sub(height)).unwrap_or(0);
            }
            Some(CanvasOverlay {
                selected: current.is_some_and(|id| id == &locus.id),
                locus_id: locus.id.as_str().to_string(),
                text,
                callout: overlay.callout,
                x,
                y,
                width,
                height,
                scale,
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

fn inspector_from_session(
    session: &StudioSession,
    locus: Option<&Locus>,
    utterance: &Utterance,
) -> InspectorView {
    let utterance = utterance_view(utterance);
    let invoked = session.invoked_this_session();
    let Some(locus) = locus else {
        let heading = if utterance.pointing == "unresolved-pointing" {
            "unresolved pointing".into()
        } else {
            "(no locus)".into()
        };
        return InspectorView {
            heading,
            origin: String::new(),
            defined_in: String::new(),
            locus_id: None,
            go_to_definition: None,
            title_fields: false,
            gain_db: None,
            fade_in: None,
            invoked,
            utterance,
        };
    };
    let file = file_label(session.path());
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
    let (gain_db, fade_in) = source_property_values(session, locus);
    InspectorView {
        heading: format!("{} \"{}\"", kind_label(locus.kind), locus.label),
        origin,
        defined_in,
        locus_id: Some(locus.id.as_str().to_string()),
        go_to_definition: locus.source_span,
        title_fields: locus.kind == LocusKind::Title,
        gain_db,
        fade_in,
        invoked,
        utterance,
    }
}

fn source_property_values(session: &StudioSession, locus: &Locus) -> (Option<i32>, Option<Time>) {
    if locus.kind != LocusKind::Source {
        return (None, None);
    }
    let Ok(timeline) = Engine::timeline(&session.compilation().project) else {
        return (Some(0), Some(Time::ZERO));
    };
    let mut gain_db = None;
    let mut fade_in = None;
    for clip in &timeline.clips {
        let source_id = session
            .compilation()
            .project
            .scenes
            .iter()
            .find_map(|scene| {
                scene
                    .placements
                    .iter()
                    .find(|placement| placement.id == clip.id)
                    .and_then(|placement| placement.source_id.clone())
            });
        let matches = source_id.as_deref() == Some(locus.node_id.as_str())
            || source_id.as_deref() == Some(locus.id.as_str());
        if !matches {
            continue;
        }
        if clip.gain_db.is_some() {
            gain_db = clip.gain_db;
        }
        if clip.fade_in.is_some() {
            fade_in = clip.fade_in;
        }
    }
    (gain_db.or(Some(0)), fade_in.or(Some(Time::ZERO)))
}

fn utterance_view(utterance: &Utterance) -> UtteranceView {
    UtteranceView {
        here: utterance.here.clone().unwrap_or_else(|| "(none)".into()),
        pointing: utterance.pointing.clone(),
        legal: utterance
            .legal
            .iter()
            .map(|edit| {
                format!(
                    "{} → {} ({}: {})",
                    edit.verb,
                    edit.target.as_str(),
                    edit.scope,
                    edit.effect
                )
            })
            .collect(),
        routed: utterance.routed.clone(),
        spoken: utterance
            .spoken
            .iter()
            .map(|clause| SpokenLine {
                text: clause.text.clone(),
                eye_target: clause.eye_target.clone(),
            })
            .collect(),
    }
}

#[allow(clippy::too_many_lines)]
fn timeline_view(
    session: &StudioSession,
    timeline: &lattice_engine::Timeline,
    current: Option<&Locus>,
) -> TimelineView {
    let current_id = current.map(|locus| locus.id.clone());
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
            let scene_id = session
                .compilation()
                .project
                .scenes
                .iter()
                .find(|scene| {
                    scene
                        .placements
                        .iter()
                        .any(|placement| placement.id == clip.id)
                })
                .map(|scene| scene.id.clone())
                .unwrap_or_default();
            let overlay = matches!(kind.as_str(), "title" | "callout");
            let source_id = session
                .compilation()
                .project
                .scenes
                .iter()
                .find_map(|scene| {
                    scene
                        .placements
                        .iter()
                        .find(|placement| placement.id == clip.id)
                        .and_then(|placement| placement.source_id.clone())
                });
            let selected = current.is_some_and(|locus| {
                if overlay {
                    return locus.id.as_str() == clip.id || locus.node_id == clip.id;
                }
                if locus.kind == LocusKind::Source {
                    return source_id.as_deref() == Some(locus.node_id.as_str())
                        || locus.id.as_str() == clip.id
                        || locus.node_id == clip.id;
                }
                if locus.kind == LocusKind::Scene {
                    return locus.scene_id.as_deref() == Some(scene_id.as_str())
                        || current_id
                            .as_ref()
                            .is_some_and(|id| id.as_str() == scene_id);
                }
                locus.id.as_str() == clip.id || locus.node_id == clip.id
            });
            let (start, duration) = crate::interaction::ephemeral_clip_span(session, &clip.id)
                .unwrap_or((clip.span.start, clip.span.duration));
            let wide_enough =
                session.viewport().delta_x(duration).abs() >= crate::gesture::MIN_DRAW_WIDTH_PX;
            let source_here = current.is_some_and(|locus| locus.kind == LocusKind::Source);
            let handles =
                selected && wide_enough && matches!(kind.as_str(), "video" | "title" | "callout");
            let fade_handle = selected && source_here && wide_enough && kind == "video";
            let gain_handle = selected && source_here && wide_enough && kind == "audio";
            TimelineClipView {
                selected,
                id: clip.id.clone(),
                kind,
                track: track.into(),
                label: clip.text.clone().unwrap_or_else(|| clip.id.clone()),
                start,
                duration,
                scene_id,
                handles,
                fade_handle,
                gain_handle,
                cut_lane: false,
                delete_handle: false,
                fade_in: clip.fade_in,
                gain_db: clip.gain_db,
            }
        })
        .collect();
    let scene_here = current.is_some_and(|locus| locus.kind == LocusKind::Scene);
    let mut scene_clips = Vec::new();
    for scene in &session.compilation().project.scenes {
        let Some(span) = scene_layout_span(session, timeline, &scene.id) else {
            continue;
        };
        let (start, duration) = (span.start, span.duration);
        let wide_enough =
            session.viewport().delta_x(duration).abs() >= crate::gesture::MIN_DRAW_WIDTH_PX;
        let selected = current.is_some_and(|locus| {
            locus.kind == LocusKind::Scene
                && (locus.id.as_str() == scene.id || locus.node_id == scene.id)
        });
        scene_clips.push(TimelineClipView {
            selected,
            id: scene.id.clone(),
            kind: "scene".into(),
            track: "scene".into(),
            label: scene.name.clone(),
            start,
            duration,
            scene_id: scene.id.clone(),
            handles: false,
            fade_handle: false,
            gain_handle: false,
            cut_lane: selected && scene_here && wide_enough,
            delete_handle: selected && scene_here && wide_enough,
            fade_in: None,
            gain_db: None,
        });
    }
    TimelineView {
        duration: timeline.duration,
        tracks: vec![
            track_named("Video", "video", &clips),
            track_named("Audio", "audio", &clips),
            track_named("Text", "text", &clips),
            TimelineTrackView {
                name: "Scene".into(),
                clips: scene_clips,
            },
        ],
        snap_indicator: session.snap_indicator(),
        insertion_marker: crate::interaction::insertion_marker(session),
        viewport_start: session.viewport().visible_start(),
        viewport_duration: session.viewport().visible_duration(),
        candidates: session
            .unresolved_pointing()
            .map(verb::candidate_cards)
            .unwrap_or_default(),
    }
}

fn scene_layout_span(
    session: &StudioSession,
    timeline: &lattice_engine::Timeline,
    scene_id: &str,
) -> Option<lattice_engine::TimeSpan> {
    let scene = session
        .compilation()
        .project
        .scenes
        .iter()
        .find(|scene| scene.id == scene_id)?;
    let mut start = None;
    let mut end = None;
    for placement in &scene.placements {
        let Some(clip) = timeline.clips.iter().find(|clip| clip.id == placement.id) else {
            continue;
        };
        start = Some(start.map_or(clip.span.start, |time: Time| time.min(clip.span.start)));
        end = Some(end.map_or(clip.span.end(), |time: Time| time.max(clip.span.end())));
    }
    let start = start?;
    let end = end?;
    Some(lattice_engine::TimeSpan::new(
        start,
        end.checked_sub(start).ok()?,
    ))
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

/// Matches `evaluate` title/callout geometry so GPUI chrome sits on the composited frame.
fn overlay_bounds(callout: bool, canvas_w: u32, canvas_h: u32) -> (i32, i32, u32, u32) {
    let (width, height) = text_overlay_size(Canvas {
        width: canvas_w,
        height: canvas_h,
    });
    if callout {
        (0, 0, width, height)
    } else {
        let y = i32::try_from(canvas_h.saturating_sub(height)).unwrap_or(0);
        (0, y, width, height)
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
