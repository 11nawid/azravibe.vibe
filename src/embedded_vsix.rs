//! The VS Code extension is embedded in the interpreter, so a single exe can
//! install editor support with `azra setup-vscode` — no `code` CLI required.

pub const VSIX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/install/azra-vibe.vsix"
));

const PACKAGE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/editors/vscode/package.json"
));
const EXTENSION_JS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/editors/vscode/extension.js"
));
const LANGUAGE_CONFIG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/editors/vscode/language-configuration.json"
));
const GRAMMAR: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/editors/vscode/syntaxes/vibe.tmLanguage.json"
));

fn extension_dirs() -> Vec<std::path::PathBuf> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_default();
    let home = std::path::PathBuf::from(home);
    vec![
        home.join(".vscode").join("extensions"),
        home.join(".vscode-insiders").join("extensions"),
        home.join(".vscode-oss").join("extensions"),
    ]
}

/// Install the extension by writing its files into VS Code's extension folder.
/// VS Code discovers the folder on startup, so this works without the CLI.
pub fn install() -> Result<String, String> {
    let mut installed = Vec::new();
    for dir in extension_dirs() {
        if !dir.parent().is_some_and(|parent| parent.exists()) {
            continue;
        }
        let target = dir.join(format!("azravibe.azra-vibe-{}", env!("CARGO_PKG_VERSION")));
        std::fs::create_dir_all(target.join("syntaxes")).map_err(|e| e.to_string())?;
        std::fs::write(target.join("package.json"), PACKAGE_JSON).map_err(|e| e.to_string())?;
        std::fs::write(target.join("extension.js"), EXTENSION_JS).map_err(|e| e.to_string())?;
        std::fs::write(target.join("language-configuration.json"), LANGUAGE_CONFIG)
            .map_err(|e| e.to_string())?;
        std::fs::write(
            target.join("syntaxes").join("vibe.tmLanguage.json"),
            GRAMMAR,
        )
        .map_err(|e| e.to_string())?;
        installed.push(target);
    }

    // Also try the CLI when available, to refresh an already running VS Code.
    let cli_result = std::process::Command::new("code")
        .arg("--install-extension")
        .arg({
            let path = std::env::temp_dir().join("azra-vibe.vsix");
            let _ = std::fs::write(&path, VSIX);
            path
        })
        .arg("--force")
        .status();

    if installed.is_empty() && cli_result.is_err() {
        return Err(
            "no VS Code extensions directory found and `code` CLI is unavailable".to_string(),
        );
    }
    let mut message = if installed.is_empty() {
        "Azra VS Code extension installed via `code` CLI".to_string()
    } else {
        format!(
            "Azra VS Code extension installed to {} (restart VS Code)",
            installed[0].display()
        )
    };
    if let Ok(status) = cli_result {
        if status.success() {
            message.push_str("; CLI refresh complete");
        }
    }
    Ok(message)
}
