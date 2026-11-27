//! Security, sandbox, resource-cap, cycle, and adversarial-input tests.

mod common;

use azra::{AzraEngine, SandboxConfig, parser};
use common::err;

#[test]
fn cyclic_containers_do_not_overflow_stack_on_render_or_json() {
    let mut engine = AzraEngine::new();
    engine
        .eval_str(
            "set a to empty list\nadd a to a\nsay a\nset m to empty map\nset key \"self\" of m to m\nsay m\n",
        )
        .unwrap();
    let out = engine.take_output();
    assert_eq!(out.len(), 2);
    assert!(out[0].text.contains("<...>"));
    assert!(out[1].text.contains("<...>"));

    let json_err = engine.eval_str("say to json a\n").unwrap_err();
    assert!(
        json_err.to_string().contains("maximum nesting depth"),
        "got: {}",
        json_err
    );
}

#[test]
fn deeply_nested_json_is_rejected_without_stack_overflow() {
    let deep = "[".repeat(300) + &"]".repeat(300);
    let mut engine = AzraEngine::new();
    let e = engine
        .eval_str(&format!("say parse json \"{}\"\n", deep))
        .unwrap_err();
    assert!(e.to_string().contains("max depth"), "got: {}", e);
}

#[test]
fn deeply_nested_syntax_is_rejected_without_stack_overflow() {
    let mut src = String::new();
    for depth in 0..200 {
        src.push_str(&"    ".repeat(depth));
        src.push_str("if true:\n");
    }
    src.push_str(&"    ".repeat(200));
    src.push_str("say 1\n");
    let e = err(&src);
    assert!(
        e.to_string().contains("maximum nesting depth"),
        "got: {}",
        e
    );

    let nots = format!("say {}true\n", "not ".repeat(200));
    let e = err(&nots);
    assert!(
        e.to_string().contains("maximum nesting depth"),
        "got: {}",
        e
    );
}

#[test]
fn empty_token_stream_does_not_panic_parser() {
    let e = parser::parse(&[]).unwrap_err();
    assert!(e.to_string().contains("empty token stream"));
}

#[test]
fn strict_sandbox_blocks_all_io_sleep_and_try_bypass() {
    // 1. Blocks `use`, `write ... to file`, `read file`, `file exists`, `parse json`, `to json`.
    for snippet in [
        "use \"other.vibe\"\n",
        "write \"x\" to file \"out.txt\"\n",
        "say read file \"Cargo.toml\"\n",
        "say file exists \"Cargo.toml\"\n",
        "say parse json \"1\"\n",
        "say to json 1\n",
    ] {
        let mut engine = AzraEngine::with_sandbox(SandboxConfig::strict());
        let e = engine.eval_str(snippet).unwrap_err();
        assert!(
            e.to_string().contains("disabled by the sandbox"),
            "snippet `{}` gave: {}",
            snippet.trim(),
            e
        );
    }

    // 2. `try:` cannot swallow instruction budget exceeded.
    let mut engine = AzraEngine::with_sandbox(SandboxConfig::strict());
    let e = engine
        .eval_str(
            "while true:\n    try:\n        while true:\n            set x to 1\n    if error as err:\n        set y to 1\n",
        )
        .unwrap_err();
    assert!(
        e.to_string().contains("instruction budget exceeded"),
        "got: {}",
        e
    );

    // 3. `try:` cannot swallow call stack overflow.
    let mut engine = AzraEngine::with_sandbox(SandboxConfig::strict());
    let e = engine
        .eval_str(
            "define boom:\n    give back boom\ntry:\n    say boom\nif error as err:\n    say \"escaped\"\n",
        )
        .unwrap_err();
    assert!(e.to_string().contains("call stack overflow"), "got: {}", e);

    // 4. `wait` does not sleep the OS thread in strict sandbox (advances clock only).
    let mut engine = AzraEngine::with_sandbox(SandboxConfig::strict());
    engine.eval_str("wait 1000 seconds\n").unwrap();
    assert_eq!(engine.now(), 1000.0);
}

#[test]
fn integer_edge_cases_do_not_panic_or_truncate() {
    // Huge power exponent must not truncate `u32` to 0 and return 1.
    let e = err("say 2 power 4294967296\n");
    assert!(e.to_string().contains("overflow"), "got: {}", e);

    // `wait` with huge float must not panic `Duration::from_secs_f64`.
    let e = err("wait 999999999999999999999999999999 seconds\n");
    assert!(e.to_string().contains("`wait` duration"), "got: {}", e);

    // `abs` on i64::MIN must return overflow error instead of panicking.
    let e = err("set m to -9223372036854775807 - 1\nsay abs m\n");
    assert!(e.to_string().contains("overflow"), "got: {}", e);
}
