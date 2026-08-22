//! Alpha VEL and the shared renderer exercised as one shipped path.

use std::path::{Path, PathBuf};

use lattice_core::MediaLocator;
use lattice_engine::{
    Engine, LocalToneProvider, PreviewFrameRequest, ResolveLock, ResolveOptions, Time,
};
use lattice_media::{
    PREVIEW_HEIGHT, PREVIEW_WIDTH, content_pixels, extract_frame, generate_av_fixture,
    mean_abs_diff, probe_duration, probe_media, title_bar_present,
};

struct TempProject(PathBuf);

impl TempProject {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lattice-alpha-renderer-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create temporary project");
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

fn sample_vel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/gameplay-commentary/main.vel")
        .canonicalize()
        .expect("gameplay-commentary main.vel")
}

fn request(at: Time) -> PreviewFrameRequest {
    PreviewFrameRequest {
        timeline_time: at,
        width: PREVIEW_WIDTH,
        height: PREVIEW_HEIGHT,
        fps_num: 10,
        fps_den: 1,
    }
}

fn abs_diff(left: Time, right: Time) -> Time {
    if left > right {
        left - right
    } else {
        right - left
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn gameplay_commentary_resolves_samples_and_exports_through_one_renderer_contract() {
    let temp = TempProject::new();
    let root = temp.path();
    let vel = root.join("main.vel");
    std::fs::copy(sample_vel(), &vel).expect("copy Alpha VEL");
    generate_av_fixture(root.join("capture.mp4"), 21).expect("generate A/V fixture");

    let engine = Engine::default();
    let compilation = engine.compile_path(&vel).expect("compile Alpha VEL");
    assert!(
        !compilation.has_errors(),
        "Alpha VEL diagnostics: {:?}",
        compilation.diagnostics
    );

    let artifact_dir = root.join(".lattice");
    let mut provider = LocalToneProvider;
    let resolution = engine
        .resolve(
            &compilation.project,
            &ResolveOptions {
                media_root: root,
                artifact_dir: &artifact_dir,
                lock: None,
            },
            &mut provider,
        )
        .expect("resolve generated speech");
    assert_eq!(
        resolution.provider_calls, 1,
        "the single speech invocation must materialize exactly once"
    );

    let speech = resolution
        .assets
        .iter()
        .find(|asset| {
            matches!(
                &asset.locator,
                MediaLocator::Generated { generator, .. } if generator == "speech"
            )
        })
        .expect("resolved speech asset");
    let speech_path = PathBuf::from(&speech.path);
    assert!(speech_path.is_file(), "missing {}", speech_path.display());
    assert!(
        speech_path.starts_with(&artifact_dir),
        "speech must be materialized below {}: {}",
        artifact_dir.display(),
        speech_path.display()
    );
    let speech_info = probe_media(&speech_path).expect("probe resolved speech");
    assert!(speech_info.has_audio && !speech_info.has_video);
    assert!(
        abs_diff(
            speech_info.duration,
            Time::from_decimal_seconds(1, 5, 1).expect("1.5s")
        ) < Time::milliseconds(20),
        "resolved speech duration {}",
        speech_info.duration
    );

    let locked_speech = resolution
        .lock
        .assets
        .iter()
        .find(|asset| asset.generator.as_deref() == Some("speech"))
        .expect("speech lock entry");
    assert_eq!(
        locked_speech.duration,
        Some(Time::from_decimal_seconds(1, 5, 1).unwrap())
    );
    let locked_path = Path::new(&locked_speech.path);
    let locked_path = if locked_path.is_absolute() {
        locked_path.to_path_buf()
    } else {
        root.join(locked_path)
    };
    assert!(
        locked_path.is_file(),
        "lock points at missing artifact {}",
        locked_path.display()
    );

    let lock_path = root.join("lattice.lock.json");
    std::fs::write(
        &lock_path,
        serde_json::to_vec_pretty(&resolution.lock).expect("serialize lock"),
    )
    .expect("persist lock");
    let lock: ResolveLock = serde_json::from_slice(
        &std::fs::read(&lock_path).expect("read persisted lattice.lock.json"),
    )
    .expect("parse persisted lattice.lock.json");
    assert_eq!(lock, resolution.lock);

    let title_time = Time::seconds(3);
    let (_scene, sampled_title) = engine
        .sample_frame(
            &compilation.project,
            &request(title_time),
            root,
            Some(&lock),
        )
        .expect("sample title frame");
    let sampled_title_path = root.join("sampled-title.ppm");
    sampled_title
        .write_ppm(&sampled_title_path)
        .expect("write sampled title frame");
    assert!(
        title_bar_present(&sampled_title_path).expect("scan sampled title"),
        "sample-at-t must include the title overlay"
    );

    let output = root.join("alpha-renderer.mp4");
    let report = engine
        .render_with_lock(&compilation.project, &output, root, Some(&lock))
        .expect("render resolved Alpha project");
    let expected_duration = Time::from_decimal_seconds(11, 5, 1).expect("11.5s");
    assert_eq!(report.duration, expected_duration);
    assert_eq!(report.plan.hold_segments, 1);
    assert_eq!(report.plan.overlays, 2, "title and callout must render");
    assert!(output.is_file(), "missing {}", output.display());
    assert_eq!(
        probe_duration(&output).expect("probe export"),
        expected_duration
    );
    let output_info = probe_media(&output).expect("probe output streams");
    assert!(output_info.has_video && output_info.has_audio);

    let exported_title = extract_frame(&output, title_time, &root.join("exported-title.ppm"))
        .expect("extract exported title frame");
    let sampled_content = content_pixels(&sampled_title_path).expect("sampled pixels");
    let exported_content = content_pixels(&exported_title).expect("exported pixels");
    let parity_delta = mean_abs_diff(&sampled_content, &exported_content);
    assert!(
        parity_delta < 12,
        "sample/export renderer parity delta {parity_delta}"
    );
    assert!(
        title_bar_present(&exported_title).expect("scan exported title"),
        "export must include the title overlay"
    );
    let before_title = extract_frame(&output, Time::seconds(1), &root.join("before-title.ppm"))
        .expect("extract frame before title");
    assert!(
        !title_bar_present(&before_title).expect("scan frame before title"),
        "title must be absent before its window"
    );
    let hold_a = extract_frame(
        &output,
        Time::from_decimal_seconds(5, 3, 1).unwrap(),
        &root.join("hold-a.ppm"),
    )
    .expect("extract first hold frame");
    let hold_b = extract_frame(
        &output,
        Time::from_decimal_seconds(6, 5, 1).unwrap(),
        &root.join("hold-b.ppm"),
    )
    .expect("extract second hold frame");
    let moving = extract_frame(&output, Time::seconds(9), &root.join("moving.ppm"))
        .expect("extract moving frame");
    let hold_a = content_pixels(&hold_a).expect("first hold pixels");
    let hold_b = content_pixels(&hold_b).expect("second hold pixels");
    let moving = content_pixels(&moving).expect("moving pixels");
    let hold_delta = mean_abs_diff(&hold_a, &hold_b);
    let motion_delta = mean_abs_diff(&hold_a, &moving);
    assert!(
        hold_delta < 8,
        "freeze frames must remain still (delta {hold_delta})"
    );
    assert!(
        motion_delta > hold_delta.saturating_add(8),
        "moving content must differ from the hold (hold {hold_delta}, moving {motion_delta})"
    );
}
