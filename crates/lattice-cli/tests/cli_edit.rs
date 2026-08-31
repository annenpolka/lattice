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

fn run_fail(args: &[&str]) -> String {
    let output = Command::new(lattice_bin())
        .args(args)
        .output()
        .expect("spawn lattice");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        !output.status.success(),
        "lattice {args:?} unexpectedly succeeded: {stderr}\n{stdout}"
    );
    format!("{stdout}{stderr}")
}

fn proposal_json(vel: &str, args: &[&str]) -> serde_json::Value {
    let mut command = vec!["--json", "propose", vel];
    command.extend_from_slice(args);
    serde_json::from_str(&run_ok(&command)).expect("proposal JSON")
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
    assert!(
        inspect.contains("\"legal\"") && inspect.contains("\"set-position\""),
        "inspect --json must expose the Engine legal set: {inspect}"
    );

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

#[test]
fn json_apply_rejects_stale_proposal() {
    let original = std::fs::read_to_string(sample_vel()).unwrap();
    let dir = std::env::temp_dir().join("lattice-cli-stale");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(&vel, &original).unwrap();
    let vel_s = vel.to_str().unwrap();
    let propose = run_ok(&[
        "--json",
        "propose",
        vel_s,
        "--locus",
        "demo:title:1",
        "--title-text",
        "World",
    ]);
    let proposal_path = dir.join("proposal.json");
    std::fs::write(&proposal_path, &propose).unwrap();
    std::fs::write(&vel, original.replace("title \"Hello\"", "title \"Later\"")).unwrap();
    let output = Command::new(lattice_bin())
        .args([
            "--json",
            "apply",
            vel_s,
            "--proposal",
            proposal_path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        !output.status.success(),
        "stale apply must fail: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("LAT-EDIT-STALE") || stdout.contains("stale"),
        "{stdout}"
    );
    assert!(
        stdout.contains("\"applied\": false") || stdout.contains("\"applied\":false"),
        "{stdout}"
    );
    let after = std::fs::read_to_string(&vel).unwrap();
    assert!(after.contains("Later"));
    assert!(!after.contains("World"));
}

#[test]
fn json_propose_callout_text_rewrites_via_semantic_edit() {
    let original = std::fs::read_to_string(sample_vel()).unwrap();
    let dir = std::env::temp_dir().join("lattice-cli-callout-body");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    std::fs::write(&vel, &original).unwrap();
    let vel_s = vel.to_str().unwrap();

    let propose = run_ok(&["--json", "propose", vel_s, "--callout-text", "Release"]);
    assert!(propose.contains("Release"), "{propose}");
    assert!(
        propose.contains("\"kind\": \"callout\"") || propose.contains("\"kind\":\"callout\""),
        "{propose}"
    );
    assert_eq!(std::fs::read_to_string(&vel).unwrap(), original);

    let proposal_path = dir.join("proposal.json");
    std::fs::write(&proposal_path, &propose).unwrap();
    let apply = run_ok(&[
        "--json",
        "apply",
        vel_s,
        "--proposal",
        proposal_path.to_str().unwrap(),
    ]);
    assert!(apply.contains("\"applied\": true") || apply.contains("\"applied\":true"));
    let after = std::fs::read_to_string(&vel).unwrap();
    assert!(after.contains("callout \"Release\""));
    assert!(!after.contains("callout \"Hold\""));
}

#[test]
#[allow(clippy::too_many_lines)]
fn json_propose_exposes_every_semantic_edit_kind() {
    let dir = std::env::temp_dir().join("lattice-cli-all-edit-kinds");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let vel = dir.join("main.vel");
    let source = r#"project "all-edits"
media game "capture.mp4"
sequence main {
  intro
  outro
}
scene intro {
  game[0s..6s] as intro_clip
  title "Hello" { at 1s for 2s }
  callout "Hold" { at 2s for 1s }
}
scene outro {
  game[6s..12s] as outro_clip
}
"#;
    std::fs::write(&vel, source).unwrap();
    let vel_s = vel.to_str().unwrap();

    let cases: &[(&[&str], &str)] = &[
        (
            &[
                "--edit",
                "title",
                "--locus",
                "intro:title:1",
                "--title-text",
                "Edited",
            ],
            "title",
        ),
        (
            &[
                "--edit",
                "callout",
                "--locus",
                "intro:callout:2",
                "--callout-text",
                "Release",
            ],
            "callout",
        ),
        (
            &[
                "--edit",
                "trim",
                "--locus",
                "source:intro_clip",
                "--trim-in",
                "1s",
                "--trim-out",
                "4s",
            ],
            "trim",
        ),
        (
            &[
                "--edit",
                "split",
                "--locus",
                "scene:intro",
                "--split-at",
                "2s",
            ],
            "split",
        ),
        (&["--edit", "delete", "--locus", "scene:outro"], "delete"),
        (
            &[
                "--edit",
                "set-gain",
                "--locus",
                "source:intro_clip",
                "--gain-db",
                "-6",
            ],
            "set-gain",
        ),
        (
            &[
                "--edit",
                "set-fade",
                "--locus",
                "source:intro_clip",
                "--fade-in",
                "0.5s",
            ],
            "set-fade",
        ),
        (
            &[
                "--edit",
                "reorder-scene",
                "--locus",
                "scene:outro",
                "--before",
                "intro",
            ],
            "reorder-scene",
        ),
        (
            &[
                "--edit",
                "set-position",
                "--locus",
                "intro:title:1",
                "--position-x",
                "12.5",
                "--position-y",
                "75%",
            ],
            "set-position",
        ),
        (
            &[
                "--edit",
                "resize-overlay",
                "--locus",
                "intro:callout:2",
                "--position-x",
                "20",
                "--position-y",
                "30",
                "--scale",
                "125.5%",
            ],
            "resize-overlay",
        ),
    ];

    for (args, expected_kind) in cases {
        let proposal = proposal_json(vel_s, args);
        assert_eq!(proposal["edit"]["kind"], *expected_kind, "{args:?}");
        assert_eq!(
            std::fs::read_to_string(&vel).unwrap(),
            source,
            "propose must not mutate VEL for {expected_kind}"
        );
    }

    let positioned = proposal_json(
        vel_s,
        &[
            "--edit",
            "set-position",
            "--locus",
            "intro:title:1",
            "--position-x",
            "12.5",
            "--position-y",
            "75",
        ],
    );
    assert_eq!(positioned["edit"]["position"]["x"], 1_250);
    assert_eq!(positioned["edit"]["position"]["y"], 7_500);

    let resized = proposal_json(
        vel_s,
        &[
            "--edit",
            "resize-overlay",
            "--locus",
            "intro:callout:2",
            "--position-x",
            "20",
            "--position-y",
            "30",
            "--scale",
            "125.5",
        ],
    );
    assert_eq!(resized["edit"]["scale"]["milli"], 1_255);
}

#[test]
fn propose_rejects_ambiguous_or_incomplete_edit_arguments() {
    let vel = sample_vel();
    let vel_s = vel.to_str().unwrap();

    let missing_locus = run_fail(&[
        "--json",
        "propose",
        vel_s,
        "--edit",
        "split",
        "--split-at",
        "2s",
    ]);
    assert!(
        missing_locus.contains("--locus is required"),
        "{missing_locus}"
    );

    let mixed_kinds = run_fail(&[
        "--json",
        "propose",
        vel_s,
        "--edit",
        "trim",
        "--locus",
        "source:gameplay",
        "--trim-in",
        "1s",
        "--gain-db",
        "-6",
    ]);
    assert!(mixed_kinds.contains("cannot be combined"), "{mixed_kinds}");

    let invalid_position = run_fail(&[
        "--json",
        "propose",
        vel_s,
        "--edit",
        "set-position",
        "--locus",
        "demo:title:1",
        "--position-x",
        "101",
        "--position-y",
        "50",
    ]);
    assert!(
        invalid_position.contains("between 0% and 100%"),
        "{invalid_position}"
    );
}
