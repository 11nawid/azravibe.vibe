//! End-to-end HTTP test for `azra serve` (C++ core + Azra handlers).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

fn free_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    listener.local_addr().unwrap().port()
}

fn request(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn serve_handles_http_and_routes_to_vibe_handlers() {
    let port = free_port();
    let mut child = Command::new(env!("CARGO_BIN_EXE_azra"))
        .args(["serve", "examples/server.vibe", "--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let mut connected = false;
    for _ in 0..40 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            connected = true;
            break;
        }
        sleep(Duration::from_millis(50));
    }
    assert!(connected, "server did not start on port {port}");

    let health = request(port, "/health");
    assert!(health.starts_with("HTTP/1.1 200"), "{health}");
    assert!(health.contains("\"status\":\"ok\""), "{health}");

    let upper = request(port, "/upper?text=azra");
    assert!(upper.ends_with("AZRA"), "{upper}");

    let missing = request(port, "/missing");
    assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");

    child.kill().unwrap();
    let _ = child.wait();
}
