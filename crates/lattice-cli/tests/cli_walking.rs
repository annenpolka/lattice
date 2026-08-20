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
}
