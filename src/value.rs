//! Runtime values for the Azra VM.

use crate::ast::Stmt;
use crate::error::AzraError;
use crate::interpreter::Scope;
use crate::token::Num;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// An object/map instance created with `make TYPE as NAME`.
#[derive(Debug, Clone)]
pub struct ObjectData {
    /// Registered engine type (`rounded_box`, `sound`, `player`, ...).
    pub type_name: String,
    /// Insertion-ordered fields.
    pub fields: Vec<(String, Value)>,
}

impl ObjectData {
    pub fn new(type_name: impl Into<String>) -> Self {
        ObjectData {
            type_name: type_name.into(),
            fields: Vec::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn set(&mut self, name: &str, value: Value) {
        if let Some(slot) = self.fields.iter_mut().find(|(k, _)| k == name) {
            slot.1 = value;
        } else {
            self.fields.push((name.to_string(), value));
        }
    }
}

/// A user-defined Azra function (`define ...`).
#[derive(Debug)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<String>,
    pub body: Rc<Vec<Stmt>>,
    /// Defining scope (also makes recursion work).
    pub closure: Rc<Scope>,
}

/// Every runtime value of the Azra language.
#[derive(Debug, Clone)]
pub enum Value {
    Number(Num),
    Str(String),
    Bool(bool),
    List(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<ObjectData>>),
    Function(Rc<FunctionValue>),
    Null,
}

impl Value {
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn object(data: ObjectData) -> Value {
        Value::Object(Rc::new(RefCell::new(data)))
    }

    /// Python-style truthiness used by `if` / `while` / `not`.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(Num::Int(i)) => *i != 0,
            Value::Number(Num::Float(f)) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(items) => !items.borrow().is_empty(),
            Value::Object(_) | Value::Function(_) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::List(_) => "list",
            Value::Object(_) => "object",
            Value::Function(_) => "function",
            Value::Null => "null",
        }
    }

    /// Structural equality for `equals` / `not_equals`.
    /// Objects compare by identity; recursion is depth-capped for safety.
    pub fn equals(&self, other: &Value) -> bool {
        self.equals_depth(other, 0)
    }

    fn equals_depth(&self, other: &Value, depth: u32) -> bool {
        if depth > 32 {
            return false;
        }
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => match (a, b) {
                (Num::Int(x), Num::Int(y)) => x == y,
                _ => a.as_f64() == b.as_f64(),
            },
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::List(a), Value::List(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|(x, y)| x.equals_depth(y, depth + 1))
            }
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// Human-readable rendering used by `say` / `warn` / `error`.
    pub fn render(&self) -> String {
        match self {
            Value::Number(n) => n.to_string(),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::List(items) => {
                let items = items.borrow();
                let parts: Vec<String> = items.iter().map(|v| v.render()).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Object(obj) => {
                let obj = obj.borrow();
                if obj.fields.is_empty() {
                    format!("<{}>", obj.type_name)
                } else {
                    let parts: Vec<String> = obj
                        .fields
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v.render()))
                        .collect();
                    format!("<{} {}>", obj.type_name, parts.join(" "))
                }
            }
            Value::Function(f) => format!("<function {}>", f.name),
        }
    }

    /// Require a number, with a friendly runtime error otherwise.
    pub fn as_number(&self, what: &str, line: usize) -> Result<Num, AzraError> {
        match self {
            Value::Number(n) => Ok(n.clone()),
            other => Err(AzraError::runtime_at(
                format!("{} must be a number, got {}", what, other.type_name()),
                line,
            )),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Format a float so whole values print as `12` and fractions as `12.5`.
pub fn format_float(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{}", f)
    }
}
