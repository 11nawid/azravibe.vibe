//! Tree-walking interpreter for Azra: scoped environment, function call
//! stack, engine event handlers, host functions and built-in statements.

use crate::ast::*;
use crate::error::AzraError;
use crate::token::Num;
use crate::value::{format_float, FunctionValue, ObjectData, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum function call depth before reporting a stack overflow.
const MAX_CALL_DEPTH: usize = 1000;
/// Safety cap for `repeat` / `while` loops.
const MAX_LOOP_ITERATIONS: u64 = 10_000_000;

/// A lexical scope chain node.
#[derive(Debug, Default)]
pub struct Scope {
    vars: RefCell<HashMap<String, Value>>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    pub fn root() -> Rc<Scope> {
        Rc::new(Scope::default())
    }

    pub fn new(parent: &Rc<Scope>) -> Rc<Scope> {
        Rc::new(Scope {
            vars: RefCell::new(HashMap::new()),
            parent: Some(parent.clone()),
        })
    }

    /// Look up a binding, walking outward through enclosing scopes.
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.vars.borrow().get(name) {
            return Some(v.clone());
        }
        self.parent.as_ref().and_then(|p| p.get(name))
    }

    /// Update the nearest existing binding; returns `false` if none exists.
    pub fn set_existing(&self, name: &str, value: Value) -> bool {
        let mut cur = Some(self);
        while let Some(scope) = cur {
            {
                let mut vars = scope.vars.borrow_mut();
                if let Some(slot) = vars.get_mut(name) {
                    *slot = value;
                    return true;
                }
            }
            cur = scope.parent.as_deref();
        }
        false
    }

    /// Bind a name in this scope only.
    pub fn define(&self, name: &str, value: Value) {
        self.vars.borrow_mut().insert(name.to_string(), value);
    }
}

/// Which stream a line of output belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// One buffered line produced by `say` / `warn` / `error` or an engine verb.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputLine {
    pub level: OutputLevel,
    /// `text` already contains the `[WARN]` / `[ERROR]` / `[ENGINE]` prefix;
    /// color is applied by the presenter (CLI).
    pub text: String,
}

impl OutputLine {
    pub fn stream(&self) -> OutputStream {
        match self.level {
            OutputLevel::Say => OutputStream::Stdout,
            OutputLevel::Warn | OutputLevel::Error => OutputStream::Stderr,
        }
    }
}

/// A registered `when <event>:` handler.
#[derive(Debug)]
struct EventHandler {
    phrase: String,
    normalized: String,
    body: Rc<Vec<Stmt>>,
    closure: Rc<Scope>,
}

type HostFn = Box<dyn Fn(&[Value]) -> Result<Value, AzraError>>;

/// Execution effect of a statement list.
enum Flow {
    Normal,
    Return(Value),
}

/// The Azra virtual machine.
pub struct Interpreter {
    globals: Rc<Scope>,
    scope: Rc<Scope>,
    handlers: Vec<EventHandler>,
    output: Vec<OutputLine>,
    host: HashMap<String, HostFn>,
    call_depth: usize,
    in_function: usize,
}

impl Interpreter {
    /// Fresh VM with mock engine types and a default `player` entity.
    pub fn new() -> Self {
        let globals = Scope::root();

        // Default player so game scripts (see docs example) run standalone.
        let mut player = ObjectData::new("player");
        player.set("name", Value::Str("player".to_string()));
        player.set("health", Value::Number(Num::Int(100)));
        player.set("position", Value::list(vec![
            Value::Number(Num::Int(0)),
            Value::Number(Num::Int(0)),
            Value::Number(Num::Int(0)),
        ]));
        globals.define("player", Value::object(player));

        Interpreter {
            globals: globals.clone(),
            scope: globals,
            handlers: Vec::new(),
            output: Vec::new(),
            host: HashMap::new(),
            call_depth: 0,
            in_function: 0,
        }
    }

    // ------------------------------------------------------------ public API

    pub fn run(&mut self, program: &[Stmt]) -> Result<(), AzraError> {
        self.scope = self.globals.clone();
        match self.exec_block(program)? {
            Flow::Return(_) => Err(AzraError::runtime(
                "`give back` outside of a function",
            )),
            Flow::Normal => Ok(()),
        }
    }

