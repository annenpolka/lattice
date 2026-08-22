use std::path::PathBuf;
use std::process::Command;

use lattice_media::generate_av_fixture;

fn lattice_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lattice"))
}

fn run(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(lattice_bin())
        .args(args)
        .output()
        .expect("spawn lattice");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn import_twice_agrees_and_compiles() {
    let dir = std::env::temp_dir().join("lattice-cli-import");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let media = dir.join("gameplay.mp4");
    generate_av_fixture(&media, 6).expect("fixture");
    let media_s = media.to_str().unwrap();
    let out1 = dir.join("run1");
    let out2 = dir.join("run2");
    let (ok1, stdout1, stderr1) = run(&["--json", "import", media_s, "-o", out1.to_str().unwrap()]);
    assert!(ok1, "import 1 failed: {stderr1}\n{stdout1}");
    let (ok2, stdout2, stderr2) = run(&["--json", "import", media_s, "-o", out2.to_str().unwrap()]);
    assert!(ok2, "import 2 failed: {stderr2}\n{stdout2}");
    assert!(stdout1.contains("gameplay.mp4") || stdout1.contains("locator"));
    assert!(stdout2.contains("gameplay.mp4") || stdout2.contains("locator"));
    let vel1 = std::fs::read_to_string(out1.join("main.vel")).unwrap();
    let vel2 = std::fs::read_to_string(out2.join("main.vel")).unwrap();
    assert_eq!(vel1, vel2);
    assert!(vel1.contains("gameplay.mp4") || vel1.contains("../gameplay.mp4"));
    let (okc, compile, errc) = run(&["--json", "compile", out1.join("main.vel").to_str().unwrap()]);
    assert!(okc, "compile imported vel: {errc}\n{compile}");
    assert!(compile.contains("\"ok\": true") || compile.contains("\"ok\":true"));
}
