//! Statement AST nodes and conditional branches.

use super::{Expr, OutputLevel, Path, Target};

/// One `if` / `otherwise if` branch.
#[derive(Debug, Clone)]
pub struct Branch {
    pub cond: Expr,
    pub body: Vec<Stmt>,
}

/// A single statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// `set <target> to <expr>, ...`
    Set {
        target: Target,
        value: Vec<Expr>,
        line: usize,
    },
    /// `make <type> as <name>`
    Make {
        type_name: String,
        name: String,
        line: usize,
    },
    /// `add <expr> to <target>` (numbers) or append to a list
    Add {
        value: Expr,
        target: Target,
        line: usize,
    },
    /// `remove <expr> from <target>` (numbers) or remove from a list
    Remove {
        value: Expr,
        target: Target,
        line: usize,
    },
    /// `remove item <index> from <target>`
    RemoveAt {
        index: Expr,
        target: Target,
        line: usize,
    },
    /// `remove key <key> from <target>`
    RemoveKey {
        key: Expr,
        target: Target,
        line: usize,
    },
    /// `subtract <expr> from <target>`
    Subtract {
        value: Expr,
        target: Target,
        line: usize,
    },
    /// `multiply <target> by <expr>`
    Multiply {
        target: Target,
        value: Expr,
        line: usize,
    },
    /// `divide <target> by <expr>`
    Divide {
        target: Target,
        value: Expr,
        line: usize,
    },
    /// `clear <target>` — empty a list or map in place
    Clear { target: Target, line: usize },
    /// `sort <target>` — sort a list of numbers or strings in place
    Sort { target: Target, line: usize },
    /// `reverse <target>` — reverse a list in place
    Reverse { target: Target, line: usize },
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
    /// `repeat with i from 1 to 10 [by 2]:`
    RepeatRange {
        var: String,
        from: Expr,
        to: Expr,
        step: Option<Expr>,
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
    /// `stop loop` / `break`
    Break { line: usize },
    /// `skip next` / `continue`
    Continue { line: usize },
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
    /// `when <event phrase> [with p1, p2]: ...` — engine event handler
    When {
        phrase: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `<func> with <a>, <b>` / `weapons.fire with 10` / engine verb
    Call {
        path: Path,
        args: Vec<Expr>,
        line: usize,
    },
    /// A stand-alone built-in expression (`round 4.6`), value discarded.
    ExprStmt { expr: Expr, line: usize },
    /// `try: ... if error as err: ...`
    Try {
        body: Vec<Stmt>,
        err_name: String,
        handler: Vec<Stmt>,
        line: usize,
    },
    /// `fail with <message>` — raise a runtime error
    Fail { message: Expr, line: usize },
    /// `use "module.vibe" [as name]`
    Use {
        path: String,
        alias: Option<String>,
        line: usize,
    },
    /// `every <seconds> seconds: ...` — recurring timer
    Every {
        interval: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `after <seconds> seconds: ...` — one-shot timer
    After {
        delay: Expr,
        body: Vec<Stmt>,
        line: usize,
    },
    /// `wait <seconds> [seconds]`
    Wait { duration: Expr, line: usize },
    /// `write <data> to file <path>`
    WriteFile {
        data: Expr,
        path: Expr,
        line: usize,
    },
}
