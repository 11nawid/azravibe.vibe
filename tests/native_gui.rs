//! Manual smoke test for the native Win32 GUI core (opens a window briefly).

#[test]
#[ignore = "opens a native window; run manually on Windows with --ignored"]
fn native_window_smoke_test() {
    let window = azra::native_gui::Window::create("Azra GUI test", 300, 160)
        .expect("native GUI requires the C++ core on Windows");
    let label = window.label("Hello from Azra", 20, 20, 240, 24).unwrap();
    let button = window.button("Click me", 20, 60, 120, 30).unwrap();
    let input = window.edit("", 20, 100, 240, 26).unwrap();
    window.set_text(label, "Native Azra GUI").unwrap();
    let typed = window.get_text(input).unwrap();
    assert!(typed.is_empty());
    window.show().unwrap();
    let _ = window.pump().unwrap();
    assert!(button > 0);
    window.close();
}
