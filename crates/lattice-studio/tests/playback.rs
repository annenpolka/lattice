//! Real-media coverage for Studio's bounded, video-only playback path.
//!
//! The fixture contains audio so the boundary stays explicit: Studio samples `RawFrame` values;
//! synchronized audio remains an export concern until Studio has an audio-device clock.

use std::path::PathBuf;
use std::sync::Arc;

use lattice_engine::{PreviewFrameRequest, RendererRequest, Time};
use lattice_studio::{PreviewInbox, PreviewPush, StudioSession};

fn unique_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("create fixture directory");
    dir
}

#[test]
#[allow(clippy::too_many_lines)] // One sampler/session must span the transport epoch transitions.
fn latest_wins_playback_rejects_results_across_pause_seek_and_scrub() {
    let dir = unique_dir("bounded-playback");
    let media = dir.join("capture.mp4");
    lattice_media::generate_av_fixture_rate(&media, 4, 30, 1).expect("generate 30fps A/V fixture");
    assert!(
        lattice_media::has_audio_stream(&media).expect("probe fixture audio"),
        "the source has audio even though Studio playback currently samples video only"
    );

    let mut session = StudioSession::open_video(&media).expect("open imported video");

    // One active and one pending job is the hard scheduler bound. New pending work replaces old
    // pending work rather than building latency behind the playhead.
    let inbox = PreviewInbox::new();
    let first = session.request_preview_job();
    let second = session.request_preview_job();
    assert_eq!(session.frame_rate(), Some((30, 1)));
    assert_eq!((first.fps_num, first.fps_den), (30, 1));
    assert_eq!((second.fps_num, second.fps_den), (30, 1));
    assert_eq!(inbox.push(first), PreviewPush::Queued);
    assert_eq!(inbox.push(second.clone()), PreviewPush::Replaced);
    let active = inbox.take_wait().expect("latest pending job");
    assert_eq!(active.generation, second.generation);

    let third = session.request_preview_job();
    let fourth = session.request_preview_job();
    assert_eq!(inbox.push(third), PreviewPush::Queued);
    assert_eq!(inbox.push(fourth.clone()), PreviewPush::Replaced);
    assert_eq!(
        inbox.stats(),
        lattice_studio::PreviewInboxStats {
            pending: 1,
            in_flight: 1,
            replaced_pending: 2,
            stopped: false,
        }
    );
    inbox.complete(active.generation);
    let next = inbox.take_wait().expect("replacement job");
    assert_eq!(next.generation, fourth.generation);
    inbox.complete(next.generation);
    inbox.stop();
    assert_eq!(
        inbox.push(session.request_preview_job()),
        PreviewPush::Stopped
    );
    let gpu_job = session.request_preview_job_with_renderer(RendererRequest::RequireGpuDx12);
    assert_eq!(gpu_job.renderer, RendererRequest::RequireGpuDx12);

    let media_root = session
        .path()
        .parent()
        .expect("imported project directory")
        .to_path_buf();
    let (width, height) = session.preview_pixel_size();
    let mut sampler = session
        .engine()
        .preview_sampler(
            &session.compilation().project,
            &PreviewFrameRequest {
                timeline_time: Time::ZERO,
                width,
                height,
                fps_num: session.frame_rate().unwrap_or((10, 1)).0,
                fps_den: session.frame_rate().unwrap_or((10, 1)).1,
            },
            &media_root,
            &media_root.join("unused-still.png"),
            None,
        )
        .expect("open reusable preview sampler");

    session.play();
    let slow = session.request_preview_job();
    session.step_clock(Time::milliseconds(40));
    let newer = session.request_preview_job();
    assert_eq!(
        newer.timeline_time,
        Time::from_frames(1, 30, 1).expect("30fps frame time"),
        "Studio display cadence must follow the probed 30fps source, not export's 10fps"
    );
    let (_, slow_frame) = sampler
        .sample(slow.timeline_time)
        .expect("sample slow frame");
    assert!(
        session.accept_preview_frame_stamped(
            slow.generation,
            Arc::new(slow_frame.clone()),
            slow.timeline_time,
            &slow.stamp,
        ),
        "a completed monotonic frame may publish while a newer Play request is queued"
    );
    let (_, newer_frame) = sampler
        .sample(newer.timeline_time)
        .expect("sample newer frame");
    assert!(session.accept_preview_frame_stamped(
        newer.generation,
        Arc::new(newer_frame.clone()),
        newer.timeline_time,
        &newer.stamp,
    ));
    assert_eq!(
        session.preview_mailbox().published_time(),
        Some(newer.timeline_time)
    );
    assert!(session.preview_mailbox().retained_frame_count() <= 2);
    let changed = slow_frame
        .rgba
        .iter()
        .zip(&newer_frame.rgba)
        .filter(|(a, b)| a.abs_diff(**b) > 8)
        .count();
    assert!(
        changed > 64,
        "moving frames must advance, changed={changed}"
    );

    // Pause creates a hard transport epoch. A decode that completed after Pause cannot flash;
    // the exact paused position is then requested and accepted while the last frame stays visible.
    session.step_clock(Time::milliseconds(250));
    let before_pause = session.request_preview_job();
    let (_, before_pause_frame) = sampler
        .sample(before_pause.timeline_time)
        .expect("sample in-flight pause frame");
    session.pause();
    assert!(!session.is_playing());
    assert!(!session.accept_preview_frame_stamped(
        before_pause.generation,
        Arc::new(before_pause_frame),
        before_pause.timeline_time,
        &before_pause.stamp,
    ));
    let paused = session.request_preview_job();
    let (_, paused_frame) = sampler
        .sample(paused.timeline_time)
        .expect("sample exact pause frame");
    assert!(session.accept_preview_frame_stamped(
        paused.generation,
        Arc::new(paused_frame),
        paused.timeline_time,
        &paused.stamp,
    ));
    assert_eq!(paused.timeline_time, session.snapped_preview_time());

    // Seek and scrub are also hard epochs and remain strict latest-only while stopped.
    session.play();
    let before_seek = session.request_preview_job();
    let (_, before_seek_frame) = sampler
        .sample(before_seek.timeline_time)
        .expect("sample in-flight seek frame");
    session.seek(Time::milliseconds(1_200));
    assert!(!session.accept_preview_frame_stamped(
        before_seek.generation,
        Arc::new(before_seek_frame),
        before_seek.timeline_time,
        &before_seek.stamp,
    ));
    let sought = session.request_preview_job();
    assert_eq!(sought.timeline_time, Time::milliseconds(1_200));
    let (_, sought_frame) = sampler
        .sample(sought.timeline_time)
        .expect("sample exact seek frame");
    assert!(session.accept_preview_frame_stamped(
        sought.generation,
        Arc::new(sought_frame),
        sought.timeline_time,
        &sought.stamp,
    ));

    session.play();
    session.step_clock(Time::milliseconds(300));
    let before_scrub = session.request_preview_job();
    let (_, before_scrub_frame) = sampler
        .sample(before_scrub.timeline_time)
        .expect("sample in-flight scrub frame");
    session.scrub(Time::milliseconds(100));
    assert!(!session.accept_preview_frame_stamped(
        before_scrub.generation,
        Arc::new(before_scrub_frame),
        before_scrub.timeline_time,
        &before_scrub.stamp,
    ));
    let scrubbed = session.request_preview_job();
    assert_eq!(scrubbed.timeline_time, Time::milliseconds(100));
    let (_, scrubbed_frame) = sampler
        .sample(scrubbed.timeline_time)
        .expect("restart decoder for backward scrub");
    assert!(session.accept_preview_frame_stamped(
        scrubbed.generation,
        Arc::new(scrubbed_frame),
        scrubbed.timeline_time,
        &scrubbed.stamp,
    ));
    assert_eq!(
        session.preview_mailbox().published_time(),
        Some(Time::milliseconds(100))
    );
    assert!(
        !media_root.join(".lattice-cache").exists(),
        "in-memory playback sampling must not write still files"
    );
}
