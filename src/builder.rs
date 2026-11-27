//! `azra build`: produce a standalone executable by copying the interpreter
//! and appending the `.vibe` script as an overlay. No compiler or dependencies
//! are required on the user's machine — the single exe is the whole app.

use std::path::{Path, PathBuf};

/// Build `script` into a standalone executable at `out_path`.
pub fn build_exe(script: &Path, _name: &str, out_path: &Path) -> Result<PathBuf, String> {
    let source = std::fs::read_to_string(script)
        .map_err(|e| format!("cannot read {}: {e}", script.display()))?;
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    crate::embedded::attach(out_path, &source)?;
    Ok(out_path.to_path_buf())
}
