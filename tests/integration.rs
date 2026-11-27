//! Core language integration tests for Azra (`.vibe`).

mod common;

use azra::{AzraEngine, AzraError, Num, OutputLevel, TokenKind, Value, lexer};
use common::{err, ok, run};

// --------------------------------------------------------------- core syntax

#[test]
fn variables_of_every_type() {
    let out = ok(
        "set health to 100\nset speed to 12.5\nset name to \"Ali\"\nset alive to true\nset extra to null\nsay health, speed, name, alive, extra\n",
    );
    assert_eq!(out, vec!["100 12.5 Ali true null"]);
}

#[test]
fn comments_and_trailing_periods_are_ignored() {
    let out = ok(
        "note: full line comment\nset x to 5. note: trailing comment\nset y to x + 2.\nsay y.\n",
    );
    assert_eq!(out, vec!["7"]);
}

#[test]
fn say_joins_values_with_spaces() {
    let out = ok("say \"Welcome to Azravibe,\", \"Ali\", 42\n");
    assert_eq!(out, vec!["Welcome to Azravibe, Ali 42"]);
}

#[test]
fn warn_and_error_print_prefixed_lines() {
    let (result, out) = run("warn \"low health\"\nerror \"dead\"\n");
    assert!(result.is_ok());
    assert_eq!(out, vec!["[WARN] low health", "[ERROR] dead"]);
}

#[test]
fn output_levels_and_streams() {
    let mut engine = AzraEngine::new();
    engine
        .eval_str("say \"a\"\nwarn \"b\"\nerror \"c\"\n")
        .unwrap();
    let lines = engine.take_output();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].level, OutputLevel::Say);
    assert_eq!(lines[0].stream(), azra::OutputStream::Stdout);
    assert_eq!(lines[1].level, OutputLevel::Warn);
    assert_eq!(lines[1].stream(), azra::OutputStream::Stderr);
    assert_eq!(lines[2].level, OutputLevel::Error);
    assert_eq!(lines[2].stream(), azra::OutputStream::Stderr);
}

#[test]
fn string_escapes() {
    let out = ok("say \"line1\\nline2\"\nsay \"tab\\there\"\n");
    assert_eq!(out, vec!["line1\nline2", "tab\there"]);
}

// --------------------------------------------------------------------- lists

#[test]
fn lists_create_add_remove() {
    let out = ok(
        "set items to \"sword\", \"shield\"\nadd \"potion\" to items\nremove \"shield\" from items\nsay items\n",
    );
    assert_eq!(out, vec!["[sword, potion]"]);
}

#[test]
fn remove_missing_list_item_is_a_noop() {
    let out = ok("set items to \"a\", \"b\"\nremove \"zz\" from items\nsay items\n");
    assert_eq!(out, vec!["[a, b]"]);
}

#[test]
fn add_bootstraps_a_undefined_list() {
    let out = ok("add \"gold\" to loot\nadd \"key\" to loot\nsay loot\n");
    assert_eq!(out, vec!["[gold, key]"]);
}

#[test]
fn list_equality_is_structural() {
    let out = ok(
        "set l to 1, 2\nset m to 1, 2\nif l equals m:\n    say \"eq\"\nif l not_equals m:\n    say \"ne\"\n",
    );
    assert_eq!(out, vec!["eq"]);
}

#[test]
fn lists_concatenate_with_plus() {
    let out = ok("set a to 1, 2\nset b to 3, 4\nset c to a + b\nsay c\n");
    assert_eq!(out, vec!["[1, 2, 3, 4]"]);
}

// ------------------------------------------------------------------ arithmetic

#[test]
fn arithmetic_statements() {
    let out = ok(
        "set health to 100\nadd 10 to health\nsubtract 25 from health\nmultiply health by 2\ndivide health by 2\nsay health\n",
    );
    assert_eq!(out, vec!["85"]);
}

