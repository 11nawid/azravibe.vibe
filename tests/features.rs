//! Tests for Azra v1.0.0 features: vec3, color, maps, string interpolation,
//! stdlib builtins, timers, lifecycle events, modules, sandbox, and try/fail.

mod common;

use azra::{AzraEngine, SandboxConfig};
use common::{err, ok};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("azra_test_{}_{}", name, nanos));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn vec3_and_color_properties_and_interpolation() {
    let out = ok(
        "set a to vec3 1, 2, 3\nset b to vec3 4, 6, 3\nsay a.x, a.y, a.z\nsay distance between a and b\nset n to normalize vec3 0, 3, 4\nsay n.y, n.z\nset c to color \"red\"\nsay c.r, c.g, c.b\nsay \"pos=({a.x}, {a.y}) color=({c.r})\"\n",
    );
    assert_eq!(out[0], "1 2 3");
    assert_eq!(out[1], "5");
    assert_eq!(out[2], "0.6 0.8");
    assert_eq!(out[3], "255 0 0");
    assert_eq!(out[4], "pos=(1, 2) color=(255)");
}

#[test]
fn maps_possessive_and_precedence() {
    let out = ok(
        "set stats to empty map\nset key \"hp\" of stats to 10\nset stats's \"mp\" to 20\nsay stats's \"hp\" + 5\nif stats's \"hp\" equals 10:\n    say \"hp_ok\"\nif stats has key \"mp\":\n    say keys of stats\nset nested to empty map\nset key \"inner\" of nested to stats\nsay nested's \"inner\"'s \"mp\"\nremove key \"mp\" from stats\nsay stats has key \"mp\"\nclear stats\nsay length of stats\n",
    );
    assert_eq!(out, vec!["15", "hp_ok", "[hp, mp]", "20", "false", "0"]);
}

#[test]
fn list_builtins_and_mutations() {
    let out = ok(
        "set nums to 30, 10, 20\nsort nums\nsay nums\nreverse nums\nsay first item of nums, item 2 of nums, last item of nums\nset first item of nums to 99\nremove item 2 from nums\nsay nums\nsay nums contains 99\nclear nums\nsay length of nums\n",
    );
    assert_eq!(
        out,
        vec!["[10, 20, 30]", "30 20 10", "[99, 10]", "true", "0"]
    );
}

#[test]
fn math_and_random_builtins_with_comparisons() {
    let out = ok(
        "say round 4.6, floor 4.9, ceil 4.1, abs -7, sqrt 16\nsay min of 9, 3, max of 9, 3\nif clamp 25 between 0 and 10 equals 10:\n    say \"clamped\"\nif distance between vec3 0, 0, 0 and vec3 3, 4, 0 less_than 6:\n    say \"near\"\nsay 2 power 3 power 2\nsay 17 modulo 5\nrepeat 20 times:\n    set r to random number from 1 to 6\n    if r less_than 1 or r greater_than 6:\n        fail with \"out of range\"\nset z to random number from 0 to 0\nsay z\n",
    );
    assert_eq!(
        out,
        vec!["5 4 5 7 4", "3 9", "clamped", "near", "512", "2", "0"]
    );
}

#[test]
fn string_builtins_and_interpolation_zero_arg_fn() {
    let out = ok(
        "set raw to \"  Hello, Azra!  \"\nset clean to trim of raw\nsay uppercase of clean\nsay lowercase of clean\nset parts to split \"a,b,c\" by \",\"\nsay join parts with \"-\"\nsay replace \"Azra\" with \"World\" in clean\nif clean starts with \"Hello\":\n    say \"starts\"\ndefine tag:\n    give back \"v1\"\nsay \"ver={tag} escaped=\\{ok\\}\"\n",
    );
    assert_eq!(
        out,
        vec![
            "HELLO, AZRA!",
            "hello, azra!",
            "a-b-c",
            "Hello, World!",
            "starts",
            "ver=v1 escaped={ok}",
        ]
    );
}

#[test]
fn repeat_range_break_continue_and_try_fail() {
    let out = ok(
        "set sum to 0\nrepeat with i from 1 to 6 by 1:\n    if i equals 2:\n        skip next\n    if i equals 5:\n        stop loop\n    add i to sum\nsay sum\ntry:\n    fail with \"boom\"\nif error as err:\n    say \"caught:\", err\n",
    );
    // 1 + 3 + 4 = 8
    assert_eq!(out, vec!["8", "caught: boom"]);
}

