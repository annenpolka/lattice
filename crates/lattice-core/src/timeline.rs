use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ir::{PlacementKind, Project, TimeSpan};
use crate::locator::MediaLocator;
use crate::time::{Time, TimeError};
use crate::time_map::TimeMap;

/// Flattened editorial timeline. Pure function of compiled Core IR.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    pub duration: Time,
    pub clips: Vec<TimelineClip>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineClip {
    pub id: String,
    pub kind: PlacementKind,
    pub span: TimeSpan,
    pub source: Option<TimelineSource>,
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacity: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_in: Option<crate::time::Time>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fade_out: Option<crate::time::Time>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain_db: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineSource {
    pub media_name: String,
    pub locator: MediaLocator,
    pub time_map: TimeMap,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimelineError {
    #[error("project has no scenes")]
    NoScenes,
    #[error("timeline has no video clip")]
    NoVideo,
    #[error("sequence refers to missing scene `{0}`")]
    MissingScene(String),
    #[error(transparent)]
    Time(#[from] TimeError),
}

/// Interpret compiled IR as a single linear timeline (sequence flow order).
pub fn flatten_project(project: &Project) -> Result<Timeline, TimelineError> {
    let scenes = ordered_scenes(project)?;
    if scenes.is_empty() {
        return Err(TimelineError::NoScenes);
    }
    let mut clips = Vec::new();
    let mut offset = Time::ZERO;
    for scene in scenes {
        for placement in &scene.placements {
            let start = offset.checked_add(placement.span.start)?;
            let source = placement.source_id.as_ref().and_then(|id| {
                let src = scene.sources.iter().find(|s| s.id == *id)?;
                let media = project.media.iter().find(|m| m.name == src.media_name);
                Some(TimelineSource {
                    media_name: src.media_name.clone(),
                    locator: media
                        .map(|m| m.locator.clone())
                        .unwrap_or(MediaLocator::File {
                            path: src.media_name.clone(),
                        }),
                    time_map: src.time_map.clone(),
                })
            });
            clips.push(TimelineClip {
                id: placement.id.clone(),
                kind: placement.kind,
                span: TimeSpan::new(start, placement.span.duration),
                source,
                text: placement
                    .visual
                    .as_ref()
                    .and_then(|visual| visual.text.clone()),
                opacity: placement.visual.as_ref().and_then(|visual| visual.opacity),
                fade_in: placement.visual.as_ref().and_then(|visual| visual.fade_in),
                fade_out: placement.visual.as_ref().and_then(|visual| visual.fade_out),
                gain_db: placement.audio.as_ref().and_then(|audio| audio.gain_db),
            });
        }
        offset = offset.checked_add(scene.duration)?;
    }
    Ok(Timeline {
        duration: offset,
        clips,
    })
}

fn ordered_scenes(project: &Project) -> Result<Vec<&crate::ir::Scene>, TimelineError> {
    if let Some(sequence) = project.sequences.first() {
        let mut scenes = Vec::new();
        for id in &sequence.scene_ids {
            let scene = project
                .scenes
                .iter()
                .find(|scene| scene.id == *id)
                .ok_or_else(|| TimelineError::MissingScene(id.clone()))?;
            scenes.push(scene);
        }
        return Ok(scenes);
    }
    Ok(project.scenes.iter().collect())
}

impl Timeline {
    pub fn video_clips(&self) -> impl Iterator<Item = &TimelineClip> {
        self.clips
            .iter()
            .filter(|clip| clip.kind == PlacementKind::Video)
    }

    pub fn title_clips(&self) -> impl Iterator<Item = &TimelineClip> {
        self.clips
            .iter()
            .filter(|clip| clip.kind == PlacementKind::Title)
    }

    pub fn callout_clips(&self) -> impl Iterator<Item = &TimelineClip> {
        self.clips
            .iter()
            .filter(|clip| clip.kind == PlacementKind::Callout)
    }

    pub fn audio_clips(&self) -> impl Iterator<Item = &TimelineClip> {
        self.clips
            .iter()
            .filter(|clip| clip.kind == PlacementKind::Audio)
    }

    pub fn freeze_segments(&self) -> Vec<&crate::time_map::TimeMapSegment> {
        self.video_clips()
            .filter_map(|clip| clip.source.as_ref())
            .flat_map(|source| source.time_map.segments.iter())
            .filter(|segment| segment.rate == Time::ZERO)
            .collect()
    }
}
