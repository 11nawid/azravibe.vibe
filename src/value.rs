//! Runtime values for the Azra VM.

use crate::ast::Stmt;
use crate::env::Scope;
use crate::error::AzraError;
use crate::token::Num;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// An object/map instance created with `make TYPE as NAME` (or a module).
#[derive(Debug, Clone)]
pub struct ObjectData {
    /// Unique id assigned by the engine (also readable as `<obj>.id`).
    pub id: u64,
    /// Registered engine type (`rounded_box`, `sound`, `player`, `module`, ...).
    pub type_name: String,
    /// Insertion-ordered fields.
    pub fields: Vec<(String, Value)>,
}

impl ObjectData {
    pub fn new(type_name: impl Into<String>) -> Self {
        ObjectData {
            id: 0,
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

/// An insertion-ordered map/dictionary created with `set m to empty map`.
#[derive(Debug, Clone, Default)]
pub struct MapData {
    pub entries: Vec<(String, Value)>,
}

impl MapData {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key.to_string(), value));
        }
    }

    pub fn remove(&mut self, key: &str) -> bool {
        if let Some(idx) = self.entries.iter().position(|(k, _)| k == key) {
            self.entries.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn keys(&self) -> Vec<Value> {
        self.entries.iter().map(|(k, _)| Value::Str(k.clone())).collect()
    }
}

/// A 3D vector created with `set pos to vec3 0, 5, 10`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3Data {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3Data {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3Data { x, y, z }
    }

    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance_to(&self, other: &Vec3Data) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// An RGB color created with `color 255, 100, 50` or `color "red"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorData {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ColorData {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        ColorData { r, g, b }
    }

    /// Look up a built-in named color (`"red"`, `"blue"`, ...).
    pub fn from_name(name: &str) -> Option<ColorData> {
        let c = match name.to_lowercase().as_str() {
            "black" => (0, 0, 0),
            "white" => (255, 255, 255),
            "red" => (255, 0, 0),
            "green" | "lime" => (0, 255, 0),
            "blue" => (0, 0, 255),
            "yellow" => (255, 255, 0),
            "orange" => (255, 165, 0),
            "purple" => (128, 0, 128),
            "pink" => (255, 192, 203),
            "cyan" => (0, 255, 255),
            "magenta" => (255, 0, 255),
            "gray" | "grey" => (128, 128, 128),
            "brown" => (139, 69, 19),
            "teal" => (0, 128, 128),
            "navy" => (0, 0, 128),
            "gold" => (255, 215, 0),
            "silver" => (192, 192, 192),
            "maroon" => (128, 0, 0),
            "olive" => (128, 128, 0),
            _ => return None,
        };
        Some(ColorData::new(c.0, c.1, c.2))
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
    Map(Rc<RefCell<MapData>>),
    Object(Rc<RefCell<ObjectData>>),
    Vec3(Vec3Data),
    Color(ColorData),
    Function(Rc<FunctionValue>),
    Null,
}

impl Value {
    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn map(entries: Vec<(String, Value)>) -> Value {
        Value::Map(Rc::new(RefCell::new(MapData { entries })))
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
            Value::Map(entries) => !entries.borrow().entries.is_empty(),
            Value::Object(_) | Value::Function(_) | Value::Vec3(_) | Value::Color(_) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Object(_) => "object",
            Value::Vec3(_) => "vec3",
            Value::Color(_) => "color",
            Value::Function(_) => "function",
            Value::Null => "null",
        }
    }

    /// Structural equality for `equals` / `is`.
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
            (Value::Map(a), Value::Map(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.entries.len() == b.entries.len()
                    && a.entries.iter().all(|(k, v)| {
                        b.get(k).is_some_and(|other| v.equals_depth(other, depth + 1))
                    })
            }
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Vec3(a), Value::Vec3(b)) => a == b,
            (Value::Color(a), Value::Color(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }

    /// Human-readable rendering used by `say` / `warn` / `error`.
    /// Recursion is depth-capped so cyclic containers cannot overflow the stack.
    pub fn render(&self) -> String {
        self.render_depth(0)
    }

    fn render_depth(&self, depth: u32) -> String {
        if depth > 32 {
            return "<...>".to_string();
        }
        match self {
            Value::Number(n) => n.to_string(),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::List(items) => {
                let items = items.borrow();
                let parts: Vec<String> =
                    items.iter().map(|v| v.render_depth(depth + 1)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Map(entries) => {
                let entries = entries.borrow();
                let parts: Vec<String> = entries
                    .entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, v.render_depth(depth + 1)))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            Value::Object(obj) => {
                let obj = obj.borrow();
                if obj.fields.is_empty() {
                    format!("<{}>", obj.type_name)
                } else {
                    let parts: Vec<String> = obj
                        .fields
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v.render_depth(depth + 1)))
                        .collect();
                    format!("<{} {}>", obj.type_name, parts.join(" "))
                }
            }
            Value::Vec3(v) => format!(
                "vec3 {}, {}, {}",
                crate::value::format_float(v.x),
                crate::value::format_float(v.y),
                crate::value::format_float(v.z)
            ),
            Value::Color(c) => format!("color {}, {}, {}", c.r, c.g, c.b),
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

    /// Require a string, with a friendly runtime error otherwise.
    pub fn as_str(&self, what: &str, line: usize) -> Result<String, AzraError> {
        match self {
            Value::Str(s) => Ok(s.clone()),
            other => Err(AzraError::runtime_at(
                format!("{} must be a string, got {}", what, other.type_name()),
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
