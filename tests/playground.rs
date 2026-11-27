//! Tests for the shared bridge used by the WASM playground.

#[test]
fn run_source_collects_output() {
    let out = azra::run_source("say \"hello\"\nsay 2 + 2\n").unwrap();
    assert_eq!(out, "hello\n4");
}

#[test]
fn run_source_reports_errors_with_prior_output() {
    let err = azra::run_source("say \"before\"\nsay 1 / 0\n").unwrap_err();
    assert!(err.contains("before"), "{err}");
    assert!(err.contains("division by zero"), "{err}");
}

#[test]
fn formatter_uses_four_space_blocks() {
    let src = "if true:\n  say 1\notherwise:\n    say 2\n";
    assert_eq!(
        azra::format::format_source(src),
        "if true:\n    say 1\notherwise:\n    say 2\n"
    );
}
