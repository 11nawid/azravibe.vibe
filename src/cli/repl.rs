//! Interactive multi-line REPL for Azra with colorful output and syntax
//! highlighting. Exit with `leave azra`, `:quit`, or Ctrl+D.

use super::output::flush_output;
use azra::AzraEngine;
use std::io::{BufRead, IsTerminal, Write};
use std::process::Command;

// ── ANSI escape codes ──────────────────────────────────────────────────────
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const MAGENTA: &str = "\x1b[35m";
const BLUE: &str = "\x1b[34m";
const WHITE: &str = "\x1b[37m";

/// `azra repl` / bare `azra` — interactive multi-line REPL (blocks keep
/// reading while the parser reports "end of input").
pub fn cmd_repl() -> i32 {
    let use_color = std::io::stdout().is_terminal();
    let mut engine = AzraEngine::new();

    // ── Welcome banner ─────────────────────────────────────────────────────
    if use_color {
        println!();
        println!(
            "  {}{}╔══════════════════════════════════════════╗{}",
            BOLD, CYAN, RESET
        );
        println!(
            "  {}{}║      {}{}  Azra Interactive Interpreter  {}{}  ║{}",
            BOLD, CYAN, RESET, BOLD, RESET, BOLD, RESET
        );
        println!(
            "  {}{}║           {}v{}{}                            {}║{}",
            BOLD,
            CYAN,
            YELLOW,
            env!("CARGO_PKG_VERSION"),
            CYAN,
            BOLD,
            RESET
        );
        println!(
            "  {}{}╚══════════════════════════════════════════╝{}",
            BOLD, CYAN, RESET
        );
        println!();
        println!(
            "  {}Type Azra code and press Enter to run it.{}",
            DIM, RESET
        );
        println!(
            "  {}Commands:  {}:help{}  {}:vars{}  {}:reset{}  {}:clear{} / {}:cls{}",
            DIM, GREEN, DIM, GREEN, DIM, GREEN, DIM, GREEN, DIM, GREEN, RESET
        );
        println!(
            "  {}Exit:      {}leave azra{}  or  {}:quit{}  or  {}Ctrl+D{}",
            DIM, MAGENTA, DIM, MAGENTA, DIM, MAGENTA, RESET
        );
        println!();
    } else {
        println!(
            "Azra REPL v{} — :help for commands, `leave azra` or :quit to exit.",
            env!("CARGO_PKG_VERSION")
        );
    }

    let stdin = std::io::stdin();
    let mut buffer = String::new();
    let mut history_count: usize = 0;

    loop {
        // ── Prompt ──────────────────────────────────────────────────────────
        if use_color {
            if buffer.is_empty() {
                history_count += 1;
                print!(
                    "{}{}[{}]{} {}azra>{} ",
                    DIM, WHITE, history_count, RESET, BOLD, RESET
                );
            } else {
                print!("{}  ...  {}", BLUE, RESET);
            }
        } else {
            let prompt = if buffer.is_empty() {
                "azra> "
            } else {
                "  ...  "
            };
            print!("{}", prompt);
        }
        let _ = std::io::stdout().flush();

        // ── Read a line ─────────────────────────────────────────────────────
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                // EOF (Ctrl+D / Ctrl+Z)
                if use_color {
                    println!("\n{}{}Goodbye from Azra! 👋{}", BOLD, CYAN, RESET);
                } else {
                    println!();
                }
                break;
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}error:{} {}", RED, RESET, err);
                break;
            }
        }
        let trimmed = line.trim_end();

        // ── "leave azra" exits the interpreter ──────────────────────────────
        let lower = trimmed.trim().to_ascii_lowercase();
        if lower == "leave azra" || lower == "exit" || lower == "quit" {
            if use_color {
                println!("\n{}{}Goodbye from Azra! 👋{}", BOLD, CYAN, RESET);
            }
            break;
        }

        // Friendly hint for common commands typed without the leading colon.
        if buffer.is_empty()
            && matches!(
                trimmed,
                "clear" | "cls" | "clean" | "help" | "vars" | "reset"
            )
        {
            let suggestion = if trimmed == "clean" { "clear" } else { trimmed };
            eprintln!(
                "hint: REPL commands start with ':'. Try ':{}' (see ':help').",
                suggestion
            );
            continue;
        }

        // ── REPL colon-commands ─────────────────────────────────────────────
        if trimmed.starts_with(':') && buffer.is_empty() {
            match trimmed {
                ":quit" | ":exit" => {
                    if use_color {
                        println!("\n{}{}Goodbye from Azra! 👋{}", BOLD, CYAN, RESET);
                    }
                    break;
                }
                ":help" => print_repl_help(use_color),
                ":clear" | ":cls" | ":screen" => {
                    clear_screen();
                    if use_color {
                        println!("{}screen and buffer cleared.{}", DIM, RESET);
                    }
                }
                ":vars" => print_vars(&engine, use_color),
                ":reset" => {
                    engine = AzraEngine::new();
                    if use_color {
                        println!("{}{}engine reset ✓{}", BOLD, GREEN, RESET);
                    } else {
                        println!("engine reset.");
                    }
                }
                other => {
                    if use_color {
                        eprintln!(
                            "{}error:{} unknown command `{}` (try {}:help{})",
                            RED, RESET, other, GREEN, RESET
                        );
                    } else {
                        eprintln!("error: unknown command `{}` (try :help)", other);
                    }
                }
            }
            continue;
        }

        // Skip blank lines when buffer is empty.
        if buffer.is_empty() && trimmed.is_empty() {
            continue;
        }

        // Allow :quit / :clear inside a multi-line block.
        if trimmed.starts_with(':') && matches!(trimmed, ":quit" | ":exit" | ":clear") {
            buffer.clear();
            continue;
        }

        buffer.push_str(trimmed);
        buffer.push('\n');

        // ── Evaluate ────────────────────────────────────────────────────────
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
                if use_color {
                    eprintln!("{}error:{} {}", RED, RESET, err);
                } else {
                    eprintln!("error: {}", err);
                }
                buffer.clear();
            }
        }
    }
    0
}

