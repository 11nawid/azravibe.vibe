//! Lexing of strings (with `{...}` interpolation), numbers, identifiers,
//! single-token operators (`less_than=`, `greater_than=`), possessive `'s`,
//! and inline `note:` comments.

use super::Lexer;
use crate::error::AzraError;
use crate::token::{Num, StrPart, TokenKind};

impl Lexer {
    pub(super) fn lex_string(&mut self, line: usize, column: usize) -> Result<(), AzraError> {
        let quote = self.peek().expect("string starts with a quote");
        self.bump(); // opening quote
        let mut parts: Vec<StrPart> = Vec::new();
        let mut text = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => {
                    return Err(AzraError::syntax_at(
                        format!(
                            "unterminated string literal (started at line {}, column {})",
                            line, column
                        ),
                        line,
                        column,
                    ));
                }
                Some('\\') => {
                    self.bump();
                    match self.peek() {
                        Some('n') => {
                            text.push('\n');
                            self.bump();
                        }
                        Some('t') => {
                            text.push('\t');
                            self.bump();
                        }
                        Some('r') => {
                            text.push('\r');
                            self.bump();
                        }
                        Some('\\') => {
                            text.push('\\');
                            self.bump();
                        }
                        Some('"') => {
                            text.push('"');
                            self.bump();
                        }
                        Some('\'') => {
                            text.push('\'');
                            self.bump();
                        }
                        // Escaped braces are literal — `{` only starts an
                        // interpolation when it is not preceded by `\`.
                        Some('{') => {
                            text.push('{');
                            self.bump();
                        }
                        Some('}') => {
                            text.push('}');
                            self.bump();
                        }
                        Some(other) => {
                            return Err(AzraError::syntax_at(
                                format!("unknown escape sequence `\\{}`", other),
                                self.line,
                                self.col,
                            ));
                        }
                        None => {
                            return Err(AzraError::syntax_at(
                                format!(
                                    "unterminated string literal (started at line {}, column {})",
                                    line, column
                                ),
                                line,
                                column,
                            ));
                        }
                    }
                }
                // `{dotted.path}` — string interpolation.
                Some('{') => {
                    if !text.is_empty() {
                        parts.push(StrPart::Text(std::mem::take(&mut text)));
                    }
                    let interp_line = self.line;
                    let interp_col = self.col;
                    self.bump(); // '{'
                    let mut path = String::new();
                    loop {
                        match self.peek() {
                            Some('}') => {
                                self.bump();
                                break;
                            }
                            Some(c) if c == quote || c == '\n' => {
                                return Err(AzraError::syntax_at(
                                    "unterminated `{...}` interpolation in string literal",
                                    interp_line,
                                    interp_col,
                                ));
                            }
                            None => {
                                return Err(AzraError::syntax_at(
                                    "unterminated `{...}` interpolation in string literal",
                                    interp_line,
                                    interp_col,
                                ));
                            }
                            Some(c) => {
                                path.push(c);
                                self.bump();
                            }
                        }
                    }
                    parts.push(StrPart::Path(parse_interpolation_path(
                        &path,
                        interp_line,
                        interp_col,
                    )?));
                }
                Some(c) if c == quote => {
                    self.bump();
                    break;
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
        if !text.is_empty() {
            parts.push(StrPart::Text(text));
        }
        self.push(TokenKind::Str(parts), line, column);
        self.line_has_tokens = true;
        Ok(())
    }

    pub(super) fn lex_number(&mut self, line: usize, column: usize) -> Result<(), AzraError> {
        let mut raw = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            raw.push(self.peek().unwrap());
            self.bump();
        }
        // A `.` only belongs to the number when a digit follows, so the
        // optional trailing statement period (`set x to 5.`) stays a Dot.
        let mut is_float = false;
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit()) {
            is_float = true;
            raw.push('.');
            self.bump();
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                raw.push(self.peek().unwrap());
                self.bump();
            }
        }
        let num = if is_float {
            Num::Float(raw.parse::<f64>().unwrap_or(f64::NAN))
        } else {
            match raw.parse::<i64>() {
                Ok(i) => Num::Int(i),
                // Integer literal too large for i64: fall back to float.
                Err(_) => Num::Float(raw.parse::<f64>().unwrap_or(f64::NAN)),
            }
        };
        self.push(TokenKind::Number(num), line, column);
        self.line_has_tokens = true;
        Ok(())
    }

    pub(super) fn lex_word(&mut self, line: usize, column: usize) -> Result<(), AzraError> {
        let mut word = String::new();
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            word.push(self.peek().unwrap());
            self.bump();
        }
        // `note:` comment (also usable inline after code).
        if word == "note" && self.peek() == Some(':') {
            while let Some(c) = self.peek() {
                if c == '\n' {
                    break;
                }
                self.bump();
            }
            return Ok(());
        }
        // Single-token English comparison operators containing `=`.
        if (word == "less_than" || word == "greater_than") && self.peek() == Some('=') {
            self.bump(); // consume '='
            let kind = if word == "less_than" {
                TokenKind::LessThanEq
            } else {
                TokenKind::GreaterThanEq
            };
            self.push(kind, line, column);
            self.line_has_tokens = true;
            return Ok(());
        }
        // Possessive `'s` glued to a word: `stats's "strength"`.
        // Only when `'s` is followed by whitespace, a quote or end of input,
        // so single-quoted strings (`'text'`) still lex normally.
        if self.peek() == Some('\'')
            && self.peek_at(1) == Some('s')
            && match self.peek_at(2) {
                None => true,
                Some(c) => c.is_whitespace() || c == '"' || c == '\'',
            }
        {
            self.push(TokenKind::Word(word), line, column);
            let (p_line, p_col) = (self.line, self.col);
            self.bump(); // '
            self.bump(); // s
            self.push(TokenKind::Possessive, p_line, p_col);
            self.line_has_tokens = true;
            return Ok(());
        }
        self.push(TokenKind::Word(word), line, column);
        self.line_has_tokens = true;
        Ok(())
    }
}

/// Validate and split the `{a.b.c}` interpolation body into a path.
fn parse_interpolation_path(body: &str, line: usize, column: usize) -> Result<Vec<String>, AzraError> {
    let body = body.trim();
    if body.is_empty() {
        return Err(AzraError::syntax_at(
            "empty `{}` interpolation — expected a variable name like `{name}`",
            line,
            column,
        ));
    }
    let mut path = Vec::new();
    for segment in body.split('.') {
        let mut chars = segment.chars();
        let valid = matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            return Err(AzraError::syntax_at(
                format!(
                    "invalid `{{{}}}` interpolation — paths must be identifiers like `player.health`",
                    body
                ),
                line,
                column,
            ));
        }
        path.push(segment.to_string());
    }
    Ok(path)
}
