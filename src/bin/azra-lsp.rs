//! Azra language server — JSON-RPC 2.0 over stdio, no external crates.
//! Provides diagnostics, completion, hover, go-to-definition and formatting.

use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};

// --------------------------------------------------------------- tiny JSON

#[derive(Debug, Clone)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(map) => map.get(key),
            _ => None,
        }
    }
    fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    fn array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }
    fn render(&self) -> String {
        match self {
            Json::Null => "null".to_string(),
            Json::Bool(b) => b.to_string(),
            Json::Num(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Json::Str(s) => {
                let mut out = String::from("\"");
                for ch in s.chars() {
                    match ch {
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
                out
            }
            Json::Arr(items) => format!(
                "[{}]",
                items.iter().map(Json::render).collect::<Vec<_>>().join(",")
            ),
            Json::Obj(map) => format!(
                "{{{}}}",
                map.iter()
                    .map(|(k, v)| format!("{}:{}", Json::Str(k.clone()).render(), v.render()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

struct JsonParser {
    chars: Vec<char>,
    pos: usize,
}

impl JsonParser {
    fn new(text: &str) -> Self {
        JsonParser {
            chars: text.chars().collect(),
            pos: 0,
        }
    }

    fn skip(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn parse(mut self) -> Result<Json, String> {
        let value = self.value()?;
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, String> {
        self.skip();
        match self.chars.get(self.pos).copied() {
            Some('"') => Ok(Json::Str(self.string()?)),
            Some('{') => self.object(),
            Some('[') => self.array(),
            Some('t') => {
                self.literal("true")?;
                Ok(Json::Bool(true))
            }
            Some('f') => {
                self.literal("false")?;
                Ok(Json::Bool(false))
            }
            Some('n') => {
                self.literal("null")?;
                Ok(Json::Null)
            }
            Some(c) if c == '-' || c.is_ascii_digit() => self.number(),
            other => Err(format!("unexpected JSON token {other:?}")),
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        for expected in word.chars() {
            if self.chars.get(self.pos) != Some(&expected) {
                return Err(format!("expected {word}"));
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1;
        let mut out = String::new();
        while let Some(&c) = self.chars.get(self.pos) {
            self.pos += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let esc = self.chars.get(self.pos).copied().ok_or("bad escape")?;
                    self.pos += 1;
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let hex: String = self.chars[self.pos..self.pos + 4].iter().collect();
                            let code = u32::from_str_radix(&hex, 16).map_err(|e| e.to_string())?;
                            self.pos += 4;
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                            }
                        }
                        other => return Err(format!("bad escape \\{other}")),
                    }
                }
                other => out.push(other),
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn object(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut map = BTreeMap::new();
        loop {
            self.skip();
            if self.chars.get(self.pos) == Some(&'}') {
                self.pos += 1;
                return Ok(Json::Obj(map));
            }
            let key = self.string()?;
            self.skip();
            if self.chars.get(self.pos) != Some(&':') {
                return Err("expected ':' in object".to_string());
            }
            self.pos += 1;
            let value = self.value()?;
            map.insert(key, value);
            self.skip();
            match self.chars.get(self.pos) {
                Some(',') => self.pos += 1,
                Some('}') => {
                    self.pos += 1;
                    return Ok(Json::Obj(map));
                }
                _ => return Err("expected ',' or '}'".to_string()),
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip();
            if self.chars.get(self.pos) == Some(&']') {
                self.pos += 1;
                return Ok(Json::Arr(items));
            }
            items.push(self.value()?);
            self.skip();
            match self.chars.get(self.pos) {
                Some(',') => self.pos += 1,
                Some(']') => {
                    self.pos += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err("expected ',' or ']'".to_string()),
            }
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.pos;
        while let Some(&c) = self.chars.get(self.pos) {
            if c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E') {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map(Json::Num)
            .map_err(|e| e.to_string())
    }
}

fn obj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn send(message: &Json) {
    let body = message.render();
    let mut stdout = io::stdout();
    let _ = write!(stdout, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = stdout.flush();
}

// --------------------------------------------------------------- LSP state

struct Document {
    uri: String,
    text: String,
}

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut documents: BTreeMap<String, Document> = BTreeMap::new();

    loop {
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            if input.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            if line.trim().is_empty() {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
        if content_length == 0 {
            continue;
        }
        let mut body = vec![0u8; content_length];
        if input.read_exact(&mut body).is_err() {
            return;
        }
        let text = String::from_utf8_lossy(&body);
        let request = match JsonParser::new(&text).parse() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let id = request.get("id").cloned();
        let method = request
            .get("method")
            .and_then(Json::str)
            .unwrap_or("")
            .to_string();
        let params = request.get("params").cloned().unwrap_or(Json::Null);

        match method.as_str() {
            "initialize" => {
                let capabilities = obj(vec![
                    ("hoverProvider", Json::Bool(true)),
                    ("definitionProvider", Json::Bool(true)),
                    (
                        "completionProvider",
                        obj(vec![("triggerCharacters", Json::Arr(vec![]))]),
                    ),
                    ("documentFormattingProvider", Json::Bool(true)),
                    (
                        "textDocumentSync",
                        obj(vec![
                            ("openClose", Json::Bool(true)),
                            ("change", Json::Num(1.0)),
                            ("save", Json::Bool(true)),
                        ]),
                    ),
                ]);
                if let Some(id) = id {
                    send(&obj(vec![
                        ("jsonrpc", Json::Str("2.0".into())),
                        ("id", id),
                        ("result", obj(vec![("capabilities", capabilities)])),
                    ]));
                }
            }
            "initialized" | "shutdown" => {
                if let Some(id) = id {
                    send(&obj(vec![
                        ("jsonrpc", Json::Str("2.0".into())),
                        ("id", id),
                        ("result", Json::Null),
                    ]));
                }
                if method == "shutdown" {
                    return;
                }
            }
            "exit" => return,
            "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
                let doc = params.get("textDocument").or_else(|| params.get("item"));
                let Some(uri) = doc.and_then(|d| d.get("uri")).and_then(Json::str) else {
                    continue;
                };
                let text = if method.ends_with("didOpen") {
                    doc.and_then(|d| d.get("text")).and_then(Json::str)
                } else if method.ends_with("didChange") {
                    params
                        .get("contentChanges")
                        .and_then(Json::array)
                        .and_then(|changes| changes.first())
                        .and_then(|c| c.get("text"))
                        .and_then(Json::str)
                } else {
                    None
                };
                if let Some(text) = text {
                    documents.insert(
                        uri.to_string(),
                        Document {
                            uri: uri.to_string(),
                            text: text.to_string(),
                        },
                    );
                }
                if let Some(doc) = documents.get(uri) {
                    publish_diagnostics(doc);
                }
            }
            "textDocument/completion" => {
                let Some(id) = id else { continue };
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(Json::str)
                    .unwrap_or("");
                let text = documents.get(uri).map(|d| d.text.as_str()).unwrap_or("");
                let items = completions(text);
                send(&obj(vec![
                    ("jsonrpc", Json::Str("2.0".into())),
                    ("id", id),
                    ("result", Json::Arr(items)),
                ]));
            }
            "textDocument/hover" => {
                let Some(id) = id else { continue };
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(Json::str)
                    .unwrap_or("");
                let line = params
                    .get("position")
                    .and_then(|p| p.get("line"))
                    .and_then(|n| match n {
                        Json::Num(v) => Some(*v as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                let character = params
                    .get("position")
                    .and_then(|p| p.get("character"))
                    .and_then(|n| match n {
                        Json::Num(v) => Some(*v as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                let text = documents.get(uri).map(|d| d.text.as_str()).unwrap_or("");
                let word = word_at(text, line, character);
                let result = documentation(word).map(|doc| {
                    obj(vec![
                        (
                            "contents",
                            obj(vec![
                                ("kind", Json::Str("markdown".into())),
                                ("value", Json::Str(format!("`{word}` — {doc}"))),
                            ]),
                        ),
                        ("range", word_range(text, line, character)),
                    ])
                });
                send(&obj(vec![
                    ("jsonrpc", Json::Str("2.0".into())),
                    ("id", id),
                    ("result", result.unwrap_or(Json::Null)),
                ]));
            }
            "textDocument/definition" => {
                let Some(id) = id else { continue };
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(Json::str)
                    .unwrap_or("")
                    .to_string();
                let line = params
                    .get("position")
                    .and_then(|p| p.get("line"))
                    .and_then(|n| match n {
                        Json::Num(v) => Some(*v as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                let character = params
                    .get("position")
                    .and_then(|p| p.get("character"))
                    .and_then(|n| match n {
                        Json::Num(v) => Some(*v as usize),
                        _ => None,
                    })
                    .unwrap_or(0);
                let text = documents.get(&uri).map(|d| d.text.as_str()).unwrap_or("");
                let word = word_at(text, line, character);
                let found = find_definition(text, word).map(|(l, c)| {
                    obj(vec![
                        ("uri", Json::Str(uri.clone())),
                        (
                            "range",
                            obj(vec![
                                (
                                    "start",
                                    obj(vec![
                                        ("line", Json::Num(l as f64)),
                                        ("character", Json::Num(c as f64)),
                                    ]),
                                ),
                                (
                                    "end",
                                    obj(vec![
                                        ("line", Json::Num(l as f64)),
                                        ("character", Json::Num((c + word.len()) as f64)),
                                    ]),
                                ),
                            ]),
                        ),
                    ])
                });
                send(&obj(vec![
                    ("jsonrpc", Json::Str("2.0".into())),
                    ("id", id),
                    ("result", Json::Arr(found.into_iter().collect())),
                ]));
            }
            "textDocument/formatting" => {
                let Some(id) = id else { continue };
                let uri = params
                    .get("textDocument")
                    .and_then(|d| d.get("uri"))
                    .and_then(Json::str)
                    .unwrap_or("");
                let text = documents.get(uri).map(|d| d.text.as_str()).unwrap_or("");
                let formatted = azra::format::format_source(text);
                let lines = text.lines().count().max(1);
                let edit = obj(vec![
                    (
                        "range",
                        obj(vec![
                            (
                                "start",
                                obj(vec![
                                    ("line", Json::Num(0.0)),
                                    ("character", Json::Num(0.0)),
                                ]),
                            ),
                            (
                                "end",
                                obj(vec![
                                    ("line", Json::Num(lines as f64)),
                                    ("character", Json::Num(0.0)),
                                ]),
                            ),
                        ]),
                    ),
                    ("newText", Json::Str(formatted)),
                ]);
                send(&obj(vec![
                    ("jsonrpc", Json::Str("2.0".into())),
                    ("id", id),
                    ("result", Json::Arr(vec![edit])),
                ]));
            }
            _ => {}
        }
    }
}

fn publish_diagnostics(doc: &Document) {
    let tokens = azra::lexer::tokenize(&doc.text);
    let error = match &tokens {
        Err(err) => Some(err.clone()),
        Ok(tokens) => azra::parser::parse(tokens).err(),
    };
    let diagnostics = match error {
        Some(err) => {
            let line = err.line.unwrap_or(1).saturating_sub(1);
            let column = err.column.unwrap_or(1).saturating_sub(1);
            vec![obj(vec![
                (
                    "range",
                    obj(vec![
                        (
                            "start",
                            obj(vec![
                                ("line", Json::Num(line as f64)),
                                ("character", Json::Num(column as f64)),
                            ]),
                        ),
                        (
                            "end",
                            obj(vec![
                                ("line", Json::Num(line as f64)),
                                ("character", Json::Num((column + 1) as f64)),
                            ]),
                        ),
                    ]),
                ),
                ("severity", Json::Num(1.0)),
                ("source", Json::Str("azra".into())),
                ("message", Json::Str(err.to_string())),
            ])]
        }
        None => Vec::new(),
    };
    send(&obj(vec![
        ("jsonrpc", Json::Str("2.0".into())),
        (
            "method",
            Json::Str("textDocument/publishDiagnostics".into()),
        ),
        (
            "params",
            obj(vec![
                ("uri", Json::Str(doc.uri.clone())),
                ("diagnostics", Json::Arr(diagnostics)),
            ]),
        ),
    ]));
}

const COMPLETIONS: &[&str] = &[
    "set",
    "make",
    "add",
    "remove",
    "subtract",
    "multiply",
    "divide",
    "say",
    "warn",
    "error",
    "if",
    "otherwise",
    "repeat",
    "for each",
    "while",
    "define",
    "give back",
    "when",
    "use",
    "try",
    "fail",
    "sort",
    "reverse",
    "clear",
    "stop loop",
    "skip next",
    "assert",
    "test",
    "equals",
    "not_equals",
    "less_than",
    "less_than=",
    "greater_than",
    "greater_than=",
    "and",
    "or",
    "not",
    "contains",
    "starts with",
    "has key",
    "modulo",
    "power",
    "pi",
    "e",
    "true",
    "false",
    "null",
    "empty list",
    "empty map",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "log",
    "log10",
    "exp",
    "lerp",
    "sqrt",
    "abs",
    "round",
    "floor",
    "ceil",
    "min",
    "max",
    "clamp",
    "random",
    "length",
    "vec3",
    "color",
    "dot",
    "cross",
    "magnitude",
    "normalize",
    "distance",
    "angle",
    "degrees",
    "radians",
    "say",
];

fn completion_kind(word: &str) -> f64 {
    if matches!(
        word,
        "set"
            | "make"
            | "if"
            | "otherwise"
            | "repeat"
            | "for each"
            | "while"
            | "define"
            | "give back"
            | "when"
            | "use"
            | "try"
            | "fail"
            | "assert"
            | "test"
    ) {
        14.0
    } else {
        3.0
    }
}

fn completions(text: &str) -> Vec<Json> {
    let mut words: Vec<String> = COMPLETIONS.iter().map(|s| s.to_string()).collect();
    for line in text.lines() {
        let trimmed = line.trim();
        for prefix in ["define ", "set "] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && !words.contains(&name) {
                    words.push(name);
                }
            }
        }
    }
    words
        .into_iter()
        .map(|word| {
            obj(vec![
                ("label", Json::Str(word.clone())),
                ("kind", Json::Num(completion_kind(&word))),
                (
                    "detail",
                    Json::Str(documentation(&word).unwrap_or("Azra").to_string()),
                ),
            ])
        })
        .collect()
}

fn documentation(word: &str) -> Option<&'static str> {
    Some(match word {
        "set" => "Create or update a variable, property, map key or list item.",
        "make" => "Spawn a mock engine object: `make rounded_box as lava_block`.",
        "say" => "Print values to stdout, joined by spaces.",
        "warn" | "error" => "Print a warning/error line to stderr.",
        "if" => "Conditional block; pair with `otherwise if` / `otherwise`.",
        "repeat" => "Loop `N` times, or `repeat with i from a to b by step`.",
        "for each" => "Iterate over a list: `for each item in items:`.",
        "while" => "Loop while a condition is true.",
        "define" => "Define a function: `define name with a, b:`.",
        "give back" => "Return a value from a function.",
        "assert" => "Test assertion: `assert equals a, b`.",
        "test" => "Register a test case: `test \"name\":`.",
        "use" => "Load a module, std module or installed package.",
        "try" => "Catch `fail with` errors with `if error as err:`.",
        "pi" | "e" => "Mathematical constant.",
        "dot" => "Scalar product: `dot of a and b`.",
        "cross" => "Vector product: `cross of a and b`.",
        "magnitude" | "normalize" | "distance" | "angle" => "3D vector operation.",
        "lerp" => "Linear interpolation: `lerp a to b by t` (numbers or vec3).",
        "log" => "Logarithm: `log of x` or `log base b of x`.",
        "degrees" | "radians" => "Angle conversion: `degrees to radians of x`.",
        _ => return None,
    })
}

fn word_at(text: &str, line: usize, character: usize) -> &str {
    let Some(source_line) = text.lines().nth(line) else {
        return "";
    };
    let chars: Vec<char> = source_line.chars().collect();
    if character >= chars.len() {
        return "";
    }
    let mut start = character;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = character;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    // char indices are not byte offsets when comments contain Unicode.
    let byte_start: usize = chars[..start].iter().map(|c| c.len_utf8()).sum();
    let byte_len: usize = chars[start..end].iter().map(|c| c.len_utf8()).sum();
    &source_line[byte_start..byte_start + byte_len]
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn word_range(text: &str, line: usize, character: usize) -> Json {
    let word_len = word_at(text, line, character).len();
    let start = character.saturating_sub(word_len);
    obj(vec![
        (
            "start",
            obj(vec![
                ("line", Json::Num(line as f64)),
                ("character", Json::Num(start as f64)),
            ]),
        ),
        (
            "end",
            obj(vec![
                ("line", Json::Num(line as f64)),
                ("character", Json::Num(character as f64)),
            ]),
        ),
    ])
}

fn find_definition(text: &str, word: &str) -> Option<(usize, usize)> {
    if word.is_empty() {
        return None;
    }
    for (line_no, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        for prefix in ["define ", "set "] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                if rest.starts_with(word)
                    && rest[word.len()..]
                        .chars()
                        .next()
                        .is_none_or(|c| !is_word_char(c))
                {
                    let col = line.find(word).unwrap_or(0);
                    return Some((line_no, col));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let value = JsonParser::new("{\"a\":[1,true,\"x\"],\"b\":null}")
            .parse()
            .unwrap();
        assert!(value.get("a").and_then(Json::array).is_some());
        assert_eq!(
            JsonParser::new(&value.render()).parse().unwrap().render(),
            value.render()
        );
    }

    #[test]
    fn finds_definitions() {
        let src = "define greet with name:\n    say name\nset score to 1\n";
        assert_eq!(find_definition(src, "greet"), Some((0, 7)));
        assert_eq!(find_definition(src, "score"), Some((2, 4)));
    }
}