#[test]
fn arithmetic_expressions_and_floats() {
    let out = ok(
        "set x to 12.5\nmultiply x by 2\nsay x\nset a to 10.0\ndivide a by 4\nsay a\nsay 10 / 4\nsay 2 + 3 * 4\nsay 0 - 5\n",
    );
    assert_eq!(out, vec!["25", "2.5", "2", "14", "-5"]);
}

#[test]
fn string_concatenation_with_plus() {
    let out = ok("say \"hp: \" + 75\nsay \"a\" + \"b\"\n");
    assert_eq!(out, vec!["hp: 75", "ab"]);
}

#[test]
fn negative_numbers() {
    let out = ok("set x to -5\nsay x\nsay -3 * 2\n");
    assert_eq!(out, vec!["-5", "-6"]);
}

// -------------------------------------------------------------------- objects

#[test]
fn make_objects_and_dot_properties() {
    let out = ok(
        "make rounded_box as lava_block\nset lava_block.color to \"red\"\nset lava_block.material to \"glowing\"\nset lava_block.position to 0, 5, 10\nset lava_block.size to 4, 1, 4\nsay lava_block.color\nsay lava_block.position\n",
    );
    assert_eq!(out, vec!["red", "[0, 5, 10]"]);
}

#[test]
fn object_rendering_shows_type_and_fields() {
    let out = ok("make player as hero\nsay hero\n");
    assert_eq!(out.len(), 1);
    assert!(out[0].starts_with("<player"), "got: {}", out[0]);
    assert!(out[0].contains("health=100"), "got: {}", out[0]);
}

#[test]
fn mock_engine_types_have_defaults() {
    let out = ok(
        "make rounded_box as box\nmake sound as sfx\nmake portal as p\nsay box.material\nsay sfx.volume\nsay p\n",
    );
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], "default");
    assert_eq!(out[1], "1");
    assert_eq!(out[2], "<portal>");
}

#[test]
fn objects_compare_by_identity() {
    let out = ok(
        "make rounded_box as a\nset b to a\nif a equals b:\n    say \"same\"\nmake rounded_box as c\nif a equals c:\n    say \"different\"\n",
    );
    assert_eq!(out, vec!["same"]);
}

#[test]
fn default_player_entity_exists() {
    let out = ok("say player.name, player.health\n");
    assert_eq!(out, vec!["player 100"]);
}

// ---------------------------------------------------- comparison & logic ops

#[test]
fn all_english_comparison_operators() {
    let out = ok(
        "set a to 5\nif a equals 5:\n    say \"eq\"\nif a not_equals 5:\n    say \"ne\"\nif a less_than 10:\n    say \"lt\"\nif a less_than= 5:\n    say \"le\"\nif a greater_than 1:\n    say \"gt\"\nif a greater_than= 5:\n    say \"ge\"\n",
    );
    assert_eq!(out, vec!["eq", "lt", "le", "gt", "ge"]);
}

#[test]
fn comparison_operators_lex_as_single_tokens() {
    let toks = lexer::tokenize("if a less_than= 1:").unwrap();
    assert!(toks.iter().any(|t| t.kind == TokenKind::LessThanEq));
    let toks = lexer::tokenize("if a greater_than= 1:").unwrap();
    assert!(toks.iter().any(|t| t.kind == TokenKind::GreaterThanEq));
}

#[test]
fn logical_operators() {
    let out = ok(
        "set x to true\nset y to false\nif x and y:\n    say \"and_true\"\nif x or y:\n    say \"or_true\"\nif not y:\n    say \"not_y\"\n",
    );
    assert_eq!(out, vec!["or_true", "not_y"]);
}

#[test]
fn operator_precedence() {
    let out = ok("say 1 less_than 2 and 3 greater_than 4 or 5 equals 5\n");
    assert_eq!(out, vec!["true"]);
}

#[test]
fn string_comparison() {
    let out = ok("if \"apple\" less_than \"banana\":\n    say \"yes\"\n");
    assert_eq!(out, vec!["yes"]);
}

