use std::path::{Path, PathBuf};

use lattice_core::{MediaLocator, ResolveLock, Time, Timeline, TimelineClip};

use crate::backend::OutputSpec;
use crate::backend::RendererRequest;
use crate::export::{ExportError, PreviewOptions};
use crate::sample::render_still;

/// Semantic preview request. Studio must not construct `FFmpeg` argv.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewFrameRequest {
    pub timeline_time: Time,
    pub width: u32,
    pub height: u32,
    pub fps_num: i64,
    pub fps_den: i64,
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

/// Composite one frame at a timeline time using the shared sample/render path.
pub fn preview_frame(
    timeline: &Timeline,
    request: &PreviewFrameRequest,
    media_root: &Path,
    output: &Path,
    allow_fixtures: bool,
) -> Result<PathBuf, ExportError> {
    let spec = OutputSpec {
        width: request.width,
        height: request.height,
        fps_num: request.fps_num,
        fps_den: request.fps_den,
        sample_rate: 44_100,
        channels: 2,
    };
    let options = PreviewOptions {
        output: output.to_path_buf(),
        media_root: media_root.to_path_buf(),
        lock: load_lock_file(media_root),
        spec,
        renderer: RendererRequest::RequireCpu,
        allow_fixtures,
        font: None,
    };
    render_still(timeline, request.timeline_time, spec, &options, output)
}

fn load_lock_file(media_root: &Path) -> Option<ResolveLock> {
    let text = std::fs::read_to_string(media_root.join("lattice.lock.json")).ok()?;
    serde_json::from_str(&text).ok()
}
