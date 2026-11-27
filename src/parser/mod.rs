//! Recursive descent parser for Azra with helpful, line-numbered errors.
//!
//! Ambiguous forms (`round` as a variable vs. the `round ...` built-in,
//! `key ... of ...` vs. a variable named `key`, ...) are resolved with small
//! backtracking attempts: a failed attempt restores the token position and
//! falls back to the plain reading.

mod blocks;
mod builtin;
mod expr;
mod stmt;
mod target;

use crate::ast::{Expr, InterpPart, Path, Stmt};
use crate::error::AzraError;
use crate::token::{StrPart, Token, TokenKind};

/// Words that can never start a statement (operators and structural glue).
/// They produce a clear parse error instead of "undefined function".
pub(super) const STATEMENT_KEYWORDS: &[&str] = &[
    "and",
    "or",
    "not",
    "equals",
    "not_equals",
    "less_than",
    "greater_than",
    "is",
    "contains",
    "starts",
    "has",
    "of",
    "from",
    "by",
    "with",
    "in",
    "times",
    "as",
    "each",
    "between",
    "true",
    "false",
    "null",
    "nothing",
    "otherwise",
    "back",
    "seconds",
    "test",
    "assert",
];

/// Words that open a built-in expression form (`round 4.6`, `length of x`, ...).
/// In expression position a failed attempt falls back to a variable read.
pub(super) const BUILTIN_WORDS: &[&str] = &[
    "vec3",
    "color",
    "item",
    "last",
    "first",
    "length",
    "keys",
    "key",
    "distance",
    "normalize",
    "round",
    "floor",
    "ceil",
    "abs",
    "sqrt",
    "sin",
    "cos",
    "min",
    "max",
    "clamp",
    "random",
    "uppercase",
    "lowercase",
    "trim",
    "split",
    "join",
    "replace",
    "current",
    "format",
    "read",
    "file",
    "parse",
    "to",
    "empty",
    // Advanced mathematics / 3D geometry.
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "pi",
    "e",
    "log",
    "log10",
    "exp",
    "lerp",
    "degrees",
    "radians",
    "dot",
    "cross",
    "magnitude",
    "angle",
];

const MAX_PARSE_DEPTH: usize = 128;

/// Parse a token stream into a program (list of statements).
pub fn parse(tokens: &[Token]) -> Result<Vec<Stmt>, AzraError> {
    if tokens.is_empty() {
        return Err(AzraError::syntax("empty token stream"));
    }
    Parser {
        tokens: tokens.to_vec(),
        pos: 0,
        allow_with: true,
        depth: 0,
    }
    .parse_program()
}

pub(super) struct Parser {
    pub(super) tokens: Vec<Token>,
    pub(super) pos: usize,
    /// Whether `name with args` inside an expression is a function call.
    /// Disabled while parsing statement-call arguments and the argument of
    /// built-ins whose grammar owns the following `with` (`join a with b`,
    /// `replace a with b in c`), where `with` acts as a separator.
    pub(super) allow_with: bool,
    pub(super) depth: usize,
}

