use std::path::PathBuf;
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

fn run_ok(args: &[&str]) -> String {
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
    stdout
}

#[test]
fn json_inspect_propose_reject_then_apply() {
    let original = std::fs::read_to_string(sample_vel()).unwrap();
    let dir = std::env::temp_dir().join("lattice-cli-edit");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(&vel, &original).unwrap();
    let vel_s = vel.to_str().unwrap();

    let inspect = run_ok(&["--json", "inspect", vel_s, "--locus", "demo:title:1"]);
    assert!(inspect.contains("demo:title:1"));
    assert!(inspect.contains("Hello"));

    let propose = run_ok(&[
        "--json",
        "propose",
        vel_s,
        "--locus",
        "demo:title:1",
        "--title-text",
        "World",
    ]);
    assert!(propose.contains("World"));
    assert_eq!(std::fs::read_to_string(&vel).unwrap(), original);

    let proposal_path = dir.join("proposal.json");
    std::fs::write(&proposal_path, &propose).unwrap();
    let reject = run_ok(&[
        "--json",
        "reject",
        vel_s,
        "--proposal",
        proposal_path.to_str().unwrap(),
    ]);
    assert!(reject.contains("\"unchanged\": true") || reject.contains("\"unchanged\":true"));
    assert_eq!(std::fs::read_to_string(&vel).unwrap(), original);

    let apply = run_ok(&[
        "--json",
        "apply",
        vel_s,
        "--proposal",
        proposal_path.to_str().unwrap(),
    ]);
    assert!(apply.contains("\"applied\": true") || apply.contains("\"applied\":true"));
    let after = std::fs::read_to_string(&vel).unwrap();
    assert!(after.contains("World"));
    assert_ne!(after, original);
}
