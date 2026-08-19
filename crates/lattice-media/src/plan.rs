use lattice_core::{Time, TimeMap, TimeSpan, Timeline, TimelineError};

use crate::PREVIEW_FPS_DEN;
use crate::PREVIEW_FPS_NUM;

/// Backend-facing plan. Built from a [`Timeline`], not from VEL.
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
}

pub fn plan_from_timeline(timeline: &Timeline) -> Result<RenderPlan, TimelineError> {
    let video = timeline
        .video_clips()
        .next()
        .ok_or(TimelineError::NoVideo)?;
    let time_map = video.source.as_ref().map_or_else(
        || TimeMap::identity(Time::ZERO, video.span.duration),
        |source| source.time_map.clone(),
    );
    let segments = time_map
        .segments
        .iter()
        .map(|segment| PlanSegment {
            local: TimeSpan::new(
                video.span.start + segment.local_start,
                segment.local_duration,
            ),
            content_start: segment.content_start,
            hold: segment.rate == Time::ZERO,
        })
        .collect();
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
    let audio = timeline
        .audio_clips()
        .map(|clip| AudioWindow {
            span: clip.span,
            gain_db: clip.gain_db,
            generated: clip.source.as_ref().is_some_and(|source| {
                matches!(source.locator, lattice_core::MediaLocator::Generated { .. })
            }),
            media_name: clip.source.as_ref().map(|source| source.media_name.clone()),
        })
        .collect();
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
