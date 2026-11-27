//! Expression parsing with operator precedence.

use super::{member_path, str_expr, Parser, BUILTIN_WORDS};
use crate::ast::{BinOp, Builtin, Expr};
use crate::error::AzraError;
use crate::token::{Token, TokenKind};

/// Left-associative operator loops are not covered by `Parser::with_depth`,
/// so cap their length explicitly. A longer chain builds an AST deep enough
/// to overflow the stack when evaluated (and when dropped).
const MAX_EXPR_CHAIN: usize = 256;

impl Parser {
    pub(super) fn starts_expr(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Number(_) | TokenKind::Str(_) | TokenKind::Minus | TokenKind::Word(_)
        )
    }

    /// Comma separated expressions (`say a, b`, `set x to a, b`).
    pub(super) fn parse_expr_list(&mut self) -> Result<Vec<Expr>, AzraError> {
        let mut exprs = Vec::new();
        if !self.starts_expr() {
            return Ok(exprs);
        }
        exprs.push(self.parse_expr()?);
        while self.check(&TokenKind::Comma) {
            self.advance();
            if !self.starts_expr() {
                break;
            }
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    pub(super) fn parse_expr(&mut self) -> Result<Expr, AzraError> {
        self.parse_or()
    }

    /// Guard the flat operator loops below; unlike recursive descent they do
    /// not increase `self.depth`, but each iteration still nests the AST.
    fn chain_step(&self, count: &mut usize) -> Result<(), AzraError> {
        *count += 1;
        if *count > MAX_EXPR_CHAIN {
            Err(self.error_here(format!(
                "expression chain too long (maximum {} operators)",
                MAX_EXPR_CHAIN
            )))
        } else {
            Ok(())
        }
    }

    fn parse_or(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_and()?;
        let mut count = 0;
        while self.check_word("or") {
            self.chain_step(&mut count)?;
            let line = self.advance().line;
            let right = self.parse_and()?;
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_not()?;
        let mut count = 0;
        while self.check_word("and") {
            self.chain_step(&mut count)?;
            let line = self.advance().line;
            let right = self.parse_not()?;
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, AzraError> {
        if self.check_word("not") {
            let line = self.advance().line;
            let expr = self.with_depth(|p| p.parse_not())?;
            return Ok(Expr::Not {
                expr: Box::new(expr),
                line,
            });
        }
        self.parse_cmp()
    }

    fn cmp_op(&mut self) -> Option<(BinOp, usize)> {
        let line = self.peek().line;
        match self.peek_kind().clone() {
            TokenKind::Word(w) => match w.as_str() {
                "equals" => {
                    self.advance();
                    Some((BinOp::Eq, line))
                }
                "not_equals" => {
                    self.advance();
                    Some((BinOp::Ne, line))
                }
                "less_than" => {
                    self.advance();
                    Some((BinOp::Lt, line))
                }
                "greater_than" => {
                    self.advance();
                    Some((BinOp::Gt, line))
                }
                // `is` / `is not` — aliases for equals / not_equals.
                "is" => {
                    self.advance();
                    if self.eat_word("not") {
                        Some((BinOp::Ne, line))
                    } else {
                        Some((BinOp::Eq, line))
                    }
                }
                "contains" => {
                    self.advance();
                    Some((BinOp::Contains, line))
                }
                "starts" if self.check_word_at(1, "with") => {
                    self.advance();
                    self.advance();
                    Some((BinOp::StartsWith, line))
                }
                "has" if self.check_word_at(1, "key") => {
                    self.advance();
                    self.advance();
                    Some((BinOp::HasKey, line))
                }
                _ => None,
            },
            TokenKind::LessThanEq => {
                self.advance();
                Some((BinOp::Le, line))
            }
            TokenKind::GreaterThanEq => {
                self.advance();
                Some((BinOp::Ge, line))
            }
            _ => None,
        }
    }

    pub(super) fn parse_cmp(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_additive()?;
        let mut count = 0;
        while let Some((op, line)) = self.cmp_op() {
            self.chain_step(&mut count)?;
            let right = self.parse_additive()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    pub(super) fn parse_additive(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_mul()?;
        let mut count = 0;
        loop {
            self.chain_step(&mut count)?;
            let op = match self.peek_kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            let line = self.advance().line;
            let right = self.parse_mul()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_pow()?;
        let mut count = 0;
        loop {
            self.chain_step(&mut count)?;
            let op = match self.peek_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Word(w) if w == "modulo" => BinOp::Mod,
                _ => break,
            };
            let line = self.advance().line;
            let right = self.parse_pow()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    /// `power` binds tighter than `*` and is right-associative:
    /// `2 * 3 power 2` == `2 * (3 power 2)` == 18.
    fn parse_pow(&mut self) -> Result<Expr, AzraError> {
        let left = self.parse_unary()?;
        if self.check_word("power") {
            let line = self.advance().line;
            let right = self.with_depth(|p| p.parse_pow())?;
            return Ok(Expr::Binary {
                op: BinOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
                line,
            });
        }
        Ok(left)
    }

    pub(super) fn parse_unary(&mut self) -> Result<Expr, AzraError> {
        if self.check(&TokenKind::Minus) {
            let line = self.advance().line;
            let expr = self.with_depth(|p| p.parse_unary())?;
            return Ok(Expr::Negate {
                expr: Box::new(expr),
                line,
            });
        }
        self.parse_postfix()
    }

    pub(super) fn parse_postfix(&mut self) -> Result<Expr, AzraError> {
        let mut expr = self.parse_primary()?;
        let mut count = 0;
        loop {
            self.chain_step(&mut count)?;
            // `.` + word = member access (a `.` at the end of a line is the
            // optional trailing statement period instead).
            if self.check(&TokenKind::Dot) && matches!(self.kind_at(1), TokenKind::Word(_)) {
                self.advance();
                let line = self.peek().line;
                let name = self.expect_name("a property name after `.`")?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    name,
                    line,
                };
                continue;
            }
            // Possessive key/property access: `stats's "strength"`.
            if self.check(&TokenKind::Possessive) {
                self.advance();
                let line = self.peek().line;
                let key = self.parse_primary()?;
                expr = Expr::Builtin {
                    name: Builtin::KeyOf,
                    args: vec![key, expr],
                    line,
                };
                continue;
            }
            // Method call: `weapons.fire with 10`.
            if self.allow_with
                && self.check_word("with")
                && let Some(path) = member_path(&expr)
            {
                self.advance();
                let line = expr.line();
                let args = self.parse_expr_list()?;
                expr = Expr::Call { path, args, line };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    pub(super) fn parse_primary(&mut self) -> Result<Expr, AzraError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n.clone()))
            }
            TokenKind::Str(parts) => {
                self.advance();
                Ok(str_expr(parts, tok.line))
            }
            TokenKind::Word(w) => match w.as_str() {
                "true" => {
                    self.advance();
                    Ok(Expr::Bool(true))
                }
                "false" => {
                    self.advance();
                    Ok(Expr::Bool(false))
                }
                "null" | "nothing" => {
                    self.advance();
                    Ok(Expr::Null)
                }
                _ if BUILTIN_WORDS.contains(&w.as_str()) => {
                    let saved = self.pos;
                    match self.parse_builtin_expr() {
                        Ok(expr) => Ok(expr),
                        // Not a built-in form after all — read it as a
                        // variable (`say round`).
                        Err(_) => {
                            self.pos = saved;
                            self.parse_var_or_call(tok)
                        }
                    }
                }
                _ => self.parse_var_or_call(tok),
            },
            other => Err(self.error_here(format!(
                "expected a value, found `{}`",
                other
            ))),
        }
    }

    fn parse_var_or_call(&mut self, tok: Token) -> Result<Expr, AzraError> {
        let name = match tok.kind {
            TokenKind::Word(w) => w,
            _ => unreachable!("variable starts with a word"),
        };
        let line = tok.line;
        self.advance();
        if self.allow_with && self.check_word("with") {
            self.advance();
            let args = self.parse_expr_list()?;
            return Ok(Expr::Call {
                path: vec![name],
                args,
                line,
            });
        }
        Ok(Expr::Var { name, line })
    }
}
