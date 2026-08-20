use std::path::{Path, PathBuf};

use lattice_core::{AssetIdentity, LockedAsset, MediaLocator};
use lattice_engine::{
    AudioMixError, Engine, EngineError, MixSpec, ResolveLock, Time, generate_av_fixture,
};

struct TempProject(PathBuf);

impl TempProject {
    fn new(label: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("lattice-{label}-{}-{nonce}", std::process::id()));
        std::fs::create_dir_all(&path).expect("temp project");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const SILENT_VEL: &str = r#"
project "silent"

sequence main {
  demo
}

scene demo {
  title "Silent" {
    at 0s for 1s
  }
}
"#;

const SPEECH_VEL: &str = r#"
project "speech"

sequence main {
  demo
}

scene demo {
  speech "Line" {
    at 0s for 1s
  }
}
"#;

#[test]
fn engine_distinguishes_no_audio_windows() {
    let engine = Engine::default();
    let compilation = engine.compile(SILENT_VEL).expect("compile silent project");
    let mixed = engine
        .prepare_audio(&compilation.project, Path::new("."), None, MixSpec::PREVIEW)
        .expect("no audio is valid");
    assert!(mixed.is_none());
}

#[test]
fn engine_reports_unresolved_speech_instead_of_silence() {
    let engine = Engine::default();
    let compilation = engine.compile(SPEECH_VEL).expect("compile speech project");
    let error = engine
        .prepare_audio(
            &compilation.project,
            Path::new("."),
            Some(&ResolveLock::new()),
            MixSpec::PREVIEW,
        )
        .expect_err("unresolved speech must fail");
    assert!(matches!(
        error,
        EngineError::Audio(AudioMixError::MissingGeneratedAsset { .. })
    ));
}

#[test]
fn engine_auto_loads_lock_and_prepares_full_pcm() {
    let temp = TempProject::new("engine-audio-monitor");
    let engine = Engine::default();
    let compilation = engine.compile(SPEECH_VEL).expect("compile speech project");
    let media = compilation
        .project
        .media
        .iter()
        .find(|media| matches!(media.locator, MediaLocator::Generated { .. }))
        .expect("generated speech media");
    let speech =
        generate_av_fixture(temp.path().join("speech.mp4"), 1).expect("speech audio fixture");
    let lock = ResolveLock {
        schema_version: 1,
        assets: vec![LockedAsset {
            id: media.id.clone(),
            generator: Some("speech".into()),
            key: "Line".into(),
            path: speech.display().to_string(),
            identity: AssetIdentity::new("speech-fixture"),
            duration: Some(Time::seconds(1)),
            provider: Some("test".into()),
            provider_version: Some("1".into()),
        }],
    };
    std::fs::write(
        temp.path().join("lattice.lock.json"),
        serde_json::to_vec_pretty(&lock).expect("serialize lock"),
    )
    .expect("write lock");

    let prepared = engine
        .prepare_audio(
            &compilation.project,
            temp.path(),
            None,
            MixSpec {
                sample_rate: 8_000,
                channels: 1,
            },
        )
        .expect("prepare locked speech")
        .expect("audio windows");
    assert_eq!(prepared.pcm().sample_rate, 8_000);
    assert_eq!(prepared.pcm().channels, 1);
    assert_eq!(prepared.pcm().frame_count(), 8_000);
    assert!(prepared.pcm().samples.iter().any(|sample| *sample != 0.0));
    assert_eq!(prepared.report().decoded_sources, [media.name.as_str()]);
    assert!(
        prepared
            .samples_from(Time::milliseconds(500))
            .expect("playhead")
            .len()
            < prepared.pcm().samples.len()
    );
}
