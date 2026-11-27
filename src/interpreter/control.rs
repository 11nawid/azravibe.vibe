//! Conditional branch and loop execution (`if`, `repeat`, `for each`, `while`).

use super::helpers::repeat_count;
use super::{Flow, Interpreter, MAX_LOOP_ITERATIONS};
use crate::ast::{Branch, Expr, Stmt};
use crate::env::Scope;
use crate::error::AzraError;
use crate::token::Num;
use crate::value::Value;

impl Interpreter {
    pub(super) fn exec_if(
        &mut self,
        branches: &[Branch],
        otherwise: &Option<Vec<Stmt>>,
    ) -> Result<Flow, AzraError> {
        for branch in branches {
            if self.eval(&branch.cond)?.is_truthy() {
                return self.exec_block(&branch.body);
            }
        }
        if let Some(body) = otherwise {
            return self.exec_block(body);
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_repeat(
        &mut self,
        count: &Expr,
        body: &[Stmt],
        line: usize,
    ) -> Result<Flow, AzraError> {
        let n = self.eval(count)?;
        let times = repeat_count(&n, line)?;
        for _ in 0..times {
            self.count_instruction()?;
            match self.exec_block(body)? {
                Flow::Normal => {}
                Flow::Break => break,
                Flow::Continue => continue,
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    pub(super) fn exec_repeat_range(
        &mut self,
        var: &str,
        from: &Expr,
        to: &Expr,
        step: &Option<Expr>,
        body: &[Stmt],
        line: usize,
    ) -> Result<Flow, AzraError> {
        let from = self
            .eval(from)?
            .as_number("`repeat ... from`", line)?
            .as_f64();
        let to = self.eval(to)?.as_number("`repeat ... to`", line)?.as_f64();
        let step = match step {
            Some(expr) => {
                let s = self
                    .eval(expr)?
                    .as_number("`repeat ... by`", line)?
                    .as_f64();
                if s == 0.0 {
                    return Err(AzraError::runtime_at(
                        "`repeat` step must not be zero",
                        line,
                    ));
                }
                s
            }
            None => {
                if from <= to {
                    1.0
                } else {
                    -1.0
                }
            }
        };
        let ascending = step > 0.0;
        let saved = self.scope.clone();
        self.scope = Scope::new(&saved);
        let mut i = from;
        let mut iterations: u64 = 0;
        let mut result = Ok(Flow::Normal);
        while (ascending && i <= to) || (!ascending && i >= to) {
            if let Err(e) = self.count_instruction() {
                result = Err(e);
                break;
            }
            iterations += 1;
            if iterations > MAX_LOOP_ITERATIONS {
                result = Err(AzraError::runtime_at(
                    format!(
                        "`repeat` loop exceeded the safety cap of {} iterations",
                        MAX_LOOP_ITERATIONS
                    ),
                    line,
                ));
                break;
            }
            let value = if i.fract() == 0.0 && i.abs() < 1e15 {
                Value::Number(Num::Int(i as i64))
            } else {
                Value::Number(Num::Float(i))
            };
            self.scope.define(var, value);
            match self.exec_block(body) {
                Ok(Flow::Normal) => {}
                Ok(Flow::Break) => break,
                Ok(Flow::Continue) => {}
                other => {
                    result = other;
                    break;
                }
            }
            i += step;
        }
        self.scope = saved;
        result
    }

    pub(super) fn exec_for_each(
        &mut self,
        var: &str,
        iter: &Expr,
        body: &[Stmt],
        line: usize,
    ) -> Result<Flow, AzraError> {
        let iterable = self.eval(iter)?;
        let snapshot = match &iterable {
            Value::List(items) => items.borrow().clone(),
            other => {
                return Err(AzraError::runtime_at(
                    format!("`for each` expects a list, got {}", other.type_name()),
                    line,
                ));
            }
        };
        let saved = self.scope.clone();
        self.scope = Scope::new(&saved);
        let mut result = Ok(Flow::Normal);
        for item in snapshot.iter() {
            if let Err(e) = self.count_instruction() {
                result = Err(e);
                break;
            }
            self.scope.define(var, item.clone());
            match self.exec_block(body) {
                Ok(Flow::Normal) => {}
                Ok(Flow::Break) => break,
                Ok(Flow::Continue) => {}
                other => {
                    result = other;
                    break;
                }
            }
        }
        self.scope = saved;
        result
    }

    pub(super) fn exec_while(
        &mut self,
        cond: &Expr,
        body: &[Stmt],
        line: usize,
    ) -> Result<Flow, AzraError> {
        let mut iterations: u64 = 0;
        while self.eval(cond)?.is_truthy() {
            self.count_instruction()?;
            iterations += 1;
            if iterations > MAX_LOOP_ITERATIONS {
                return Err(AzraError::runtime_at(
                    format!(
                        "`while` loop exceeded the safety cap of {} iterations",
                        MAX_LOOP_ITERATIONS
                    ),
                    line,
                ));
            }
            match self.exec_block(body)? {
                Flow::Normal => {}
                Flow::Break => break,
                Flow::Continue => continue,
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }
}
