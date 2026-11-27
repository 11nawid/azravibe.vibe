//! Regression tests for language-level bugs found during the Azra language audit.

mod common;

use common::{err, ok};

#[test]
fn hash_comment_line_does_not_change_indentation() {
    let out = ok("repeat 2 times:\n# a comment at column zero\n    say 1\nsay \"end\"\n");
    assert_eq!(out, vec!["1", "1", "end"]);
}

#[test]
fn modulo_has_one_sign_rule_for_ints_and_floats() {
    let out = ok("say -5 modulo 2\nsay -5 modulo 2.0\nsay 5 modulo -2\nsay 5 modulo -2.0\n");
    assert_eq!(out, vec!["1", "1", "1", "1"]);
}

#[test]
fn nan_comparisons_are_rejected() {
    let e = err("set n to -8 power 0.5\nsay n less_than= 0\n");
    assert!(e.to_string().contains("cannot compare NaN"), "got: {}", e);
}

#[test]
fn clamp_with_nan_bound_is_an_error_not_a_panic() {
    let e = err("set n to -8 power 0.5\nsay clamp 5 between n and 10\n");
    assert!(
        e.to_string().contains("clamp bounds must not be NaN"),
        "got: {}",
        e
    );
}

#[test]
fn min_and_max_reject_nan() {
    let e = err("set n to -8 power 0.5\nsay min of n, 3\n");
    assert!(e.to_string().contains("cannot compare NaN"), "got: {}", e);
}

#[test]
fn sorting_non_finite_numbers_is_rejected() {
    let e = err("set xs to 1, -8 power 0.5, 3\nsort xs\n");
    assert!(e.to_string().contains("finite"), "got: {}", e);
}

#[test]
fn a_container_equals_itself() {
    let out = ok("set a to empty list\nadd a to a\nif a equals a:\n    say \"same\"\n");
    assert_eq!(out, vec!["same"]);
}

#[test]
fn zero_to_a_negative_power_is_division_by_zero() {
    let e = err("say 0 power -1\n");
    assert!(e.to_string().contains("division by zero"), "got: {}", e);
}

#[test]
fn non_finite_timer_interval_is_rejected() {
    let e = err("set n to -8 power 0.5\nafter n seconds:\n    say \"never\"\n");
    assert!(e.to_string().contains("finite"), "got: {}", e);
}

#[test]
fn very_long_expression_chains_are_rejected_instead_of_overflowing() {
    let src = format!("say {}\n", vec!["1"; 1_000].join(" + "));
    let e = err(&src);
    assert!(
        e.to_string().contains("expression chain too long"),
        "got: {}",
        e
    );
}
