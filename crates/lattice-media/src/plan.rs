use lattice_core::{MediaLocator, Time, TimeMap, TimeSpan, Timeline, TimelineError};

use crate::PREVIEW_FPS_DEN;
use crate::PREVIEW_FPS_NUM;

/// Backend-facing plan. Built from a [`Timeline`], not from VEL.
///
/// Represents every video cut, audio window, and overlay — not the first
/// video clip as if it were the whole movie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderPlan {
    pub duration: Time,
    pub fps_num: i64,
    pub fps_den: i64,
    pub segments: Vec<PlanSegment>,
    pub overlays: Vec<OverlayWindow>,
    pub fade_in: Option<Time>,
    pub audio: Vec<AudioWindow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanSegment {
    pub local: TimeSpan,
    pub content_start: Time,
    pub hold: bool,
    pub media_name: String,
    pub locator: MediaLocator,
    pub fade_in: Option<Time>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayWindow {
    pub span: TimeSpan,
    pub text: Option<String>,
    pub opacity: Option<u8>,
    pub callout: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioWindow {
    pub span: TimeSpan,
    pub gain_db: Option<i32>,
    pub generated: bool,
    pub media_name: Option<String>,
    pub locator: Option<MediaLocator>,
    pub content_start: Time,
    pub hold: bool,
}

pub fn plan_from_timeline(timeline: &Timeline) -> Result<RenderPlan, TimelineError> {
    if timeline.video_clips().next().is_none() {
        return Err(TimelineError::NoVideo);
    }
    let mut segments = Vec::new();
    for video in timeline.video_clips() {
        let source = video.source.as_ref();
        let time_map = source.map_or_else(
            || TimeMap::identity(Time::ZERO, video.span.duration),
            |source| source.time_map.clone(),
        );
        let locator = source.map_or(
            MediaLocator::File {
                path: String::new(),
            },
            |source| source.locator.clone(),
        );
        let media_name = source
            .map(|source| source.media_name.clone())
            .unwrap_or_default();
        for (index, segment) in time_map.segments.iter().enumerate() {
            segments.push(PlanSegment {
                local: TimeSpan::new(
                    video.span.start + segment.local_start,
                    segment.local_duration,
                ),
                content_start: segment.content_start,
                hold: segment.rate == Time::ZERO,
                media_name: media_name.clone(),
                locator: locator.clone(),
                fade_in: (index == 0).then_some(video.fade_in).flatten(),
            });
        }
    }
    let mut overlays: Vec<OverlayWindow> = timeline
        .title_clips()
        .map(|clip| OverlayWindow {
            span: clip.span,
            text: clip.text.clone(),
            opacity: clip.opacity,
            callout: false,
        })
        .collect();
    overlays.extend(timeline.callout_clips().map(|clip| OverlayWindow {
        span: clip.span,
        text: clip.text.clone(),
        opacity: clip.opacity,
        callout: true,
    }));
    let fade_in = timeline.video_clips().next().and_then(|clip| clip.fade_in);
    let mut audio = Vec::new();
    for clip in timeline.audio_clips() {
        let generated = clip
            .source
            .as_ref()
            .is_some_and(|source| matches!(source.locator, MediaLocator::Generated { .. }));
        if generated {
            audio.push(AudioWindow {
                span: clip.span,
                gain_db: clip.gain_db,
                generated: true,
                media_name: clip.source.as_ref().map(|source| source.media_name.clone()),
                locator: clip.source.as_ref().map(|source| source.locator.clone()),
                content_start: Time::ZERO,
                hold: false,
            });
            continue;
        }
        let source = clip.source.as_ref();
        let time_map = source.map_or_else(
            || TimeMap::identity(Time::ZERO, clip.span.duration),
            |source| source.time_map.clone(),
        );
        for segment in &time_map.segments {
            audio.push(AudioWindow {
                span: TimeSpan::new(
                    clip.span.start + segment.local_start,
                    segment.local_duration,
                ),
                gain_db: clip.gain_db,
                generated: false,
                media_name: source.map(|source| source.media_name.clone()),
                locator: source.map(|source| source.locator.clone()),
                content_start: segment.content_start,
                hold: segment.rate == Time::ZERO,
            });
        }
    }
    Ok(RenderPlan {
        duration: timeline.duration,
        fps_num: PREVIEW_FPS_NUM,
        fps_den: PREVIEW_FPS_DEN,
        segments,
        overlays,
        fade_in,
        audio,
    })
}
