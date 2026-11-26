//! Math, trigonometry, vector, and random built-ins.

use super::{as_int, as_vec3, expect_args, num, BuiltinCtx};
use crate::ast::Builtin;
use crate::error::AzraError;
use crate::token::Num;
use crate::value::{Value, Vec3Data};

pub(super) fn eval_math(
    name: Builtin,
    args: &[Value],
    ctx: &mut BuiltinCtx,
    line: usize,
) -> Result<Value, AzraError> {
    match name {
        Builtin::Distance => {
            expect_args(name, args, 2, line)?;
            let a = as_vec3(&args[0], "distance", line)?;
            let b = as_vec3(&args[1], "distance", line)?;
            Ok(num(Num::Float(a.distance_to(&b))))
        }
        Builtin::Normalize => {
            expect_args(name, args, 1, line)?;
            let v = as_vec3(&args[0], "normalize", line)?;
            let len = v.length();
            if len == 0.0 {
                return Err(AzraError::runtime_at(
                    "cannot normalize a zero-length vec3",
                    line,
                ));
            }
            Ok(Value::Vec3(Vec3Data::new(v.x / len, v.y / len, v.z / len)))
        }
        Builtin::Round => {
            expect_args(name, args, 1, line)?;
            let f = args[0].as_number("round", line)?.as_f64().round();
            float_to_int_value(f, "round", line)
        }
        Builtin::Floor => {
            expect_args(name, args, 1, line)?;
            let f = args[0].as_number("floor", line)?.as_f64().floor();
            float_to_int_value(f, "floor", line)
        }
        Builtin::Ceil => {
            expect_args(name, args, 1, line)?;
            let f = args[0].as_number("ceil", line)?.as_f64().ceil();
            float_to_int_value(f, "ceil", line)
        }
        Builtin::Abs => {
            expect_args(name, args, 1, line)?;
            match args[0].clone() {
                Value::Number(Num::Int(i)) => i
                    .checked_abs()
                    .map(|v| num(Num::Int(v)))
                    .ok_or_else(|| AzraError::runtime_at("integer overflow in `abs`", line)),
                Value::Number(Num::Float(f)) => Ok(num(Num::Float(f.abs()))),
                other => Err(AzraError::runtime_at(
                    format!("`abs` needs a number, got {}", other.type_name()),
                    line,
                )),
            }
        }
        Builtin::Sqrt => {
            expect_args(name, args, 1, line)?;
            let n = args[0].as_number("sqrt", line)?.as_f64();
            if n < 0.0 {
                return Err(AzraError::runtime_at(
                    format!("`sqrt` of a negative number ({})", n),
                    line,
                ));
            }
            Ok(num(Num::Float(n.sqrt())))
        }
        Builtin::Sin => {
            expect_args(name, args, 1, line)?;
            Ok(num(Num::Float(
                args[0].as_number("sin", line)?.as_f64().sin(),
            )))
        }
        Builtin::Cos => {
            expect_args(name, args, 1, line)?;
            Ok(num(Num::Float(
                args[0].as_number("cos", line)?.as_f64().cos(),
            )))
        }
        Builtin::Min | Builtin::Max => {
            expect_args(name, args, 2, line)?;
            let want_min = name == Builtin::Min;
            let pick = match (&args[0], &args[1]) {
                (Value::Number(a), Value::Number(b)) => {
                    let a = a.as_f64();
                    let b = b.as_f64();
                    let keep_a = if want_min { a <= b } else { a >= b };
                    if keep_a { a } else { b }
                }
                _ => {
                    return Err(AzraError::runtime_at(
                        format!("`{}` needs two numbers", name.as_str()),
                        line,
                    ))
                }
            };
            // Preserve int-ness when both inputs were integers.
            if matches!(
                (&args[0], &args[1]),
                (Value::Number(Num::Int(_)), Value::Number(Num::Int(_)))
            ) {
                Ok(num(Num::Int(pick as i64)))
            } else {
                Ok(num(Num::Float(pick)))
            }
        }
        Builtin::Clamp => {
            expect_args(name, args, 3, line)?;
            let v = args[0].as_number("clamp value", line)?.as_f64();
            let lo = args[1].as_number("clamp lower bound", line)?.as_f64();
            let hi = args[2].as_number("clamp upper bound", line)?.as_f64();
            if lo > hi {
                return Err(AzraError::runtime_at(
                    format!(
                        "clamp lower bound ({}) is greater than upper bound ({})",
                        lo, hi
                    ),
                    line,
                ));
            }
            let clamped = v.clamp(lo, hi);
            if matches!(
                (&args[0], &args[1], &args[2]),
                (
                    Value::Number(Num::Int(_)),
                    Value::Number(Num::Int(_)),
                    Value::Number(Num::Int(_))
                )
            ) {
                Ok(num(Num::Int(clamped as i64)))
            } else {
                Ok(num(Num::Float(clamped)))
            }
        }
        Builtin::RandomNumber => {
            expect_args(name, args, 2, line)?;
            let lo = as_int(&args[0], "random number from", line)?;
            let hi = as_int(&args[1], "random number to", line)?;
            Ok(num(Num::Int(ctx.rng.next_range(lo, hi))))
        }
        Builtin::RandomDecimal => {
            if !args.is_empty() {
                return Err(AzraError::runtime_at(
                    "`random decimal` takes no arguments",
                    line,
                ));
            }
            Ok(num(Num::Float(ctx.rng.next_f64())))
        }
        _ => unreachable!("non-math builtin dispatched to eval_math"),
    }
}

fn float_to_int_value(f: f64, op: &str, line: usize) -> Result<Value, AzraError> {
    if !f.is_finite() {
        return Err(AzraError::runtime_at(
            format!("`{}` cannot convert non-finite number {}", op, f),
            line,
        ));
    }
    if f < i64::MIN as f64 || f >= -(i64::MIN as f64) {
        return Err(AzraError::runtime_at(
            format!("integer overflow in `{}`", op),
            line,
        ));
    }
    Ok(num(Num::Int(f as i64)))
}
