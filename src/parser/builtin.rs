//! Parsing of built-in English expression forms (`round 4.6`, `length of items`, ...).

use super::Parser;
use crate::ast::{Builtin, Expr};
use crate::error::AzraError;
use crate::token::TokenKind;

impl Parser {
    pub(super) fn parse_builtin_expr(&mut self) -> Result<Expr, AzraError> {
        self.with_depth(|p| p.parse_builtin_expr_inner())
    }

    fn parse_builtin_expr_inner(&mut self) -> Result<Expr, AzraError> {
        let tok = self.peek().clone();
        let w = match &tok.kind {
            TokenKind::Word(w) => w.clone(),
            other => {
                return Err(self.error_here(format!(
                    "expected a built-in expression, found `{}`",
                    other
                )))
            }
        };
        let line = tok.line;
        let (name, args): (Builtin, Vec<Expr>) = match w.as_str() {
            "vec3" => {
                self.advance();
                let x = self.parse_additive()?;
                self.expect_comma()?;
                let y = self.parse_additive()?;
                self.expect_comma()?;
                let z = self.parse_additive()?;
                if self.check(&TokenKind::Comma) {
                    return Err(self.error_here("`vec3` expects exactly 3 numbers"));
                }
                (Builtin::Vec3, vec![x, y, z])
            }
            "color" => {
                self.advance();
                let mut args = vec![self.parse_additive()?];
                while self.check(&TokenKind::Comma) {
                    self.advance();
                    if !self.starts_expr() {
                        break;
                    }
                    args.push(self.parse_additive()?);
                }
                if args.len() != 1 && args.len() != 3 {
                    return Err(self.error_here(
                        "`color` expects a color name or 3 numbers, e.g. `color 255, 100, 50`",
                    ));
                }
                (Builtin::Color, args)
            }
            "item" => {
                self.advance();
                let index = self.parse_cmp()?;
                self.expect_word("of", "in `item <index> of <list>`")?;
                let list = self.parse_postfix()?;
                (Builtin::ItemOf, vec![index, list])
            }
            "last" => {
                self.advance();
                self.expect_word("item", "in `last item of <list>`")?;
                self.expect_word("of", "in `last item of <list>`")?;
                let list = self.parse_postfix()?;
                (Builtin::LastItemOf, vec![list])
            }
            "first" => {
                self.advance();
                self.expect_word("item", "in `first item of <list>`")?;
                self.expect_word("of", "in `first item of <list>`")?;
                let list = self.parse_postfix()?;
                (Builtin::FirstItemOf, vec![list])
            }
            "length" => {
                self.advance();
                self.expect_word("of", "in `length of <value>`")?;
                let target = self.parse_postfix()?;
                (Builtin::LengthOf, vec![target])
            }
            "keys" => {
                self.advance();
                self.expect_word("of", "in `keys of <map>`")?;
                let map = self.parse_postfix()?;
                (Builtin::KeysOf, vec![map])
            }
            "key" => {
                self.advance();
                let key = self.parse_cmp()?;
                self.expect_word("of", "in `key <key> of <map>`")?;
                let map = self.parse_postfix()?;
                (Builtin::KeyOf, vec![key, map])
            }
            "distance" => {
                self.advance();
                self.expect_word("between", "in `distance between <a> and <b>`")?;
                let a = self.parse_additive()?;
                self.expect_word("and", "in `distance between <a> and <b>`")?;
                let b = self.parse_additive()?;
                (Builtin::Distance, vec![a, b])
            }
            "normalize" => {
                self.advance();
                let v = self.parse_unary()?;
                (Builtin::Normalize, vec![v])
            }
            "round" | "floor" | "ceil" | "abs" | "sqrt" | "sin" | "cos" => {
                self.advance();
                let arg = self.parse_unary()?;
                let name = match w.as_str() {
                    "round" => Builtin::Round,
                    "floor" => Builtin::Floor,
                    "ceil" => Builtin::Ceil,
                    "abs" => Builtin::Abs,
                    "sqrt" => Builtin::Sqrt,
                    "sin" => Builtin::Sin,
                    _ => Builtin::Cos,
                };
                (name, vec![arg])
            }
            "min" | "max" => {
                self.advance();
                self.expect_word("of", "in `min of <a>, <b>` / `max of <a>, <b>`")?;
                let a = self.parse_additive()?;
                self.expect_comma()?;
                let b = self.parse_additive()?;
                let name = if w == "min" { Builtin::Min } else { Builtin::Max };
                (name, vec![a, b])
            }
            "clamp" => {
                self.advance();
                let value = self.parse_additive()?;
                self.expect_word("between", "in `clamp <v> between <lo> and <hi>`")?;
                let lo = self.parse_additive()?;
                self.expect_word("and", "in `clamp <v> between <lo> and <hi>`")?;
                let hi = self.parse_additive()?;
                (Builtin::Clamp, vec![value, lo, hi])
            }
            "random" => {
                self.advance();
                if self.eat_word("number") {
                    self.expect_word("from", "in `random number from <a> to <b>`")?;
                    let from = self.parse_additive()?;
                    self.expect_word("to", "in `random number from <a> to <b>`")?;
                    let to = self.parse_additive()?;
                    (Builtin::RandomNumber, vec![from, to])
                } else if self.eat_word("decimal") {
                    (Builtin::RandomDecimal, Vec::new())
                } else {
                    return Err(self.error_here(
                        "expected `number` or `decimal` after `random`",
                    ));
                }
            }
            "uppercase" | "lowercase" | "trim" => {
                self.advance();
                self.expect_word("of", "in `uppercase of <text>` etc.")?;
                let text = self.parse_postfix()?;
                let name = match w.as_str() {
                    "uppercase" => Builtin::Uppercase,
                    "lowercase" => Builtin::Lowercase,
                    _ => Builtin::Trim,
                };
                (name, vec![text])
            }
            "split" => {
                self.advance();
                let text = self.parse_additive()?;
                self.expect_word("by", "in `split <text> by <separator>`")?;
                let sep = self.parse_additive()?;
                (Builtin::Split, vec![text, sep])
            }
            "join" => {
                self.advance();
                // The grammar owns the following `with` — don't read the
                // first argument as a call (`join items with ", "`).
                let list = self.with_allow_with(false, |p| p.parse_additive())?;
                self.expect_word("with", "in `join <list> with <separator>`")?;
                let sep = self.parse_additive()?;
                (Builtin::Join, vec![list, sep])
            }
            "replace" => {
                self.advance();
                // Same as `join`: the grammar owns `with` / `in`.
                let needle = self.with_allow_with(false, |p| p.parse_additive())?;
                self.expect_word("with", "in `replace <a> with <b> in <text>`")?;
                let replacement = self.parse_additive()?;
                self.expect_word("in", "in `replace <a> with <b> in <text>`")?;
                let text = self.parse_additive()?;
                (
                    Builtin::Replace,
                    vec![needle, replacement, text],
                )
            }
            "current" => {
                self.advance();
                self.expect_word("time", "in `current time`")?;
                (Builtin::CurrentTime, Vec::new())
            }
            "format" => {
                self.advance();
                self.expect_word("time", "in `format time <timestamp>`")?;
                let args = if self.starts_expr() {
                    vec![self.parse_additive()?]
                } else {
                    Vec::new()
                };
                (Builtin::FormatTime, args)
            }
            "read" => {
                self.advance();
                self.expect_word("file", "in `read file <path>`")?;
                let path = self.parse_additive()?;
                (Builtin::ReadFile, vec![path])
            }
            "file" => {
                self.advance();
                self.expect_word("exists", "in `file exists <path>`")?;
                let path = self.parse_additive()?;
                (Builtin::FileExists, vec![path])
            }
            "parse" => {
                self.advance();
                self.expect_word("json", "in `parse json <text>`")?;
                let text = self.parse_additive()?;
                (Builtin::ParseJson, vec![text])
            }
            "to" => {
                self.advance();
                self.expect_word("json", "in `to json <value>`")?;
                let value = self.parse_additive()?;
                (Builtin::ToJson, vec![value])
            }
            "empty" => {
                self.advance();
                if self.eat_word("list") {
                    (Builtin::EmptyList, Vec::new())
                } else if self.eat_word("map") {
                    (Builtin::EmptyMap, Vec::new())
                } else {
                    return Err(self.error_here("expected `list` or `map` after `empty`"));
                }
            }
            other => {
                return Err(self.error_here(format!(
                    "`{}` is not a built-in expression",
                    other
                )))
            }
        };
        Ok(Expr::Builtin { name, args, line })
    }
}
