//! `AudioPlan` PCM reaches the Studio monitor boundary without an exported movie.

use std::path::PathBuf;

use lattice_studio::{AudioDeviceFormat, AudioProgram, StudioSession};

fn unique_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lattice-studio-audio-e2e-{nanos}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn imported_av_prepares_audio_plan_pcm_without_exporting_mp4() {
    let dir = unique_dir();
    let media = dir.join("capture.mp4");
    lattice_media::generate_av_fixture(&media, 2).expect("A/V fixture");
    let session = StudioSession::open_video(&media).expect("import/open");
    let job = session.request_audio_prepare_job();

    let program = AudioProgram::prepare_job(
        &job,
        AudioDeviceFormat {
            sample_rate: 44_100,
            channels: 2,
        },
    )
    .expect("shared export mix path")
    .expect("imported A/V has an AudioPlan");

    assert_eq!(program.format().sample_rate, 44_100);
    assert_eq!(program.format().channels, 2);
    assert_eq!(
        program.frame_count(),
        u64::try_from(program.report().frame_count).unwrap()
    );
    assert_eq!(program.report().window_count, 1);
    assert_eq!(program.report().decoded_sources.len(), 1);
    assert!(
        program.peak() > 0.01,
        "fixture tone must not become silence"
    );
    assert!(
        !job.media_root().join(".lattice-audio-monitor").exists(),
        "monitor preparation must not render an intermediate movie"
    );

    drop(program);
    drop(session);
    std::fs::remove_dir_all(dir).unwrap();
}
