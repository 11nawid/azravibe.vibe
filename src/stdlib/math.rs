//! Math, trigonometry, vector, and random built-ins.

use super::{BuiltinCtx, as_int, as_vec3, expect_args, num};
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
                    if a.is_nan() || b.is_nan() {
                        return Err(AzraError::runtime_at(
                            format!("`{}` cannot compare NaN", name.as_str()),
                            line,
                        ));
                    }
                    let keep_a = if want_min { a <= b } else { a >= b };
                    if keep_a { a } else { b }
                }
                _ => {
                    return Err(AzraError::runtime_at(
                        format!("`{}` needs two numbers", name.as_str()),
                        line,
                    ));
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
            if lo.is_nan() || hi.is_nan() {
                return Err(AzraError::runtime_at("clamp bounds must not be NaN", line));
            }
            if lo > hi {
                return Err(AzraError::runtime_at(
                    format!(
                        "clamp lower bound ({}) is greater than upper bound ({})",
                        lo, hi
                    ),
                    line,
                ));
            }
            // `f64::clamp` panics when either bound is NaN; the guard above
            // keeps that failure inside the language's error system so `try:`
            // can catch it and the CLI never aborts with a Rust panic.
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
        Builtin::Tan => {
            expect_args(name, args, 1, line)?;
            let x = args[0].as_number("tan", line)?.as_f64();
            let v = x.tan();
            if !v.is_finite() {
                return Err(AzraError::runtime_at(
                    "`tan` is undefined at this angle",
                    line,
                ));
            }
            Ok(num(Num::Float(v)))
        }
        Builtin::Asin | Builtin::Acos => {
            expect_args(name, args, 1, line)?;
            let op = name.as_str();
            let x = args[0].as_number(op, line)?.as_f64();
            if !(-1.0..=1.0).contains(&x) {
                return Err(AzraError::runtime_at(
                    format!("`{}` needs a value between -1 and 1, got {}", op, x),
                    line,
                ));
            }
            let v = if name == Builtin::Asin {
                x.asin()
            } else {
                x.acos()
            };
            Ok(num(Num::Float(v)))
        }
        Builtin::Atan => {
            expect_args(name, args, 1, line)?;
            Ok(num(Num::Float(
                args[0].as_number("atan", line)?.as_f64().atan(),
            )))
        }
        Builtin::Atan2 => {
            expect_args(name, args, 2, line)?;
            let y = args[0].as_number("atan2 y", line)?.as_f64();
            let x = args[1].as_number("atan2 x", line)?.as_f64();
            Ok(num(Num::Float(y.atan2(x))))
        }
        Builtin::Pi => {
            expect_args(name, args, 0, line)?;
            Ok(num(Num::Float(std::f64::consts::PI)))
        }
        Builtin::Euler => {
            expect_args(name, args, 0, line)?;
            Ok(num(Num::Float(std::f64::consts::E)))
        }
        Builtin::Log => {
            if args.is_empty() || args.len() > 2 {
                return Err(AzraError::runtime_at(
                    "`log` expects 1 or 2 arguments: `log of x` or `log base b of x`",
                    line,
                ));
            }
            let x = args[0].as_number("log", line)?.as_f64();
            if !x.is_finite() || x <= 0.0 {
                return Err(AzraError::runtime_at(
                    format!("`log` needs a positive finite number, got {}", x),
                    line,
                ));
            }
            if args.len() == 2 {
                let base = args[1].as_number("log base", line)?.as_f64();
                if !base.is_finite() || base <= 0.0 || base == 1.0 {
                    return Err(AzraError::runtime_at(
                        format!("`log` base must be positive and not 1, got {}", base),
                        line,
                    ));
                }
                Ok(num(Num::Float(x.ln() / base.ln())))
            } else {
                Ok(num(Num::Float(x.ln())))
            }
        }
        Builtin::Log10 => {
            expect_args(name, args, 1, line)?;
            let x = args[0].as_number("log10", line)?.as_f64();
            if !x.is_finite() || x <= 0.0 {
                return Err(AzraError::runtime_at(
                    format!("`log10` needs a positive finite number, got {}", x),
                    line,
                ));
            }
            Ok(num(Num::Float(x.log10())))
        }
        Builtin::Exp => {
            expect_args(name, args, 1, line)?;
            let x = args[0].as_number("exp", line)?.as_f64();
            let v = x.exp();
            if !v.is_finite() {
                return Err(AzraError::runtime_at("`exp` overflowed", line));
            }
            Ok(num(Num::Float(v)))
        }
        Builtin::Lerp => {
            expect_args(name, args, 3, line)?;
            let t = args[2].as_number("lerp amount", line)?.as_f64();
            if !t.is_finite() {
                return Err(AzraError::runtime_at("`lerp` amount must be finite", line));
            }
            match (&args[0], &args[1]) {
                (Value::Vec3(a), Value::Vec3(b)) => Ok(Value::Vec3(a.lerp(b, t))),
                (a, b) => {
                    let a = a.as_number("lerp from", line)?.as_f64();
                    let b = b.as_number("lerp to", line)?.as_f64();
                    Ok(num(Num::Float(a + (b - a) * t)))
                }
            }
        }
        Builtin::DegreesToRadians | Builtin::RadiansToDegrees => {
            expect_args(name, args, 1, line)?;
            let x = args[0].as_number(name.as_str(), line)?.as_f64();
            let v = if name == Builtin::DegreesToRadians {
                x.to_radians()
            } else {
                x.to_degrees()
            };
            Ok(num(Num::Float(v)))
        }
        Builtin::Dot => {
            expect_args(name, args, 2, line)?;
            let a = as_vec3(&args[0], "dot", line)?;
            let b = as_vec3(&args[1], "dot", line)?;
            Ok(num(Num::Float(a.dot(&b))))
        }
        Builtin::Cross => {
            expect_args(name, args, 2, line)?;
            let a = as_vec3(&args[0], "cross", line)?;
            let b = as_vec3(&args[1], "cross", line)?;
            Ok(Value::Vec3(a.cross(&b)))
        }
        Builtin::Magnitude => {
            expect_args(name, args, 1, line)?;
            match &args[0] {
                Value::Vec3(v) => Ok(num(Num::Float(v.length()))),
                other => {
                    let x = other.as_number("magnitude", line)?.as_f64();
                    Ok(num(Num::Float(x.abs())))
                }
            }
        }
        Builtin::AngleBetween => {
            expect_args(name, args, 2, line)?;
            let a = as_vec3(&args[0], "angle between", line)?;
            let b = as_vec3(&args[1], "angle between", line)?;
            let (la, lb) = (a.length(), b.length());
            if la == 0.0 || lb == 0.0 {
                return Err(AzraError::runtime_at(
                    "`angle between` needs two non-zero vectors",
                    line,
                ));
            }
            let cos = (a.dot(&b) / (la * lb)).clamp(-1.0, 1.0);
            Ok(num(Num::Float(cos.acos())))
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
