//! Minimal hand-written JSON codec (objects, arrays, strings, numbers,
//! `true`/`false`/`null`). No external dependencies.

use crate::error::AzraError;
use crate::token::Num;
use crate::value::{format_float, Value};

const MAX_JSON_DEPTH: usize = 128;

/// Parse JSON text into Azra runtime values.
pub fn parse_json(text: &str, line: usize) -> Result<Value, AzraError> {
    let mut p = JsonParser {
        bytes: text.as_bytes(),
        pos: 0,
        line,
        depth: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.bytes.len() {
        return Err(AzraError::runtime_at(
            format!("unexpected trailing JSON at position {}", p.pos),
            line,
        ));
    }
    Ok(value)
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    depth: usize,
}

impl JsonParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn err(&self, msg: impl Into<String>) -> AzraError {
        AzraError::runtime_at(
            format!("invalid JSON: {} (at byte {})", msg.into(), self.pos),
            self.line,
        )
    }

    fn parse_value(&mut self) -> Result<Value, AzraError> {
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Value::Str(self.parse_string()?)),
            Some(b't') => self.expect_lit("true").map(|_| Value::Bool(true)),
            Some(b'f') => self.expect_lit("false").map(|_| Value::Bool(false)),
            Some(b'n') => self.expect_lit("null").map(|_| Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(self.err(format!("unexpected character `{}`", c as char))),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn expect_lit(&mut self, lit: &str) -> Result<(), AzraError> {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(self.err(format!("expected `{}`", lit)))
        }
    }

    fn parse_object(&mut self) -> Result<Value, AzraError> {
        if self.depth >= MAX_JSON_DEPTH {
            return Err(self.err(format!("JSON nesting exceeds max depth of {}", MAX_JSON_DEPTH)));
        }
        self.depth += 1;
        let res = self.parse_object_inner();
        self.depth -= 1;
        res
    }

    fn parse_object_inner(&mut self) -> Result<Value, AzraError> {
        self.pos += 1; // '{'
        let mut entries = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::map(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected a quoted object key"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.err("expected `:` after an object key"));
            }
            self.pos += 1;
            self.skip_ws();
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::map(entries));
                }
                _ => return Err(self.err("expected `,` or `}` in object")),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Value, AzraError> {
        if self.depth >= MAX_JSON_DEPTH {
            return Err(self.err(format!("JSON nesting exceeds max depth of {}", MAX_JSON_DEPTH)));
        }
        self.depth += 1;
        let res = self.parse_array_inner();
        self.depth -= 1;
        res
    }

    fn parse_array_inner(&mut self) -> Result<Value, AzraError> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::list(items));
        }
        loop {
            self.skip_ws();
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::list(items));
                }
                _ => return Err(self.err("expected `,` or `]` in array")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, AzraError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some(b'"') => {
                            out.push('"');
                            self.pos += 1;
                        }
                        Some(b'\\') => {
                            out.push('\\');
                            self.pos += 1;
                        }
                        Some(b'/') => {
                            out.push('/');
                            self.pos += 1;
                        }
                        Some(b'n') => {
                            out.push('\n');
                            self.pos += 1;
                        }
                        Some(b't') => {
                            out.push('\t');
                            self.pos += 1;
                        }
                        Some(b'r') => {
                            out.push('\r');
                            self.pos += 1;
                        }
                        Some(b'b') => {
                            out.push('\u{0008}');
                            self.pos += 1;
                        }
                        Some(b'f') => {
                            out.push('\u{000C}');
                            self.pos += 1;
                        }
                        Some(b'u') => {
                            self.pos += 1;
                            let hex = self
                                .bytes
                                .get(self.pos..self.pos + 4)
                                .ok_or_else(|| self.err("truncated \\u escape"))?;
                            let code = u32::from_str_radix(
                                std::str::from_utf8(hex).map_err(|_| self.err("bad \\u escape"))?,
                                16,
                            )
                            .map_err(|_| self.err("bad \\u escape"))?;
                            self.pos += 4;
                            out.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        }
                        _ => return Err(self.err("unknown escape sequence")),
                    }
                }
                Some(_) => {
                    // Copy one UTF-8 character.
                    let rest = &self.bytes[self.pos..];
                    let s = std::str::from_utf8(rest).map_err(|_| self.err("invalid UTF-8"))?;
                    let c = s.chars().next().unwrap();
                    out.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
    }

    fn parse_number(&mut self) -> Result<Value, AzraError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.err("bad number"))?;
        if is_float {
            text.parse::<f64>()
                .map(|f| Value::Number(Num::Float(f)))
                .map_err(|_| self.err("bad number"))
        } else {
            text.parse::<i64>()
                .map(|i| Value::Number(Num::Int(i)))
                .or_else(|_| {
                    text.parse::<f64>()
                        .map(|f| Value::Number(Num::Float(f)))
                })
                .map_err(|_| self.err("bad number"))
        }
    }
}

/// Serialize a value as JSON text (rejects cyclic / overly deep structures).
pub(super) fn try_to_json(v: &Value, line: usize) -> Result<String, AzraError> {
    let mut out = String::new();
    write_json_depth(v, &mut out, 0).map_err(|msg| AzraError::runtime_at(msg, line))?;
    Ok(out)
}

/// Serialize a value as JSON text.
pub fn to_json(v: &Value) -> String {
    let mut out = String::new();
    if write_json_depth(v, &mut out, 0).is_err() {
        return "\"<max depth exceeded>\"".to_string();
    }
    out
}

fn write_json_depth(v: &Value, out: &mut String, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err("`to json` exceeded maximum nesting depth (possible cycle)".to_string());
    }
    match v {
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::Str(s) => write_json_string(s, out),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Null => out.push_str("null"),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json_depth(item, out, depth + 1)?;
            }
            out.push(']');
        }
        Value::Map(m) => {
            out.push('{');
            for (i, (k, val)) in m.borrow().entries.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json_string(k, out);
                out.push(':');
                write_json_depth(val, out, depth + 1)?;
            }
            out.push('}');
        }
        Value::Object(o) => {
            // Objects serialize as a map of their fields (plus type/id).
            out.push('{');
            let o = o.borrow();
            out.push_str("\"type\":");
            write_json_string(&o.type_name, out);
            out.push_str(",\"id\":");
            out.push_str(&o.id.to_string());
            for (k, val) in &o.fields {
                out.push(',');
                write_json_string(k, out);
                out.push(':');
                write_json_depth(val, out, depth + 1)?;
            }
            out.push('}');
        }
        Value::Vec3(v) => {
            out.push_str(&format!(
                "{{\"x\":{},\"y\":{},\"z\":{}}}",
                format_float(v.x),
                format_float(v.y),
                format_float(v.z)
            ));
        }
        Value::Color(c) => {
            out.push_str(&format!("{{\"r\":{},\"g\":{},\"b\":{}}}", c.r, c.g, c.b));
        }
        Value::Function(f) => {
            write_json_string(&format!("<function {}>", f.name), out);
        }
    }
    Ok(())
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
