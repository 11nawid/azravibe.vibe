//! The VS Code extension is embedded in the interpreter so a single exe can
//! install editor support on demand (`azra setup-vscode`).

pub const VSIX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/install/azra-vibe.vsix"
));

/// Write the embedded extension to a temporary `.vsix` and install it with
/// the VS Code CLI if available.
pub fn install() -> Result<String, String> {
    let path = std::env::temp_dir().join("azra-vibe.vsix");
    std::fs::write(&path, VSIX).map_err(|e| e.to_string())?;
    let status = std::process::Command::new("code")
        .arg("--install-extension")
        .arg(&path)
        .arg("--force")
        .status()
        .map_err(|e| format!("VS Code CLI (`code`) not found: {e}"))?;
    if status.success() {
        Ok("Azra VS Code extension installed".to_string())
    } else {
        Err(format!("VS Code extension install failed (exit {status})"))
    }
}
