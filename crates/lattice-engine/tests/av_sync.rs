//! Video/audio duration must stay locked after freeze on high-fps sources.

use lattice_core::Time;
use lattice_engine::Engine;
use lattice_media::{generate_av_fixture_rate, probe_duration, probe_media};

#[test]
fn freeze_on_30fps_source_keeps_video_and_audio_duration() {
    let dir = std::env::temp_dir().join("lattice-av-sync-30fps");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    generate_av_fixture_rate(dir.join("capture.mp4"), 16, 30, 1).unwrap();
    let vel = r#"project "sync"
convention commentary
media game "capture.mp4"
scene demo {
  game[0s..10s] as fight
  freeze fight at 8s for 1.5s
}
"#;
    let engine = Engine::default();
    let compilation = engine.compile(vel).unwrap();
    let out = dir.join("out.mp4");
    let report = engine
        .render(&compilation.project, &out, &dir)
        .expect("render");
    let expected = Time::from_decimal_seconds(11, 5, 1).unwrap();
    assert_eq!(report.duration, expected);

    let format = probe_duration(&out).unwrap();
    let info = probe_media(&out).unwrap();
    assert!(info.has_video && info.has_audio);

    let (video_dur, audio_dur) = probe_stream_durations(&out);
    let drift = abs_diff(video_dur, audio_dur);
    assert!(
        drift < Time::milliseconds(150),
        "video {video_dur} vs audio {audio_dur} (drift {drift})"
    );
    let vs_plan = abs_diff(video_dur, expected);
    assert!(
        vs_plan < Time::milliseconds(150),
        "video {video_dur} vs timeline {expected}"
    );
    let _ = format;
}

fn abs_diff(a: Time, b: Time) -> Time {
    if a > b { a - b } else { b - a }
}

fn probe_stream_durations(path: &std::path::Path) -> (Time, Time) {
    let output = std::process::Command::new(lattice_media::ffprobe_bin())
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .expect("ffprobe");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let mut video = None;
    let mut audio = None;
    for line in text.lines() {
        let mut parts = line.split(',');
        let kind = parts.next().unwrap_or("");
        let dur = parts.next().unwrap_or("");
        let parsed = parse_seconds(dur);
        match kind {
            "video" => video = Some(parsed),
            "audio" => audio = Some(parsed),
            _ => {}
        }
    }
    (
        video.expect("video duration"),
        audio.expect("audio duration"),
    )
}

fn parse_seconds(text: &str) -> Time {
    let (whole, frac) = text.split_once('.').unwrap_or((text, "0"));
    let whole: i64 = whole.parse().unwrap_or(0);
    let digits: String = frac.chars().filter(char::is_ascii_digit).collect();
    let mut padded = digits;
    while padded.len() < 3 {
        padded.push('0');
    }
    let millis: i64 = padded[..3].parse().unwrap_or(0);
    Time::milliseconds(whole * 1000 + millis)
}
