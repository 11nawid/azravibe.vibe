//! String manipulation built-ins.

use super::expect_args;
use crate::ast::Builtin;
use crate::error::AzraError;
use crate::value::Value;

pub(super) fn eval_text(name: Builtin, args: &[Value], line: usize) -> Result<Value, AzraError> {
    match name {
        Builtin::Uppercase => {
            expect_args(name, args, 1, line)?;
            Ok(Value::Str(
                args[0].as_str("uppercase of", line)?.to_uppercase(),
            ))
        }
        Builtin::Lowercase => {
            expect_args(name, args, 1, line)?;
            Ok(Value::Str(
                args[0].as_str("lowercase of", line)?.to_lowercase(),
            ))
        }
        Builtin::Trim => {
            expect_args(name, args, 1, line)?;
            Ok(Value::Str(
                args[0].as_str("trim of", line)?.trim().to_string(),
            ))
        }
        Builtin::Split => {
            expect_args(name, args, 2, line)?;
            let text = args[0].as_str("split", line)?;
            let sep = args[1].as_str("separator", line)?;
            let parts = if sep.is_empty() {
                text.chars().map(|c| Value::Str(c.to_string())).collect()
            } else {
                text.split(&sep)
                    .map(|p| Value::Str(p.to_string()))
                    .collect()
            };
            Ok(Value::list(parts))
        }
        Builtin::Join => {
            expect_args(name, args, 2, line)?;
            let sep = args[1].as_str("join separator", line)?;
            match &args[0] {
                Value::List(items) => {
                    let items = items.borrow();
                    let joined: Vec<String> = items.iter().map(|v| v.render()).collect();
                    Ok(Value::Str(joined.join(&sep)))
                }
                other => Err(AzraError::runtime_at(
                    format!("`join` needs a list, got {}", other.type_name()),
                    line,
                )),
            }
        }
        Builtin::Replace => {
            expect_args(name, args, 3, line)?;
            let needle = args[0].as_str("replace", line)?;
            let replacement = args[1].as_str("replacement", line)?;
            let text = args[2].as_str("text", line)?;
            if needle.is_empty() {
                return Err(AzraError::runtime_at(
                    "cannot replace an empty string",
                    line,
                ));
            }
            Ok(Value::Str(text.replace(&needle, &replacement)))
        }
        _ => unreachable!("non-text builtin dispatched to eval_text"),
    }
}
