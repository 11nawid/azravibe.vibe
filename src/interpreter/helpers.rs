//! Shared interpreter utilities: line numbers, event phrase normalization,
//! repeat counts, value comparison and checked number arithmetic.

use crate::ast::Stmt;
use crate::error::AzraError;
use crate::token::Num;
use crate::value::{format_float, Value};

use super::MAX_LOOP_ITERATIONS;

/// Source line of a statement (used to attach lines to runtime errors).
pub(super) fn stmt_line(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Set { line, .. }
        | Stmt::Make { line, .. }
        | Stmt::Add { line, .. }
        | Stmt::Remove { line, .. }
        | Stmt::RemoveAt { line, .. }
        | Stmt::RemoveKey { line, .. }
        | Stmt::Subtract { line, .. }
        | Stmt::Multiply { line, .. }
        | Stmt::Divide { line, .. }
        | Stmt::Clear { line, .. }
        | Stmt::Sort { line, .. }
        | Stmt::Reverse { line, .. }
        | Stmt::Output { line, .. }
        | Stmt::If { line, .. }
        | Stmt::Repeat { line, .. }
        | Stmt::RepeatRange { line, .. }
        | Stmt::ForEach { line, .. }
        | Stmt::While { line, .. }
        | Stmt::Break { line }
        | Stmt::Continue { line }
        | Stmt::Define { line, .. }
        | Stmt::Return { line, .. }
        | Stmt::When { line, .. }
        | Stmt::Call { line, .. }
        | Stmt::ExprStmt { line, .. }
        | Stmt::Try { line, .. }
        | Stmt::Fail { line, .. }
        | Stmt::Use { line, .. }
        | Stmt::Every { line, .. }
        | Stmt::After { line, .. }
        | Stmt::Wait { line, .. }
        | Stmt::WriteFile { line, .. } => *line,
    }
}

/// Attach a line number to errors that lack one (literals have no position).
pub(super) fn attach_line(err: AzraError, line: usize) -> AzraError {
    if err.line.is_none() && line > 0 {
        AzraError {
            line: Some(line),
            ..err
        }
    } else {
        err
    }
}

/// Normalize an event phrase for matching: lowercase, collapse whitespace,
/// drop a trailing period and ignore quote style (`key "space"` == `key space`).
pub(super) fn normalize_phrase(phrase: &str) -> String {
    let lower = phrase.to_lowercase();
    let collapsed = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_end_matches('.');
    trimmed.replace(['"', '\''], "")
}

pub(super) fn repeat_count(value: &Value, line: usize) -> Result<u64, AzraError> {
    match value {
        Value::Number(Num::Int(i)) => {
            if *i < 0 {
                Err(AzraError::runtime_at(
                    "repeat count must not be negative",
                    line,
                ))
            } else if *i as u64 > MAX_LOOP_ITERATIONS {
                Err(AzraError::runtime_at(
                    format!(
                        "repeat count exceeds the safety cap of {}",
                        MAX_LOOP_ITERATIONS
                    ),
                    line,
                ))
            } else {
                Ok(*i as u64)
            }
        }
        Value::Number(Num::Float(f)) => {
            if *f < 0.0 {
                Err(AzraError::runtime_at(
                    "repeat count must not be negative",
                    line,
                ))
            } else if f.fract() != 0.0 {
                Err(AzraError::runtime_at(
                    format!("repeat count must be a whole number, got {}", format_float(*f)),
                    line,
                ))
            } else if *f > MAX_LOOP_ITERATIONS as f64 {
                Err(AzraError::runtime_at(
                    format!(
                        "repeat count exceeds the safety cap of {}",
                        MAX_LOOP_ITERATIONS
                    ),
                    line,
                ))
            } else {
                Ok(*f as u64)
            }
        }
        other => Err(AzraError::runtime_at(
            format!("repeat count must be a number, got {}", other.type_name()),
            line,
        )),
    }
}

pub(super) fn compare_values(
    l: &Value,
    r: &Value,
    line: usize,
) -> Result<std::cmp::Ordering, AzraError> {
    match (l, r) {
        (Value::Number(a), Value::Number(b)) => Ok(a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Str(a), Value::Str(b)) => Ok(a.cmp(b)),
        (Value::Bool(a), Value::Bool(b)) => Ok(a.cmp(b)),
        _ => Err(AzraError::runtime_at(
            format!(
                "cannot compare {} with {} (numbers, strings or bools only)",
                l.type_name(),
                r.type_name()
            ),
            line,
        )),
    }
}

pub(super) fn num_add(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x
            .checked_add(*y)
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in addition", line)),
        _ => Ok(Num::Float(a.as_f64() + b.as_f64())),
    }
}

pub(super) fn num_sub(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x
            .checked_sub(*y)
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in subtraction", line)),
        _ => Ok(Num::Float(a.as_f64() - b.as_f64())),
    }
}

pub(super) fn num_mul(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x
            .checked_mul(*y)
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in multiplication", line)),
        _ => Ok(Num::Float(a.as_f64() * b.as_f64())),
    }
}

pub(super) fn num_div(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => {
            if *y == 0 {
                return Err(AzraError::runtime_at("division by zero", line));
            }
            x.checked_div(*y)
                .map(Num::Int)
                .ok_or_else(|| AzraError::runtime_at("integer overflow in division", line))
        }
        _ => {
            if b.as_f64() == 0.0 {
                return Err(AzraError::runtime_at("division by zero", line));
            }
            Ok(Num::Float(a.as_f64() / b.as_f64()))
        }
    }
}

pub(super) fn num_mod(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => {
            if *y == 0 {
                return Err(AzraError::runtime_at("modulo by zero", line));
            }
            x.checked_rem(*y)
                .map(Num::Int)
                .ok_or_else(|| AzraError::runtime_at("integer overflow in modulo", line))
        }
        _ => {
            if b.as_f64() == 0.0 {
                return Err(AzraError::runtime_at("modulo by zero", line));
            }
            Ok(Num::Float(a.as_f64().rem_euclid(b.as_f64())))
        }
    }
}

pub(super) fn num_pow(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(1), Num::Int(_)) => Ok(Num::Int(1)),
        (Num::Int(-1), Num::Int(y)) => Ok(Num::Int(if *y % 2 == 0 { 1 } else { -1 })),
        (Num::Int(0), Num::Int(y)) if *y > 0 => Ok(Num::Int(0)),
        (Num::Int(x), Num::Int(y)) if *y >= 0 => match u32::try_from(*y) {
            Ok(exp) => x
                .checked_pow(exp)
                .map(Num::Int)
                .ok_or_else(|| AzraError::runtime_at("integer overflow in `power`", line)),
            Err(_) => Err(AzraError::runtime_at("integer overflow in `power`", line)),
        },
        _ => Ok(Num::Float(a.as_f64().powf(b.as_f64()))),
    }
}

pub(super) fn num_neg(a: &Num, line: usize) -> Result<Num, AzraError> {
    match a {
        Num::Int(i) => i
            .checked_neg()
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in negation", line)),
        Num::Float(f) => Ok(Num::Float(-f)),
    }
}
