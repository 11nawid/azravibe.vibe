//! Error diagnostics and example-file end-to-end tests.

mod common;

use azra::{lexer, parser, AzraEngine};
use common::err;

#[test]
fn syntax_error_reports_line_numbers() {
    let e = err("set a to 1\nset b to\n");
    let msg = e.to_string();
    assert!(msg.contains("line 2"), "got: {}", msg);
    assert!(msg.contains("syntax error"), "got: {}", msg);
}

#[test]
fn undefined_variable_error() {
    let e = err("say missing_var\n");
    let msg = e.to_string();
    assert!(msg.contains("undefined variable `missing_var`"), "got: {}", msg);
    assert!(msg.contains("line 1"), "got: {}", msg);
}

#[test]
fn division_by_zero_error() {
    let e = err("set x to 10\ndivide x by 0\n");
    assert!(e.to_string().contains("division by zero"), "got: {}", e);
}

#[test]
fn tab_indentation_is_rejected() {
    let e = err("if true:\n\tsay 1\n");
    assert!(e.to_string().contains("tabs are not allowed"), "got: {}", e);
}

#[test]
fn bad_indentation_is_rejected() {
    let e = err("set x to 1\n  say x\n");
    assert!(
        e.to_string().contains("unexpected indentation"),
        "got: {}",
        e
    );
}

#[test]
fn inconsistent_dedent_is_rejected() {
    let e = err("if true:\n    say \"a\"\n  say \"b\"\n");
    assert!(
        e.to_string().contains("inconsistent indentation"),
        "got: {}",
        e
    );
}

#[test]
fn missing_property_error() {
    let e = err("make rounded_box as b\nsay b.nope\n");
    assert!(
        e.to_string().contains("has no property `nope`"),
        "got: {}",
        e
    );
}

#[test]
fn property_on_non_object_error() {
    let e = err("set health to 1\nsay health.x\n");
    assert!(
        e.to_string().contains("only objects have properties"),
        "got: {}",
        e
    );
}

#[test]
fn undefined_function_error() {
    let e = err("no_such_func with 1\n");
    assert!(
        e.to_string().contains("undefined function `no_such_func`"),
        "got: {}",
        e
    );
}

#[test]
fn stray_otherwise_error() {
    let e = err("otherwise:\n    say \"x\"\n");
    assert!(e.to_string().contains("stray `otherwise`"), "got: {}", e);
}

#[test]
fn standalone_equals_sign_is_rejected() {
    let e = err("set x to 1\nset y to x = 1\n");
    assert!(e.to_string().contains("English operators"), "got: {}", e);
}

#[test]
fn unterminated_string_error() {
    let e = err("say \"oops\n");
    assert!(e.to_string().contains("unterminated string"), "got: {}", e);
}

#[test]
fn parser_reports_incomplete_input_for_repl() {
    let toks = lexer::tokenize("if true:").unwrap();
    let e = parser::parse(&toks).unwrap_err();
    assert!(e.is_incomplete());

    let toks = lexer::tokenize("set x to 5").unwrap();
    assert!(parser::parse(&toks).is_ok());
}

#[test]
fn game_demo_file_runs_end_to_end() {
    let mut engine = AzraEngine::new();
    engine.eval_file("examples/game_demo.vibe").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out[0], "Welcome to Azravibe, Ali");
    assert_eq!(out[1], "Starting health: 100");
    assert_eq!(out[2], "Current health is 75");
    assert!(out.iter().any(|l| l == "Countdown tick..."));
    assert!(out.iter().any(|l| l == "Inventory slot: gold_key"));
    assert!(out.iter().any(|l| l == "Healed by 25 points."));
    assert!(out.iter().any(|l| l == "System report: all systems go"));

    engine.trigger_event("player touches lava_block").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert!(out.iter().any(|l| l.starts_with("[ENGINE] play")));
    assert!(out.iter().any(|l| l.starts_with("[ENGINE] destroy")));
    assert!(out.iter().any(|l| l == "Game Over!"));

    engine.trigger_event("key space is pressed").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert!(out.iter().any(|l| l.starts_with("[ENGINE] jump")));
}

#[test]
fn hello_file_runs() {
    let mut engine = AzraEngine::new();
    engine.eval_file("examples/hello.vibe").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out[0], "Hello from Azra!");
    assert!(out.iter().any(|l| l == "welcome, player"));
}
