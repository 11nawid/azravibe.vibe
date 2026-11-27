//! Built-in expression forms (`round 4.6`, `length of items`, ...) plus the
//! optional file/JSON helpers.
//!
//! Everything here is dependency-free: JSON is a hand-written codec and the
//! random generator is a small xorshift seeded from the system clock.

mod json;
mod math;
mod rng;
mod text;
mod time;

pub use json::{parse_json, to_json};
pub use rng::Rng;
pub use time::format_utc;

use crate::ast::Builtin;
use crate::error::AzraError;
use crate::sandbox::SandboxConfig;
use crate::token::Num;
use crate::value::{ColorData, Value, Vec3Data};
use std::cell::RefCell;
use std::rc::Rc;

/// Context a built-in form needs at runtime: sandbox rules + randomness.
pub struct BuiltinCtx<'a> {
    pub sandbox: &'a SandboxConfig,
    pub rng: &'a mut Rng,
}

/// Evaluate a built-in expression form.
pub fn eval_builtin(
    name: Builtin,
    args: Vec<Value>,
    ctx: &mut BuiltinCtx,
    line: usize,
) -> Result<Value, AzraError> {
    match name {
        Builtin::Vec3 => {
            expect_args(name, &args, 3, line)?;
            let x = args[0].as_number("vec3 x", line)?.as_f64();
            let y = args[1].as_number("vec3 y", line)?.as_f64();
            let z = args[2].as_number("vec3 z", line)?.as_f64();
            Ok(Value::Vec3(Vec3Data::new(x, y, z)))
        }
        Builtin::Color => eval_color(name, &args, line),
        Builtin::ItemOf => {
            expect_args(name, &args, 2, line)?;
            let index = as_index(&args[0], "item", line)?;
            let list = as_list(&args[1], line)?;
            let items = list.borrow();
            let item = items.get(index as usize).ok_or_else(|| {
                AzraError::runtime_at(
                    format!(
                        "item {} of list is out of range (list has {} item(s))",
                        index + 1,
                        items.len()
                    ),
                    line,
                )
            })?;
            Ok(item.clone())
        }
        Builtin::FirstItemOf => {
            expect_args(name, &args, 1, line)?;
            let list = as_list(&args[0], line)?;
            let items = list.borrow();
            items
                .first()
                .cloned()
                .ok_or_else(|| AzraError::runtime_at("first item of an empty list", line))
        }
        Builtin::LastItemOf => {
            expect_args(name, &args, 1, line)?;
            let list = as_list(&args[0], line)?;
            let items = list.borrow();
            items
                .last()
                .cloned()
                .ok_or_else(|| AzraError::runtime_at("last item of an empty list", line))
        }
        Builtin::LengthOf => {
            expect_args(name, &args, 1, line)?;
            let len = match &args[0] {
                Value::List(items) => items.borrow().len() as i64,
                Value::Map(m) => m.borrow().entries.len() as i64,
                Value::Str(s) => s.chars().count() as i64,
                Value::Object(o) => o.borrow().fields.len() as i64,
                // A vec3 has no "number of items" — its length is its magnitude.
                Value::Vec3(v) => return Ok(num(Num::Float(v.length()))),
                other => {
                    return Err(AzraError::runtime_at(
                        format!(
                            "`length of` needs a list, map, string or vec3, got {}",
                            other.type_name()
                        ),
                        line,
                    ));
                }
            };
            Ok(num(Num::Int(len)))
        }
        Builtin::KeysOf => {
            expect_args(name, &args, 1, line)?;
            match &args[0] {
                Value::Map(m) => Ok(Value::list(m.borrow().keys())),
                other => Err(AzraError::runtime_at(
                    format!("`keys of` needs a map, got {}", other.type_name()),
                    line,
                )),
            }
        }
        Builtin::KeyOf => {
            expect_args(name, &args, 2, line)?;
            let key = args[0].as_str("key", line)?;
            match &args[1] {
                Value::Map(m) => Ok(m.borrow().get(&key).cloned().unwrap_or(Value::Null)),
                Value::Object(o) => Ok(o.borrow().get(&key).cloned().unwrap_or(Value::Null)),
                other => Err(AzraError::runtime_at(
                    format!(
                        "`key \"{}\" of` needs a map or object, got {}",
                        key,
                        other.type_name()
                    ),
                    line,
                )),
            }
        }
        Builtin::Distance
        | Builtin::Normalize
        | Builtin::Round
        | Builtin::Floor
        | Builtin::Ceil
        | Builtin::Abs
        | Builtin::Sqrt
        | Builtin::Sin
        | Builtin::Cos
        | Builtin::Min
        | Builtin::Max
        | Builtin::Clamp
        | Builtin::RandomNumber
        | Builtin::RandomDecimal
        | Builtin::Tan
        | Builtin::Asin
        | Builtin::Acos
        | Builtin::Atan
        | Builtin::Atan2
        | Builtin::Pi
        | Builtin::Euler
        | Builtin::Log
        | Builtin::Log10
        | Builtin::Exp
        | Builtin::Lerp
        | Builtin::DegreesToRadians
        | Builtin::RadiansToDegrees
        | Builtin::Dot
        | Builtin::Cross
        | Builtin::Magnitude
        | Builtin::AngleBetween => math::eval_math(name, &args, ctx, line),
        Builtin::Uppercase
        | Builtin::Lowercase
        | Builtin::Trim
        | Builtin::Split
        | Builtin::Join
        | Builtin::Replace => text::eval_text(name, &args, line),
        Builtin::CurrentTime | Builtin::FormatTime => time::eval_time(name, &args, line),
        Builtin::ReadFile => {
            require_io(ctx.sandbox, "read file", line)?;
            expect_args(name, &args, 1, line)?;
            let path = args[0].as_str("read file path", line)?;
            std::fs::read_to_string(&path)
                .map(Value::Str)
                .map_err(|e| AzraError::io(format!("cannot read file `{}`: {}", path, e)))
        }
        Builtin::FileExists => {
            require_io(ctx.sandbox, "file exists", line)?;
            expect_args(name, &args, 1, line)?;
            let path = args[0].as_str("file exists path", line)?;
            Ok(Value::Bool(std::path::Path::new(&path).exists()))
        }
        Builtin::ParseJson => {
            require_io(ctx.sandbox, "parse json", line)?;
            expect_args(name, &args, 1, line)?;
            let text = args[0].as_str("parse json", line)?;
            parse_json(&text, line)
        }
        Builtin::ToJson => {
            require_io(ctx.sandbox, "to json", line)?;
            expect_args(name, &args, 1, line)?;
            json::try_to_json(&args[0], line).map(Value::Str)
        }
        Builtin::EmptyList => {
            if !args.is_empty() {
                return Err(AzraError::runtime_at(
                    "`empty list` takes no arguments",
                    line,
                ));
            }
            Ok(Value::list(Vec::new()))
        }
        Builtin::EmptyMap => {
            if !args.is_empty() {
                return Err(AzraError::runtime_at(
                    "`empty map` takes no arguments",
                    line,
                ));
            }
            Ok(Value::map(Vec::new()))
        }
    }
}