impl Parser {
    pub(super) fn with_depth<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, AzraError>,
    ) -> Result<T, AzraError> {
        if self.depth >= MAX_PARSE_DEPTH {
            return Err(self.error_here(format!(
                "maximum nesting depth ({}) exceeded",
                MAX_PARSE_DEPTH
            )));
        }
        self.depth += 1;
        let result = f(self);
        self.depth -= 1;
        result
    }
    // ---------------------------------------------------------------- helpers

    pub(super) fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream ends with Eof"))
    }

    /// Kind of the token `offset` positions ahead (clamped to `Eof`).
    pub(super) fn kind_at(&self, offset: usize) -> &TokenKind {
        self.tokens
            .get(self.pos + offset)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    pub(super) fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    pub(super) fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    pub(super) fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    pub(super) fn check_word(&self, word: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Word(w) if w == word)
    }

    pub(super) fn check_word_at(&self, offset: usize, word: &str) -> bool {
        matches!(self.kind_at(offset), TokenKind::Word(w) if w == word)
    }

    pub(super) fn eat_word(&mut self, word: &str) -> bool {
        if self.check_word(word) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn expect_word(&mut self, word: &str, what: &str) -> Result<Token, AzraError> {
        if self.check_word(word) {
            Ok(self.advance())
        } else {
            Err(self.error_here(format!(
                "expected `{}` {}, found `{}`",
                word,
                what,
                self.peek_kind()
            )))
        }
    }

    pub(super) fn expect_comma(&mut self) -> Result<(), AzraError> {
        if self.check(&TokenKind::Comma) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(format!("expected `,` , found `{}`", self.peek_kind())))
        }
    }

    /// Run `f` with `allow_with` temporarily set to `allow`, always restoring
    /// the previous value — even when `f` fails (failed attempts backtrack).
    pub(super) fn with_allow_with<T>(
        &mut self,
        allow: bool,
        f: impl FnOnce(&mut Self) -> Result<T, AzraError>,
    ) -> Result<T, AzraError> {
        let saved = self.allow_with;
        self.allow_with = allow;
        let result = f(self);
        self.allow_with = saved;
        result
    }

    /// Build an error at the current token; running out of input marks the
    /// error as *incomplete* so the REPL keeps reading lines.
    pub(super) fn error_here(&self, message: impl Into<String>) -> AzraError {
        let tok = self.peek();
        let mut err = AzraError::syntax_at(message, tok.line, tok.column);
        err.incomplete = matches!(tok.kind, TokenKind::Eof);
        err
    }

    pub(super) fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    /// Consume the optional trailing period and the end of a statement line.
    pub(super) fn end_statement(&mut self) -> Result<(), AzraError> {
        if self.check(&TokenKind::Dot) {
            self.advance();
        }
        match self.peek_kind() {
            TokenKind::Newline => {
                self.advance();
                Ok(())
            }
            TokenKind::Eof => Ok(()),
            other => Err(self.error_here(format!("expected end of line, found `{}`", other))),
        }
    }

    /// Any word — identifiers may shadow keywords (`for each item in bag:`).
    pub(super) fn expect_name(&mut self, what: &str) -> Result<String, AzraError> {
        match self.peek_kind().clone() {
            TokenKind::Word(w) => {
                self.advance();
                Ok(w)
            }
            other => Err(self.error_here(format!("expected {}, found `{}`", what, other))),
        }
    }

    // -------------------------------------------------------------- program

    fn parse_program(&mut self) -> Result<Vec<Stmt>, AzraError> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::Eof) {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }
}

/// Flatten `a.b.c` into a path for method calls; `None` when the receiver is
/// not a plain variable chain.
pub(super) fn member_path(expr: &Expr) -> Option<Path> {
    match expr {
        Expr::Var { name, .. } => Some(vec![name.clone()]),
        Expr::Member { object, name, .. } => {
            let mut path = member_path(object)?;
            path.push(name.clone());
            Some(path)
        }
        _ => None,
    }
}

/// Build a string expression: plain text collapses to `Expr::Str`, anything
/// with `{...}` becomes `Expr::Interp`.
pub(super) fn str_expr(parts: &[StrPart], line: usize) -> Expr {
    if parts.iter().all(|p| matches!(p, StrPart::Text(_))) {
        let text = parts
            .iter()
            .map(|p| match p {
                StrPart::Text(s) => s.clone(),
                StrPart::Path(_) => unreachable!(),
            })
            .collect();
        return Expr::Str(text);
    }
    let interp = parts
        .iter()
        .map(|p| match p {
            StrPart::Text(s) => InterpPart::Text(s.clone()),
            StrPart::Path(path) => InterpPart::Path {
                path: path.clone(),
                line,
            },
        })
        .collect();
    Expr::Interp {
        parts: interp,
        line,
    }
}
