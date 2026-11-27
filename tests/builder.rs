//! Verifies that `azra build` produces a working standalone executable.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn builds_and_runs_a_standalone_executable() {
    let out = std::env::temp_dir().join("azra_builder_test.exe");
    let build = Command::new(env!("CARGO_BIN_EXE_azra"))
        .args([
            "build",
            "examples/hello.vibe",
            "--name",
            "buildertest",
            "--out",
            &out.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(PathBuf::from(&out).exists());
    let run = Command::new(&out).output().unwrap();
    assert!(run.status.success());
    assert!(String::from_utf8_lossy(&run.stdout).contains("Hello from Azra!"));
    let _ = std::fs::remove_file(&out);
}