fn eval_color(name: Builtin, args: &[Value], line: usize) -> Result<Value, AzraError> {
    if args.len() == 1 {
        let cname = args[0].as_str("color name", line)?;
        return ColorData::from_name(&cname)
            .map(Value::Color)
            .ok_or_else(|| {
                AzraError::runtime_at(
                    format!(
                        "unknown color `{}` — try a name like \"red\" or 3 numbers",
                        cname
                    ),
                    line,
                )
            });
    }
    expect_args(name, args, 3, line)?;
    let ch = |v: &Value, what: &str| -> Result<u8, AzraError> {
        let n = v.as_number(what, line)?.as_f64();
        if n < 0.0 || n > 255.0 || n.fract() != 0.0 {
            return Err(AzraError::runtime_at(
                format!("{} must be a whole number 0-255, got {}", what, n),
                line,
            ));
        }
        Ok(n as u8)
    };
    Ok(Value::Color(ColorData::new(
        ch(&args[0], "color red")?,
        ch(&args[1], "color green")?,
        ch(&args[2], "color blue")?,
    )))
}

// ----------------------------------------------------------------- helpers

pub(super) fn num(n: Num) -> Value {
    Value::Number(n)
}

pub(super) fn expect_args(
    name: Builtin,
    args: &[Value],
    want: usize,
    line: usize,
) -> Result<(), AzraError> {
    if args.len() == want {
        Ok(())
    } else {
        Err(AzraError::runtime_at(
            format!(
                "`{}` expects {} argument(s), got {}",
                name.as_str(),
                want,
                args.len()
            ),
            line,
        ))
    }
}

fn require_io(sandbox: &SandboxConfig, what: &str, line: usize) -> Result<(), AzraError> {
    if sandbox.allow_io {
        Ok(())
    } else {
        Err(AzraError::runtime_at(
            format!("`{}` is disabled by the sandbox (allow_io is off)", what),
            line,
        ))
    }
}

/// Require a whole number (allowing 0 and negative values).
pub(super) fn as_int(v: &Value, what: &str, line: usize) -> Result<i64, AzraError> {
    match v.as_number(what, line)? {
        Num::Int(i) => Ok(i),
        Num::Float(f) => {
            if f.fract() != 0.0 || !f.is_finite() {
                return Err(AzraError::runtime_at(
                    format!("{} must be a whole number, got {}", what, f),
                    line,
                ));
            }
            Ok(f as i64)
        }
    }
}

/// 1-based script index → 0-based Rust index, with friendly errors.
fn as_index(v: &Value, what: &str, line: usize) -> Result<i64, AzraError> {
    let i = as_int(v, &format!("{} index", what), line)?;
    if i < 1 {
        return Err(AzraError::runtime_at(
            format!("{} index must be 1 or greater (got {})", what, i),
            line,
        ));
    }
    Ok(i - 1)
}

fn as_list(v: &Value, line: usize) -> Result<Rc<RefCell<Vec<Value>>>, AzraError> {
    match v {
        Value::List(items) => Ok(items.clone()),
        other => Err(AzraError::runtime_at(
            format!("expected a list, got {}", other.type_name()),
            line,
        )),
    }
}

pub(super) fn as_vec3(v: &Value, what: &str, line: usize) -> Result<Vec3Data, AzraError> {
    match v {
        Value::Vec3(vec) => Ok(*vec),
        other => Err(AzraError::runtime_at(
            format!("`{}` needs a vec3, got {}", what, other.type_name()),
            line,
        )),
    }
}
