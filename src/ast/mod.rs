//! Abstract syntax tree for Azra programs.

mod expr;
mod stmt;

pub use expr::{BinOp, Builtin, Expr, InterpPart};
pub use stmt::{Branch, Stmt};

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

/// An assignment target: a plain path, a map key, or a list index.
///
/// `set player.health to 100`        → `Path(["player", "health"])`
/// `set key "hp" of stats to 15`     → `Key { key: "hp", map: Path(["stats"]) }`
/// `set item 1 of items to "x"`      → `Item { index: 1, list: Path(["items"]) }`
#[derive(Debug, Clone)]
pub enum Target {
    Path { path: Path, line: usize },
    /// `key <expr> of <target>` / `<map>'s <key>`
    Key {
        key: Expr,
        map: Box<Target>,
        line: usize,
    },
    /// `item <index> of <target>` (1-based)
    Item {
        index: Expr,
        list: Box<Target>,
        line: usize,
    },
    /// `first item of <target>`
    First {
        list: Box<Target>,
        line: usize,
    },
    /// `last item of <target>`
    Last {
        list: Box<Target>,
        line: usize,
    },
}

impl Target {
    pub fn line(&self) -> usize {
        match self {
            Target::Path { line, .. }
            | Target::Key { line, .. }
            | Target::Item { line, .. }
            | Target::First { line, .. }
            | Target::Last { line, .. } => *line,
        }
    }
}
