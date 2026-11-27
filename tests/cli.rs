//! CLI binary tests for `azra`.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn azra_bin() -> &'static str {
    env!("CARGO_BIN_EXE_azra")
}

fn temp_script(name: &str, body: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("azra_cli_{}_{}.vibe", name, nanos));
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn cli_run_succeeds_without_when_game_starts() {
    let out = Command::new(azra_bin())
        .args(["run", "examples/hello.vibe"])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Hello from Azra!"));
}

#[test]
fn cli_run_succeeds_with_ticks_and_optional_lifecycle_handlers() {
    let script = temp_script(
        "ticks_no_handlers",
        "set x to 0\nafter 0.5 seconds:\n    say \"timer_fired\"\n",
    );
    let out = Command::new(azra_bin())
        .args(["run", script.to_str().unwrap(), "--ticks", "30"])
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&script);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.trim(), "timer_fired");
}

#[test]
fn cli_run_fires_when_game_starts_and_updates_when_present() {
    let script = temp_script(
        "with_lifecycle",
        "set frames to 0\nwhen game starts:\n    say \"boot\"\nwhen game updates with dt:\n    add 1 to frames\nafter 0.1 seconds:\n    say \"frames=\", frames\n",
    );
    let out = Command::new(azra_bin())
        .args(["run", script.to_str().unwrap(), "--ticks", "6"])
        .output()
        .unwrap();
    let _ = std::fs::remove_file(&script);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["boot", "frames= 6"]);
}

#[test]
fn cli_run_trigger_and_unknown_trigger_behavior() {
    let out = Command::new(azra_bin())
        .args([
            "run",
            "examples/game_demo.vibe",
            "--trigger",
            "player touches lava_block",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Game Over!"));

    let bad = Command::new(azra_bin())
        .args(["run", "examples/hello.vibe", "--trigger", "missing event"])
        .output()
        .unwrap();
    assert!(!bad.status.success());
}

#[test]
fn cli_check_and_eval() {
    let check_ok = Command::new(azra_bin())
        .args(["check", "examples/game_demo.vibe"])
        .output()
        .unwrap();
    assert!(check_ok.status.success());

    let eval_ok = Command::new(azra_bin())
        .args(["eval", "say 2 + 3"])
        .output()
        .unwrap();
    assert!(eval_ok.status.success());
    assert_eq!(String::from_utf8_lossy(&eval_ok.stdout).trim(), "5");
}
