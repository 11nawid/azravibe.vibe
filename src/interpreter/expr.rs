//! Expression evaluation: variables, members, calls, interpolation,
//! built-in forms and binary operators.

use super::Interpreter;
use super::helpers::{
    compare_values, num_add, num_div, num_mod, num_mul, num_neg, num_pow, num_sub,
};
use super::target::read_property;
use crate::ast::*;
use crate::error::AzraError;
use crate::stdlib::{self, BuiltinCtx};
use crate::value::Value;

impl Interpreter {
    pub(super) fn eval(&mut self, expr: &Expr) -> Result<Value, AzraError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(n.clone())),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),
            Expr::Var { name, line } => match self.scope.get(name) {
                // A bound zero-argument function is auto-invoked so scripts
                // can write `set x to get_time` without parentheses.
                Some(Value::Function(f)) if f.params.is_empty() => {
                    self.call_function(f, Vec::new(), *line)
                }
                Some(v) => Ok(v),
                None => Err(AzraError::runtime_at(
                    format!("undefined variable `{}`", name),
                    *line,
                )),
            },
            Expr::Member { object, name, line } => {
                let obj = self.eval(object)?;
                read_property(&obj, name, *line)
            }
            Expr::Call { path, args, line } => {
                let mut values = Vec::with_capacity(args.len());
                for e in args {
                    values.push(self.eval(e)?);
                }
                self.call_path(path, values, *line)
            }
            Expr::Interp { parts, .. } => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        InterpPart::Text(t) => out.push_str(t),
                        InterpPart::Path { path, line } => {
                            out.push_str(&self.eval_path(path, *line)?.render());
                        }
                    }
                }
                Ok(Value::Str(out))
            }
            Expr::Builtin { name, args, line } => {
                let mut values = Vec::with_capacity(args.len());
                for e in args {
                    values.push(self.eval(e)?);
                }
                let mut ctx = BuiltinCtx {
                    sandbox: &self.sandbox,
                    rng: &mut self.rng,
                };
                stdlib::eval_builtin(*name, values, &mut ctx, *line)
            }
            Expr::Not { expr, line: _ } => {
                let v = self.eval(expr)?;
                Ok(Value::Bool(!v.is_truthy()))
            }
            Expr::Negate { expr, line } => {
                let v = self.eval(expr)?;
                match v {
                    Value::Number(n) => Ok(Value::Number(num_neg(&n, *line)?)),
                    other => Err(AzraError::runtime_at(
                        format!("cannot negate {}", other.type_name()),
                        *line,
                    )),
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                line,
            } => self.eval_binary(*op, left, right, *line),
        }
    }

    fn eval_binary(
        &mut self,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        line: usize,
    ) -> Result<Value, AzraError> {
        // Short-circuit logic
        if op == BinOp::And {
            let l = self.eval(left)?;
            if !l.is_truthy() {
                return Ok(Value::Bool(false));
            }
            let r = self.eval(right)?;
            return Ok(Value::Bool(r.is_truthy()));
        }
        if op == BinOp::Or {
            let l = self.eval(left)?;
            if l.is_truthy() {
                return Ok(Value::Bool(true));
            }
            let r = self.eval(right)?;
            return Ok(Value::Bool(r.is_truthy()));
        }

        let l = self.eval(left)?;
        let r = self.eval(right)?;
        match op {
            BinOp::Eq => Ok(Value::Bool(l.equals(&r))),
            BinOp::Ne => Ok(Value::Bool(!l.equals(&r))),
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let ord = compare_values(&l, &r, line)?;
                let result = match op {
                    BinOp::Lt => ord == std::cmp::Ordering::Less,
                    BinOp::Le => ord != std::cmp::Ordering::Greater,
                    BinOp::Gt => ord == std::cmp::Ordering::Greater,
                    BinOp::Ge => ord != std::cmp::Ordering::Less,
                    _ => unreachable!(),
                };
                Ok(Value::Bool(result))
            }
            BinOp::Add => {
                // vec3 + vec3, color + color (saturating)
                if let (Value::Vec3(a), Value::Vec3(b)) = (&l, &r) {
                    return Ok(Value::Vec3(crate::value::Vec3Data::new(
                        a.x + b.x,
                        a.y + b.y,
                        a.z + b.z,
                    )));
                }
                if let (Value::Color(a), Value::Color(b)) = (&l, &r) {
                    return Ok(Value::Color(crate::value::ColorData::new(
                        a.r.saturating_add(b.r),
                        a.g.saturating_add(b.g),
                        a.b.saturating_add(b.b),
                    )));
                }
                // A string operand on either side concatenates (rendered).
                if matches!(l, Value::Str(_)) || matches!(r, Value::Str(_)) {
                    return Ok(Value::Str(format!("{}{}", l.render(), r.render())));
                }
                match (&l, &r) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(num_add(a, b, line)?)),
                    (Value::List(a), Value::List(b)) => {
                        let mut items = a.borrow().clone();
                        items.extend(b.borrow().iter().cloned());
                        Ok(Value::list(items))
                    }
                    _ => Err(AzraError::runtime_at(
                        format!(
                            "cannot add {} and {} together",
                            l.type_name(),
                            r.type_name()
                        ),
                        line,
                    )),
                }
            }
            BinOp::Sub => {
                if let (Value::Vec3(a), Value::Vec3(b)) = (&l, &r) {
                    return Ok(Value::Vec3(crate::value::Vec3Data::new(
                        a.x - b.x,
                        a.y - b.y,
                        a.z - b.z,
                    )));
                }
                if let (Value::Color(a), Value::Color(b)) = (&l, &r) {
                    return Ok(Value::Color(crate::value::ColorData::new(
                        a.r.saturating_sub(b.r),
                        a.g.saturating_sub(b.g),
                        a.b.saturating_sub(b.b),
                    )));
                }
                let a = l.as_number("operator `-` expects numbers", line)?;
                let b = r.as_number("operator `-` expects numbers", line)?;
                Ok(Value::Number(num_sub(&a, &b, line)?))
            }
            BinOp::Mul => {
                // vec3 * vec3 / vec3 * number, color * number (clamped)
                if let (Value::Vec3(a), Value::Vec3(b)) = (&l, &r) {
                    return Ok(Value::Vec3(crate::value::Vec3Data::new(
                        a.x * b.x,
                        a.y * b.y,
                        a.z * b.z,
                    )));
                }
                if let (Value::Vec3(v), Value::Number(n)) = (&l, &r) {
                    let n = n.as_f64();
                    return Ok(Value::Vec3(crate::value::Vec3Data::new(
                        v.x * n,
                        v.y * n,
                        v.z * n,
                    )));
                }
                if let (Value::Number(n), Value::Vec3(v)) = (&l, &r) {
                    let n = n.as_f64();
                    return Ok(Value::Vec3(crate::value::Vec3Data::new(
                        v.x * n,
                        v.y * n,
                        v.z * n,
                    )));
                }
                if let (Value::Color(c), Value::Number(n)) = (&l, &r) {
                    let scale = n.as_f64();
                    let ch = |v: u8| -> u8 { ((v as f64 * scale).round()).clamp(0.0, 255.0) as u8 };
                    return Ok(Value::Color(crate::value::ColorData::new(
                        ch(c.r),
                        ch(c.g),
                        ch(c.b),
                    )));
                }
                let a = l.as_number("operator `*` expects numbers", line)?;
                let b = r.as_number("operator `*` expects numbers", line)?;
                Ok(Value::Number(num_mul(&a, &b, line)?))
            }
            BinOp::Div => {
                let a = l.as_number("operator `/` expects numbers", line)?;
                let b = r.as_number("operator `/` expects numbers", line)?;
                Ok(Value::Number(num_div(&a, &b, line)?))
            }
            BinOp::Mod => {
                let a = l.as_number("operator `modulo` expects numbers", line)?;
                let b = r.as_number("operator `modulo` expects numbers", line)?;
                Ok(Value::Number(num_mod(&a, &b, line)?))
            }
            BinOp::Pow => {
                let a = l.as_number("operator `power` expects numbers", line)?;
                let b = r.as_number("operator `power` expects numbers", line)?;
                Ok(Value::Number(num_pow(&a, &b, line)?))
            }
            BinOp::Contains => match (&l, &r) {
                (Value::Str(hay), Value::Str(needle)) => Ok(Value::Bool(hay.contains(needle))),
                (Value::List(items), _) => {
                    Ok(Value::Bool(items.borrow().iter().any(|x| x.equals(&r))))
                }
                _ => Err(AzraError::runtime_at(
                    format!(
                        "`contains` needs a string or list on the left, got {}",
                        l.type_name()
                    ),
                    line,
                )),
            },
            BinOp::StartsWith => match (&l, &r) {
                (Value::Str(hay), Value::Str(needle)) => {
                    Ok(Value::Bool(hay.starts_with(needle.as_str())))
                }
                _ => Err(AzraError::runtime_at(
                    format!(
                        "`starts with` needs two strings, got {} and {}",
                        l.type_name(),
                        r.type_name()
                    ),
                    line,
                )),
            },
            BinOp::HasKey => {
                let key = r.as_str("key", line)?;
                match &l {
                    Value::Map(m) => Ok(Value::Bool(m.borrow().get(&key).is_some())),
                    Value::Object(o) => Ok(Value::Bool(o.borrow().get(&key).is_some())),
                    other => Err(AzraError::runtime_at(
                        format!("`has key` needs a map or object, got {}", other.type_name()),
                        line,
                    )),
                }
            }
            BinOp::And | BinOp::Or => unreachable!("handled above"),
        }
    }
}
