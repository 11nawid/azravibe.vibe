//! Control-flow, function, event, timer, and indented block parsing.

use super::stmt::TimerStmt;
use super::Parser;
use crate::ast::{Branch, Stmt};
use crate::error::AzraError;
use crate::token::{render_str_parts, TokenKind};

impl Parser {
    pub(super) fn parse_if(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // if
        let cond = self.parse_expr()?;
        self.expect_colon_block("after the `if` condition")?;
        let body = self.parse_block("an indented block after `if ...:`")?;
        let mut branches = vec![Branch { cond, body }];
        let mut otherwise = None;
        while self.check_word("otherwise") {
            self.advance();
            if self.eat_word("if") {
                let cond = self.parse_expr()?;
                self.expect_colon_block("after the `otherwise if` condition")?;
                let body = self.parse_block("an indented block after `otherwise if ...:`")?;
                branches.push(Branch { cond, body });
            } else {
                self.expect_colon_block("after `otherwise`")?;
                let body = self.parse_block("an indented block after `otherwise:`")?;
                otherwise = Some(body);
                break;
            }
        }
        Ok(Stmt::If {
            branches,
            otherwise,
            line,
        })
    }

    pub(super) fn parse_repeat(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // repeat

        // `repeat with i from 1 to 10 [by 2]:`
        if self.check_word("with") {
            self.advance();
            let var = self.expect_name("a loop variable after `repeat with`")?;
            self.expect_word("from", "in `repeat with <i> from <a> to <b>:`")?;
            let from = self.parse_cmp()?;
            self.expect_word("to", "in `repeat with <i> from <a> to <b>:`")?;
            let to = self.parse_cmp()?;
            let step = if self.check_word("by") || self.check_word("step") {
                self.advance();
                Some(self.parse_cmp()?)
            } else {
                None
            };
            self.expect_colon_block("after the `repeat ... from ... to ...` header")?;
            let body = self.parse_block("an indented block after `repeat with ...:`")?;
            return Ok(Stmt::RepeatRange {
                var,
                from,
                to,
                step,
                body,
                line,
            });
        }

        let count = self.parse_expr()?;
        self.expect_word("times", "in `repeat <count> times:`")?;
        self.expect_colon_block("after `repeat ... times`")?;
        let body = self.parse_block("an indented block after `repeat ... times:`")?;
        Ok(Stmt::Repeat { count, body, line })
    }

    pub(super) fn parse_for(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // for
        self.expect_word("each", "in `for each <item> in <list>:`")?;
        let var = self.expect_name("a loop variable after `each`")?;
        self.expect_word("in", "in `for each <item> in <list>:`")?;
        let iter = self.parse_expr()?;
        self.expect_colon_block("after the `for each ... in ...` header")?;
        let body = self.parse_block("an indented block after `for each ...:`")?;
        Ok(Stmt::ForEach {
            var,
            iter,
            body,
            line,
        })
    }

    pub(super) fn parse_while(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // while
        let cond = self.parse_expr()?;
        self.expect_colon_block("after the `while` condition")?;
        let body = self.parse_block("an indented block after `while ...:`")?;
        Ok(Stmt::While { cond, body, line })
    }

