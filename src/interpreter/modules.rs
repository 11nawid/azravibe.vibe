//! `use "<file>.vibe" [as alias]` module loading: relative resolution,
//! the `.vibe` extension, an import cache and circular-import detection.

use super::{Flow, Interpreter};
use crate::env::Scope;
use crate::error::AzraError;
use crate::value::{ObjectData, Value};
use std::path::{Path, PathBuf};

impl Interpreter {
    /// Load a module and bind it: plain `use` merges the module's exports
    /// into the current scope, `use as name` binds a module object.
    pub(super) fn exec_use(
        &mut self,
        spec: &str,
        alias: &Option<String>,
        line: usize,
    ) -> Result<(), AzraError> {
        if !self.sandbox.allow_io {
            return Err(AzraError::runtime_at(
                "`use` is disabled by the sandbox (allow_io is off)",
                line,
            ));
        }
        // Standard-library modules are embedded in the executable, so an
        // installed interpreter needs no adjacent `std/` directory.
        if let Some(name) = spec.strip_prefix("std/") {
            let name = name.trim_end_matches(".vibe");
            if let Some(source) = crate::stdlib_embed::get(name) {
                return self.exec_embedded_use(name, source, alias, line);
            }
        }
        let resolved = resolve_module_path(self.current_dir.as_deref(), spec);
        let key = std::fs::canonicalize(&resolved).unwrap_or_else(|_| resolved.clone());

        if self.module_stack.contains(&key) {
            let mut chain: Vec<String> = self.module_stack.iter().map(|p| file_name(p)).collect();
            chain.push(file_name(&key));
            return Err(AzraError::runtime_at(
                format!("circular import detected: {}", chain.join(" -> ")),
                line,
            ));
        }

        // Cached: bind the previous exports without re-running the module.
        if let Some(exports) = self.modules.get(&key).cloned() {
            self.bind_exports(&exports, alias)?;
            return Ok(());
        }

        let source = std::fs::read_to_string(&resolved)
            .map_err(|e| AzraError::io(format!("cannot read module `{}`: {}", spec, e)))?;
        let tokens = crate::lexer::tokenize(&source)?;
        let program = crate::parser::parse(&tokens)?;

        let module_scope = Scope::new(&self.globals);
        let importer_scope = self.scope.clone();
        let importer_dir = self.current_dir.clone();

        self.module_stack.push(key.clone());
        self.scope = module_scope.clone();
        self.current_dir = key.parent().map(|p| p.to_path_buf());

        let flow = match self.exec_block(&program) {
            Ok(f) => f,
            Err(e) => {
                self.module_stack.pop();
                self.scope = importer_scope;
                self.current_dir = importer_dir;
                return Err(e);
            }
        };
        let flow_err = match flow {
            Flow::Normal => None,
            Flow::Return(_) => Some(AzraError::runtime_at(
                "`give back` outside of a function (in a module)",
                line,
            )),
            Flow::Break | Flow::Continue => Some(AzraError::runtime_at(
                "`stop loop` / `skip next` outside of a loop (in a module)",
                line,
            )),
        };
        self.module_stack.pop();
        self.scope = importer_scope;
        self.current_dir = importer_dir;
        if let Some(e) = flow_err {
            return Err(e);
        }

        let exports = module_scope.entries();
        self.modules.insert(key, exports.clone());
        self.bind_exports(&exports, alias)?;
        Ok(())
    }

    /// Load a stdlib module compiled into the binary (no filesystem access).
    fn exec_embedded_use(
        &mut self,
        name: &str,
        source: &str,
        alias: &Option<String>,
        line: usize,
    ) -> Result<(), AzraError> {
        let key = PathBuf::from(format!("std://{name}.vibe"));
        if self.module_stack.contains(&key) {
            return Err(AzraError::runtime_at(
                format!("circular import detected: std/{name}"),
                line,
            ));
        }
        if let Some(exports) = self.modules.get(&key).cloned() {
            self.bind_exports(&exports, alias)?;
            return Ok(());
        }
        let tokens = crate::lexer::tokenize(source)?;
        let program = crate::parser::parse(&tokens)?;
        let module_scope = Scope::new(&self.globals);
        let importer_scope = self.scope.clone();
        self.module_stack.push(key.clone());
        self.scope = module_scope.clone();
        let flow = self.exec_block(&program);
        self.module_stack.pop();
        self.scope = importer_scope;
        match flow? {
            Flow::Normal => {}
            Flow::Return(_) => {
                return Err(AzraError::runtime_at(
                    "`give back` outside of a function (in a module)",
                    line,
                ));
            }
            Flow::Break | Flow::Continue => {
                return Err(AzraError::runtime_at(
                    "`stop loop` / `skip next` outside of a loop (in a module)",
                    line,
                ));
            }
        }
        let exports = module_scope.entries();
        self.modules.insert(key, exports.clone());
        self.bind_exports(&exports, alias)
    }

    /// Bind module exports into the importing scope.
    fn bind_exports(
        &mut self,
        exports: &[(String, Value)],
        alias: &Option<String>,
    ) -> Result<(), AzraError> {
        match alias {
            Some(name) => {
                let mut obj = ObjectData::new("module");
                self.next_id += 1;
                obj.id = self.next_id;
                for (k, v) in exports {
                    obj.set(k, v.clone());
                }
                self.scope.define(name, Value::object(obj));
            }
            None => {
                for (k, v) in exports {
                    self.scope.define(k, v.clone());
                }
            }
        }
        Ok(())
    }
}

/// Resolve a `use "..."` module path: adds `.vibe` when missing and makes
/// relative specs absolute against `base_dir` (the importing file's folder).
pub fn resolve_module_path(base_dir: Option<&Path>, spec: &str) -> PathBuf {
    let mut path = PathBuf::from(spec);
    if path.extension().is_none() {
        path.set_extension("vibe");
    }
    if path.is_absolute() {
        return path;
    }

    // 1. Modules shipped with the interpreter (`use "std/math"`).
    let std_relative = if spec.starts_with("std/") {
        let mut p = PathBuf::from(&spec["std/".len()..]);
        if p.extension().is_none() {
            p.set_extension("vibe");
        }
        p
    } else {
        path.clone()
    };
    let std_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("std")
        .join(std_relative);
    if std_path.exists() {
        return std_path;
    }

    let Some(base) = base_dir else {
        return path;
    };

    // 2. A file relative to the importing script.
    let direct = base.join(&path);
    if direct.exists() {
        return direct;
    }

    // 3. Installed packages: walk up to find `.azra/packages`, then look for
    //    `<package>@<version>/<path>` or `<path>` inside any installed package.
    let mut dir = Some(base);
    while let Some(current) = dir {
        let root = current.join(".azra").join("packages");
        if root.is_dir() {
            for pkg in std::fs::read_dir(&root).into_iter().flatten().flatten() {
                let candidate = pkg.path().join(&path);
                if candidate.exists() {
                    return candidate;
                }
            }
            // `use "math/core"` → first installed `math@<version>` package.
            let mut parts = path.components();
            if let Some(first) = parts.next() {
                let prefix = format!("{}@", first.as_os_str().to_string_lossy());
                let rest: PathBuf = parts.collect();
                for pkg in std::fs::read_dir(&root).into_iter().flatten().flatten() {
                    if pkg.file_name().to_string_lossy().starts_with(&prefix) {
                        let candidate = pkg.path().join(rest.as_os_str());
                        if candidate.exists() {
                            return candidate;
                        }
                    }
                }
            }
        }
        dir = current.parent();
    }
    direct
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
