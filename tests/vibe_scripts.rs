//! Runs the `.vibe` stress suites in `tests/vibe-test/` through the real CLI.
//! Keeping the suites as scripts makes them runnable by hand as well.

use std::process::Command;

fn run(script: &str) -> Vec<String> {
    let path = format!("tests/vibe-test/{script}");
    let out = Command::new(env!("CARGO_BIN_EXE_azra"))
        .args(["run", &path])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{script} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

fn near(line: &str, expected: f64) {
    let actual: f64 = line
        .parse()
        .unwrap_or_else(|_| panic!("expected a number, got `{line}`"));
    let tolerance = expected.abs().max(1.0) * 1e-9;
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {expected}, got {actual}"
    );
}

fn vec3(line: &str) -> [f64; 3] {
    let body = line
        .strip_prefix("vec3 ")
        .unwrap_or_else(|| panic!("expected a vec3, got `{line}`"));
    let mut out = [0.0; 3];
    for (i, part) in body.split(',').enumerate() {
        out[i] = part.trim().parse().unwrap();
    }
    out
}

fn close_vec(actual: [f64; 3], expected: [f64; 3]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!(
            (a - e).abs() <= e.abs().max(1.0) * 1e-9,
            "expected {expected:?}, got {actual:?}"
        );
    }
}

#[test]
fn math_features_script() {
    let out = run("01_math_features.vibe");
    assert_eq!(out.len(), 31, "unexpected output: {out:?}");
    near(&out[0], std::f64::consts::PI);
    near(&out[1], std::f64::consts::E);
    near(&out[2], 1.0);
    near(&out[3], 0.5235987755982989);
    near(&out[7], 1.0);
    near(&out[8], 10.0);
    near(&out[9], 6.0);
    near(&out[11], 4.0);
    near(&out[12], std::f64::consts::PI);
    near(&out[13], 180.0);
    near(&out[14], 5.0);
    near(&out[18], 90.0);
    close_vec(vec3(&out[19]), [0.6, 0.8, 0.0]);
    close_vec(vec3(&out[21]), [0.5, 0.5, 0.0]);
    assert_eq!(&out[22..], &["5", "-5", "-4", "7", "4", "1024", "3", "3", "3.5"]);
}

#[test]
fn recursion_script() {
    assert_eq!(
        run("02_recursion.vibe"),
        vec!["6765", "2432902008176640000", "61"]
    );
}

#[test]
fn prime_sieve_script() {
    assert_eq!(run("03_prime_sieve.vibe"), vec!["1229"]);
}

#[test]
fn newton_raphson_script() {
    let out = run("04_newton_raphson.vibe");
    assert_eq!(out.len(), 2);
    near(&out[0], 351.363060095964);
    near(&out[1], 351.363060095964);
}

#[test]
fn quadratic_script() {
    let out = run("05_quadratic.vibe");
    assert_eq!(out[0], "3");
    assert_eq!(out[1], "2");
    assert!(out[2].contains("caught:"));
}

#[test]
fn projectile_script() {
    let out = run("06_projectile.vibe");
    assert_eq!(out.len(), 4);
    near(&out[0], 163.09887869520895);
    near(&out[1], 40.773336617639515);
    let landing = vec3(&out[2]);
    assert!(landing[0] > 162.0 && landing[0] < 164.0);
    assert!(landing[1].abs() < 1.0);
    let velocity = vec3(&out[3]);
    near_line_vec(velocity[0], 28.284271247461902);
    near_line_vec(velocity[1], -28.284271247461902);
}

fn near_line_vec(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= expected.abs() * 1e-9,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn error_handling_script() {
    let out = run("07_error_handling.vibe");
    let expectations = [
        "division by zero",
        "negative number",
        "integer overflow",
        "modulo by zero",
        "between -1 and 1",
        "positive finite",
        "not 1",
        "zero-length",
        "exp` overflowed",
    ];
    assert_eq!(out.len(), expectations.len(), "{out:?}");
    for (line, expected) in out.iter().zip(expectations) {
        assert!(line.contains(expected), "expected `{expected}` in `{line}`");
    }
}

#[test]
#[ignore = "7.4M-entry sieve: about 9s in release, 24s in debug"]
fn euler500_script() {
    assert_eq!(run("09_euler500.vibe"), vec!["35407281"]);
}

#[test]
fn three_body_script() {
    let out = run("10_three_body.vibe");
    assert_eq!(out[0], "reason: close binary formed");
    assert_eq!(out[4], "initial energy: -11.75");
    let final_energy: f64 = out[5]
        .strip_prefix("final energy: ")
        .unwrap()
        .parse()
        .unwrap();
    assert!((final_energy + 11.75).abs() < 1e-8);
    let drift: f64 = out[6]
        .strip_prefix("max relative drift: ")
        .unwrap()
        .parse()
        .unwrap();
    assert!(drift < 1e-9, "energy drift too large: {drift}");
}

#[test]
fn combined_math_stress_script() {
    let out = run("00_math_stress.vibe");
    let joined = out.join("\n");
    for expected in [
        "ackermann 3,3: 61",
        "primes up to 10000: 1229",
        "dot: 32",
        "angle degrees: 90",
        "roots: 3 2",
        "overflow: integer overflow in addition",
    ] {
        assert!(joined.contains(expected), "missing `{expected}` in:\n{joined}");
    }
}
