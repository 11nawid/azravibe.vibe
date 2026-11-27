//! Standalone executable builder: generates a tiny Rust crate that embeds a
//! `.vibe` script plus the Azra runtime, then compiles it with Cargo.

use std::path::{Path, PathBuf};
use std::process::Command;

fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "azra_app".to_string()
    } else {
        format!("azra_app_{cleaned}")
    }
}

/// Build `script` into a standalone executable at `out_path`.
pub fn build_exe(script: &Path, name: &str, out_path: &Path) -> Result<PathBuf, String> {
    let source = std::fs::read_to_string(script)
        .map_err(|e| format!("cannot read {}: {e}", script.display()))?;
    let package = safe_name(name);
    let crate_root = std::env::temp_dir().join(format!("{package}_build"));
    let src_dir = crate_root.join("src");
    std::fs::create_dir_all(&src_dir).map_err(|e| e.to_string())?;

    // The generated crate depends on this crate by absolute path, so the
    // resulting executable embeds the same runtime the builder was built from.
    let runtime_toml = env!("CARGO_MANIFEST_DIR").replace('\\', "/");

    std::fs::write(
        crate_root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nazra = {{ path = \"{runtime_toml}\" }}\n"
        ),
    )
    .map_err(|e| e.to_string())?;

    std::fs::write(
        src_dir.join("main.rs"),
        "fn main() {\n    let source = include_str!(\"../app.vibe\");\n    match azra::run_source_with_hosts(source) {\n        Ok(text) => print!(\"{text}\"),\n        Err(error) => eprintln!(\"{error}\"),\n    }\n}\n",
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(crate_root.join("app.vibe"), &source).map_err(|e| e.to_string())?;

    let status = Command::new("cargo")
        .args(["build", "--release", "--manifest-path"])
        .arg(crate_root.join("Cargo.toml"))
        .status()
        .map_err(|e| format!("cargo is required to build executables: {e}"))?;
    if !status.success() {
        return Err("cargo failed to build the generated app".to_string());
    }

    let binary = crate_root
        .join("target")
        .join("release")
        .join(if cfg!(windows) {
            format!("{package}.exe")
        } else {
            package.clone()
        });
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&binary, out_path).map_err(|e| format!("cannot copy built executable: {e}"))?;
    Ok(out_path.to_path_buf())
}
