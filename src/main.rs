//! Azra CLI — `azra run <file.vibe>`, `azra check`, `azra repl` and
//! `azra eval`.

mod cli;

use azra::AzraEngine;
use cli::output::flush_output;
use cli::repl::cmd_repl;
use std::path::PathBuf;

const USAGE: &str = "\
Azra (.vibe) interpreter — Azravibe ecosystem

USAGE:
    azra                               start the interactive interpreter
    azra run <script.vibe> [--ticks <n>] [--trigger \"<event phrase>\"]...
    azra check <script.vibe>
    azra repl
    azra eval <code>
    azra serve <app.vibe> [--port 8080]      run an HTTP server app
    azra gui <app.vibe> [--port 8080]        serve a GUI app and open a browser
    azra gui-native <app.vibe> [--title T]   native Win32 window (no browser)
    azra build <app.vibe> [--name N] [--out P]  build a standalone executable
    azra test [file-or-directory]            run `test \"...\":` blocks
    azra fmt [--check] <file.vibe>           format a source file
    azra init [name]                       create azra.toml + main.vibe
    azra pkg add <name[@version]>          add a dependency
    azra pkg remove <name>                 remove a dependency
    azra pkg install                       install packages + write azra.lock
    azra pkg tree                          show the locked dependency tree
    azra pkg publish <dir>                 publish a local package
    azra pkg list                          list registry packages
    azra lsp                                   language server (stdio JSON-RPC)
    azra setup-vscode                          install editor support
    azra setup-path                            add this exe's folder to PATH
    azra --help
    azra --version

OPTIONS:
    --vm              execute with the bytecode VM instead of the tree walker
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
    // Executables produced by `azra build` carry their script as an overlay
    // and run it directly — no interpreter arguments or extra files needed.
    if let Some(source) = azra::embedded::read_self() {
        match azra::run_source_with_hosts(&source) {
            Ok(text) => print!("{text}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }

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
        Some("lsp") => {
            azra::lsp::run();
            0
        }
        Some("setup-path") => match azra::path_setup::setup() {
            Ok(message) => {
                println!("{message}");
                0
            }
            Err(error) => {
                eprintln!("error: {error}");
                1
            }
        },
        Some("setup-vscode") => match azra::embedded_vsix::install() {
            Ok(message) => {
                println!("{message}");
                0
            }
            Err(error) => {
                eprintln!("error: {error}");
                1
            }
        },
        Some("serve") => cmd_serve(&args[1..]),
        Some("gui") => cmd_gui(&args[1..]),
        Some("gui-native") => cmd_gui_native(&args[1..]),
        Some("build") => cmd_build(&args[1..]),
        Some("test") => cmd_test(&args[1..]),
        Some("fmt") => cmd_fmt(&args[1..]),
        Some("init") => cmd_init(&args[1..]),
        Some("pkg") => cmd_pkg(&args[1..]),
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
    let mut engine = AzraEngine::with_sandbox(azra::SandboxConfig {
        max_call_depth: 512,
        ..azra::SandboxConfig::permissive()
    });
    azra::host::register_common(&mut engine);
    engine
}

/// `azra run <file> [--ticks n] [--trigger phrase]...`
fn cmd_run(args: &[String]) -> i32 {
    let mut file: Option<String> = None;
    let mut triggers: Vec<String> = Vec::new();
    let mut ticks: u64 = 0;
    let mut use_vm = false;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--vm" {
            use_vm = true;
        } else if arg == "--trigger" {
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

    if use_vm {
        let source = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("error: cannot read `{file}`: {error}");
                return 1;
            }
        };
        return match azra::eval_str_vm(&source) {
            Ok(lines) => {
                for line in lines {
                    println!("{line}");
                }
                0
            }
            Err(error) => {
                eprintln!("error: {error}");
                1
            }
        };
    }

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

fn collect_vibe_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_vibe_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("vibe"))
        {
            out.push(path);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn app_and_port(args: &[String]) -> Result<(String, u16), i32> {
    let mut file = None;
    let mut port = 8080u16;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = match args.get(i).and_then(|s| s.parse().ok()) {
                    Some(p) => p,
                    None => {
                        eprintln!("error: `--port` needs a number");
                        return Err(2);
                    }
                };
            }
            arg if arg.starts_with("--port=") => {
                port = match arg[7..].parse() {
                    Ok(p) => p,
                    Err(_) => {
                        eprintln!("error: `--port` needs a number");
                        return Err(2);
                    }
                };
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option `{arg}`");
                return Err(2);
            }
            arg if file.is_none() => file = Some(arg.to_string()),
            arg => {
                eprintln!("error: unexpected argument `{arg}`");
                return Err(2);
            }
        }
        i += 1;
    }
    file.map(|f| (f, port)).ok_or_else(|| {
        eprintln!("error: an app file is required");
        2
    })
}