    pub(super) fn parse_define(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // define
        let name = self.expect_name("a function name after `define`")?;
        let mut params = Vec::new();
        if self.eat_word("with") {
            loop {
                params.push(self.expect_name("a parameter name")?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        self.expect_colon_block("after the `define` header")?;
        let body = self.parse_block("an indented block after `define ...:`")?;
        Ok(Stmt::Define {
            name,
            params,
            body,
            line,
        })
    }

    pub(super) fn parse_when(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // when
        let (phrase, params) = self.parse_event_header()?;
        self.expect_colon_block("after the `when <event>` phrase")?;
        let body = self.parse_block("an indented block after `when ...:`")?;
        Ok(Stmt::When {
            phrase,
            params,
            body,
            line,
        })
    }

    /// Collect the free-form event phrase until `:` — or until a trailing
    /// `with p1, p2` parameter list (as in `when game updates with delta_time:`).
    fn parse_event_header(&mut self) -> Result<(String, Vec<String>), AzraError> {
        let mut parts: Vec<String> = Vec::new();
        loop {
            if self.check_word("with") && self.looks_like_event_params() {
                break;
            }
            match self.peek_kind().clone() {
                TokenKind::Colon => break,
                TokenKind::Eof | TokenKind::Newline => {
                    return Err(self.error_here(
                        "expected `:` after the event phrase in `when <event>:`",
                    ))
                }
                TokenKind::Word(w) => {
                    parts.push(w);
                    self.advance();
                }
                TokenKind::Str(s) => {
                    parts.push(format!("\"{}\"", render_str_parts(&s)));
                    self.advance();
                }
                TokenKind::Number(n) => {
                    parts.push(n.to_string());
                    self.advance();
                }
                TokenKind::Dot => {
                    if let Some(last) = parts.last_mut() {
                        last.push('.');
                    } else {
                        parts.push(".".to_string());
                    }
                    self.advance();
                }
                TokenKind::Possessive => {
                    parts.push("'s".to_string());
                    self.advance();
                }
                TokenKind::LessThanEq => {
                    parts.push("less_than=".to_string());
                    self.advance();
                }
                TokenKind::GreaterThanEq => {
                    parts.push("greater_than=".to_string());
                    self.advance();
                }
                TokenKind::Plus => {
                    parts.push("+".to_string());
                    self.advance();
                }
                TokenKind::Minus => {
                    parts.push("-".to_string());
                    self.advance();
                }
                TokenKind::Star => {
                    parts.push("*".to_string());
                    self.advance();
                }
                TokenKind::Slash => {
                    parts.push("/".to_string());
                    self.advance();
                }
                TokenKind::Comma => {
                    parts.push(",".to_string());
                    self.advance();
                }
                other => {
                    return Err(self.error_here(format!(
                        "unexpected `{}` in a `when` event phrase",
                        other
                    )))
                }
            }
        }
        if parts.is_empty() {
            return Err(self.error_here(
                "expected an event phrase after `when`, e.g. `when player touches lava_block:`",
            ));
        }
        // `with <ident> (, <ident>)*` before the `:` declares handler parameters.
        let mut params = Vec::new();
        if self.eat_word("with") {
            loop {
                params.push(self.expect_name("an event parameter after `with`")?);
                if !self.check(&TokenKind::Comma) {
                    break;
                }
                self.advance();
            }
        }
        Ok((parts.join(" "), params))
    }

    /// Lookahead: are we at `with <ident> (, <ident>)* :` (parameter list)
    /// rather than `with` as part of the event phrase?
    fn looks_like_event_params(&self) -> bool {
        let mut i = 1usize;
        loop {
            match self.kind_at(i) {
                TokenKind::Word(_) => i += 1,
                TokenKind::Comma => {
                    if matches!(self.kind_at(i + 1), TokenKind::Word(_)) {
                        i += 2;
                    } else {
                        return false;
                    }
                }
                TokenKind::Colon => return true,
                _ => return false,
            }
        }
    }

    pub(super) fn parse_try(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // try
        self.expect_colon_block("after `try`")?;
        let body = self.parse_block("an indented block after `try:`")?;
        // `if error as err:` handler
        if !(self.check_word("if") && matches!(self.kind_at(1), TokenKind::Word(w) if w == "error")) {
            return Err(self.error_here(
                "expected `if error as <name>:` after the `try` block",
            ));
        }
        self.advance(); // if
        self.advance(); // error
        self.expect_word("as", "in `if error as <name>:`")?;
        let err_name = self.expect_name("an error variable after `as`")?;
        self.expect_colon_block("after `if error as ...`")?;
        let handler = self.parse_block("an indented block after `if error as ...:`")?;
        Ok(Stmt::Try {
            body,
            err_name,
            handler,
            line,
        })
    }

    pub(super) fn parse_timer(&mut self, kind: TimerStmt) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // every / after
        let amount = self.parse_expr()?;
        self.expect_word("seconds", "in `every <n> seconds:` / `after <n> seconds:`")?;
        self.expect_colon_block("after the timer header")?;
        let body = self.parse_block("an indented block after the timer header")?;
        Ok(match kind {
            TimerStmt::Every => Stmt::Every {
                interval: amount,
                body,
                line,
            },
            TimerStmt::After => Stmt::After {
                delay: amount,
                body,
                line,
            },
        })
    }

    // ---------------------------------------------------------------- blocks

    /// `:` + optional trailing period + end of line.
    fn expect_colon_block(&mut self, what: &str) -> Result<(), AzraError> {
        if !self.check(&TokenKind::Colon) {
            return Err(self.error_here(format!(
                "expected `:` {}, found `{}`",
                what,
                self.peek_kind()
            )));
        }
        self.advance();
        if self.check(&TokenKind::Dot) {
            self.advance();
        }
        if self.check(&TokenKind::Newline) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(format!(
                "expected end of line after `:` {}",
                what
            )))
        }
    }

    fn parse_block(&mut self, what: &str) -> Result<Vec<Stmt>, AzraError> {
        if !self.check(&TokenKind::Indent) {
            return Err(self.error_here(format!("expected {}, found `{}`", what, self.peek_kind())));
        }
        self.with_depth(|p| {
            p.advance();
            let mut body = Vec::new();
            loop {
                p.skip_newlines();
                match p.peek_kind() {
                    TokenKind::Dedent => {
                        p.advance();
                        return Ok(body);
                    }
                    TokenKind::Eof => {
                        return Err(p.error_here(format!(
                            "expected the end of the indented block ({})",
                            what
                        )));
                    }
                    _ => body.push(p.parse_stmt()?),
                }
            }
        })
    }
}
