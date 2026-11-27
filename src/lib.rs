//! Azra (`.vibe`) language — public embedding API.
//!
//! ```no_run
//! use azra::AzraEngine;
//!
//! let mut engine = AzraEngine::new();
//! engine.eval_str(r#"say "hello from Rust!""#).unwrap();
//! engine.start().unwrap();
//! engine.tick(1.0 / 60.0).unwrap();
//! ```

pub mod ast;
#[cfg(not(target_arch = "wasm32"))]
pub mod builder;
#[cfg(not(target_arch = "wasm32"))]
pub mod embedded;
pub mod embedded_vsix;
pub mod env;
pub mod error;
pub mod format;
#[cfg(not(target_arch = "wasm32"))]
pub mod gui_app;
pub mod host;
pub mod interpreter;
pub mod lexer;
pub mod lsp;
pub mod manifest;
#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_gui;
pub mod parser;
pub mod path_setup;
pub mod pkg;
pub mod sandbox;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
pub mod stdlib;
pub mod stdlib_embed;
pub mod token;
pub mod updater;
pub mod value;
pub mod vm;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use ast::OutputLevel;
pub use error::{AzraError, ErrorKind};
pub use interpreter::{OutputLine, OutputStream, TestResult};
pub use sandbox::SandboxConfig;
pub use token::{Num, Token, TokenKind};
pub use value::{ObjectData, Value};

use interpreter::Interpreter;
use std::path::{Path, PathBuf};

/// Like [`run_source`], but also registers the common host function library
/// (crypto, encoding, CLI args, environment, browser opener, ...). This is
/// what generated standalone executables call.
pub fn run_source_with_hosts(source: &str) -> Result<String, String> {
    let mut engine = AzraEngine::new();
    host::register_common(&mut engine);
    let result = engine.eval_str(source);
    let output = engine
        .take_output()
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n");
    match result {
        Ok(()) => Ok(output),
        Err(err) => Err(if output.is_empty() {
            err.to_string()
        } else {
            format!("{output}\n{err}")
        }),
    }
}

/// Compile and execute a script on the bytecode VM instead of the tree walker.
/// Returns the captured output lines.
pub fn eval_str_vm(source: &str) -> Result<Vec<String>, AzraError> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let tokens = lexer::tokenize(source)?;
    let program = parser::parse(&tokens)?;
    let bytecode = vm::compile(&program)?;
    vm::run(&bytecode)
}

/// Run a source string and return its buffered output as a single string.
/// Used by the WebAssembly playground and by embedding hosts that only need
/// text. Errors include any output produced before the failure.
pub fn run_source(source: &str) -> Result<String, String> {
    let mut engine = AzraEngine::new();
    let result = engine.eval_str(source);
    let output = engine
        .take_output()
        .into_iter()
        .map(|line| line.text)
        .collect::<Vec<_>>()
        .join("\n");
    match result {
        Ok(()) => Ok(output),
        Err(err) => Err(if output.is_empty() {
            err.to_string()
        } else {
            format!("{output}\n{err}")
        }),
    }
}

/// The embedding handle for the Azra virtual machine.
///
/// Keeps the global scope, registered event handlers, timers, loaded
/// modules, host functions and the buffered output stream between
/// [`eval_str`](AzraEngine::eval_str) calls.
pub struct AzraEngine {
    interp: Interpreter,
}

impl Default for AzraEngine {
    fn default() -> Self {
        AzraEngine::new()
    }
}

impl AzraEngine {
    /// Create a fresh engine with mock engine types and a default `player`
    /// entity registered, so game scripts run standalone (permissive sandbox).
    pub fn new() -> Self {
        AzraEngine {
            interp: Interpreter::new(),
        }
    }

    /// Create an engine with an explicit [`SandboxConfig`] (e.g.
    /// [`SandboxConfig::strict`] for Azravibe Player).
    pub fn with_sandbox(config: SandboxConfig) -> Self {
        AzraEngine {
            interp: Interpreter::with_sandbox(config),
        }
    }

    /// Tokenize, parse and execute a source string in the global scope.
    pub fn eval_str(&mut self, source: &str) -> Result<(), AzraError> {
        // Tolerate a UTF-8 BOM produced by some Windows editors.
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let tokens = lexer::tokenize(source)?;
        let program = parser::parse(&tokens)?;
        self.interp.run(&program)?;
        Ok(())
    }

    /// Run every `test "name":` block collected by [`AzraEngine::eval_str`].
    /// Each test runs in a fresh child scope; results are also cached on the
    /// engine and available through [`AzraEngine::test_results`].
    pub fn run_tests(&mut self) -> Vec<TestResult> {
        self.interp.run_tests()
    }

    /// Results of the most recent [`AzraEngine::run_tests`] call.
    pub fn test_results(&self) -> Vec<TestResult> {
        self.interp.test_results()
    }

