//! Statement execution dispatch: assignment, objects, functions,
//! events, timers, modules, and I/O statements.

use super::helpers::{attach_line, normalize_phrase, stmt_line};
use super::mock::mock_object;
use super::{EventHandler, Flow, Interpreter, OutputLine, Timer, ENGINE_VERBS};
use crate::ast::*;
use crate::error::AzraError;
use crate::value::{FunctionValue, Value};
use std::rc::Rc;

impl Interpreter {
    pub(super) fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Flow, AzraError> {
        for stmt in stmts {
            match self.exec_stmt(stmt)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, AzraError> {
        self.count_instruction()?;
        let line = stmt_line(stmt);
        self.exec_stmt_inner(stmt)
            .map_err(|e| attach_line(e, line))
    }

    fn exec_stmt_inner(&mut self, stmt: &Stmt) -> Result<Flow, AzraError> {
        match stmt {
            Stmt::Set { target, value, line } => {
                let val = if value.len() == 1 {
                    self.eval(&value[0])?
                } else {
                    let mut items = Vec::with_capacity(value.len());
                    for e in value {
                        items.push(self.eval(e)?);
                    }
                    Value::list(items)
                };
                self.assign_target(target, val, *line)?;
                Ok(Flow::Normal)
            }
            Stmt::Make {
                type_name,
                name,
                line,
            } => {
                let mut obj = mock_object(type_name);
                self.next_id += 1;
                obj.id = self.next_id;
                let value = Value::object(obj);
                self.world.push(value.clone());
                self.scope.define(name, value);
                let _ = line;
                Ok(Flow::Normal)
            }
            Stmt::Add { value, target, line } => self.exec_add(value, target, *line),
            Stmt::Remove { value, target, line } => self.exec_remove(value, target, *line),
            Stmt::RemoveAt { index, target, line } => self.exec_remove_at(index, target, *line),
            Stmt::RemoveKey { key, target, line } => self.exec_remove_key(key, target, *line),
            Stmt::Subtract { value, target, line } => self.exec_subtract(value, target, *line),
            Stmt::Multiply { target, value, line } => self.exec_multiply(target, value, *line),
            Stmt::Divide { target, value, line } => self.exec_divide(target, value, *line),
            Stmt::Clear { target, line } => self.exec_clear(target, *line),
            Stmt::Sort { target, line } => self.exec_sort(target, *line),
            Stmt::Reverse { target, line } => self.exec_reverse(target, *line),
            Stmt::Output { level, args, .. } => {
                let mut parts = Vec::with_capacity(args.len());
                for e in args {
                    parts.push(self.eval(e)?.render());
                }
                let text = parts.join(" ");
                let text = match level {
                    OutputLevel::Say => text,
                    OutputLevel::Warn => format!("[WARN] {}", text),
                    OutputLevel::Error => format!("[ERROR] {}", text),
                };
                self.output.push(OutputLine { level: *level, text });
                Ok(Flow::Normal)
            }
            Stmt::If {
                branches,
                otherwise,
                ..
            } => self.exec_if(branches, otherwise),
            Stmt::Repeat { count, body, line } => self.exec_repeat(count, body, *line),
            Stmt::RepeatRange {
                var,
                from,
                to,
                step,
                body,
                line,
            } => self.exec_repeat_range(var, from, to, step, body, *line),
            Stmt::ForEach {
                var,
                iter,
                body,
                line,
            } => self.exec_for_each(var, iter, body, *line),
            Stmt::While { cond, body, line } => self.exec_while(cond, body, *line),
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
            Stmt::Define {
                name,
                params,
                body,
                ..
            } => {
                let func = Rc::new(FunctionValue {
                    name: name.clone(),
                    params: params.clone(),
                    body: Rc::new(body.clone()),
                    closure: self.scope.clone(),
                });
                self.scope.define(name, Value::Function(func));
                Ok(Flow::Normal)
            }
            Stmt::Return { value, line } => {
                if self.in_function == 0 {
                    return Err(AzraError::runtime_at(
                        "`give back` outside of a function",
                        *line,
                    ));
                }
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Null,
                };
                Ok(Flow::Return(v))
            }
            Stmt::When {
                phrase,
                params,
                body,
                ..
            } => {
                self.handlers.push(EventHandler {
                    phrase: phrase.clone(),
                    normalized: normalize_phrase(phrase),
                    params: params.clone(),
                    body: Rc::new(body.clone()),
                    closure: self.scope.clone(),
                });
                Ok(Flow::Normal)
            }
            Stmt::Call { path, args, line } => {
                // Engine verbs accept bare names as arguments
                // (`spawn zombie` — `zombie` need not be defined).
                let lenient = path.len() == 1 && ENGINE_VERBS.contains(&path[0].as_str());
                let mut values = Vec::with_capacity(args.len());
                for e in args {
                    values.push(self.eval_arg(e, lenient)?);
                }
                self.call_path(path, values, *line)?;
                Ok(Flow::Normal)
            }
            Stmt::ExprStmt { expr, .. } => {
                self.eval(expr)?;
                Ok(Flow::Normal)
            }
            Stmt::Try {
                body,
                err_name,
                handler,
                ..
            } => match self.exec_block(body) {
                Ok(flow) => Ok(flow),
                Err(err) => {
                    if !err.is_catchable() {
                        return Err(err);
                    }
                    self.scope.define(err_name, Value::Str(err.message.clone()));
                    self.exec_block(handler)
                }
            },
            Stmt::Fail { message, line } => {
                let message = self.eval(message)?.render();
                Err(AzraError::runtime_at(message, *line))
            }
            Stmt::Use { path, alias, line } => {
                self.exec_use(path, alias, *line)?;
                Ok(Flow::Normal)
            }
            Stmt::Every {
                interval,
                body,
                line,
            } => self.exec_timer(interval, body, true, *line),
            Stmt::After { delay, body, line } => self.exec_timer(delay, body, false, *line),
            Stmt::Wait { duration, line } => {
                let secs = self
                    .eval(duration)?
                    .as_number("`wait` duration", *line)?
                    .as_f64();
                let dur = std::time::Duration::try_from_secs_f64(secs).map_err(|_| {
                    AzraError::runtime_at(
                        format!(
                            "`wait` duration must be a valid non-negative finite number, got {}",
                            secs
                        ),
                        *line,
                    )
                })?;
                if !self.in_tick && self.sandbox.allow_io {
                    // Standalone/CLI mode: sleep for real (blocked in strict sandbox).
                    std::thread::sleep(dur);
                }
                // In tick/sandbox mode the script clock advances instead of
                // blocking the host thread.
                self.now += secs;
                Ok(Flow::Normal)
            }
            Stmt::WriteFile { data, path, line } => {
                if !self.sandbox.allow_io {
                    return Err(AzraError::runtime_at(
                        "`write ... to file` is disabled by the sandbox (allow_io is off)",
                        *line,
                    ));
                }
                let data = self.eval(data)?.render();
                let path = self.eval(path)?.as_str("file path", *line)?;
                std::fs::write(&path, data)
                    .map_err(|e| AzraError::io(format!("cannot write file `{}`: {}", path, e)))?;
                Ok(Flow::Normal)
            }
        }
    }

    fn exec_timer(
        &mut self,
        expr: &Expr,
        body: &[Stmt],
        repeating: bool,
        line: usize,
    ) -> Result<Flow, AzraError> {
        let what = if repeating {
            "`every` interval"
        } else {
            "`after` delay"
        };
        let secs = self.eval(expr)?.as_number(what, line)?.as_f64();
        let invalid = if repeating {
            secs.is_nan() || secs <= 0.0
        } else {
            secs.is_nan() || secs < 0.0
        };
        if invalid {
            let msg = if repeating {
                "`every` interval must be greater than 0 seconds"
            } else {
                "`after` delay must not be negative"
            };
            return Err(AzraError::runtime_at(msg, line));
        }
        self.timers.push(Timer {
            interval: secs,
            next_due: self.now + secs,
            repeating,
            body: Rc::new(body.to_vec()),
            closure: self.scope.clone(),
            line,
        });
        Ok(Flow::Normal)
    }
}