/// `azra gui-native <app.vibe>` — real Win32 window driven by Azra.
#[cfg(not(target_arch = "wasm32"))]
fn cmd_gui_native(args: &[String]) -> i32 {
    let mut file = None;
    let mut title = "Azra App".to_string();
    let mut width = 480i32;
    let mut height = 320i32;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--title" => {
                i += 1;
                title = args.get(i).cloned().unwrap_or(title);
            }
            "--width" => {
                i += 1;
                width = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(width);
            }
            "--height" => {
                i += 1;
                height = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(height);
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option `{arg}`");
                return 2;
            }
            arg if file.is_none() => file = Some(arg.to_string()),
            arg => {
                eprintln!("error: unexpected argument `{arg}`");
                return 2;
            }
        }
        i += 1;
    }
    let Some(file) = file else {
        eprintln!("error: `azra gui-native` requires an app file");
        return 2;
    };
    match azra::gui_app::NativeApp::new(std::path::Path::new(&file), &title, width, height) {
        Ok(mut app) => match app.run() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn cmd_gui_native(_args: &[String]) -> i32 {
    eprintln!("error: `azra gui-native` is unavailable in WebAssembly builds");
    2
}

/// `azra gui <app.vibe> [--port 8080]` — serve a GUI app and open the browser.
#[cfg(not(target_arch = "wasm32"))]
fn cmd_gui(args: &[String]) -> i32 {
    let (file, port) = match app_and_port(args) {
        Ok(value) => value,
        Err(code) => return code,
    };
    match azra::server::ServerApp::new(std::path::Path::new(&file), port) {
        Ok(mut app) => {
            let url = format!("http://127.0.0.1:{port}/");
            match azra::host::open_url(&url) {
                Ok(true) => println!("Opened {url} in your browser"),
                Ok(false) => println!("Open {url} in your browser"),
                Err(e) => println!("Open {url} in your browser ({e})"),
            }
            match app.run() {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("error: {e}");
                    1
                }
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn cmd_gui(_args: &[String]) -> i32 {
    eprintln!("error: `azra gui` is unavailable in WebAssembly builds");
    2
}

/// `azra build <app.vibe> [--name N] [--out P]`
#[cfg(not(target_arch = "wasm32"))]
fn cmd_build(args: &[String]) -> i32 {
    let mut file = None;
    let mut name = "app".to_string();
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--name" => {
                i += 1;
                name = args.get(i).cloned().unwrap_or_else(|| "app".to_string());
            }
            "--out" => {
                i += 1;
                out = args.get(i).map(PathBuf::from);
            }
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option `{arg}`");
                return 2;
            }
            arg if file.is_none() => file = Some(arg.to_string()),
            arg => {
                eprintln!("error: unexpected argument `{arg}`");
                return 2;
            }
        }
        i += 1;
    }
    let Some(file) = file else {
        eprintln!("error: `azra build` requires an app file");
        return 2;
    };
    let source = PathBuf::from(&file);
    let dest = out.unwrap_or_else(|| {
        PathBuf::from(format!(
            "{}.{}",
            source.file_stem().unwrap_or_default().to_string_lossy(),
            if cfg!(windows) { "exe" } else { "bin" }
        ))
    });
    match azra::builder::build_exe(&source, &name, &dest) {
        Ok(path) => {
            println!("built: {}", path.display());
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn cmd_build(_args: &[String]) -> i32 {
    eprintln!("error: `azra build` is unavailable in WebAssembly builds");
    2
}

/// `azra serve <app.vibe> [--port 8080]` — run an HTTP server app.
#[cfg(not(target_arch = "wasm32"))]
fn cmd_serve(args: &[String]) -> i32 {
    let mut file: Option<String> = None;
    let mut port: u16 = 8080;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<u16>().ok()) {
                    Some(p) => port = p,
                    None => {
                        eprintln!("error: `--port` requires a number between 0 and 65535");
                        return 2;
                    }
                }
            }
            arg if arg.starts_with("--port=") => match arg[7..].parse::<u16>() {
                Ok(p) => port = p,
                Err(_) => {
                    eprintln!("error: `--port` requires a valid port number");
                    return 2;
                }
            },
            arg if arg.starts_with('-') => {
                eprintln!("error: unknown option `{arg}`");
                return 2;
            }
            arg if file.is_none() => file = Some(arg.to_string()),
            arg => {
                eprintln!("error: unexpected argument `{arg}`");
                return 2;
            }
        }
        i += 1;
    }
    let Some(file) = file else {
        eprintln!("error: `azra serve` requires an app file\n");
        eprint!("{}", USAGE);
        return 2;
    };
    match azra::server::ServerApp::new(std::path::Path::new(&file), port) {
        Ok(mut app) => match app.run() {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn cmd_serve(_args: &[String]) -> i32 {
    eprintln!("error: `azra serve` is unavailable in WebAssembly builds");
    2
}

/// `azra test [file-or-directory]` — run all `test "name":` blocks.
fn cmd_test(args: &[String]) -> i32 {
    let target = args.first().map(String::as_str).unwrap_or("tests");
    let path = std::path::Path::new(target);
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        collect_vibe_files(path, &mut files);
    } else {
        eprintln!("error: no such file or directory: {target}");
        return 2;
    }
    files.retain(|p| std::fs::read_to_string(p).is_ok_and(|text| text.contains("test \"")));
    files.sort();
    if files.is_empty() {
        println!("no .vibe test files found under {target}");
        return 0;
    }

    let mut passed = 0usize;
    let mut failed = 0usize;
    for file in files {
        let mut engine = cli_engine();
        if let Err(err) = engine.eval_file(&file) {
            println!("FAIL {} (load error): {}", file.display(), err);
            failed += 1;
            continue;
        }
        let results = engine.run_tests();
        for line in engine.take_output() {
            println!("  | {}", line.text);
        }
        if results.is_empty() {
            println!("SKIP {} (no `test \"...\":` blocks)", file.display());
        }
        for result in results {
            if result.passed {
                println!("PASS {} :: {}", file.display(), result.name);
                passed += 1;
            } else {
                println!(
                    "FAIL {} :: {}: {}",
                    file.display(),
                    result.name,
                    result.message
                );
                failed += 1;
            }
        }
    }
    println!("\n{passed} passed, {failed} failed");
    if failed == 0 { 0 } else { 1 }
}

/// `azra fmt [--check] <file.vibe>`
fn cmd_fmt(args: &[String]) -> i32 {
    let check = args.iter().any(|a| a == "--check");
    let Some(file) = args.iter().find(|a| !a.starts_with("--")) else {
        eprintln!("error: `azra fmt [--check] <file.vibe>` requires a file");
        return 2;
    };
    let path = std::path::Path::new(file);
    let Ok(source) = std::fs::read_to_string(path) else {
        eprintln!("error: cannot read {file}");
        return 1;
    };
    let formatted = azra::format::format_source(&source);
    if check {
        if source == formatted {
            println!("ok: {file}");
            0
        } else {
            println!("needs formatting: {file}");
            1
        }
    } else {
        match std::fs::write(path, formatted) {
            Ok(()) => {
                println!("formatted: {file}");
                0
            }
            Err(e) => {
                eprintln!("error: cannot write {file}: {e}");
                1
            }
        }
    }
}

fn cmd_init(args: &[String]) -> i32 {
    let name = args.first().cloned().unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_else(|| "my-azra-project".to_string())
    });
    match azra::pkg::init(std::path::Path::new("."), &name) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn cmd_pkg(args: &[String]) -> i32 {
    let project = std::path::Path::new(".");
    let result = match args.first().map(String::as_str) {
        Some("add") => match args.get(1) {
            Some(spec) => azra::pkg::add(project, spec),
            None => Err("`azra pkg add <name[@version]>` requires a name".to_string()),
        },
        Some("remove") => match args.get(1) {
            Some(name) => azra::pkg::remove(project, name),
            None => Err("`azra pkg remove <name>` requires a name".to_string()),
        },
        Some("install") => azra::pkg::install(project),
        Some("tree") => azra::pkg::tree(project),
        Some("publish") => match args.get(1) {
            Some(dir) => azra::pkg::publish(std::path::Path::new(dir)),
            None => Err("`azra pkg publish <dir>` requires a directory".to_string()),
        },
        Some("list") => azra::pkg::registry_list(),
        _ => Err("usage: azra pkg add|remove|install|tree|publish|list [args]".to_string()),
    };
    match result {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
