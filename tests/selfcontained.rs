//! The installed interpreter must work without a `std/` directory.

#[test]
fn stdlib_modules_are_compiled_into_the_binary() {
    let math = azra::stdlib_embed::get("math").expect("math is embedded");
    assert!(math.contains("define square with x:"));
    assert!(azra::stdlib_embed::get("http").is_some());
    assert!(azra::stdlib_embed::get("nativegui").is_some());
    assert!(azra::stdlib_embed::get("does_not_exist").is_none());
    assert!(!azra::stdlib_embed::names().is_empty());
}

#[test]
fn embedded_modules_execute_from_any_directory() {
    let out = azra::run_source("use \"std/math\"\nsay square with 7\n").unwrap();
    assert_eq!(out, "49");
}
