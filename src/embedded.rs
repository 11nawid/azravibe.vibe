//! Attach a `.vibe` script to a copy of the interpreter executable.
//!
//! The script is appended as a PE/ELF overlay with a marker and length. On
//! startup the executable detects the marker and runs the embedded script, so
//! `azra build` needs no Rust toolchain, C++ compiler or extra files on the
//! user's machine — just the `azra` executable itself.

use std::path::Path;

const MARKER: &[u8] = b"\n__AZRA_EMBEDDED_SCRIPT_V1__\n";

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

/// Copy the running interpreter to `out_path` and append `script` as an overlay.
pub fn attach(out_path: &Path, script: &str) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut bytes = read_file(&executable)?;
    if let Some(pos) = find_overlay(&bytes) {
        bytes.truncate(pos);
    }
    bytes.extend_from_slice(MARKER);
    bytes.extend_from_slice(&(script.len() as u64).to_le_bytes());
    bytes.extend_from_slice(script.as_bytes());
    write_file(out_path, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(out_path)
            .map_err(|e| e.to_string())?
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(out_path, permissions).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Read the embedded script from the running executable, if any.
pub fn read_self() -> Option<String> {
    let executable = std::env::current_exe().ok()?;
    let bytes = read_file(&executable).ok()?;
    let position = find_overlay(&bytes)?;
    let start = position + MARKER.len();
    let length = overlay_length(&bytes, start)?;
    let script_start = start + 8;
    let script = bytes.get(script_start..script_start + length)?;
    String::from_utf8(script.to_vec()).ok()
}

fn overlay_length(bytes: &[u8], start: usize) -> Option<usize> {
    let length_bytes: [u8; 8] = bytes.get(start..start + 8)?.try_into().ok()?;
    let length = u64::from_le_bytes(length_bytes) as usize;
    // Only accept a marker whose payload reaches exactly the end of the file.
    // The marker literal also exists in the compiled code section; that
    // occurrence must never be mistaken for an overlay.
    if length > 64 * 1024 * 1024 || start + 8 + length != bytes.len() {
        return None;
    }
    Some(length)
}

fn find_overlay(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < MARKER.len() {
        return None;
    }
    let mut from = bytes.len();
    while let Some(position) = bytes[..from]
        .windows(MARKER.len())
        .rposition(|w| w == MARKER)
    {
        let start = position + MARKER.len();
        if overlay_length(bytes, start).is_some() {
            return Some(position);
        }
        from = position;
    }
    None
}
