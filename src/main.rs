//! Azra CLI — `azra run <file.vibe>`, `azra check`, `azra repl` and
//! `azra eval`.

mod cli;

use azra::AzraEngine;
use cli::output::flush_output;
use cli::repl::cmd_repl;

const USAGE: &str = "\
Azra (.vibe) interpreter — Azravibe ecosystem

USAGE:
    azra                               start the interactive interpreter
    azra run <script.vibe> [--ticks <n>] [--trigger \"<event phrase>\"]...
    azra check <script.vibe>
    azra repl
    azra eval <code>
    azra --help
    azra --version

OPTIONS:
    --ticks <n>       run n engine frames (1/60 s each) after loading the script
    --trigger <text>  fire a `when <event>:` handler after loading (repeatable)

EXAMPLES:
    azra                              (opens interactive interpreter)
    azra run examples/hello.vibe
    azra run examples/game_demo.vibe --ticks 120
    azra run examples/game_demo.vibe --trigger \"player touches lava_block\"
    azra check examples/game_demo.vibe
    azra repl
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let launched_bare = args.is_empty();

    // Recursive Azra functions (e.g. Ackermann) use Rust stack frames for
    // every call. The OS default 1 MiB main-thread stack is too small, so run
    // the CLI on a worker with room for deep user code.
    let code = std::thread::Builder::new()
        .name("azra-interpreter".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || cli_main(&args))
        .expect("could not start the Azra interpreter thread")
        .join()
        .unwrap_or(1);

    // If launched by double-clicking (no args), wait for Enter before closing
    // so the user can see any output. When run from an existing terminal this
    // doesn't trigger because there were args (even `repl` counts).
    if launched_bare {
        use std::io::Write;
        println!();
        print!("Press Enter to close this window...");
        let _ = std::io::stdout().flush();
        let _ = std::io::stdin().read_line(&mut String::new());
    }

    std::process::exit(code);
}

fn cli_main(args: &[String]) -> i32 {
    let code = match args.first().map(String::as_str) {
        Some("run") => cmd_run(&args[1..]),
        Some("check") => cmd_check(&args[1..]),
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
        Some(other) if other.to_ascii_lowercase().ends_with(".vibe") => cmd_run(&args),
        Some(other) => {
            eprintln!("error: unknown command `{}`\n", other);
            eprint!("{}", USAGE);
            2
        }
        None => {
            // No arguments: start the interactive interpreter
            cmd_repl()
        }
    };
    code
}

/// Engine used by `run` / `eval`: permissive like the library default, with a
/// higher call-depth cap for recursive algorithms (the CLI has a large stack).
fn cli_engine() -> AzraEngine {
    AzraEngine::with_sandbox(azra::SandboxConfig {
        max_call_depth: 512,
        ..azra::SandboxConfig::permissive()
    })
}

/// `azra run <file> [--ticks n] [--trigger phrase]...`
fn cmd_run(args: &[String]) -> i32 {
    let mut file: Option<String> = None;
    let mut triggers: Vec<String> = Vec::new();
    let mut ticks: u64 = 0;
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
        } else if arg == "--ticks" {
            i += 1;
            match args.get(i).map(|s| s.parse::<u64>()) {
                Some(Ok(n)) => ticks = n,
                Some(Err(_)) => {
                    eprintln!("error: `--ticks` requires a whole number");
                    return 2;
                }
                None => {
                    eprintln!("error: `--ticks` requires a number");
                    return 2;
                }
            }
        } else if let Some(n) = arg.strip_prefix("--ticks=") {
            match n.parse::<u64>() {
                Ok(v) => ticks = v,
                Err(_) => {
                    eprintln!("error: `--ticks` requires a whole number");
                    return 2;
                }
            }
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

    let mut engine = cli_engine();
    if let Err(err) = engine.eval_file(&file) {
        flush_output(&mut engine);
        eprintln!("error: {}", err);
        return 1;
    }
    flush_output(&mut engine);

    // Engine start: run optional `when game starts:` handlers.
    if let Err(err) = engine.start() {
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

    // Engine loop: n frames at 60 FPS (fires optional `when game updates` + timers).
    for _ in 0..ticks {
        if let Err(err) = engine.tick(1.0 / 60.0) {
            flush_output(&mut engine);
            eprintln!("error: {}", err);
            return 1;
        }
        flush_output(&mut engine);
    }
    0
}

/// `azra check <file>` — parse-only validation including imports.
fn cmd_check(args: &[String]) -> i32 {
    let file = match args.first() {
        Some(f) => f,
        None => {
            eprintln!("error: `check` requires a script file\n");
            eprint!("{}", USAGE);
            return 2;
        }
    };
    if args.len() > 1 {
        eprintln!("error: unexpected argument `{}`", args[1]);
        return 2;
    }
    match AzraEngine::check_file(file) {
        Ok(()) => {
            println!("ok: {}", file);
            0
        }
        Err(err) => {
            eprintln!("error: {}", err);
            1
        }
    }
}

/// `azra eval <code>` — run a one-liner from the command line.
fn cmd_eval(args: &[String]) -> i32 {
    let source = args.join(" ");
    if source.trim().is_empty() {
        eprintln!("error: `eval` requires source code");
        return 2;
    }
    let mut engine = cli_engine();
    if let Err(err) = engine.eval_str(&source) {
        flush_output(&mut engine);
        eprintln!("error: {}", err);
        return 1;
    }
    flush_output(&mut engine);
    0
}