#[test]
fn lifecycle_triggers_are_optional_and_timers_fire_on_ticks() {
    // Script with no `when game starts:` or `when game updates:` succeeds.
    let mut engine = AzraEngine::new();
    engine
        .eval_str(
            "set count to 0\nafter 1 seconds:\n    say \"once\"\nevery 0.5 seconds:\n    add 1 to count\n",
        )
        .unwrap();
    engine.start().unwrap();
    for _ in 0..60 {
        engine.tick(1.0 / 60.0).unwrap();
    }
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out, vec!["once"]);
    assert_eq!(engine.get_variable("count").unwrap().render(), "2");

    // Script WITH `when game starts:` and `when game updates with dt:`.
    let mut game = AzraEngine::new();
    game.eval_str(
        "set total_dt to 0\nwhen game starts:\n    say \"started\"\nwhen game updates with dt:\n    add dt to total_dt\n",
    )
    .unwrap();
    game.start().unwrap();
    game.tick(0.25).unwrap();
    game.tick(0.25).unwrap();
    let out: Vec<String> = game.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out, vec!["started"]);
    assert_eq!(game.get_variable("total_dt").unwrap().render(), "0.5");
}

#[test]
fn modules_use_alias_cache_and_cycle_detection() {
    let dir = temp_dir("modules");
    let helper = dir.join("math_helpers.vibe");
    let main_file = dir.join("main.vibe");
    std::fs::write(
        &helper,
        "set pi to 3\ndefine double with x:\n    give back x * 2\n",
    )
    .unwrap();
    std::fs::write(
        &main_file,
        "use \"math_helpers\" as mh\nsay mh.pi\nsay mh.double with 7\nuse \"math_helpers\"\nsay double with 5\n",
    )
    .unwrap();

    AzraEngine::check_file(&main_file).unwrap();
    let mut engine = AzraEngine::new();
    engine.eval_file(&main_file).unwrap();
    let out: Vec<String> = engine.take_output().into_iter().map(|l| l.text).collect();
    assert_eq!(out, vec!["3", "14", "10"]);

    // Circular import error.
    let a = dir.join("a.vibe");
    let b = dir.join("b.vibe");
    std::fs::write(&a, "use \"b.vibe\"\n").unwrap();
    std::fs::write(&b, "use \"a.vibe\"\n").unwrap();
    let mut cyc_engine = AzraEngine::new();
    let cyc_err = cyc_engine.eval_file(&a).unwrap_err();
    assert!(
        cyc_err.to_string().contains("circular import detected"),
        "got: {}",
        cyc_err
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn file_io_json_and_sandbox_strict() {
    let dir = temp_dir("io");
    let file_path = dir.join("data.json");
    let path_str = file_path.to_string_lossy().replace('\\', "/");

    let script = format!(
        "set m to empty map\nset key \"score\" of m to 42\nwrite to json m to file \"{}\"\nsay file exists \"{}\"\nset loaded to parse json read file \"{}\"\nsay loaded's \"score\"\n",
        path_str, path_str, path_str
    );
    let out = ok(&script);
    assert_eq!(out, vec!["true", "42"]);

    // Strict sandbox blocks I/O and caps instructions.
    let mut strict = AzraEngine::with_sandbox(SandboxConfig::strict());
    let io_err = strict.eval_str("say parse json \"\\{\\}\"\n").unwrap_err();
    assert!(
        io_err.to_string().contains("disabled by the sandbox"),
        "got: {}",
        io_err
    );

    let budget_err = strict
        .eval_str("while true:\n    set x to 1\n")
        .unwrap_err();
    assert!(
        budget_err
            .to_string()
            .contains("instruction budget exceeded"),
        "got: {}",
        budget_err
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn time_builtins_work() {
    let out = ok("set ts to current time\nsay ts greater_than 0\nsay format time 0\n");
    assert_eq!(out, vec!["true", "1970-01-01 00:00:00"]);
    let _ = err("format time 1, 2\n");
}
