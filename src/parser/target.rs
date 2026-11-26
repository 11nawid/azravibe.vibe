//! Assignment target parsing (`set <target> to ...`, `add ... to <target>`, etc.).

use super::Parser;
use crate::ast::{Path, Target};
use crate::error::AzraError;
use crate::token::TokenKind;

impl Parser {
    /// `<path>` | `key <k> of <target>` | `item <i> of <target>` |
    /// `first item of <target>` | `last item of <target>` | `<map>'s <k>`
    pub(super) fn parse_target(&mut self) -> Result<Target, AzraError> {
        self.with_depth(|p| p.parse_target_inner())
    }

    fn parse_target_inner(&mut self) -> Result<Target, AzraError> {
        let saved = self.pos;
        if let Ok(target) = self.try_key_target() {
            return Ok(target);
        }
        self.pos = saved;
        if let Ok(target) = self.try_item_target() {
            return Ok(target);
        }
        self.pos = saved;

        let line = self.peek().line;
        let path = self.parse_path()?;
        let mut target = Target::Path { path, line };
        while self.check(&TokenKind::Possessive) {
            self.advance();
            let p_line = self.peek().line;
            let key = self.parse_primary()?;
            target = Target::Key {
                key,
                map: Box::new(target),
                line: p_line,
            };
        }
        Ok(target)
    }

    fn try_key_target(&mut self) -> Result<Target, AzraError> {
        if !self.check_word("key") {
            return Err(self.error_here("not a `key ... of ...` target"));
        }
        self.advance(); // key
        let line = self.peek().line;
        let key = self.parse_cmp()?;
        self.expect_word("of", "in `key <key> of <map>`")?;
        let map = self.parse_target()?;
        Ok(Target::Key {
            key,
            map: Box::new(map),
            line,
        })
    }

    fn try_item_target(&mut self) -> Result<Target, AzraError> {
        let line = self.peek().line;
        if self.check_word("item") {
            self.advance(); // item
            let index = self.parse_cmp()?;
            self.expect_word("of", "in `item <index> of <list>`")?;
            let list = self.parse_target()?;
            return Ok(Target::Item {
                index,
                list: Box::new(list),
                line,
            });
        }
        if self.check_word("first") || self.check_word("last") {
            let first = self.check_word("first");
            self.advance();
            self.expect_word("item", "in `first item of <list>` / `last item of <list>`")?;
            self.expect_word("of", "in `first item of <list>` / `last item of <list>`")?;
            let list = self.parse_target()?;
            return Ok(if first {
                Target::First {
                    list: Box::new(list),
                    line,
                }
            } else {
                Target::Last {
                    list: Box::new(list),
                    line,
                }
            });
        }
        Err(self.error_here("not an `item ... of ...` target"))
    }

    fn parse_path(&mut self) -> Result<Path, AzraError> {
        let mut path = vec![self.expect_name("a variable or property name")?];
        while self.check(&TokenKind::Dot) && matches!(self.kind_at(1), TokenKind::Word(_)) {
            self.advance();
            path.push(self.expect_name("a property name after `.`")?);
        }
        Ok(path)
    }
}
