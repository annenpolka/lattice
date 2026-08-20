use std::path::{Path, PathBuf};

use lattice_core::{MediaLocator, Time, Timeline, TimelineClip};

use crate::export::{ExportError, resolve_media_path, run_ffmpeg_extract_frame};

/// Semantic preview request. Studio must not construct `FFmpeg` argv.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFrameRequest {
    pub timeline_time: Time,
    pub width: u32,
    pub height: u32,
}

/// Map a timeline time through scene offsets / trim / `TimeMap` / freeze to source content time.
pub fn map_timeline_to_source(
    timeline: &Timeline,
    timeline_time: Time,
) -> Result<(MediaLocator, Time), ExportError> {
    let clip = video_clip_at(timeline, timeline_time).ok_or(ExportError::TimeOutOfRange)?;
    let source = clip.source.as_ref().ok_or(ExportError::MissingSource)?;
    let local = timeline_time
        .checked_sub(clip.span.start)
        .map_err(ExportError::from)?;
    let content = source
        .time_map
        .content_at(local)
        .map_err(|err| ExportError::Map(err.to_string()))?;
    Ok((source.locator.clone(), content))
}

fn video_clip_at(timeline: &Timeline, time: Time) -> Option<&TimelineClip> {
    let hits: Vec<&TimelineClip> = timeline
        .video_clips()
        .filter(|clip| {
            time >= clip.span.start
                && (time < clip.span.end()
                    || (clip.span.duration.is_zero() && time == clip.span.start)
                    || (time == timeline.duration && clip.span.end() == timeline.duration))
        })
        .collect();
    if hits.is_empty() && time == timeline.duration {
        return timeline.video_clips().last();
    }
    hits.into_iter().max_by_key(|clip| clip.span.start)
}

/// Extract one source/rendered frame at a timeline time.
pub fn preview_frame(
    timeline: &Timeline,
    request: &PreviewFrameRequest,
    media_root: &Path,
    output: &Path,
    allow_fixtures: bool,
) -> Result<PathBuf, ExportError> {
    let (locator, content_time) = map_timeline_to_source(timeline, request.timeline_time)?;
    let input = resolve_media_path(&locator, media_root, output, allow_fixtures)?;
    run_ffmpeg_extract_frame(
        &input,
        content_time,
        output,
        Some((request.width, request.height)),
    )
}
