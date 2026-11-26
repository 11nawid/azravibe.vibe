#![allow(dead_code)]
//! Shared test helpers for Azra integration tests.

use azra::{AzraEngine, AzraError};

/// Run a snippet and return `(result, captured output lines)`.
pub fn run(src: &str) -> (Result<(), AzraError>, Vec<String>) {
    let mut engine = AzraEngine::new();
    let result = engine.eval_str(src);
    let out = engine
        .take_output()
        .into_iter()
        .map(|l| l.text)
        .collect();
    (result, out)
}

/// Run a snippet that must succeed and return its output.
pub fn ok(src: &str) -> Vec<String> {
    let (result, out) = run(src);
    match result {
        Ok(()) => out,
        Err(e) => panic!("expected success, got error: {}", e),
    }
}

/// Run a snippet that must fail and return the error.
pub fn err(src: &str) -> AzraError {
    let (result, _) = run(src);
    match result {
        Ok(()) => panic!("expected an error, but the script succeeded"),
        Err(e) => e,
    }
}