// --------------------------------------------------------------- control flow

#[test]
fn if_elseif_else_chain() {
    let out = ok(
        "set health to 75\nif health less_than= 0:\n    say \"dead\"\notherwise if health less_than 30:\n    say \"low\"\notherwise if health greater_than= 100:\n    say \"full\"\notherwise:\n    say \"mid\", health\n",
    );
    assert_eq!(out, vec!["mid 75"]);
}

#[test]
fn nested_if_inside_loop() {
    let out = ok(
        "set hits to 0\nrepeat 4 times:\n    set n to 0\n    if n equals 0:\n        add 1 to hits\nsay hits\n",
    );
    assert_eq!(out, vec!["4"]);
}

#[test]
fn repeat_loop() {
    let out = ok("set count to 0\nrepeat 3 times:\n    add 1 to count\nsay count\n");
    assert_eq!(out, vec!["3"]);
}

#[test]
fn while_loop() {
    let out = ok("set i to 0\nwhile i less_than 5:\n    add 1 to i\nsay i\n");
    assert_eq!(out, vec!["5"]);
}

#[test]
fn for_each_loop() {
    let out = ok(
        "set items to \"a\", \"b\", \"c\"\nset total to 0\nfor each it in items:\n    say it\n    add 1 to total\nsay total\n",
    );
    assert_eq!(out, vec!["a", "b", "c", "3"]);
}

#[test]
fn negative_repeat_is_rejected() {
    let e = err("repeat 0 - 1 times:\n    say \"x\"\n");
    assert!(e.to_string().contains("negative"), "got: {}", e);
}

// ------------------------------------------------------------------ functions

#[test]
fn functions_with_parameters_and_return() {
    let out = ok(
        "define heal with target, amount:\n    add amount to target.health\n    give back target.health\nset hp to heal with player, 25\nsay hp\nsay player.health\n",
    );
    assert_eq!(out, vec!["125", "125"]);
}

#[test]
fn recursion_factorial() {
    let out = ok(
        "define fact with n:\n    if n less_than= 1:\n        give back 1\n    give back n * fact with n - 1\nset r to fact with 5\nsay r\n",
    );
    assert_eq!(out, vec!["120"]);
}

#[test]
fn zero_argument_function_call_in_expression() {
    let out = ok("define get_answer:\n    give back 42\nset a to get_answer\nsay a\n");
    assert_eq!(out, vec!["42"]);
}

#[test]
fn function_call_statement() {
    let out = ok("define shout with word:\n    say word, \"!\"\nshout with \"hi\"\n");
    assert_eq!(out, vec!["hi !"]);
}

#[test]
fn function_updates_outer_scope() {
    let out = ok("set counter to 0\ndefine bump:\n    add 1 to counter\nbump\nbump\nsay counter\n");
    assert_eq!(out, vec!["2"]);
}

#[test]
fn function_alias_variable() {
    let mut engine = AzraEngine::new();
    engine
        .eval_str("define f with a:\n    give back a * 2\nset g to f\nset r to g with 21\n")
        .unwrap();
    match engine.get_variable("r") {
        Some(Value::Number(Num::Int(42))) => {}
        other => panic!("expected 42, got {:?}", other),
    }
}

#[test]
fn give_back_outside_function_is_an_error() {
    let e = err("give back 5\n");
    assert!(
        e.to_string().contains("outside of a function"),
        "got: {}",
        e
    );
}

#[test]
fn arity_mismatch_is_an_error() {
    let e = err("define f with a:\n    give back a\nset r to f with 1, 2\n");
    assert!(
        e.to_string().contains("expects 1 argument(s), got 2"),
        "got: {}",
        e
    );
}

// ------------------------------------------------------------ engine features

