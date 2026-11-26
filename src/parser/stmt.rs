//! Statement dispatch and simple / single-line statements.

use super::{Parser, BUILTIN_WORDS, STATEMENT_KEYWORDS};
use crate::ast::{Expr, OutputLevel, Stmt, Target};
use crate::error::AzraError;
use crate::token::{StrPart, TokenKind};

pub(super) enum ListStmt {
    Sort,
    Reverse,
    Clear,
}

pub(super) enum TimerStmt {
    Every,
    After,
}

impl Parser {
    pub(super) fn parse_stmt(&mut self) -> Result<Stmt, AzraError> {
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
                "use" => self.parse_use(),
                "try" => self.parse_try(),
                "fail" => self.parse_fail(),
                "sort" => self.parse_list_stmt(ListStmt::Sort),
                "reverse" => self.parse_list_stmt(ListStmt::Reverse),
                "clear" => self.parse_list_stmt(ListStmt::Clear),
                "write" => self.parse_write(),
                "wait" => self.parse_wait(),
                "every" => self.parse_timer(TimerStmt::Every),
                "after" => self.parse_timer(TimerStmt::After),
                "stop" => {
                    let line = self.advance().line;
                    self.expect_word("loop", "in `stop loop`")?;
                    self.end_statement()?;
                    Ok(Stmt::Break { line })
                }
                "break" => {
                    let line = self.advance().line;
                    self.end_statement()?;
                    Ok(Stmt::Break { line })
                }
                "skip" => {
                    let line = self.advance().line;
                    self.expect_word("next", "in `skip next`")?;
                    self.end_statement()?;
                    Ok(Stmt::Continue { line })
                }
                "continue" => {
                    let line = self.advance().line;
                    self.end_statement()?;
                    Ok(Stmt::Continue { line })
                }
                "otherwise" => Err(self.error_here(
                    "stray `otherwise` without a matching `if`",
                )),
                _ if STATEMENT_KEYWORDS.contains(&word.as_str()) => Err(self.error_here(format!(
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

    fn parse_set(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // set
        let target = self.parse_target()?;
        self.expect_word("to", "in `set <target> to <value>`")?;
        let value = self.parse_expr_list()?;
        if value.is_empty() {
            return Err(self.error_here("expected a value after `to`"));
        }
        self.end_statement()?;
        Ok(Stmt::Set { target, value, line })
    }

    fn parse_make(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // make
        let type_name = self.expect_name("a type name after `make`")?;
        self.expect_word("as", "in `make <type> as <name>`")?;
        let name = self.expect_name("an object name after `as`")?;
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
        self.expect_word("to", "in `add <value> to <target>`")?;
        let target = self.parse_target()?;
        self.end_statement()?;
        Ok(Stmt::Add { value, target, line })
    }

    fn parse_remove(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // remove

        // `remove item <index> from <target>`
        let saved = self.pos;
        if self.check_word("item") {
            let attempt: Result<(Expr, Target), AzraError> = (|| {
                self.advance(); // item
                let index = self.parse_cmp()?;
                self.expect_word("from", "in `remove item <index> from <list>`")?;
                let target = self.parse_target()?;
                Ok((index, target))
            })();
            if let Ok((index, target)) = attempt {
                self.end_statement()?;
                return Ok(Stmt::RemoveAt {
                    index,
                    target,
                    line,
                });
            }
            self.pos = saved;
        }

        // `remove key <key> from <target>`
        if self.check_word("key") {
            let attempt: Result<(Expr, Target), AzraError> = (|| {
                self.advance(); // key
                let key = self.parse_cmp()?;
                self.expect_word("from", "in `remove key <key> from <map>`")?;
                let target = self.parse_target()?;
                Ok((key, target))
            })();
            if let Ok((key, target)) = attempt {
                self.end_statement()?;
                return Ok(Stmt::RemoveKey { key, target, line });
            }
            self.pos = saved;
        }

        // `remove <value> from <target>`
        let value = self.parse_expr()?;
        self.expect_word("from", "in `remove <value> from <target>`")?;
        let target = self.parse_target()?;
        self.end_statement()?;
        Ok(Stmt::Remove { value, target, line })
    }

    fn parse_subtract(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // subtract
        let value = self.parse_expr()?;
        self.expect_word("from", "in `subtract <value> from <target>`")?;
        let target = self.parse_target()?;
        self.end_statement()?;
        Ok(Stmt::Subtract { value, target, line })
    }

    fn parse_multiply(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // multiply
        let target = self.parse_target()?;
        self.expect_word("by", "in `multiply <target> by <value>`")?;
        let value = self.parse_expr()?;
        self.end_statement()?;
        Ok(Stmt::Multiply { target, value, line })
    }

    fn parse_divide(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // divide
        let target = self.parse_target()?;
        self.expect_word("by", "in `divide <target> by <value>`")?;
        let value = self.parse_expr()?;
        self.end_statement()?;
        Ok(Stmt::Divide { target, value, line })
    }

    fn parse_list_stmt(&mut self, kind: ListStmt) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // sort / reverse / clear
        let target = self.parse_target()?;
        self.end_statement()?;
        Ok(match kind {
            ListStmt::Sort => Stmt::Sort { target, line },
            ListStmt::Reverse => Stmt::Reverse { target, line },
            ListStmt::Clear => Stmt::Clear { target, line },
        })
    }

    fn parse_output(&mut self, level: OutputLevel) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // say / warn / error
        let args = self.parse_expr_list()?;
        self.end_statement()?;
        Ok(Stmt::Output { level, args, line })
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

    fn parse_use(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // use
        let path = match self.peek_kind().clone() {
            TokenKind::Str(parts) => {
                if parts.iter().all(|p| matches!(p, StrPart::Text(_))) {
                    parts
                        .iter()
                        .map(|p| match p {
                            StrPart::Text(s) => s.clone(),
                            StrPart::Path(_) => unreachable!(),
                        })
                        .collect::<String>()
                } else {
                    return Err(self.error_here(
                        "a module path must be a plain string without `{...}` interpolation",
                    ));
                }
            }
            other => Err(self.error_here(format!(
                "expected a module path string after `use` (e.g. `use \"weapons.vibe\"`), found `{}`",
                other
            )))?,
        };
        self.advance();
        let alias = if self.eat_word("as") {
            Some(self.expect_name("a module alias after `as`")?)
        } else {
            None
        };
        self.end_statement()?;
        Ok(Stmt::Use { path, alias, line })
    }

    fn parse_fail(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // fail
        self.expect_word("with", "in `fail with <message>`")?;
        let message = self.parse_expr()?;
        self.end_statement()?;
        Ok(Stmt::Fail { message, line })
    }

    fn parse_write(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // write
        let data = self.parse_expr()?;
        self.expect_word("to", "in `write <data> to file <path>`")?;
        self.expect_word("file", "in `write <data> to file <path>`")?;
        let path = self.parse_expr()?;
        self.end_statement()?;
        Ok(Stmt::WriteFile { data, path, line })
    }

    /// `wait <duration> [seconds]` — or the no-argument engine verb `wait`.
    fn parse_wait(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        self.advance(); // wait
        if self.starts_expr() {
            let duration = self.parse_expr()?;
            self.eat_word("seconds");
            self.end_statement()?;
            Ok(Stmt::Wait { duration, line })
        } else {
            self.end_statement()?;
            Ok(Stmt::Call {
                path: vec!["wait".to_string()],
                args: Vec::new(),
                line,
            })
        }
    }

    /// `<func> with <a>, <b>` / `weapons.fire with 10` / engine verb `play boom`
    fn parse_call_stmt(&mut self) -> Result<Stmt, AzraError> {
        let line = self.peek().line;
        // A built-in word at statement start: evaluate and discard the value
        // (`round 4.6`), unless it reads better as a plain call (`key with 1`).
        if matches!(self.peek_kind(), TokenKind::Word(w) if BUILTIN_WORDS.contains(&w.as_str())) {
            let saved = self.pos;
            match self.parse_builtin_expr() {
                Ok(expr) => {
                    self.end_statement()?;
                    return Ok(Stmt::ExprStmt { expr, line });
                }
                Err(builtin_err) => {
                    self.pos = saved;
                    if let Ok(stmt) = self.parse_path_call(line) {
                        return Ok(stmt);
                    }
                    return Err(builtin_err);
                }
            }
        }
        self.parse_path_call(line)
    }

    fn parse_path_call(&mut self, line: usize) -> Result<Stmt, AzraError> {
        let mut path = vec![self.expect_name("a function or command name")?];
        while self.check(&TokenKind::Dot) && matches!(self.kind_at(1), TokenKind::Word(_)) {
            self.advance();
            path.push(self.expect_name("a property name after `.`")?);
        }
        if self.check_word("with") {
            self.advance();
        }
        let mut args = Vec::new();
        self.with_allow_with(false, |p| {
            loop {
                if !p.starts_expr() {
                    break;
                }
                args.push(p.parse_expr()?);
                while p.check(&TokenKind::Comma) {
                    p.advance();
                    if !p.starts_expr() {
                        break;
                    }
                    args.push(p.parse_expr()?);
                }
                if p.check_word("with") {
                    p.advance();
                    continue;
                }
                break;
            }
            Ok(())
        })?;
        self.end_statement()?;
        Ok(Stmt::Call { path, args, line })
    }
}
