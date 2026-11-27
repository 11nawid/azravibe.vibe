//! Source formatter shared by `azra fmt` and the language server.
//!
//! Azra has no brackets, so indentation *is* the structure: this formatter
//! normalizes every block to exactly four spaces, removes trailing whitespace,
//! expands tabs and keeps comment-only lines aligned with their block.

/// Format Azra source text. The result always ends with one newline.
pub fn format_source(source: &str) -> String {
    let mut lines = Vec::new();
    let mut level = 0usize;
    for raw in source.lines() {
        let line = raw.replace('\t', "    ");
        let trimmed = line.trim();
        if trimmed.is_empty() {
            lines.push(String::new());
            continue;
        }
        // `otherwise` / `if error as` close the previous branch before opening
        // a sibling one.
        if trimmed.starts_with("otherwise") || trimmed.starts_with("if error as") {
            level = level.saturating_sub(1);
        }
        lines.push(format!("{}{}", "    ".repeat(level), trimmed));
        if trimmed.ends_with(':') {
            level += 1;
        }
    }
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// True when `format_source(source)` would leave the source unchanged.
pub fn is_formatted(source: &str) -> bool {
    source == format_source(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_indentation_and_whitespace() {
        let src = "if true:\n  say 1   \notherwise:\n    say 2\n";
        let expected = "if true:\n    say 1\notherwise:\n    say 2\n";
        assert_eq!(format_source(src), expected);
        assert!(!is_formatted(src));
        assert!(is_formatted(expected));
    }

    #[test]
    fn keeps_blank_lines_and_comments() {
        let src = "note: hi\n\nsay 1\n";
        assert_eq!(format_source(src), src);
    }
}
