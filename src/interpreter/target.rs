//! Assignment targets (`set ... to ...`) and path reads/writes:
//! plain paths, `key ... of ...`, `item N of ...`, `first`/`last item of`.

use super::Interpreter;
use crate::ast::{Expr, Target};
use crate::error::AzraError;
use crate::token::Num;
use crate::value::Value;

impl Interpreter {
    /// Read the current value of an assignment target.
    pub(super) fn eval_target(
        &mut self,
        target: &Target,
        _line: usize,
    ) -> Result<Value, AzraError> {
        match target {
            Target::Path { path, line } => self.eval_path(path, *line),
            Target::Key { key, map, line } => {
                let key = self.eval(key)?.as_str("key", *line)?;
                let container = self.eval_target(map, *line)?;
                match &container {
                    Value::Map(m) => Ok(m.borrow().get(&key).cloned().unwrap_or(Value::Null)),
                    Value::Object(o) => Ok(o.borrow().get(&key).cloned().unwrap_or(Value::Null)),
                    other => Err(AzraError::runtime_at(
                        format!(
                            "cannot read key `{}` of {} (only maps and objects have keys)",
                            key,
                            other.type_name()
                        ),
                        *line,
                    )),
                }
            }
            Target::Item { index, list, line } => {
                let idx = self.eval_index(index, *line)?;
                let container = self.eval_target(list, *line)?;
                match &container {
                    Value::List(items) => {
                        let items = items.borrow();
                        items.get(idx).cloned().ok_or_else(|| {
                            AzraError::runtime_at(
                                format!(
                                    "item {} of list is out of range (list has {} item(s))",
                                    idx + 1,
                                    items.len()
                                ),
                                *line,
                            )
                        })
                    }
                    other => Err(AzraError::runtime_at(
                        format!("`item ... of` expects a list, got {}", other.type_name()),
                        *line,
                    )),
                }
            }
            Target::First { list, line } => {
                let container = self.eval_target(list, *line)?;
                match &container {
                    Value::List(items) => {
                        items.borrow().first().cloned().ok_or_else(|| {
                            AzraError::runtime_at("first item of an empty list", *line)
                        })
                    }
                    other => Err(AzraError::runtime_at(
                        format!("`first item of` expects a list, got {}", other.type_name()),
                        *line,
                    )),
                }
            }
            Target::Last { list, line } => {
                let container = self.eval_target(list, *line)?;
                match &container {
                    Value::List(items) => {
                        items.borrow().last().cloned().ok_or_else(|| {
                            AzraError::runtime_at("last item of an empty list", *line)
                        })
                    }
                    other => Err(AzraError::runtime_at(
                        format!("`last item of` expects a list, got {}", other.type_name()),
                        *line,
                    )),
                }
            }
        }
    }

    /// Write `value` into an assignment target.
    pub(super) fn assign_target(
        &mut self,
        target: &Target,
        value: Value,
        _line: usize,
    ) -> Result<(), AzraError> {
        match target {
            Target::Path { path, line } => self.assign_path(path, value, *line),
            Target::Key { key, map, line } => {
                let key = self.eval(key)?.as_str("key", *line)?;
                let container = self.eval_target(map, *line)?;
                match &container {
                    Value::Map(m) => {
                        m.borrow_mut().set(&key, value);
                        Ok(())
                    }
                    Value::Object(o) => {
                        o.borrow_mut().set(&key, value);
                        Ok(())
                    }
                    other => Err(AzraError::runtime_at(
                        format!(
                            "cannot set key `{}` of {} (only maps and objects have keys)",
                            key,
                            other.type_name()
                        ),
                        *line,
                    )),
                }
            }
            Target::Item { index, list, line } => {
                let idx = self.eval_index(index, *line)?;
                let container = self.eval_target(list, *line)?;
                match &container {
                    Value::List(items) => {
                        let mut items = items.borrow_mut();
                        if idx >= items.len() {
                            return Err(AzraError::runtime_at(
                                format!(
                                    "item {} of list is out of range (list has {} item(s))",
                                    idx + 1,
                                    items.len()
                                ),
                                *line,
                            ));
                        }
                        items[idx] = value;
                        Ok(())
                    }
                    other => Err(AzraError::runtime_at(
                        format!("`item ... of` expects a list, got {}", other.type_name()),
                        *line,
                    )),
                }
            }
            Target::First { list, line } => {
                let container = self.eval_target(list, *line)?;
                match &container {
                    Value::List(items) => {
                        let mut items = items.borrow_mut();
                        if items.is_empty() {
                            return Err(AzraError::runtime_at(
                                "first item of an empty list",
                                *line,
                            ));
                        }
                        items[0] = value;
                        Ok(())
                    }
                    other => Err(AzraError::runtime_at(
                        format!("`first item of` expects a list, got {}", other.type_name()),
                        *line,
                    )),
                }
            }
            Target::Last { list, line } => {
                let container = self.eval_target(list, *line)?;
                match &container {
                    Value::List(items) => {
                        let mut items = items.borrow_mut();
                        if items.is_empty() {
                            return Err(AzraError::runtime_at("last item of an empty list", *line));
                        }
                        let last = items.len() - 1;
                        items[last] = value;
                        Ok(())
                    }
                    other => Err(AzraError::runtime_at(
                        format!("`last item of` expects a list, got {}", other.type_name()),
                        *line,
                    )),
                }
            }
        }
    }

