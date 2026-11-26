//! Recursive descent parser for Azra with helpful, line-numbered errors.

use crate::ast::*;
use crate::error::AzraError;
use crate::token::{Token, TokenKind};

/// Reserved words: they may not be used as plain identifiers.
const KEYWORDS: &[&str] = &[
    "set", "make", "add", "remove", "subtract", "multiply", "divide", "say", "warn", "error",
    "if", "otherwise", "repeat", "times", "for", "each", "in", "while", "define", "with", "give",
    "back", "when", "to", "from", "by", "and", "or", "not", "equals", "not_equals", "less_than",
    "greater_than", "true", "false", "null",
];

/// Parse a token stream into a program (list of statements).
pub fn parse(tokens: &[Token]) -> Result<Vec<Stmt>, AzraError> {
    Parser {
        tokens: tokens.to_vec(),
        pos: 0,
        allow_with: true,
    }
    .parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Whether `name with args` inside an expression is a function call.
    /// Disabled while parsing statement-call arguments, where `with` acts as
    /// an argument separator (`jump player with 15`).
    allow_with: bool,
}

impl Parser {
    // ---------------------------------------------------------------- helpers

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("token stream ends with Eof"))
    }

    /// Kind of the token `offset` positions ahead (clamped to `Eof`).
    fn kind_at(&self, offset: usize) -> &TokenKind {
        self.tokens
            .get(self.pos + offset)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn check_word(&self, word: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Word(w) if w == word)
    }

    fn eat_word(&mut self, word: &str) -> bool {
        if self.check_word(word) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_word(&mut self, word: &str, what: &str) -> Result<Token, AzraError> {
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

    /// Build an error at the current token; running out of input marks the
    /// error as *incomplete* so the REPL keeps reading lines.
    fn error_here(&self, message: impl Into<String>) -> AzraError {
        let tok = self.peek();
        let mut err = AzraError::syntax_at(message, tok.line, tok.column);
        err.incomplete = matches!(tok.kind, TokenKind::Eof);
        err
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    /// Consume the optional trailing period and the end of a statement line.
    fn end_statement(&mut self) -> Result<(), AzraError> {
        if self.check(&TokenKind::Dot) {
            self.advance();
        }
        match self.peek_kind() {
            TokenKind::Newline => {
                self.advance();
                Ok(())
            }
            TokenKind::Eof => Ok(()),
            other => Err(self.error_here(format!(
                "expected end of line, found `{}`",
                other
            ))),
        }
    }

    fn expect_ident(&mut self, what: &str) -> Result<String, AzraError> {
        match self.peek_kind().clone() {
            TokenKind::Word(w) if !KEYWORDS.contains(&w.as_str()) => {
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

    fn parse_stmt(&mut self) -> Result<Stmt, AzraError> {
        match self.peek_kind().clone() {
            TokenKind::Word(word) => match word.as_str() {
                "set" => self.parse_set(),
                "make" => self.parse_make(),
                "add" => self.parse_add(),
                "remove" => self.parse_remove(),
                "subtract" => self.parse_subtract(),
                "multiply" => self.parse_multiply(),
                "divide" => self.parse_divide(),
                "say" => self.parse_output(OutputLevel::Say),
                "warn" => self.parse_output(OutputLevel::Warn),
                "error" => self.parse_output(OutputLevel::Error),
                "if" => self.parse_if(),
                "repeat" => self.parse_repeat(),
                "for" => self.parse_for(),
                "while" => self.parse_while(),
                "define" => self.parse_define(),
                "give" => self.parse_give_back(),
                "when" => self.parse_when(),
                "otherwise" => Err(self.error_here(
                    "stray `otherwise` without a matching `if`",
                )),
                _ if KEYWORDS.contains(&word.as_str()) => Err(self.error_here(format!(
                    "unexpected keyword `{}` at the start of a statement",
                    word
                ))),
                _ => self.parse_call_stmt(),
            },
            other => Err(self.error_here(format!(
                "expected a statement, found `{}`",
                other
            ))),
        }
    }

    // ----------------------------------------------------------- statements

    fn parse_set(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // set
        let path = self.parse_path()?;
        self.expect_word("to", "in `set <name> to <value>`")?;
        let value = self.parse_expr_list()?;
        if value.is_empty() {
            return Err(self.error_here("expected a value after `to`"));
        }
        self.end_statement()?;
        Ok(Stmt::Set { path, value, line })
    }

    fn parse_make(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // make
        let type_name = self.expect_ident("a type name after `make`")?;
        self.expect_word("as", "in `make <type> as <name>`")?;
        let name = self.expect_ident("an object name after `as`")?;
        self.end_statement()?;
        Ok(Stmt::Make {
            type_name,
            name,
            line,
        })
    }

    fn parse_add(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // add
        let value = self.parse_expr()?;
        self.expect_word("to", "in `add <value> to <name>`")?;
        let path = self.parse_path()?;
        self.end_statement()?;
        Ok(Stmt::Add { value, path, line })
    }

    fn parse_remove(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // remove
        let value = self.parse_expr()?;
        self.expect_word("from", "in `remove <value> from <name>`")?;
        let path = self.parse_path()?;
        self.end_statement()?;
        Ok(Stmt::Remove { value, path, line })
    }

    fn parse_subtract(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // subtract
        let value = self.parse_expr()?;
        self.expect_word("from", "in `subtract <value> from <name>`")?;
        let path = self.parse_path()?;
        self.end_statement()?;
        Ok(Stmt::Subtract { value, path, line })
    }

    fn parse_multiply(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // multiply
        let path = self.parse_path()?;
        self.expect_word("by", "in `multiply <name> by <value>`")?;
        let value = self.parse_expr()?;
        self.end_statement()?;
        Ok(Stmt::Multiply { path, value, line })
    }

    fn parse_divide(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // divide
        let path = self.parse_path()?;
        self.expect_word("by", "in `divide <name> by <value>`")?;
        let value = self.parse_expr()?;
        self.end_statement()?;
        Ok(Stmt::Divide { path, value, line })
    }

    fn parse_output(&mut self, level: OutputLevel) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // say / warn / error
        let args = self.parse_expr_list()?;
        self.end_statement()?;
        Ok(Stmt::Output { level, args, line })
    }

    fn parse_if(&mut self) -> Result<Stmt, AzraError> {
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

    fn parse_repeat(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // repeat
        let count = self.parse_expr()?;
        self.expect_word("times", "in `repeat <count> times:`")?;
        self.expect_colon_block("after `repeat ... times`")?;
        let body = self.parse_block("an indented block after `repeat ... times:`")?;
        Ok(Stmt::Repeat { count, body, line })
    }

    fn parse_for(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // for
        self.expect_word("each", "in `for each <item> in <list>:`")?;
        let var = self.expect_ident("a loop variable after `each`")?;
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

    fn parse_while(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // while
        let cond = self.parse_expr()?;
        self.expect_colon_block("after the `while` condition")?;
        let body = self.parse_block("an indented block after `while ...:`")?;
        Ok(Stmt::While { cond, body, line })
    }

    fn parse_define(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // define
        let name = self.expect_ident("a function name after `define`")?;
        let mut params = Vec::new();
        if self.eat_word("with") {
            loop {
                params.push(self.expect_ident("a parameter name")?);
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

    fn parse_give_back(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // give
        self.expect_word("back", "in `give back <value>`")?;
        let value = if self.starts_expr() {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.end_statement()?;
        Ok(Stmt::Return { value, line })
    }

    fn parse_when(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // when
        let phrase = self.parse_event_phrase()?;
        self.expect_colon_block("after the `when <event>` phrase")?;
        let body = self.parse_block("an indented block after `when ...:`")?;
        Ok(Stmt::When { phrase, body, line })
    }

    /// Collect the free-form event phrase until the closing `:`.
    fn parse_event_phrase(&mut self) -> Result<String, AzraError> {
        let mut parts: Vec<String> = Vec::new();
        loop {
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
                    parts.push(format!("\"{}\"", s));
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
                TokenKind::LessThanEq => {
                    parts.push("less_than=".to_string());
                    self.advance();
                }
                TokenKind::GreaterThanEq => {
                    parts.push("greater_than=".to_string());
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
        // Stop at (without consuming) the closing ':' — expect_colon_block
        // consumes it together with the end-of-line check.
        if parts.is_empty() {
            return Err(self.error_here(
                "expected an event phrase after `when`, e.g. `when player touches lava_block:`",
            ));
        }
        Ok(parts.join(" "))
    }

    /// `<name> with a, b` / `play boom_sfx` / `jump player with 15`
    fn parse_call_stmt(&mut self) -> Result<Stmt, AzraError> {
        let tok = self.advance();
        let name = match tok.kind {
            TokenKind::Word(w) => w,
            _ => unreachable!("call statement starts with a word"),
        };
        if self.check_word("with") {
            self.advance();
        }
        let saved = self.allow_with;
        self.allow_with = false;
        let mut args = Vec::new();
        loop {
            if !self.starts_expr() {
                break;
            }
            args.push(self.parse_expr()?);
            while self.check(&TokenKind::Comma) {
                self.advance();
                if !self.starts_expr() {
                    break;
                }
                args.push(self.parse_expr()?);
            }
            if self.check_word("with") {
                self.advance();
                continue;
            }
            break;
        }
        self.allow_with = saved;
        self.end_statement()?;
        Ok(Stmt::Call {
            name,
            args,
            line: tok.line,
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
        self.advance();
        let mut body = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Dedent => {
                    self.advance();
                    return Ok(body);
                }
                TokenKind::Eof => {
                    return Err(self.error_here(format!(
                        "expected the end of the indented block ({})",
                        what
                    )));
                }
                _ => body.push(self.parse_stmt()?),
            }
        }
    }

    // ------------------------------------------------------------ expressions

    fn parse_path(&mut self) -> Result<Path, AzraError> {
        let mut path = vec![self.expect_ident("a variable or property name")?];
        while self.check(&TokenKind::Dot) {
            self.advance();
            path.push(self.expect_ident("a property name after `.`")?);
        }
        Ok(path)
    }

    fn starts_expr(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Number(_) | TokenKind::Str(_) | TokenKind::Minus | TokenKind::Word(_)
        )
    }

    /// Comma separated expressions (`say a, b`, `set x to a, b`).
    fn parse_expr_list(&mut self) -> Result<Vec<Expr>, AzraError> {
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

    fn parse_expr(&mut self) -> Result<Expr, AzraError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_and()?;
        while self.check_word("or") {
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
        while self.check_word("and") {
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
            let expr = self.parse_not()?;
            return Ok(Expr::Not {
                expr: Box::new(expr),
                line,
            });
        }
        self.parse_cmp()
    }

    fn cmp_op(&self) -> Option<BinOp> {
        match self.peek_kind() {
            TokenKind::Word(w) => match w.as_str() {
                "equals" => Some(BinOp::Eq),
                "not_equals" => Some(BinOp::Ne),
                "less_than" => Some(BinOp::Lt),
                "greater_than" => Some(BinOp::Gt),
                _ => None,
            },
            TokenKind::LessThanEq => Some(BinOp::Le),
            TokenKind::GreaterThanEq => Some(BinOp::Ge),
            _ => None,
        }
    }

    fn parse_cmp(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_additive()?;
        while let Some(op) = self.cmp_op() {
            let line = self.advance().line;
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

    fn parse_additive(&mut self) -> Result<Expr, AzraError> {
        let mut left = self.parse_mul()?;
        loop {
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
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            let line = self.advance().line;
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                line,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, AzraError> {
        if self.check(&TokenKind::Minus) {
            let line = self.advance().line;
            let expr = self.parse_unary()?;
            return Ok(Expr::Negate {
                expr: Box::new(expr),
                line,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, AzraError> {
        let mut expr = self.parse_primary()?;
        while self.check(&TokenKind::Dot) {
            // A `.` at the end of a line is the optional trailing statement
            // period, not member access — only `.` + property name counts.
            let is_property =
                matches!(self.kind_at(1), TokenKind::Word(w) if !KEYWORDS.contains(&w.as_str()));
            if !is_property {
                break;
            }
            self.advance();
            let line = self.peek().line;
            let name = self.expect_ident("a property name after `.`")?;
            expr = Expr::Member {
                object: Box::new(expr),
                name,
                line,
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, AzraError> {
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Number(n.clone()))
            }
            TokenKind::Str(s) => {
                self.advance();
                Ok(Expr::Str(s.clone()))
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
                "null" => {
                    self.advance();
                    Ok(Expr::Null)
                }
                _ if KEYWORDS.contains(&w.as_str()) => Err(self.error_here(format!(
                    "unexpected keyword `{}` where a value was expected",
                    w
                ))),
                _ => {
                    let name = w.clone();
                    let line = tok.line;
                    self.advance();
                    if self.allow_with && self.check_word("with") {
                        self.advance();
                        let args = self.parse_expr_list()?;
                        Ok(Expr::Call { name, args, line })
                    } else {
                        Ok(Expr::Var { name, line })
                    }
                }
            },
            other => Err(self.error_here(format!(
                "expected a value, found `{}`",
                other
            ))),
        }
    }
}
