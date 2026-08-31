use std::path::PathBuf;
use std::process::Command;

fn lattice_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lattice"))
}

#[test]
fn format_check_write_and_invalid_input_are_safe() {
    let dir = std::env::temp_dir().join("lattice-cli-format");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    let messy = "project   \"demo\"\nscene demo{title \"Hello\"{at 1s for 2s}}\n";
    std::fs::write(&vel, messy).unwrap();

    let check = Command::new(lattice_bin())
        .args(["--json", "fmt", vel.to_str().unwrap(), "--check"])
        .output()
        .unwrap();
    assert_eq!(check.status.code(), Some(1));
    let check_json: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(check_json["ok"], false);
    assert_eq!(check_json["changed"], true);
    assert_eq!(check_json["written"], false);
    assert_eq!(std::fs::read_to_string(&vel).unwrap(), messy);

    let write = Command::new(lattice_bin())
        .args(["--json", "fmt", vel.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(write.status.success());
    let write_json: serde_json::Value = serde_json::from_slice(&write.stdout).unwrap();
    assert_eq!(write_json["changed"], true);
    assert_eq!(write_json["written"], true);
    let formatted = std::fs::read_to_string(&vel).unwrap();
    assert_eq!(
        formatted,
        "project \"demo\"\nscene demo {\n  title \"Hello\" {\n    at 1s for 2s\n  }\n}\n"
    );

    let second = Command::new(lattice_bin())
        .args(["--json", "fmt", vel.to_str().unwrap(), "--check"])
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(second_json["changed"], false);

    let invalid = "scene demo { title @ }\n";
    std::fs::write(&vel, invalid).unwrap();
    let failure = Command::new(lattice_bin())
        .args(["--json", "fmt", vel.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert_eq!(std::fs::read_to_string(&vel).unwrap(), invalid);
}
