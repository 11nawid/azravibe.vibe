//! Azra CLI — `azra run <file.vibe>` and the interactive `azra repl`.

use azra::{AzraError, AzraEngine, OutputLine, OutputStream};
use std::io::{BufRead, IsTerminal, Write};

const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

const USAGE: &str = "\
Azra (.vibe) interpreter — Azravibe ecosystem

USAGE:
    azra run <script.vibe> [--trigger \"<event phrase>\"]...
    azra repl
    azra eval <code>
    azra --help
    azra --version

EXAMPLES:
    azra run examples/hello.vibe
    azra run examples/game_demo.vibe --trigger \"player touches lava_block\"
    azra repl
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("repl") => cmd_repl(),
        Some("eval") => cmd_eval(&args[1..]),
        Some("--help") | Some("-h") | Some("help") => {
            print!("{}", USAGE);
            0
        }
        Some("--version") | Some("-v") | Some("version") => {
            println!("azra {}", env!("CARGO_PKG_VERSION"));
            0
        }
        Some(other) if other.ends_with(".vibe") => cmd_run(&args),
        Some(other) => {
            eprintln!("error: unknown command `{}`\n", other);
            eprint!("{}", USAGE);
            2
        }
        None => {
            print!("{}", USAGE);
            0
        }
    };
    std::process::exit(code);
}

/// `azra run <file> [--trigger phrase]...`
fn cmd_run(args: &[String]) -> i32 {
    let mut file: Option<String> = None;
    let mut triggers: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--trigger" {
            i += 1;
            match args.get(i) {
                Some(phrase) => triggers.push(phrase.clone()),
                None => {
                    eprintln!("error: `--trigger` requires an event phrase");
                    return 2;
                }
            }
        } else if let Some(phrase) = arg.strip_prefix("--trigger=") {
            triggers.push(phrase.to_string());
        } else if arg.starts_with("--") {
            eprintln!("error: unknown option `{}`", arg);
            return 2;
        } else if file.is_none() {
            file = Some(arg.clone());
        } else {
            eprintln!("error: unexpected argument `{}`", arg);
            return 2;
        }
        i += 1;
    }
    let file = match file {
        Some(f) => f,
        None => {
            eprintln!("error: missing script file\n");
            eprint!("{}", USAGE);
            return 2;
        }
    };

    let mut engine = AzraEngine::new();
    if let Err(err) = engine.eval_file(&file) {
        flush_output(&mut engine);
        eprintln!("error: {}", err);
        return 1;
    }
    flush_output(&mut engine);

    for phrase in triggers {
        if let Err(err) = engine.trigger_event(&phrase) {
            flush_output(&mut engine);
            eprintln!("error: {}", err);
            return 1;
        }
        flush_output(&mut engine);
    }
    0
}

/// `azra eval <code>` — run a one-liner from the command line.
fn cmd_eval(args: &[String]) -> i32 {
    let source = args.join(" ");
    if source.trim().is_empty() {
        eprintln!("error: `eval` requires source code");
        return 2;
    }
    let mut engine = AzraEngine::new();
    if let Err(err) = engine.eval_str(&source) {
        flush_output(&mut engine);
        eprintln!("error: {}", err);
        return 1;
    }
    flush_output(&mut engine);
    0
}

/// Write buffered output to stdout/stderr, coloring warn/error on a TTY.
fn flush_output(engine: &mut AzraEngine) {
    let stderr_tty = std::io::stderr().is_terminal();
    for line in engine.take_output() {
        write_line(&line, stderr_tty);
    }
}

fn write_line(line: &OutputLine, stderr_tty: bool) {
    match line.stream() {
        OutputStream::Stdout => {
            let stdout = std::io::stdout();
            let mut out = stdout.lock();
            let _ = writeln!(out, "{}", line.text);
            let _ = out.flush();
        }
        OutputStream::Stderr => {
            if stderr_tty {
                let color = match line.level {
                    azra::OutputLevel::Warn => YELLOW,
                    _ => RED,
                };
                eprintln!("{}{}{}", color, line.text, RESET);
            } else {
                eprintln!("{}", line.text);
            }
        }
    }
}

/// `azra repl` — interactive multi-line REPL (blocks keep reading while the
/// parser reports "end of input").
fn cmd_repl() -> i32 {
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

        match eval_source(&mut engine, &buffer) {
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

fn eval_source(engine: &mut AzraEngine, source: &str) -> Result<(), AzraError> {
    engine.eval_str(source)
}

fn print_repl_help() {
    println!(
        "{}",
        "REPL commands:
    :help    show this help
    :clear   discard the current multi-line buffer
    :quit    exit the REPL (also :exit)

Statements are entered one line at a time; a line ending with `:` starts an
indented block - keep typing lines with 4-space indentation until the block
parses. Examples:

    say \"hello\"
    repeat 3 times:
        say \"tick\"
    define greet with name:
        say \"hi\", name
"
    );
}