    /// Read a `.vibe` file and execute it (also sets the directory used to
    /// resolve `use "..."` imports inside the file).
    pub fn eval_file(&mut self, path: impl AsRef<Path>) -> Result<(), AzraError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|e| AzraError::io(format!("cannot read `{}`: {}", path.display(), e)))?;
        self.interp
            .set_current_dir(path.parent().map(|p| p.to_path_buf()));
        self.eval_str(&source)
    }

    /// Parse-only check of a source string (no execution). Also validates
    /// the syntax of `use` statements present in the source.
    pub fn check_str(source: &str) -> Result<(), AzraError> {
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
        let tokens = lexer::tokenize(source)?;
        parser::parse(&tokens)?;
        Ok(())
    }

    /// Parse-only check of a `.vibe` file **including its imports**
    /// (recursively; circular imports are reported, not followed forever).
    pub fn check_file(path: impl AsRef<Path>) -> Result<(), AzraError> {
        let path = path.as_ref();
        let mut seen = Vec::new();
        check_file_recursive(path, &mut seen)
    }

    /// Engine start: run every `when game starts:` handler.
    pub fn start(&mut self) -> Result<(), AzraError> {
        self.interp.start()
    }

    /// Advance the engine by `delta_time` seconds (call at your frame rate,
    /// e.g. 60 times per second with `1.0 / 60.0`).
    pub fn tick(&mut self, delta_time: f64) -> Result<(), AzraError> {
        self.interp.tick(delta_time)
    }

    /// Register a native host function callable from Azra scripts as
    /// `my_func with arg1, arg2`.
    pub fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[Value]) -> Result<Value, AzraError> + 'static,
    {
        self.interp.register_function(name, func);
    }

    /// Fire a registered `when <event>:` handler by phrase
    /// (e.g. `"player touches lava_block"`).
    pub fn trigger_event(&mut self, phrase: &str) -> Result<(), AzraError> {
        self.interp.trigger_event(phrase)
    }

    /// Fire a registered `when <event>:` handler with arguments bound to
    /// the handler's `with` parameters.
    pub fn trigger_event_with_args(
        &mut self,
        phrase: &str,
        args: &[Value],
    ) -> Result<(), AzraError> {
        self.interp.trigger_event_with_args(phrase, args)
    }

    /// Phrases of all registered `when` handlers.
    pub fn registered_events(&self) -> Vec<String> {
        self.interp.registered_events()
    }

    /// Objects created with `make`, in creation order (host world queries).
    pub fn get_world_objects(&self) -> Vec<Value> {
        self.interp.get_world_objects()
    }

    /// Current value of the script clock in seconds.
    pub fn now(&self) -> f64 {
        self.interp.now()
    }

    /// Drain buffered output (`say` / `warn` / `error` / engine logs).
    pub fn take_output(&mut self) -> Vec<OutputLine> {
        self.interp.take_output()
    }

    /// Read a variable from the global scope.
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.interp.get_global(name)
    }

    /// Create or overwrite a global variable.
    pub fn set_variable(&mut self, name: &str, value: Value) {
        self.interp.set_global(name, value);
    }

    /// Names of all global variables (used by the REPL `:vars` command).
    pub fn variable_names(&self) -> Vec<String> {
        self.interp.global_names()
    }
}

/// Recursively parse a file and every module it `use`s.
fn check_file_recursive(path: &Path, seen: &mut Vec<PathBuf>) -> Result<(), AzraError> {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if seen.contains(&canonical) {
        return Ok(());
    }
    seen.push(canonical.clone());

    let source = std::fs::read_to_string(path)
        .map_err(|e| AzraError::io(format!("cannot read `{}`: {}", path.display(), e)))?;
    let source = source.strip_prefix('\u{feff}').unwrap_or(&source);
    let tokens = lexer::tokenize(source)?;
    let program = parser::parse(&tokens)?;

    let base = path.parent();
    for spec in collect_imports(&program) {
        let resolved = interpreter::resolve_module_path(base, &spec);
        check_file_recursive(&resolved, seen)?;
    }
    Ok(())
}

/// Collect every `use "<path>"` spec in a parsed program (recursively).
fn collect_imports(program: &[ast::Stmt]) -> Vec<String> {
    let mut specs = Vec::new();
    collect_imports_into(program, &mut specs);
    specs
}

fn collect_imports_into(program: &[ast::Stmt], specs: &mut Vec<String>) {
    use ast::Stmt;
    for stmt in program {
        match stmt {
            Stmt::Use { path, .. } => specs.push(path.clone()),
            Stmt::If {
                branches,
                otherwise,
                ..
            } => {
                for branch in branches {
                    collect_imports_into(&branch.body, specs);
                }
                if let Some(body) = otherwise {
                    collect_imports_into(body, specs);
                }
            }
            Stmt::Repeat { body, .. }
            | Stmt::RepeatRange { body, .. }
            | Stmt::ForEach { body, .. }
            | Stmt::While { body, .. }
            | Stmt::Define { body, .. }
            | Stmt::When { body, .. }
            | Stmt::Every { body, .. }
            | Stmt::After { body, .. } => collect_imports_into(body, specs),
            Stmt::Try { body, handler, .. } => {
                collect_imports_into(body, specs);
                collect_imports_into(handler, specs);
            }
            _ => {}
        }
    }
}
