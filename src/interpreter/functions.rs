//! Function and command calls: user-defined `define` functions, host
//! functions registered by the embedder, method calls (`weapons.fire`) and
//! the mock engine verbs.

use super::{ENGINE_VERBS, Flow, Interpreter, OutputLevel, OutputLine};
use crate::ast::Expr;
use crate::env::Scope;
use crate::error::AzraError;
use crate::value::{FunctionValue, Value};
use std::rc::Rc;

impl Interpreter {
    /// Evaluate a call argument; engine verbs treat an undefined bare name
    /// as the string value of that name (`spawn zombie`).
    pub(super) fn eval_arg(&mut self, expr: &Expr, lenient: bool) -> Result<Value, AzraError> {
        if lenient
            && let Expr::Var { name, line: _ } = expr
            && self.scope.get(name).is_none()
        {
            return Ok(Value::Str(name.clone()));
        }
        self.eval(expr)
    }

    /// Call `path` — a plain function or a method chain such as
    /// `weapons.fire`.
    pub(super) fn call_path(
        &mut self,
        path: &[String],
        args: Vec<Value>,
        line: usize,
    ) -> Result<Value, AzraError> {
        if path.len() == 1 {
            return self.call(&path[0], args, line);
        }
        // Method call: resolve the final value, then invoke it.
        let mut current = self.scope.get(&path[0]).ok_or_else(|| {
            AzraError::runtime_at(format!("undefined function `{}`", path.join(".")), line)
        })?;
        for seg in &path[1..] {
            current = match &current {
                Value::Object(o) => o.borrow().get(seg).cloned(),
                Value::Map(m) => m.borrow().get(seg).cloned(),
                _ => None,
            }
            .ok_or_else(|| {
                AzraError::runtime_at(format!("undefined function `{}`", path.join(".")), line)
            })?;
        }
        match current {
            Value::Function(f) => self.call_function(f, args, line),
            _ => Err(AzraError::runtime_at(
                format!("undefined function `{}`", path.join(".")),
                line,
            )),
        }
    }

    pub(super) fn call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        line: usize,
    ) -> Result<Value, AzraError> {
        // 1. user-defined function visible from the current scope
        if let Some(Value::Function(f)) = self.scope.get(name) {
            return self.call_function(f, args, line);
        }
        // 2. host function registered through the embedding API
        if let Some(func) = self.host.get(name) {
            return func(&args);
        }
        // 3. mock engine verbs log their action in standalone mode
        if ENGINE_VERBS.contains(&name) {
            let mut text = format!("[ENGINE] {}", name);
            for a in &args {
                text.push(' ');
                text.push_str(&a.render());
            }
            self.output.push(OutputLine {
                level: OutputLevel::Say,
                text,
            });
            return Ok(Value::Null);
        }
        Err(AzraError::runtime_at(
            format!("undefined function `{}`", name),
            line,
        ))
    }

    pub(super) fn call_function(
        &mut self,
        func: Rc<FunctionValue>,
        args: Vec<Value>,
        line: usize,
    ) -> Result<Value, AzraError> {
        if args.len() != func.params.len() {
            return Err(AzraError::runtime_at(
                format!(
                    "function `{}` expects {} argument(s), got {}",
                    func.name,
                    func.params.len(),
                    args.len()
                ),
                line,
            ));
        }
        if self.call_depth >= self.sandbox.max_call_depth {
            return Err(AzraError::fatal_runtime_at(
                format!(
                    "call stack overflow (max depth {})",
                    self.sandbox.max_call_depth
                ),
                line,
            ));
        }
        let saved_scope = self.scope.clone();
        let saved_in_function = self.in_function;
        let scope = Scope::new(&func.closure);
        for (param, arg) in func.params.iter().zip(args) {
            scope.define(param, arg);
        }
        self.scope = scope;
        self.call_depth += 1;
        self.in_function += 1;
        let result = self.exec_block(&func.body);
        self.call_depth -= 1;
        self.in_function = saved_in_function;
        self.scope = saved_scope;
        match result? {
            Flow::Return(v) => Ok(v),
            Flow::Normal => Ok(Value::Null),
            Flow::Break | Flow::Continue => Err(AzraError::runtime(
                "`stop loop` / `skip next` outside of a loop",
            )),
        }
    }
}
