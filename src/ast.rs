//! Abstract syntax tree for Azra programs.

use crate::token::Num;

/// A dotted name such as `player.health` or `lava_block.position`.
pub type Path = Vec<String>;

/// Output level of a printing statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputLevel {
    /// `say` — plain stdout
    Say,
    /// `warn` — yellow `[WARN] ...`
    Warn,
    /// `error` — red `[ERROR] ...`
    Error,
}

/// One `if` / `otherwise if` branch.
#[derive(Debug, Clone)]
pub struct Branch {
    pub cond: Expr,
    pub body: Vec<Stmt>,
}

/// A single statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// `set <path> to <expr>, ...`
    Set {
        path: Path,
        value: Vec<Expr>,
        line: usize,
    },
    /// `make <type> as <name>`
    Make {
        type_name: String,
        name: String,
        line: usize,
    },
    /// `add <expr> to <path>` (numbers) or append to a list
    Add {
        value: Expr,
        path: Path,
        line: usize,
    },
    /// `remove <expr> from <path>` (numbers) or remove from a list
    Remove {
        value: Expr,
        path: Path,
        line: usize,
    },
    /// `subtract <expr> from <path>`
    Subtract {
        value: Expr,
        path: Path,
        line: usize,
    },
    /// `multiply <path> by <expr>`
    Multiply {
        path: Path,
        value: Expr,
        line: usize,
    },
    /// `divide <path> by <expr>`
    Divide {
        path: Path,
        value: Expr,
        line: usize,
    },
    /// `say` / `warn` / `error`
    Output {
        level: OutputLevel,
        args: Vec<Expr>,
        line: usize,
    },
    /// `if ...: ... otherwise if ...: ... otherwise: ...`
    If {
        branches: Vec<Branch>,
        otherwise: Option<Vec<Stmt>>,
        line: usize,
    },
    /// `repeat <expr> times:`
    Repeat {
        count: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `for each <name> in <expr>:`
    ForEach {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `while <cond>:`
    While {
        cond: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `define <name> with <p1>, <p2>: ...`
    Define {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `give back <expr>`
    Return {
        value: Option<Expr>,
        line: usize,
    },
    /// `when <event phrase>: ...` — registered engine event handler
    When {
        phrase: String,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `<func> with <a>, <b>` or engine verb `play boom_sfx`
    Call {
        name: String,
        args: Vec<Expr>,
        line: usize,
    },
}

/// Binary operators (English keywords and math symbols).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Eq => "equals",
            BinOp::Ne => "not_equals",
            BinOp::Lt => "less_than",
            BinOp::Le => "less_than=",
            BinOp::Gt => "greater_than",
            BinOp::Ge => "greater_than=",
            BinOp::And => "and",
            BinOp::Or => "or",
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
    /// `func with a, b` (no parentheses)
    Call {
        name: String,
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
            | Expr::Not { line, .. }
            | Expr::Negate { line, .. }
            | Expr::Binary { line, .. } => *line,
        }
    }
}
