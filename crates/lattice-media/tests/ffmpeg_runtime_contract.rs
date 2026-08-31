use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use lattice_core::Time;
use lattice_media::{extract_frame, probe_media};

fn scratch(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lattice-ffmpeg-contract-{}-{nonce}-{name}",
        std::process::id()
    ))
}

#[test]
fn runtime_contract_child() {
    let Ok(scenario) = std::env::var("LATTICE_RUNTIME_CONTRACT_SCENARIO") else {
        return;
    };
    match scenario.as_str() {
        "missing-ffprobe" => {
            let missing = std::env::var("LATTICE_FFPROBE").unwrap();
            let message = probe_media("unused.mp4").unwrap_err().to_string();
            assert!(message.contains(&missing), "{message}");
            assert!(message.contains("probing media"), "{message}");
            assert!(message.contains("LATTICE_FFPROBE"), "{message}");
        }
        "missing-ffmpeg" => {
            let missing = std::env::var("LATTICE_FFMPEG").unwrap();
            let output = scratch("frame.ppm");
            let message = extract_frame(std::path::Path::new("unused.mp4"), Time::ZERO, &output)
                .unwrap_err()
                .to_string();
            assert!(message.contains(&missing), "{message}");
            assert!(message.contains("extracting a video frame"), "{message}");
            assert!(message.contains("LATTICE_FFMPEG"), "{message}");
        }
        "invalid-codec" => {
            let input = scratch("invalid.mp4");
            let output = scratch("invalid.ppm");
            std::fs::write(&input, b"not a media container").unwrap();

            let probe = probe_media(&input).unwrap_err().to_string();
            assert!(
                probe.contains("ffprobe failed while probing media"),
                "{probe}"
            );
            assert!(probe.contains("status"), "{probe}");

            let decode = extract_frame(&input, Time::ZERO, &output)
                .unwrap_err()
                .to_string();
            assert!(
                decode.contains("ffmpeg failed while extracting a video frame"),
                "{decode}"
            );
            assert!(decode.contains("status"), "{decode}");

            std::fs::remove_file(input).unwrap();
        }
        other => panic!("unknown runtime contract scenario `{other}`"),
    }
}

#[test]
fn missing_tools_and_codec_failures_are_actionable_in_isolated_processes() {
    let test_binary = std::env::current_exe().unwrap();
    for scenario in ["missing-ffprobe", "missing-ffmpeg", "invalid-codec"] {
        let mut command = Command::new(&test_binary);
        command
            .args(["--exact", "runtime_contract_child", "--nocapture"])
            .env("LATTICE_RUNTIME_CONTRACT_SCENARIO", scenario);
        if scenario == "missing-ffprobe" {
            command.env("LATTICE_FFPROBE", scratch("missing-ffprobe"));
        }
        if scenario == "missing-ffmpeg" {
            command.env("LATTICE_FFMPEG", scratch("missing-ffmpeg"));
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "scenario {scenario} failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
