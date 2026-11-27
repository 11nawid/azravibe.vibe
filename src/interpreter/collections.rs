//! List, map, and numeric target mutations (`add`, `remove`, `subtract`,
//! `multiply`, `divide`, `clear`, `sort`, `reverse`).

use super::helpers::{num_add, num_div, num_mul, num_sub};
use super::{Flow, Interpreter};
use crate::ast::{Expr, Target};
use crate::error::AzraError;
use crate::value::Value;

impl Interpreter {
    pub(super) fn exec_add(
        &mut self,
        value: &Expr,
        target: &Target,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let value = self.eval(value)?;
        // Bootstrap: `add "key" to loot` creates the list when the
        // variable does not exist yet (numbers need `set x to 0`).
        if let Target::Path { path, .. } = target
            && self.scope.get(&path[0]).is_none()
            && !matches!(value, Value::Number(_))
        {
            self.assign_target(target, Value::list(vec![value]), line)?;
            return Ok(Flow::Normal);
        }
        let current = self.eval_target(target, line)?;
        match current {
            Value::Null if !matches!(value, Value::Number(_)) => {
                self.assign_target(target, Value::list(vec![value]), line)?;
            }
            Value::Number(n) => {
                let v = value.as_number("`add` expects a number", line)?;
                let sum = num_add(&n, &v, line)?;
                self.assign_target(target, Value::Number(sum), line)?;
            }
            Value::List(items) => {
                items.borrow_mut().push(value);
            }
            other => {
                return Err(AzraError::runtime_at(
                    format!(
                        "`add` expects a number or a list, got {}",
                        other.type_name()
                    ),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_remove(
        &mut self,
        value: &Expr,
        target: &Target,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let value = self.eval(value)?;
        let current = self.eval_target(target, line)?;
        match current {
            Value::Number(n) => {
                let v = value.as_number("`remove` expects a number", line)?;
                let diff = num_sub(&n, &v, line)?;
                self.assign_target(target, Value::Number(diff), line)?;
            }
            Value::List(items) => {
                let mut items = items.borrow_mut();
                if let Some(idx) = items.iter().position(|x| x.equals(&value)) {
                    items.remove(idx);
                }
            }
            other => {
                return Err(AzraError::runtime_at(
                    format!(
                        "`remove` expects a number or a list, got {}",
                        other.type_name()
                    ),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_remove_at(
        &mut self,
        index: &Expr,
        target: &Target,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let idx = self.eval_index(index, line)?;
        let current = self.eval_target(target, line)?;
        match current {
            Value::List(items) => {
                let mut items = items.borrow_mut();
                if idx >= items.len() {
                    return Err(AzraError::runtime_at(
                        format!(
                            "item {} of list is out of range (list has {} item(s))",
                            idx + 1,
                            items.len()
                        ),
                        line,
                    ));
                }
                items.remove(idx);
            }
            other => {
                return Err(AzraError::runtime_at(
                    format!("`remove item` expects a list, got {}", other.type_name()),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_remove_key(
        &mut self,
        key: &Expr,
        target: &Target,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let key = self.eval(key)?.as_str("key", line)?;
        let current = self.eval_target(target, line)?;
        match current {
            Value::Map(m) => {
                m.borrow_mut().remove(&key);
            }
            other => {
                return Err(AzraError::runtime_at(
                    format!("`remove key` expects a map, got {}", other.type_name()),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_subtract(
        &mut self,
        value: &Expr,
        target: &Target,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let current = self.eval_target(target, line)?;
        let n = current.as_number("`subtract` expects a number", line)?;
        let value = self.eval(value)?;
        let v = value.as_number("`subtract` expects a number", line)?;
        let diff = num_sub(&n, &v, line)?;
        self.assign_target(target, Value::Number(diff), line)?;
        Ok(Flow::Normal)
    }

    pub(super) fn exec_multiply(
        &mut self,
        target: &Target,
        value: &Expr,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let current = self.eval_target(target, line)?;
        let n = current.as_number("`multiply` expects a number", line)?;
        let value = self.eval(value)?;
        let v = value.as_number("`multiply` expects a number", line)?;
        let product = num_mul(&n, &v, line)?;
        self.assign_target(target, Value::Number(product), line)?;
        Ok(Flow::Normal)
    }

    pub(super) fn exec_divide(
        &mut self,
        target: &Target,
        value: &Expr,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let current = self.eval_target(target, line)?;
        let n = current.as_number("`divide` expects a number", line)?;
        let value = self.eval(value)?;
        let v = value.as_number("`divide` expects a number", line)?;
        let quotient = num_div(&n, &v, line)?;
        self.assign_target(target, Value::Number(quotient), line)?;
        Ok(Flow::Normal)
    }

    pub(super) fn exec_clear(&mut self, target: &Target, line: usize) -> Result<Flow, AzraError> {
        let current = self.eval_target(target, line)?;
        match current {
            Value::List(items) => items.borrow_mut().clear(),
            Value::Map(m) => m.borrow_mut().entries.clear(),
            other => {
                return Err(AzraError::runtime_at(
                    format!("`clear` expects a list or map, got {}", other.type_name()),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_sort(&mut self, target: &Target, line: usize) -> Result<Flow, AzraError> {
        let current = self.eval_target(target, line)?;
        match current {
            Value::List(items) => {
                let mut items = items.borrow_mut();
                // NaN has no defined position in an ordering, and a
                // comparator that calls NaN "equal" to everything produces
                // an arbitrary (or panicking) sort.
                let all_numbers = items
                    .iter()
                    .all(|v| matches!(v, Value::Number(n) if n.as_f64().is_finite()));
                let all_strings = items.iter().all(|v| matches!(v, Value::Str(_)));
                if all_numbers {
                    items.sort_by(|a, b| {
                        let (a, b) = match (a, b) {
                            (Value::Number(x), Value::Number(y)) => (x.as_f64(), y.as_f64()),
                            _ => (0.0, 0.0),
                        };
                        a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                } else if all_strings {
                    items.sort_by(|a, b| match (a, b) {
                        (Value::Str(x), Value::Str(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    });
                } else {
                    return Err(AzraError::runtime_at(
                        "`sort` needs a list of all finite numbers or all strings",
                        line,
                    ));
                }
            }
            other => {
                return Err(AzraError::runtime_at(
                    format!("`sort` expects a list, got {}", other.type_name()),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_reverse(&mut self, target: &Target, line: usize) -> Result<Flow, AzraError> {
        let current = self.eval_target(target, line)?;
        match current {
            Value::List(items) => items.borrow_mut().reverse(),
            other => {
                return Err(AzraError::runtime_at(
                    format!("`reverse` expects a list, got {}", other.type_name()),
                    line,
                ));
            }
        }
        Ok(Flow::Normal)
    }
}
