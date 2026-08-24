use std::path::{Path, PathBuf};
use std::process::Command;

fn lattice_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lattice"))
}

fn sample_vel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/gameplay-commentary/main.vel")
        .canonicalize()
        .expect("sample vel")
}

fn run_ok(args: &[&str]) -> (i32, String) {
    let output = Command::new(lattice_bin())
        .args(args)
        .output()
        .expect("spawn lattice");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        output.status.success(),
        "lattice {args:?} failed: {stderr}\n{stdout}"
    );
    (output.status.code().unwrap_or(1), stdout)
}

#[test]
fn compile_json_twice_agrees() {
    let vel = sample_vel();
    let vel = vel.to_str().unwrap();
    let (_, a) = run_ok(&["--json", "compile", vel, "--emit-ir"]);
    let (_, b) = run_ok(&["--json", "compile", vel, "--emit-ir"]);
    assert_eq!(a, b);
    assert!(a.contains("\"ok\": true") || a.contains("\"ok\":true"));
    assert!(a.contains("freeze") || a.contains("\"rate\": {\n        \"num\": 0"));
    assert!(a.contains("Hello"));
    assert!(a.contains("capture.mp4"));
}

#[test]
fn render_writes_mp4() {
    let dir = std::env::temp_dir().join("lattice-cli-walking");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(sample_vel(), dir.join("main.vel")).unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 21).unwrap();
    let vel = dir.join("main.vel");
    let out = dir.join("preview.mp4");
    let (_, stdout) = run_ok(&[
        "--json",
        "render",
        vel.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
    ]);
    assert!(Path::new(&out).is_file(), "{stdout}");
    assert!(stdout.contains("\"ok\": true") || stdout.contains("\"ok\":true"));
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("render JSON");
    assert_eq!(payload["spec"]["width"], 320);
    assert_eq!(payload["spec"]["height"], 180);
    assert_eq!(payload["spec"]["fps_num"], 10);
    assert_eq!(payload["spec"]["fps_den"], 1);
    assert_eq!(payload["renderer"]["requested"], "require_cpu");
    assert_eq!(payload["renderer"]["active"], "cpu");
    assert!(payload["renderer"]["adapter"].is_null());
    let media = lattice_media::probe_media(&out).expect("probe default output");
    assert_eq!(media.width, Some(320));
    assert_eq!(media.height, Some(180));
    assert_eq!(media.frame_rate_num, Some(10));
    assert_eq!(media.frame_rate_den, Some(1));
}

#[test]
fn render_and_preview_help_expose_renderer_and_output_spec() {
    for command in ["render", "preview"] {
        let output = Command::new(lattice_bin())
            .args([command, "--help"])
            .output()
            .expect("spawn lattice output help");
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        for option in ["--renderer", "--width", "--height", "--fps"] {
            assert!(
                stdout.contains(option),
                "{command} help missing {option}: {stdout}"
            );
        }
        assert!(stdout.contains("gpu-dx12"), "{stdout}");
        assert!(stdout.contains("30000/1001"), "{stdout}");
    }
}

#[test]
fn preview_command_exports_and_reports_1080p_at_30fps() {
    let dir = std::env::temp_dir().join("lattice-cli-1080p");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "hd"

convention commentary

media game "capture.mp4"

sequence main {
  shot
}

scene shot {
  game[0s..1s] as clip
}
"#,
    )
    .unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 1).unwrap();
    let out = dir.join("hd.mp4");
    let (_, stdout) = run_ok(&[
        "--json",
        "preview",
        vel.to_str().unwrap(),
        "-o",
        out.to_str().unwrap(),
        "--width",
        "1920",
        "--height",
        "1080",
        "--fps",
        "30",
    ]);

    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("1080p JSON report");
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["spec"]["width"], 1920);
    assert_eq!(payload["spec"]["height"], 1080);
    assert_eq!(payload["spec"]["fps_num"], 30);
    assert_eq!(payload["spec"]["fps_den"], 1);
    assert_eq!(payload["spec"]["sample_rate"], 44_100);
    assert_eq!(payload["spec"]["channels"], 2);

    let media = lattice_media::probe_media(&out).expect("probe 1080p output");
    assert_eq!(media.width, Some(1920));
    assert_eq!(media.height, Some(1080));
    assert_eq!(media.frame_rate_num, Some(30));
    assert_eq!(media.frame_rate_den, Some(1));
    assert!(media.has_video);
    assert!(media.has_audio);
}

#[test]
fn compile_json_stray_top_level_has_diagnostic_payload() {
    let dir = std::env::temp_dir().join("lattice-cli-stray-top-level");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "stray"

media game "capture.mp4"

sequence main {
  shot
}

stray game[0s..2s]

scene shot {
  game[0s..1s] as clip
}
"#,
    )
    .unwrap();
    let output = Command::new(lattice_bin())
        .args(["--json", "compile", vel.to_str().unwrap()])
        .output()
        .expect("spawn compile");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "compile diagnostics use exit 1 with JSON, not exit 2 Debug: {stderr}\n{stdout}"
    );
    let payload: serde_json::Value = serde_json::from_str(&stdout).expect("compile JSON");
    assert_eq!(payload["ok"], false);
    let diagnostics = payload["diagnostics"]
        .as_array()
        .expect("diagnostics array");
    let stray = diagnostics
        .iter()
        .find(|diag| diag["code"] == "LAT-DSL-001")
        .expect("LAT-DSL-001");
    assert!(stray["span"].is_object(), "{stray}");
    assert!(
        stray["message"]
            .as_str()
            .is_some_and(|message| message.contains("`stray`")),
        "{stray}"
    );
    assert!(
        !stdout.contains("Index") && !stderr.contains("Index"),
        "must not leak Rust Debug: {stderr}\n{stdout}"
    );
}

#[test]
fn gpu_json_failure_is_machine_readable() {
    let dir = std::env::temp_dir().join("lattice-cli-gpu-json");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(
        &vel,
        r#"project "gpu-json"

convention commentary

media game "capture.mp4"

sequence main {
  shot
}

scene shot {
  game[0s..1s] as clip
}
"#,
    )
    .unwrap();
    lattice_media::generate_av_fixture(dir.join("capture.mp4"), 1).unwrap();
    let out = dir.join("gpu.mp4");
    let output = Command::new(lattice_bin())
        .args([
            "--json",
            "render",
            vel.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--renderer",
            "gpu-dx12",
        ])
        .output()
        .expect("spawn GPU render");

    // A real DX12 adapter may complete the request. Headless/non-Windows CI takes
    // the failure path below, which is also covered deterministically by unit tests.
    if output.status.success() {
        let payload: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("GPU success JSON");
        assert_eq!(payload["renderer"]["requested"], "require_gpu_dx12");
        assert_eq!(payload["renderer"]["active"], "gpu_dx12");
        assert!(
            payload["renderer"]["adapter"]
                .as_str()
                .is_some_and(|adapter| !adapter.is_empty())
        );
        return;
    }

    assert_eq!(output.status.code(), Some(2));
    let payload: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("GPU failure JSON");
    assert_eq!(payload["ok"], false);
    assert_eq!(payload["renderer"]["requested"], "require_gpu_dx12");
    assert!(payload["renderer"]["active"].is_null());
    assert!(payload["renderer"]["adapter"].is_null());
    assert!(
        payload["renderer"]["reason"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(matches!(
        payload["failure"]["phase"].as_str(),
        Some("initialization" | "render")
    ));
    assert!(
        payload["failure"]["kind"]
            .as_str()
            .is_some_and(|kind| !kind.is_empty())
    );
    assert!(
        payload["failure"]["reason"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
}
