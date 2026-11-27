//! `azra setup-path`: add the directory containing the running executable to
//! the user's PATH. Both installers call this command, so the exe and script
//! installers behave identically.

use std::path::PathBuf;

fn executable_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "cannot find the directory of azra.exe".to_string())
}

#[cfg(windows)]
fn persistent_path() -> String {
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetEnvironmentVariable('Path','User')",
        ])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

#[cfg(windows)]
fn set_persistent_path(path: &str) -> Result<(), String> {
    let status = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::SetEnvironmentVariable('Path', $env:AZRA_NEW_PATH, 'User')",
        ])
        .env("AZRA_NEW_PATH", path)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("failed to update the user PATH".to_string())
    }
}

/// Add the executable's directory to PATH and refresh the current process.
pub fn setup() -> Result<String, String> {
    let dir = executable_dir()?;
    let text = dir.to_string_lossy().to_string();
    let mut changed = false;

    #[cfg(windows)]
    {
        let current = persistent_path();
        let separator = ';';
        if !current.split(separator).any(|p| p.trim() == text) {
            let updated = if current.is_empty() {
                text.clone()
            } else {
                format!("{current}{separator}{text}")
            };
            set_persistent_path(&updated)?;
            changed = true;
        }
        let session = std::env::var("PATH").unwrap_or_default();
        if !session.split(separator).any(|p| p.trim() == text) {
            unsafe { std::env::set_var("PATH", format!("{session}{separator}{text}")) };
        }
    }

    #[cfg(unix)]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        let home = std::path::PathBuf::from(home);
        for profile in [".profile", ".bashrc", ".zshrc"] {
            let path = home.join(profile);
            if !path.exists() {
                continue;
            }
            let contents = std::fs::read_to_string(&path).unwrap_or_default();
            let line = format!("export PATH=\"{text}:$PATH\" # Azra");
            if !contents.contains(&line) {
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .map_err(|e| e.to_string())?;
                let _ = writeln!(file, "\n{line}");
                changed = true;
            }
        }
        let session = std::env::var("PATH").unwrap_or_default();
        if !session.split(':').any(|p| p.trim() == text) {
            unsafe { std::env::set_var("PATH", format!("{text}:{session}")) };
        }
    }

    Ok(if changed {
        format!("Added {text} to your PATH (restart your terminal for other apps)")
    } else {
        format!("Azra is already on PATH: {text}")
    })
}
