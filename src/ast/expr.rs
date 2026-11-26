//! Expression AST nodes, binary operators, and built-in expression forms.

use super::Path;
use crate::token::Num;

/// One piece of an interpolated string literal.
#[derive(Debug, Clone)]
pub enum InterpPart {
    Text(String),
    /// `{player.health}` — evaluated and rendered at runtime.
    Path { path: Path, line: usize },
}

/// Built-in expression forms (`round 4.6`, `length of items`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Vec3,
    Color,
    ItemOf,
    FirstItemOf,
    LastItemOf,
    LengthOf,
    KeysOf,
    KeyOf,
    Distance,
    Normalize,
    Round,
    Floor,
    Ceil,
    Abs,
    Sqrt,
    Sin,
    Cos,
    Min,
    Max,
    Clamp,
    RandomNumber,
    RandomDecimal,
    Uppercase,
    Lowercase,
    Trim,
    Split,
    Join,
    Replace,
    CurrentTime,
    FormatTime,
    ReadFile,
    FileExists,
    ParseJson,
    ToJson,
    EmptyList,
    EmptyMap,
}

impl Builtin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Builtin::Vec3 => "vec3",
            Builtin::Color => "color",
            Builtin::ItemOf => "item ... of ...",
            Builtin::FirstItemOf => "first item of ...",
            Builtin::LastItemOf => "last item of ...",
            Builtin::LengthOf => "length of ...",
            Builtin::KeysOf => "keys of ...",
            Builtin::KeyOf => "key ... of ...",
            Builtin::Distance => "distance between ... and ...",
            Builtin::Normalize => "normalize ...",
            Builtin::Round => "round ...",
            Builtin::Floor => "floor ...",
            Builtin::Ceil => "ceil ...",
            Builtin::Abs => "abs ...",
            Builtin::Sqrt => "sqrt ...",
            Builtin::Sin => "sin ...",
            Builtin::Cos => "cos ...",
            Builtin::Min => "min of ..., ...",
            Builtin::Max => "max of ..., ...",
            Builtin::Clamp => "clamp ... between ... and ...",
            Builtin::RandomNumber => "random number from ... to ...",
            Builtin::RandomDecimal => "random decimal",
            Builtin::Uppercase => "uppercase of ...",
            Builtin::Lowercase => "lowercase of ...",
            Builtin::Trim => "trim of ...",
            Builtin::Split => "split ... by ...",
            Builtin::Join => "join ... with ...",
            Builtin::Replace => "replace ... with ... in ...",
            Builtin::CurrentTime => "current time",
            Builtin::FormatTime => "format time ...",
            Builtin::ReadFile => "read file ...",
            Builtin::FileExists => "file exists ...",
            Builtin::ParseJson => "parse json ...",
            Builtin::ToJson => "to json ...",
            Builtin::EmptyList => "empty list",
            Builtin::EmptyMap => "empty map",
        }
    }
}

/// Binary operators (English keywords and math symbols).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    /// `haystack contains needle`
    Contains,
    /// `text starts with prefix`
    StartsWith,
    /// `map has key "k"`
    HasKey,
}

impl BinOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "modulo",
            BinOp::Pow => "power",
            BinOp::Eq => "equals",
            BinOp::Ne => "not_equals",
            BinOp::Lt => "less_than",
            BinOp::Le => "less_than=",
            BinOp::Gt => "greater_than",
            BinOp::Ge => "greater_than=",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Contains => "contains",
            BinOp::StartsWith => "starts with",
            BinOp::HasKey => "has key",
        }
    }
}

/// A single expression.
#[derive(Debug, Clone)]
pub enum Expr {
    Number(Num),
    Str(String),
    Bool(bool),
    Null,
    /// Variable reference
    Var { name: String, line: usize },
    /// `obj.prop`
    Member {
        object: Box<Expr>,
        name: String,
        line: usize,
    },
    /// `func with a, b` / `weapons.fire with 10` (no parentheses)
    Call {
        path: Path,
        args: Vec<Expr>,
        line: usize,
    },
    /// `"Hello {name}, HP: {health}"` — string interpolation
    Interp {
        parts: Vec<InterpPart>,
        line: usize,
    },
    /// A built-in expression form (`round 4.6`, `length of items`, ...)
    Builtin {
        name: Builtin,
        args: Vec<Expr>,
        line: usize,
    },
    /// `not expr`
    Not { expr: Box<Expr>, line: usize },
    /// `-expr`
    Negate { expr: Box<Expr>, line: usize },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        line: usize,
    },
}

impl Expr {
    /// Source line used for runtime error reporting.
    pub fn line(&self) -> usize {
        match self {
            Expr::Number(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Null => 0,
            Expr::Var { line, .. }
            | Expr::Member { line, .. }
            | Expr::Call { line, .. }
            | Expr::Interp { line, .. }
            | Expr::Builtin { line, .. }
            | Expr::Not { line, .. }
            | Expr::Negate { line, .. }
            | Expr::Binary { line, .. } => *line,
        }
    }
}
