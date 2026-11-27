//! Minimal TOML reader/writer for `azra.toml` and `azra.lock`.
//!
//! The crate deliberately has no dependencies, so only the small subset Azra
//! needs is supported: sections, `key = "string"`, booleans, and inline tables
//! such as `utils = { path = "../utils", version = "0.1.0" }`.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Bool(bool),
    Table(BTreeMap<String, String>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
}

/// A parsed file as `section -> key -> value`. Keys in the empty section are
/// stored under `""`. Repeated sections (e.g. `[[package]]`) are numbered:
/// `package`, `package#1`, ...
#[derive(Debug, Clone, Default)]
pub struct Toml {
    pub sections: BTreeMap<String, BTreeMap<String, Value>>,
}

impl Toml {
    pub fn get(&self, section: &str, key: &str) -> Option<&Value> {
        self.sections.get(section)?.get(key)
    }

    pub fn str(&self, section: &str, key: &str) -> Option<&str> {
        self.get(section, key).and_then(Value::as_str)
    }

    pub fn entries(&self, section: &str) -> Vec<&BTreeMap<String, Value>> {
        let mut out = Vec::new();
        if let Some(first) = self.sections.get(section) {
            out.push(first);
        }
        let mut i = 1;
        while let Some(next) = self.sections.get(&format!("{section}#{i}")) {
            out.push(next);
            i += 1;
        }
        out
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        let inner = &s[1..s.len() - 1];
        inner
            .replace("\\\"", "\"")
            .replace("\\n", "\n")
            .replace("\\\\", "\\")
    } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    let mut prev = '\0';
    for (i, ch) in line.char_indices() {
        match ch {
            '"' | '\'' if prev != '\\' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
            }
            '#' if quote.is_none() => return &line[..i],
            _ => {}
        }
        prev = ch;
    }
    line
}

fn parse_value(raw: &str) -> Value {
    let raw = raw.trim();
    if raw.starts_with('{') && raw.ends_with('}') {
        let mut table = BTreeMap::new();
        let inner = &raw[1..raw.len() - 1];
        for part in split_top_level(inner, ',') {
            if let Some((k, v)) = split_once(part, '=') {
                table.insert(k.trim().to_string(), unquote(v));
            }
        }
        Value::Table(table)
    } else if raw.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if raw.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else {
        Value::Str(unquote(raw))
    }
}

fn split_top_level(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' | '\'' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
            }
            '{' | '[' if quote.is_none() => depth += 1,
            '}' | ']' if quote.is_none() => depth = depth.saturating_sub(1),
            c if c == sep && depth == 0 && quote.is_none() => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

fn split_once(s: &str, sep: char) -> Option<(&str, &str)> {
    let mut quote = None;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' | '\'' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
            }
            c if c == sep && quote.is_none() => return Some((&s[..i], &s[i + 1..])),
            _ => {}
        }
    }
    None
}

pub fn parse(text: &str) -> Toml {
    let mut toml = Toml::default();
    let mut section = String::new();
    let mut repeated: BTreeMap<String, usize> = BTreeMap::new();
    for line in text.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("[[") && line.ends_with("]]") {
            let name = line[2..line.len() - 2].trim().to_string();
            let n = repeated.entry(name.clone()).or_insert(0);
            section = if *n == 0 { name } else { format!("{name}#{n}") };
            *n += 1;
            toml.sections.entry(section.clone()).or_default();
        } else if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            toml.sections.entry(section.clone()).or_default();
        } else if let Some((key, raw)) = split_once(line, '=') {
            toml.sections
                .entry(section.clone())
                .or_default()
                .insert(key.trim().to_string(), parse_value(raw));
        }
    }
    toml
}

pub fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

// --------------------------------------------------------------- Azra manifest

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub entry: String,
    pub dependencies: Vec<Dependency>,
}

impl Manifest {
    pub fn parse(text: &str) -> Self {
        let toml = parse(text);
        let dependencies = toml
            .entries("dependencies")
            .into_iter()
            .flat_map(|table| {
                table.iter().map(|(name, value)| Dependency {
                    name: name.clone(),
                    version: match value {
                        Value::Str(v) => Some(v.clone()),
                        Value::Table(t) => t.get("version").cloned(),
                        Value::Bool(_) => None,
                    },
                    path: match value {
                        Value::Table(t) => t.get("path").map(PathBuf::from),
                        _ => None,
                    },
                })
            })
            .collect();
        Manifest {
            name: toml
                .str("package", "name")
                .unwrap_or("untitled")
                .to_string(),
            version: toml
                .str("package", "version")
                .unwrap_or("0.1.0")
                .to_string(),
            description: toml.str("package", "description").unwrap_or("").to_string(),
            entry: toml
                .str("package", "entry")
                .unwrap_or("main.vibe")
                .to_string(),
            dependencies,
        }
    }

    pub fn render(&self) -> String {
        let mut out = format!(
            "[package]\nname = {}\nversion = {}\ndescription = {}\nentry = {}\n",
            quote(&self.name),
            quote(&self.version),
            quote(&self.description),
            quote(&self.entry)
        );
        if !self.dependencies.is_empty() {
            out.push_str("\n[dependencies]\n");
            for dep in &self.dependencies {
                match (&dep.path, &dep.version) {
                    (Some(path), Some(ver)) => out.push_str(&format!(
                        "{} = {{ path = {}, version = {} }}\n",
                        dep.name,
                        quote(&path.to_string_lossy()),
                        quote(ver)
                    )),
                    (Some(path), None) => out.push_str(&format!(
                        "{} = {{ path = {} }}\n",
                        dep.name,
                        quote(&path.to_string_lossy())
                    )),
                    (None, Some(ver)) => out.push_str(&format!("{} = {}\n", dep.name, quote(ver))),
                    (None, None) => out.push_str(&format!("{} = \"*\"\n", dep.name)),
                }
            }
        }
        out
    }
}

/// `1.2` means `>=1.2.0, <2.0.0`; `*` accepts everything.
pub fn version_matches(requirement: &str, version: &str) -> bool {
    let req = requirement.trim().trim_start_matches(['^', '~', '=']);
    if req.is_empty() || req == "*" {
        return true;
    }
    let rv = version_numbers(req);
    let vv = version_numbers(version);
    match (rv, vv) {
        (Some((major, minor, _)), Some((vmajor, vminor, vpatch))) => {
            vmajor == major && (vminor, vpatch) >= (minor, 0)
        }
        _ => requirement == version,
    }
}

fn version_numbers(s: &str) -> Option<(u64, u64, u64)> {
    let mut parts = s.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_subset() {
        let src = "[package]\nname = \"demo\" # comment\nversion = \"1.2.3\"\n\n\
                   [dependencies]\nmath = \"1.0\"\nutils = { path = \"../utils\" }\n";
        let m = Manifest::parse(src);
        assert_eq!(m.name, "demo");
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(m.dependencies[0].version.as_deref(), Some("1.0"));
        assert_eq!(
            m.dependencies[1].path.as_deref(),
            Some(std::path::Path::new("../utils"))
        );
        assert!(Manifest::parse(&m.render()).name == "demo");
    }

    #[test]
    fn version_requirements() {
        assert!(version_matches("1.0", "1.4.2"));
        assert!(!version_matches("1.0", "2.0.0"));
        assert!(version_matches("*", "9.9.9"));
        assert!(version_matches("0.1", "0.1.7"));
    }
}