// ── :vars ───────────────────────────────────────────────────────────────────
fn print_vars(engine: &AzraEngine, use_color: bool) {
    let names = engine.variable_names();
    if names.is_empty() {
        if use_color {
            println!("{}(no variables){}", DIM, RESET);
        } else {
            println!("(no variables)");
        }
    } else {
        for name in names {
            if let Some(v) = engine.get_variable(&name) {
                if use_color {
                    println!("  {}{}{} {} {}", CYAN, name, RESET, DIM, RESET);
                    println!("    = {}{}{}", YELLOW, v.render(), RESET);
                } else {
                    println!("{} = {}", name, v.render());
                }
            }
        }
    }
}

/// Clear the terminal screen (works with classic Windows consoles too).
fn clear_screen() {
    let status = if cfg!(windows) {
        Command::new("cmd").args(["/C", "cls"]).status()
    } else {
        Command::new("clear").status()
    };
    if status.map(|s| s.success()).unwrap_or(false) {
        return;
    }
    // ANSI fallback for terminals where `cls`/`clear` is unavailable.
    print!("\x1b[2J\x1b[H");
    let _ = std::io::stdout().flush();
}

// ── :help ───────────────────────────────────────────────────────────────────
fn print_repl_help(use_color: bool) {
    if use_color {
        println!();
        println!(
            "  {}{}Azra Interactive Interpreter — Help{}",
            BOLD, CYAN, RESET
        );
        println!("  {}{}", DIM, "─".repeat(40));
        println!("{}", RESET);
        println!("  {}Commands:{}", BOLD, RESET);
        println!("    {}:help{}    show this help", GREEN, RESET);
        println!(
            "    {}:vars{}    list current global variables",
            GREEN, RESET
        );
        println!(
            "    {}:reset{}   reset the engine (clear everything)",
            GREEN, RESET
        );
        println!(
            "    {}:clear{}   clear the screen and the current multi-line buffer (:cls)",
            GREEN, RESET
        );
        println!();
        println!("  {}Exit the interpreter:{}", BOLD, RESET);
        println!(
            "    {}leave azra{}   exit and return to your terminal",
            MAGENTA, RESET
        );
        println!("    {}:quit{}         same as leave azra", MAGENTA, RESET);
        println!("    {}Ctrl+D{}        same as leave azra", MAGENTA, RESET);
        println!();
        println!("  {}Writing code:{}", BOLD, RESET);
        println!(
            "    {}Type any Azra statement and press Enter.{}",
            DIM, RESET
        );
        println!(
            "    {}A line ending with `:` starts a block —{}",
            DIM, RESET
        );
        println!("    {}keep typing with 4-space indentation.{}", DIM, RESET);
        println!();
        println!("  {}Examples:{}", BOLD, RESET);
        println!("    {}say \"hello world\"{}", YELLOW, RESET);
        println!("    {}repeat 3 times:{}", YELLOW, RESET);
        println!("    {}    say \"tick\"{}", YELLOW, RESET);
        println!("    {}define greet with name:{}", YELLOW, RESET);
        println!("    {}    say \"hi\", name{}", YELLOW, RESET);
        println!();
    } else {
        println!(
            r#"REPL commands:
    :help    show this help
    :vars    list the current global variables
    :reset   reset the engine (clear variables, handlers and timers)
    :clear   clear the terminal screen and the current multi-line buffer (:cls / :screen)

Exit:
    leave azra   exit and return to your terminal
    :quit        same as leave azra (also :exit)
    Ctrl+D       same as leave azra

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
}