    pub fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[Value]) -> Result<Value, AzraError> + 'static,
    {
        self.host.insert(name.to_string(), Box::new(func));
    }

    pub fn take_output(&mut self) -> Vec<OutputLine> {
        std::mem::take(&mut self.output)
    }

    pub fn get_global(&self, name: &str) -> Option<Value> {
        self.globals.get(name)
    }

    pub fn set_global(&mut self, name: &str, value: Value) {
        self.globals.define(name, value);
    }

    pub fn registered_events(&self) -> Vec<String> {
        self.handlers.iter().map(|h| h.phrase.clone()).collect()
    }

    /// Run every `when` handler whose phrase matches `phrase`.
    pub fn trigger_event(&mut self, phrase: &str) -> Result<(), AzraError> {
        let normalized = normalize_phrase(phrase);
        let bodies: Vec<(Rc<Vec<Stmt>>, Rc<Scope>)> = self
            .handlers
            .iter()
            .filter(|h| h.normalized == normalized)
            .map(|h| (h.body.clone(), h.closure.clone()))
            .collect();
        if bodies.is_empty() {
            let known = self.registered_events();
            let hint = if known.is_empty() {
                "no `when` handlers are registered".to_string()
            } else {
                format!("registered events: {}", known.join(", | "))
            };
            return Err(AzraError::runtime(format!(
                "no event handler matches `{}` ({})",
                phrase, hint
            )));
        }
        let saved = self.scope.clone();
        for (body, closure) in bodies {
            self.scope = closure;
            let flow = self.exec_block(&body);
            self.scope = saved.clone();
            match flow? {
                Flow::Return(_) => {}
                Flow::Normal => {}
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------ statements

    fn exec_block(&mut self, stmts: &[Stmt]) -> Result<Flow, AzraError> {
        for stmt in stmts {
            match self.exec_stmt(stmt)? {
                Flow::Normal => {}
                Flow::Return(v) => return Ok(Flow::Return(v)),
            }
        }
        Ok(Flow::Normal)
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow, AzraError> {
        let line = stmt_line(stmt);
        self.exec_stmt_inner(stmt).map_err(|e| attach_line(e, line))
    }

    fn exec_stmt_inner(&mut self, stmt: &Stmt) -> Result<Flow, AzraError> {
        match stmt {
            Stmt::Set { path, value, line } => {
                let val = if value.len() == 1 {
                    self.eval(&value[0])?
                } else {
                    let mut items = Vec::with_capacity(value.len());
                    for e in value {
                        items.push(self.eval(e)?);
                    }
                    Value::list(items)
                };
                self.assign_path(path, val, *line)?;
                Ok(Flow::Normal)
            }
            Stmt::Make {
                type_name,
                name,
                line,
            } => {
                let obj = Self::mock_object(type_name);
                self.scope.define(name, Value::object(obj));
                let _ = line;
                Ok(Flow::Normal)
            }
            Stmt::Add { value, path, line } => {
                let value = self.eval(value)?;
                // Bootstrap: `add "key" to loot` creates the list when the
                // variable does not exist yet (numbers need `set x to 0`).
                if self.scope.get(&path[0]).is_none() {
                    if !matches!(value, Value::Number(_)) {
                        self.assign_path(path, Value::list(vec![value]), *line)?;
                        return Ok(Flow::Normal);
                    }
                }
                let current = self.eval_path(path, *line)?;
                match current {
                    Value::Null if !matches!(value, Value::Number(_)) => {
                        self.assign_path(path, Value::list(vec![value]), *line)?;
                    }
                    Value::Number(n) => {
                        let v = value.as_number("`add` expects a number", *line)?;
                        let sum = num_add(&n, &v, *line)?;
                        self.assign_path(path, Value::Number(sum), *line)?;
                    }
                    Value::List(items) => {
                        items.borrow_mut().push(value);
                    }
                    other => {
                        return Err(AzraError::runtime_at(
                            format!("`add` expects a number or a list, got {}", other.type_name()),
                            *line,
                        ));
                    }
                }
                Ok(Flow::Normal)
            }
            Stmt::Remove { value, path, line } => {
                let value = self.eval(value)?;
                let current = self.eval_path(path, *line)?;
                match current {
                    Value::Number(n) => {
                        let v = value.as_number("`remove` expects a number", *line)?;
                        let diff = num_sub(&n, &v, *line)?;
                        self.assign_path(path, Value::Number(diff), *line)?;
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
                            *line,
                        ));
                    }
                }
                Ok(Flow::Normal)
            }
            Stmt::Subtract { value, path, line } => {
                let current = self.eval_path(path, *line)?;
                let n = current.as_number("`subtract` expects a number", *line)?;
                let value = self.eval(value)?;
                let v = value.as_number("`subtract` expects a number", *line)?;
                let diff = num_sub(&n, &v, *line)?;
                self.assign_path(path, Value::Number(diff), *line)?;
                Ok(Flow::Normal)
            }
            Stmt::Multiply { path, value, line } => {
                let current = self.eval_path(path, *line)?;
                let n = current.as_number("`multiply` expects a number", *line)?;
                let value = self.eval(value)?;
                let v = value.as_number("`multiply` expects a number", *line)?;
                let product = num_mul(&n, &v, *line)?;
                self.assign_path(path, Value::Number(product), *line)?;
                Ok(Flow::Normal)
            }
            Stmt::Divide { path, value, line } => {
                let current = self.eval_path(path, *line)?;
                let n = current.as_number("`divide` expects a number", *line)?;
                let value = self.eval(value)?;
                let v = value.as_number("`divide` expects a number", *line)?;
                let quotient = num_div(&n, &v, *line)?;
                self.assign_path(path, Value::Number(quotient), *line)?;
                Ok(Flow::Normal)
            }
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
            } => {
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
            Stmt::Repeat { count, body, line } => {
                let n = self.eval(count)?;
                let times = repeat_count(&n, *line)?;
                for _ in 0..times {
                    match self.exec_block(body)? {
                        Flow::Normal => {}
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Normal)
            }
            Stmt::While { cond, body, line } => {
                let mut iterations: u64 = 0;
                while self.eval(cond)?.is_truthy() {
                    iterations += 1;
                    if iterations > MAX_LOOP_ITERATIONS {
                        return Err(AzraError::runtime_at(
                            format!(
                                "`while` loop exceeded the safety cap of {} iterations",
                                MAX_LOOP_ITERATIONS
                            ),
                            *line,
                        ));
                    }
                    match self.exec_block(body)? {
                        Flow::Normal => {}
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Normal)
            }
            Stmt::ForEach {
                var,
                iter,
                body,
                line,
            } => {
                let iterable = self.eval(iter)?;
                let items = match iterable {
                    Value::List(items) => items.clone(),
                    other => {
                        return Err(AzraError::runtime_at(
                            format!(
                                "`for each` expects a list, got {}",
                                other.type_name()
                            ),
                            *line,
                        ));
                    }
                };
                let saved = self.scope.clone();
                self.scope = Scope::new(&saved);
                let snapshot = items.borrow().clone();
                let mut result = Ok(Flow::Normal);
                for item in snapshot.iter() {
                    self.scope.define(var, item.clone());
                    match self.exec_block(body) {
                        Ok(Flow::Normal) => {}
                        Ok(Flow::Return(v)) => {
                            result = Ok(Flow::Return(v));
                            break;
                        }
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    }
                }
                self.scope = saved;
                result
            }
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
            Stmt::When { phrase, body, .. } => {
                self.handlers.push(EventHandler {
                    phrase: phrase.clone(),
                    normalized: normalize_phrase(phrase),
                    body: Rc::new(body.clone()),
                    closure: self.scope.clone(),
                });
                Ok(Flow::Normal)
            }
            Stmt::Call { name, args, line } => {
                // Engine verbs accept bare names as arguments
                // (`spawn zombie` — `zombie` need not be defined).
                let lenient = ENGINE_VERBS.contains(&name.as_str());
                let mut values = Vec::with_capacity(args.len());
                for e in args {
                    values.push(self.eval_arg(e, lenient)?);
                }
                self.call(name, values, *line)?;
                Ok(Flow::Normal)
            }
        }
    }

    // ------------------------------------------------------------- functions

    /// Evaluate a call argument; engine verbs treat an undefined bare name
    /// as the string value of that name (`spawn zombie`).
    fn eval_arg(&mut self, expr: &Expr, lenient: bool) -> Result<Value, AzraError> {
        if lenient {
            if let Expr::Var { name, line: _ } = expr {
                if self.scope.get(name).is_none() {
                    return Ok(Value::Str(name.clone()));
                }
            }
        }
        self.eval(expr)
    }

    fn call(&mut self, name: &str, args: Vec<Value>, line: usize) -> Result<Value, AzraError> {
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

    fn call_function(
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
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(AzraError::runtime_at(
                format!("call stack overflow (max depth {})", MAX_CALL_DEPTH),
                line,
            ));
        }
        let saved_scope = self.scope.clone();
        let saved_in_function = self.in_function;
        let scope = Scope::new(&func.closure);
        for (param, arg) in func.params.iter().zip(args.into_iter()) {
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
        }
    }

    // ------------------------------------------------------------- expressions

    fn eval(&mut self, expr: &Expr) -> Result<Value, AzraError> {
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
                match &obj {
                    Value::Object(data) => data
                        .borrow()
                        .get(name)
                        .cloned()
                        .ok_or_else(|| {
                            let type_name = data.borrow().type_name.clone();
                            AzraError::runtime_at(
                                format!("object of type `{}` has no property `{}`", type_name, name),
                                *line,
                            )
                        }),
                    other => Err(AzraError::runtime_at(
                        format!(
                            "cannot read property `{}` of {} (only objects have properties)",
                            name,
                            other.type_name()
                        ),
                        *line,
                    )),
                }
            }
            Expr::Call { name, args, line } => {
                let mut values = Vec::with_capacity(args.len());
                for e in args {
                    values.push(self.eval(e)?);
                }
                self.call(name, values, *line)
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
                // A string operand on either side concatenates (rendered).
                if matches!(l, Value::Str(_)) || matches!(r, Value::Str(_)) {
                    return Ok(Value::Str(format!("{}{}", l.render(), r.render())));
                }
                match (&l, &r) {
                    (Value::Number(a), Value::Number(b)) => {
                        Ok(Value::Number(num_add(a, b, line)?))
                    }
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
                let a = l.as_number("operator `-` expects numbers", line)?;
                let b = r.as_number("operator `-` expects numbers", line)?;
                Ok(Value::Number(num_sub(&a, &b, line)?))
            }
            BinOp::Mul => {
                let a = l.as_number("operator `*` expects numbers", line)?;
                let b = r.as_number("operator `*` expects numbers", line)?;
                Ok(Value::Number(num_mul(&a, &b, line)?))
            }
            BinOp::Div => {
                let a = l.as_number("operator `/` expects numbers", line)?;
                let b = r.as_number("operator `/` expects numbers", line)?;
                Ok(Value::Number(num_div(&a, &b, line)?))
            }
            BinOp::And | BinOp::Or => unreachable!("handled above"),
        }
    }

    // ----------------------------------------------------------------- paths

    fn eval_path(&mut self, path: &[String], line: usize) -> Result<Value, AzraError> {
        let mut current = self.scope.get(&path[0]).ok_or_else(|| {
            AzraError::runtime_at(format!("undefined variable `{}`", path[0]), line)
        })?;
        for seg in &path[1..] {
            current = match &current {
                Value::Object(data) => {
                    let data = data.clone();
                    let found = data.borrow().get(seg).cloned();
                    match found {
                        Some(v) => v,
                        None => {
                            let type_name = data.borrow().type_name.clone();
                            return Err(AzraError::runtime_at(
                                format!(
                                    "object of type `{}` has no property `{}`",
                                    type_name, seg
                                ),
                                line,
                            ));
                        }
                    }
                }
                other => {
                    return Err(AzraError::runtime_at(
                        format!(
                            "cannot access property `{}` of {} (only objects have properties)",
                            seg,
                            other.type_name()
                        ),
                        line,
                    ))
                }
            };
        }
        Ok(current)
    }

    fn assign_path(&mut self, path: &[String], value: Value, line: usize) -> Result<(), AzraError> {
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
                Value::Object(data) => data
                    .borrow()
                    .get(seg)
                    .cloned()
                    .ok_or_else(|| {
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
                    ))
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

    // ------------------------------------------------------------ mock engine

    /// Create a default instance of a registered engine type.
    fn mock_object(type_name: &str) -> ObjectData {
        let mut obj = ObjectData::new(type_name);
        let int = |i: i64| Value::Number(Num::Int(i));
        match type_name {
            "rounded_box" => {
                obj.set("color", Value::Str("white".to_string()));
                obj.set("material", Value::Str("default".to_string()));
                obj.set("position", Value::list(vec![int(0), int(0), int(0)]));
                obj.set("size", Value::list(vec![int(1), int(1), int(1)]));
            }
            "sound" => {
                obj.set("file", Value::Str(String::new()));
                obj.set("volume", Value::Number(Num::Float(1.0)));
            }
            "player" => {
                obj.set("name", Value::Str("player".to_string()));
                obj.set("health", int(100));
                obj.set("position", Value::list(vec![int(0), int(0), int(0)]));
            }
            _ => {}
        }
        obj
    }
}

// ------------------------------------------------------------------ helpers

/// Mock engine verbs understood in standalone CLI mode.
const ENGINE_VERBS: &[&str] = &[
    "play", "destroy", "jump", "spawn", "move", "rotate", "wait",
];

/// Normalize an event phrase for matching: lowercase, collapse whitespace,
/// drop a trailing period and ignore quote style (`key "space"` == `key space`).
fn normalize_phrase(phrase: &str) -> String {
    let lower = phrase.to_lowercase();
    let collapsed = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_end_matches('.');
    trimmed.replace(['"', '\''], "")
}

fn stmt_line(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Set { line, .. }
        | Stmt::Make { line, .. }
        | Stmt::Add { line, .. }
        | Stmt::Remove { line, .. }
        | Stmt::Subtract { line, .. }
        | Stmt::Multiply { line, .. }
        | Stmt::Divide { line, .. }
        | Stmt::Output { line, .. }
        | Stmt::If { line, .. }
        | Stmt::Repeat { line, .. }
        | Stmt::ForEach { line, .. }
        | Stmt::While { line, .. }
        | Stmt::Define { line, .. }
        | Stmt::Return { line, .. }
        | Stmt::When { line, .. }
        | Stmt::Call { line, .. } => *line,
    }
}

/// Attach a line number to errors that lack one (literals have no position).
fn attach_line(err: AzraError, line: usize) -> AzraError {
    if err.line.is_none() && line > 0 {
        AzraError {
            line: Some(line),
            ..err
        }
    } else {
        err
    }
}

fn repeat_count(value: &Value, line: usize) -> Result<u64, AzraError> {
    match value {
        Value::Number(Num::Int(i)) => {
            if *i < 0 {
                Err(AzraError::runtime_at(
                    "repeat count must not be negative",
                    line,
                ))
            } else if *i as u64 > MAX_LOOP_ITERATIONS {
                Err(AzraError::runtime_at(
                    format!("repeat count exceeds the safety cap of {}", MAX_LOOP_ITERATIONS),
                    line,
                ))
            } else {
                Ok(*i as u64)
            }
        }
        Value::Number(Num::Float(f)) => {
            if *f < 0.0 {
                Err(AzraError::runtime_at(
                    "repeat count must not be negative",
                    line,
                ))
            } else if f.fract() != 0.0 {
                Err(AzraError::runtime_at(
                    format!("repeat count must be a whole number, got {}", format_float(*f)),
                    line,
                ))
            } else if *f > MAX_LOOP_ITERATIONS as f64 {
                Err(AzraError::runtime_at(
                    format!("repeat count exceeds the safety cap of {}", MAX_LOOP_ITERATIONS),
                    line,
                ))
            } else {
                Ok(*f as u64)
            }
        }
        other => Err(AzraError::runtime_at(
            format!("repeat count must be a number, got {}", other.type_name()),
            line,
        )),
    }
}

fn compare_values(l: &Value, r: &Value, line: usize) -> Result<std::cmp::Ordering, AzraError> {
    match (l, r) {
        (Value::Number(a), Value::Number(b)) => Ok(a
            .as_f64()
            .partial_cmp(&b.as_f64())
            .unwrap_or(std::cmp::Ordering::Equal)),
        (Value::Str(a), Value::Str(b)) => Ok(a.cmp(b)),
        _ => Err(AzraError::runtime_at(
            format!(
                "cannot compare {} with {} (numbers or strings only)",
                l.type_name(),
                r.type_name()
            ),
            line,
        )),
    }
}

fn num_add(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x
            .checked_add(*y)
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in addition", line)),
        _ => Ok(Num::Float(a.as_f64() + b.as_f64())),
    }
}

fn num_sub(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x
            .checked_sub(*y)
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in subtraction", line)),
        _ => Ok(Num::Float(a.as_f64() - b.as_f64())),
    }
}

fn num_mul(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => x
            .checked_mul(*y)
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in multiplication", line)),
        _ => Ok(Num::Float(a.as_f64() * b.as_f64())),
    }
}

fn num_div(a: &Num, b: &Num, line: usize) -> Result<Num, AzraError> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => {
            if *y == 0 {
                return Err(AzraError::runtime_at("division by zero", line));
            }
            x.checked_div(*y)
                .map(Num::Int)
                .ok_or_else(|| AzraError::runtime_at("integer overflow in division", line))
        }
        _ => {
            if b.as_f64() == 0.0 {
                return Err(AzraError::runtime_at("division by zero", line));
            }
            Ok(Num::Float(a.as_f64() / b.as_f64()))
        }
    }
}

fn num_neg(a: &Num, line: usize) -> Result<Num, AzraError> {
    match a {
        Num::Int(i) => i
            .checked_neg()
            .map(Num::Int)
            .ok_or_else(|| AzraError::runtime_at("integer overflow in negation", line)),
        Num::Float(f) => Ok(Num::Float(-f)),
    }
}
