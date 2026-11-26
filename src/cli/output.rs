//! Output presentation for `say`, `warn`, and `error` lines.

use azra::{AzraEngine, OutputLine, OutputStream};
use std::io::{IsTerminal, Write};

const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const RESET: &str = "\x1b[0m";

/// Write buffered output to stdout/stderr, coloring warn/error on a TTY.
pub fn flush_output(engine: &mut AzraEngine) {
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