    /// Evaluate a 1-based index expression to a 0-based Rust index.
    pub(super) fn eval_index(&mut self, expr: &Expr, line: usize) -> Result<usize, AzraError> {
        let v = self.eval(expr)?;
        let n = v.as_number("item index", line)?.as_f64();
        if n.fract() != 0.0 {
            return Err(AzraError::runtime_at(
                format!("item index must be a whole number, got {}", n),
                line,
            ));
        }
        let i = n as i64;
        if i < 1 {
            return Err(AzraError::runtime_at(
                format!("item index must be 1 or greater (got {})", i),
                line,
            ));
        }
        Ok((i - 1) as usize)
    }

    /// Read a dotted path (`player.position.x` style chains of objects).
    pub(super) fn eval_path(&mut self, path: &[String], line: usize) -> Result<Value, AzraError> {
        let mut current = match self.scope.get(&path[0]) {
            Some(Value::Function(f)) if path.len() == 1 && f.params.is_empty() => {
                self.call_function(f, Vec::new(), line)?
            }
            Some(v) => v,
            None => {
                return Err(AzraError::runtime_at(
                    format!("undefined variable `{}`", path[0]),
                    line,
                ));
            }
        };
        for seg in &path[1..] {
            current = read_property(&current, seg, line)?;
        }
        Ok(current)
    }

    /// Write a dotted path (`set player.health to 100`).
    pub(super) fn assign_path(
        &mut self,
        path: &[String],
        value: Value,
        line: usize,
    ) -> Result<(), AzraError> {
        if path.len() == 1 {
            if !self.scope.set_existing(&path[0], value.clone()) {
                self.scope.define(&path[0], value);
            }
            return Ok(());
        }
        let root = self.scope.get(&path[0]).ok_or_else(|| {
            AzraError::runtime_at(format!("undefined variable `{}`", path[0]), line)
        })?;
        let mut current = root;
        for seg in &path[1..path.len() - 1] {
            current = match &current {
                Value::Object(data) => data.borrow().get(seg).cloned().ok_or_else(|| {
                    let type_name = data.borrow().type_name.clone();
                    AzraError::runtime_at(
                        format!("object of type `{}` has no property `{}`", type_name, seg),
                        line,
                    )
                })?,
                other => {
                    return Err(AzraError::runtime_at(
                        format!(
                            "cannot access property `{}` of {} (only objects have properties)",
                            seg,
                            other.type_name()
                        ),
                        line,
                    ));
                }
            };
        }
        let last = path.last().expect("path non-empty");
        match &current {
            Value::Object(data) => {
                data.borrow_mut().set(last, value);
                Ok(())
            }
            other => Err(AzraError::runtime_at(
                format!(
                    "cannot set property `{}` on {} (only objects have properties)",
                    last,
                    other.type_name()
                ),
                line,
            )),
        }
    }
}

/// Read a named property from an object, `vec3` (`x`/`y`/`z`), or `color` (`r`/`g`/`b`).
pub(super) fn read_property(obj: &Value, name: &str, line: usize) -> Result<Value, AzraError> {
    match obj {
        Value::Object(data) => {
            let data = data.clone();
            let found = data.borrow().get(name).cloned();
            match found {
                Some(v) => Ok(v),
                None if name == "id" => Ok(Value::Number(Num::Int(data.borrow().id as i64))),
                None => {
                    let type_name = data.borrow().type_name.clone();
                    Err(AzraError::runtime_at(
                        format!("object of type `{}` has no property `{}`", type_name, name),
                        line,
                    ))
                }
            }
        }
        Value::Map(data) => {
            let found = data.borrow().get(name).cloned();
            found.ok_or_else(|| AzraError::runtime_at(format!("map has no key `{name}`"), line))
        }
        Value::Vec3(v) => match name {
            "x" => Ok(Value::Number(Num::Float(v.x))),
            "y" => Ok(Value::Number(Num::Float(v.y))),
            "z" => Ok(Value::Number(Num::Float(v.z))),
            _ => Err(AzraError::runtime_at(
                format!("vec3 has no property `{}` (only x, y and z)", name),
                line,
            )),
        },
        Value::Color(c) => match name {
            "r" => Ok(Value::Number(Num::Int(c.r as i64))),
            "g" => Ok(Value::Number(Num::Int(c.g as i64))),
            "b" => Ok(Value::Number(Num::Int(c.b as i64))),
            _ => Err(AzraError::runtime_at(
                format!("color has no property `{}` (only r, g and b)", name),
                line,
            )),
        },
        other => Err(AzraError::runtime_at(
            format!(
                "cannot read property `{}` of {} (only objects have properties)",
                name,
                other.type_name()
            ),
            line,
        )),
    }
}
