//! Interactive multi-line REPL for Azra.

use super::output::flush_output;
use azra::AzraEngine;
use std::io::{BufRead, Write};

/// `azra repl` — interactive multi-line REPL (blocks keep reading while the
/// parser reports "end of input").
pub fn cmd_repl() -> i32 {
    let mut engine = AzraEngine::new();
    println!(
        "Azra REPL v{} — `:help` for commands, `:quit` to exit.",
        env!("CARGO_PKG_VERSION")
    );
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    loop {
        let prompt = if buffer.is_empty() { "azra> " } else { " ...   " };
        print!("{}", prompt);
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("error: {}", err);
                break;
            }
        }
        let trimmed = line.trim_end();

        // REPL commands work at any time.
        if trimmed.starts_with(':') && buffer.is_empty() {
            match trimmed {
                ":quit" | ":exit" => break,
                ":help" => print_repl_help(),
                ":clear" => {}
                ":vars" => print_vars(&engine),
                ":reset" => {
                    engine = AzraEngine::new();
                    println!("engine reset.");
                }
                other => eprintln!("error: unknown command `{}` (try `:help`)", other),
            }
            continue;
        }
        if buffer.is_empty() && trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with(':') && matches!(trimmed, ":quit" | ":exit" | ":clear") {
            buffer.clear();
            continue;
        }

        buffer.push_str(trimmed);
        buffer.push('\n');

        match engine.eval_str(&buffer) {
            Ok(()) => {
                flush_output(&mut engine);
                buffer.clear();
            }
            Err(err) if err.is_incomplete() => {
                // Parser ran out of input: keep reading the block.
            }
            Err(err) => {
                flush_output(&mut engine);
                eprintln!("error: {}", err);
                buffer.clear();
            }
        }
    }
    0
}

/// `:vars` — list the current global bindings.
fn print_vars(engine: &AzraEngine) {
    let names = engine.variable_names();
    if names.is_empty() {
        println!("(no variables)");
    } else {
        for name in names {
            if let Some(v) = engine.get_variable(&name) {
                println!("{} = {}", name, v.render());
            }
        }
    }
}

fn print_repl_help() {
    println!(
        r#"REPL commands:
    :help    show this help
    :vars    list the current global variables
    :reset   reset the engine (clear variables, handlers and timers)
    :clear   discard the current multi-line buffer
    :quit    exit the REPL (also :exit)

Statements are entered one line at a time; a line ending with `:` starts an
indented block - keep typing lines with 4-space indentation until the block
parses. Examples:

    say "hello"
    repeat 3 times:
        say "tick"
    define greet with name:
        say "hi", name
"#
    );
}