#[test]
fn engine_verbs_log_actions() {
    let out = ok(
        "make sound as boom\nplay boom\ndestroy boom\njump player with 15\nspawn zombie\nmove player\nrotate boom\nwait\n",
    );
    assert!(out[0].starts_with("[ENGINE] play"), "got: {}", out[0]);
    assert!(out[1].starts_with("[ENGINE] destroy"), "got: {}", out[1]);
    assert!(
        out[2].starts_with("[ENGINE] jump") && out[2].contains("15"),
        "got: {}",
        out[2]
    );
    assert!(out[3].starts_with("[ENGINE] spawn"), "got: {}", out[3]);
    assert!(out[4].starts_with("[ENGINE] move"), "got: {}", out[4]);
    assert!(out[5].starts_with("[ENGINE] rotate"), "got: {}", out[5]);
    assert!(out[6].starts_with("[ENGINE] wait"), "got: {}", out[6]);
}

#[test]
fn when_handlers_register_and_trigger() {
    let mut engine = AzraEngine::new();
    engine
        .eval_str(
            "make rounded_box as crate\nwhen player touches crate:\n    set player.health to 0\n    destroy crate\n    say \"ouch\"\n",
        )
        .unwrap();
    assert_eq!(engine.registered_events(), vec!["player touches crate"]);

    engine.trigger_event("player touches crate").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out.len(), 2);
    assert!(
        out[0].starts_with("[ENGINE] destroy <rounded_box color=white material=default position=[0, 0, 0] size=[1, 1, 1]"),
        "unexpected destroy line: {}",
        out[0]
    );
    assert!(out[0].contains("anchored=true"));
    assert!(out[0].contains("visible=true"));
    assert_eq!(out[1], "ouch");

    match engine.get_variable("player").unwrap() {
        Value::Object(obj) => {
            assert!(
                obj.borrow()
                    .get("health")
                    .unwrap()
                    .equals(&Value::Number(Num::Int(0)))
            );
        }
        other => panic!("expected player object, got {:?}", other),
    }
}

#[test]
fn when_handler_with_quoted_phrase_and_trigger_matching() {
    let mut engine = AzraEngine::new();
    engine
        .eval_str("when key \"space\" is pressed:\n    jump player with 15\n")
        .unwrap();
    engine.trigger_event("key \"SPACE\" is pressed").unwrap();
    engine.trigger_event("key space is pressed").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out.len(), 2);
    assert!(out[0].starts_with("[ENGINE] jump"));
    assert!(out[1].starts_with("[ENGINE] jump"));
}

#[test]
fn unknown_event_trigger_is_an_error() {
    let mut engine = AzraEngine::new();
    engine
        .eval_str("when player touches lava_block:\n    say \"x\"\n")
        .unwrap();
    let e = engine.trigger_event("dragon appears").unwrap_err();
    let msg = e.to_string();
    assert!(msg.contains("no event handler matches"), "got: {}", msg);
    assert!(msg.contains("player touches lava_block"), "got: {}", msg);
}

#[test]
fn host_function_registration() {
    let mut engine = AzraEngine::new();
    engine.register_function("triple", |args| match args.first() {
        Some(Value::Number(Num::Int(i))) => Ok(Value::Number(Num::Int(i * 3))),
        Some(Value::Number(Num::Float(f))) => Ok(Value::Number(Num::Float(f * 3.0))),
        _ => Err(AzraError::runtime("triple expects a number")),
    });
    engine.eval_str("set x to triple with 14").unwrap();
    match engine.get_variable("x") {
        Some(Value::Number(Num::Int(42))) => {}
        other => panic!("expected 42, got {:?}", other),
    }
    engine.eval_str("set y to 0\ntriple with 7\n").unwrap();
}

#[test]
fn state_persists_across_eval_calls() {
    let mut engine = AzraEngine::new();
    engine.eval_str("set x to 1").unwrap();
    engine.eval_str("add 41 to x").unwrap();
    engine.eval_str("say x").unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out, vec!["42"]);
}
