//! Error types for the Azra interpreter.

use std::fmt;

/// Category of an error produced while lexing, parsing or running a script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    Runtime,
    Io,
}

impl ErrorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorKind::Syntax => "syntax error",
            ErrorKind::Runtime => "runtime error",
            ErrorKind::Io => "io error",
        }
    }
}

/// A located Azra error.
///
/// Syntax errors carry `line`/`column` of the offending token, runtime errors
/// carry the `line` of the offending statement/expression.
#[derive(Debug, Clone, PartialEq)]
pub struct AzraError {
    pub kind: ErrorKind,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// `true` when parsing ran out of input (used by the REPL to keep reading).
    pub incomplete: bool,
    /// `true` for sandbox resource aborts (budget / call stack) that `try:` must not swallow.
    pub(crate) fatal: bool,
}

impl AzraError {
    pub fn syntax(message: impl Into<String>) -> Self {
        AzraError {
            kind: ErrorKind::Syntax,
            message: message.into(),
            line: None,
            column: None,
            incomplete: false,
            fatal: false,
        }
    }

    pub fn syntax_at(message: impl Into<String>, line: usize, column: usize) -> Self {
        AzraError {
            kind: ErrorKind::Syntax,
            message: message.into(),
            line: Some(line),
            column: Some(column),
            incomplete: false,
            fatal: false,
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        AzraError {
            kind: ErrorKind::Runtime,
            message: message.into(),
            line: None,
            column: None,
            incomplete: false,
            fatal: false,
        }
    }

    pub fn runtime_at(message: impl Into<String>, line: usize) -> Self {
        AzraError {
            kind: ErrorKind::Runtime,
            message: message.into(),
            line: Some(line),
            column: None,
            incomplete: false,
            fatal: false,
        }
    }

    pub(crate) fn fatal_runtime(message: impl Into<String>) -> Self {
        AzraError {
            kind: ErrorKind::Runtime,
            message: message.into(),
            line: None,
            column: None,
            incomplete: false,
            fatal: true,
        }
    }

    pub(crate) fn fatal_runtime_at(message: impl Into<String>, line: usize) -> Self {
        AzraError {
            kind: ErrorKind::Runtime,
            message: message.into(),
            line: Some(line),
            column: None,
            incomplete: false,
            fatal: true,
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        AzraError {
            kind: ErrorKind::Io,
            message: message.into(),
            line: None,
            column: None,
            incomplete: false,
            fatal: false,
        }
    }

    pub fn with_pos(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// `true` when the parser simply ran out of input (more lines may follow).
    pub fn is_incomplete(&self) -> bool {
        self.incomplete
    }

    /// `false` for sandbox resource limits that `try:` blocks cannot catch.
    pub fn is_catchable(&self) -> bool {
        !self.fatal
    }
}

impl fmt::Display for AzraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind.as_str())?;
        match (self.line, self.column) {
            (Some(l), Some(c)) => write!(f, " at line {}, column {}", l, c)?,
            (Some(l), None) => write!(f, " at line {}", l)?,
            (None, Some(c)) => write!(f, " at column {}", c)?,
            (None, None) => {}
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for AzraError {}
