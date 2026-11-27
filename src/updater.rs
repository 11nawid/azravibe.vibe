//! `azra update` — download the latest release and replace the running exe.

use std::path::{Path, PathBuf};

fn asset_name() -> &'static str {
    if cfg!(windows) {
        "azra-windows-x86_64.zip"
    } else if cfg!(target_os = "macos") {
        "azra-macos-x86_64.tar.gz"
    } else {
        "azra-linux-x86_64.tar.gz"
    }
}

fn download(url: &str, destination: &Path) -> Result<(), String> {
    let status = std::process::Command::new("curl")
        .arg("-fsSL")
        .arg(url)
        .arg("-o")
        .arg(destination)
        .status()
        .map_err(|e| format!("curl is required to update: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("failed to download the latest release".to_string())
    }
}

fn extract(archive: &Path, directory: &Path) -> Result<(), String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let status = if cfg!(windows) {
        let command = format!(
            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
            archive.display().to_string().replace('\'', "''"),
            directory.display().to_string().replace('\'', "''")
        );
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &command])
            .status()
    } else {
        std::process::Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(directory)
            .status()
    }
    .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("failed to extract the release archive".to_string())
    }
}

fn version_of(exe: &Path) -> String {
    std::process::Command::new(exe)
        .arg("--version")
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Replace the current executable with the latest release.
pub fn update() -> Result<String, String> {
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let temp = std::env::temp_dir().join(format!("azra-update-{}", std::process::id()));
    let archive = temp.join(asset_name());
    std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;

    let url = format!(
        "https://github.com/11nawid/azravibe.vibe/releases/latest/download/{}",
        asset_name()
    );
    println!("Checking {url} ...");
    download(&url, &archive)?;
    extract(&archive, &temp)?;

    let downloaded = temp.join(if cfg!(windows) { "azra.exe" } else { "azra" });
    let new_version = version_of(&downloaded);
    let current_version = format!("azra {}", env!("CARGO_PKG_VERSION"));
    if new_version == current_version {
        let _ = std::fs::remove_dir_all(&temp);
        return Ok(format!("Already up to date ({current_version})"));
    }

    #[cfg(windows)]
    {
        // A running exe cannot overwrite itself on Windows: launch a detached
        // PowerShell helper that waits for this process to exit, then replaces it.
        let script = temp.join("replace.ps1");
        let source = downloaded.to_string_lossy().replace('\'', "''");
        let target = current.to_string_lossy().replace('\'', "''");
        let cleanup = temp.to_string_lossy().replace('\'', "''");
        let body = format!(
            "Start-Sleep -Seconds 2\nCopy-Item -LiteralPath '{source}' -DestinationPath '{target}' -Force\n& '{target}' setup-path\n& '{target}' setup-vscode\nRemove-Item -LiteralPath '{cleanup}' -Recurse -Force -ErrorAction SilentlyContinue\n"
        );
        std::fs::write(&script, body).map_err(|e| e.to_string())?;
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script.to_string_lossy(),
            ])
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(unix)]
    {
        std::fs::copy(&downloaded, &current).map_err(|e| e.to_string())?;
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&current)
            .map_err(|e| e.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&current, permissions).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&temp);
    }

    Ok(format!(
        "Updating {current_version} → {new_version}; restarting setup in a moment"
    ))
}

/// Recommended manual update path (works on every installed version).
pub fn reinstall_hint() -> &'static str {
    "Re-run the one-line installer to download and replace the current executable."
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_names_are_platform_specific() {
        let name = asset_name();
        assert!(name.starts_with("azra-") && name.contains("x86_64"));
    }
}
