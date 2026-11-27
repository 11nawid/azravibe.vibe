//! End-to-end JSON-RPC test for the `azra-lsp` language server.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};

fn server() -> Child {
    Command::new(env!("CARGO_BIN_EXE_azra-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap()
}

fn send(child: &mut Child, json: &str) {
    let stdin = child.stdin.as_mut().unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n{}", json.len(), json).unwrap();
    stdin.flush().unwrap();
}

fn receive(child: &mut Child) -> String {
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().unwrap();
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).unwrap();
    let text = String::from_utf8(body).unwrap();
    child.stdout = Some(reader.into_inner());
    text
}

#[test]
fn lsp_handles_initialize_diagnostics_completion_and_shutdown() {
    let mut child = server();
    send(
        &mut child,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}",
    );
    let init = receive(&mut child);
    assert!(init.contains("\"capabilities\""), "{init}");
    assert!(init.contains("\"hoverProvider\":true"), "{init}");

    let source = "set x to\n";
    let escaped = source.replace('\n', "\\n").replace('"', "\\\"");
    let did_open = format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{{\"textDocument\":{{\"uri\":\"file:///demo.vibe\",\"languageId\":\"vibe\",\"version\":1,\"text\":\"{escaped}\"}}}}}}"
    );
    send(&mut child, &did_open);
    let diagnostics = receive(&mut child);
    assert!(diagnostics.contains("publishDiagnostics"), "{diagnostics}");
    assert!(diagnostics.contains("syntax error"), "{diagnostics}");

    send(
        &mut child,
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/completion\",\"params\":{\"textDocument\":{\"uri\":\"file:///demo.vibe\"}}}",
    );
    let completion = receive(&mut child);
    assert!(completion.contains("\"label\":\"say\""), "{completion}");
    assert!(completion.contains("\"label\":\"define\""), "{completion}");

    send(
        &mut child,
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"shutdown\",\"params\":{}}",
    );
    let shutdown = receive(&mut child);
    assert!(shutdown.contains("\"id\":3"), "{shutdown}");
    send(
        &mut child,
        "{\"jsonrpc\":\"2.0\",\"method\":\"exit\",\"params\":{}}",
    );
    assert!(child.wait().unwrap().success());
}
