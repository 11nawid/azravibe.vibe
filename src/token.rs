//! Token definitions for the Azra (`.vibe`) language.

use std::fmt;

/// A numeric literal: integers stay integers so `say` prints `100`, floats
/// keep fractional precision (`12.5`).
#[derive(Debug, Clone, PartialEq)]
pub enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    pub fn as_f64(&self) -> f64 {
        match self {
            Num::Int(i) => *i as f64,
            Num::Float(f) => *f,
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Num::Int(i) => *i < 0,
            Num::Float(f) => *f < 0.0,
        }
    }
}

impl fmt::Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Num::Int(i) => write!(f, "{}", i),
            Num::Float(v) => write!(f, "{}", crate::value::format_float(*v)),
        }
    }
}

/// Every token the Azra lexer can produce.
///
/// English keywords (`set`, `if`, `equals`, ...) are lexed as `Word` and
/// recognized by the parser. The comparison operators `less_than=` and
/// `greater_than=` contain an `=`, so they are single dedicated tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Word(String),
    Number(Num),
    Str(String),
    Plus,
    Minus,
    Star,
    Slash,
    Comma,
    Colon,
    Dot,
    /// `less_than=` (single token, no spaces allowed)
    LessThanEq,
    /// `greater_than=` (single token, no spaces allowed)
    GreaterThanEq,
    Newline,
    Indent,
    Dedent,
    Eof,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Word(w) => write!(f, "`{}`", w),
            TokenKind::Number(n) => write!(f, "`{}`", n),
            TokenKind::Str(s) => write!(f, "`\"{}\"`", s),
            TokenKind::Plus => write!(f, "`+`"),
            TokenKind::Minus => write!(f, "`-`"),
            TokenKind::Star => write!(f, "`*`"),
            TokenKind::Slash => write!(f, "`/`"),
            TokenKind::Comma => write!(f, "`,`"),
            TokenKind::Colon => write!(f, "`:`"),
            TokenKind::Dot => write!(f, "`.`"),
            TokenKind::LessThanEq => write!(f, "`less_than=`"),
            TokenKind::GreaterThanEq => write!(f, "`greater_than=`"),
            TokenKind::Newline => write!(f, "end of line"),
            TokenKind::Indent => write!(f, "start of an indented block"),
            TokenKind::Dedent => write!(f, "end of an indented block"),
            TokenKind::Eof => write!(f, "end of input"),
        }
    }
}

/// A token with its 1-based source position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Token { kind, line, column }
    }
}
