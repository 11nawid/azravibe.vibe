//! Azra (`.vibe`) language — public embedding API.
//!
//! ```no_run
//! use azra::AzraEngine;
//!
//! let mut engine = AzraEngine::new();
//! engine.eval_str(r#"say "hello from Rust!""#).unwrap();
//! ```

pub mod ast;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod value;

pub use ast::OutputLevel;
pub use error::{AzraError, ErrorKind};
pub use interpreter::{OutputLine, OutputStream};
pub use token::{Num, Token, TokenKind};
pub use value::{ObjectData, Value};

use interpreter::Interpreter;
use std::path::Path;

/// The embedding handle for the Azra virtual machine.
///
/// Keeps the global scope, registered event handlers, host functions and the
/// buffered output stream between [`eval_str`](AzraEngine::eval_str) calls.
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
    /// entity registered, so game scripts run standalone.
    pub fn new() -> Self {
        AzraEngine {
            interp: Interpreter::new(),
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

    /// Read a `.vibe` file and execute it.
    pub fn eval_file(&mut self, path: impl AsRef<Path>) -> Result<(), AzraError> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|e| {
            AzraError::io(format!("cannot read `{}`: {}", path.display(), e))
        })?;
        self.eval_str(&source)
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

    /// Phrases of all registered `when` handlers.
    pub fn registered_events(&self) -> Vec<String> {
        self.interp.registered_events()
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
}
