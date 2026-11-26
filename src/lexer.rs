//! Tokenizer for Azra: line/column tracking, strings, numbers,
//! `note:` comments, trailing periods, and the INDENT/DEDENT stack.

use crate::error::AzraError;
use crate::token::{Num, Token, TokenKind};

/// Tokenize a whole `.vibe` source string.
pub fn tokenize(source: &str) -> Result<Vec<Token>, AzraError> {
    Lexer::new(source).run()
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    /// Indentation stack, always starts with `0`.
    indent_stack: Vec<usize>,
    tokens: Vec<Token>,
    at_line_start: bool,
    /// Whether the current line produced any real token.
    line_has_tokens: bool,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            tokens: Vec::new(),
            at_line_start: true,
            line_has_tokens: false,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    /// Consume one character, advancing the column.
    fn bump(&mut self) {
        self.pos += 1;
        self.col += 1;
    }

    /// Consume a newline, moving to the next line.
    fn bump_newline(&mut self) {
        self.pos += 1;
        self.line += 1;
        self.col = 1;
        self.at_line_start = true;
    }

    fn push(&mut self, kind: TokenKind, line: usize, column: usize) {
        self.tokens.push(Token::new(kind, line, column));
    }

    fn run(mut self) -> Result<Vec<Token>, AzraError> {
        while self.pos < self.chars.len() {
            if self.at_line_start {
                self.handle_line_start()?;
                continue;
            }
            self.lex_token()?;
        }

        // Flush the final line and any open blocks.
        if self.line_has_tokens {
            let (line, col) = (self.line, self.col);
            self.push(TokenKind::Newline, line, col);
            self.line_has_tokens = false;
        }
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            let (line, col) = (self.line, self.col);
            self.push(TokenKind::Dedent, line, col);
        }
        let (line, col) = (self.line, self.col);
        self.push(TokenKind::Eof, line, col);
        Ok(self.tokens)
    }

    /// Process indentation, blank lines and comment-only lines.
    fn handle_line_start(&mut self) -> Result<(), AzraError> {
        let line = self.line;
        let mut spaces = 0usize;
        while self.peek() == Some(' ') {
            self.bump();
            spaces += 1;
        }
        if self.peek() == Some('\t') {
            return Err(AzraError::syntax_at(
                "tabs are not allowed; Azra blocks use 4-space indentation",
                self.line,
                self.col,
            ));
        }
        match self.peek() {
            // Blank line: no tokens, no indentation change.
            None => {
                self.at_line_start = false;
                self.line_has_tokens = false;
            }
            Some('\n') => {
                self.bump_newline();
                self.line_has_tokens = false;
            }
            Some('\r') => {
                self.bump();
            }
            _ => {
                // Comment-only line: ignored entirely (indentation untouched).
                if self.starts_with_comment() {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    if self.peek() == Some('\n') {
                        self.bump_newline();
                    } else {
                        self.at_line_start = false;
                    }
                    self.line_has_tokens = false;
                    return Ok(());
                }
                self.process_indent(spaces, line)?;
                self.at_line_start = false;
                self.line_has_tokens = false;
            }
        }
        Ok(())
    }

    fn starts_with_comment(&self) -> bool {
        self.peek() == Some('n')
            && self.peek_at(1) == Some('o')
            && self.peek_at(2) == Some('t')
            && self.peek_at(3) == Some('e')
            && self.peek_at(4) == Some(':')
    }

    /// Maintain the INDENT/DEDENT stack (4-space steps only).
    fn process_indent(&mut self, spaces: usize, line: usize) -> Result<(), AzraError> {
        let top = *self.indent_stack.last().expect("indent stack non-empty");
        if spaces > top {
            if spaces != top + 4 {
                return Err(AzraError::syntax_at(
                    format!(
                        "unexpected indentation: expected {} spaces at the start of this line, found {}",
                        top + 4,
                        spaces
                    ),
                    line,
                    spaces + 1,
                ));
            }
            self.indent_stack.push(spaces);
            self.push(TokenKind::Indent, line, spaces + 1);
        } else if spaces < top {
            while let Some(&t) = self.indent_stack.last() {
                if t > spaces {
                    self.indent_stack.pop();
                    self.push(TokenKind::Dedent, line, spaces + 1);
                } else {
                    break;
                }
            }
            if *self.indent_stack.last().expect("indent stack non-empty") != spaces {
                return Err(AzraError::syntax_at(
                    "inconsistent indentation: this line does not match any enclosing block",
                    line,
                    spaces + 1,
                ));
            }
        }
        Ok(())
    }

    fn lex_token(&mut self) -> Result<(), AzraError> {
        while self.peek() == Some(' ') {
            self.bump();
        }
        let line = self.line;
        let column = self.col;
        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(()),
        };
        match c {
            '\n' => {
                self.bump_newline();
                self.push(TokenKind::Newline, line, column);
                // The newline ends the current line's token count.
                self.line_has_tokens = false;
            }
            '\r' => {
                self.bump();
            }
            '+' => self.single(TokenKind::Plus, line, column),
            '-' => self.single(TokenKind::Minus, line, column),
            '*' => self.single(TokenKind::Star, line, column),
            '/' => self.single(TokenKind::Slash, line, column),
            ',' => self.single(TokenKind::Comma, line, column),
            ':' => self.single(TokenKind::Colon, line, column),
            '.' => self.single(TokenKind::Dot, line, column),
            '"' | '\'' => self.lex_string(line, column)?,
            '0'..='9' => self.lex_number(line, column)?,
            '=' => {
                return Err(AzraError::syntax_at(
                    "unexpected `=`: Azra uses English operators — use `equals`, or `less_than=` / `greater_than=` with no space",
                    line,
                    column,
                ));
            }
            c if c.is_ascii_alphabetic() || c == '_' => self.lex_word(line, column)?,
            c => {
                return Err(AzraError::syntax_at(
                    format!("unexpected character `{}`", c),
                    line,
                    column,
                ));
            }
        }
        Ok(())
    }

    fn single(&mut self, kind: TokenKind, line: usize, column: usize) {
        self.bump();
        self.push(kind, line, column);
        self.line_has_tokens = true;
    }

    fn lex_string(&mut self, line: usize, column: usize) -> Result<(), AzraError> {
        let quote = self.peek().expect("string starts with a quote");
        self.bump(); // opening quote
        let mut out = String::new();
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
                            out.push('\n');
                            self.bump();
                        }
                        Some('t') => {
                            out.push('\t');
                            self.bump();
                        }
                        Some('r') => {
                            out.push('\r');
                            self.bump();
                        }
                        Some('\\') => {
                            out.push('\\');
                            self.bump();
                        }
                        Some('"') => {
                            out.push('"');
                            self.bump();
                        }
                        Some('\'') => {
                            out.push('\'');
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
                Some(c) if c == quote => {
                    self.bump();
                    break;
                }
                Some(c) => {
                    out.push(c);
                    self.bump();
                }
            }
        }
        self.push(TokenKind::Str(out), line, column);
        self.line_has_tokens = true;
        Ok(())
    }

    fn lex_number(&mut self, line: usize, column: usize) -> Result<(), AzraError> {
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

    fn lex_word(&mut self, line: usize, column: usize) -> Result<(), AzraError> {
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
        self.push(TokenKind::Word(word), line, column);
        self.line_has_tokens = true;
        Ok(())
    }
}
