//! Tree-walking interpreter for Azra v1.0.0: scoped environment, function
//! call stack, engine event handlers, timers, modules, host functions and
//! the built-in statements.
//!
//! The interpreter is embeddable: a host such as *Azravibe Studio/Player*
//! drives it with [`Interpreter::start`] and [`Interpreter::tick`] at 60 FPS,
//! while the CLI runs scripts to completion and sleeps on `wait`.
//!
//! The implementation is split into focused submodules:
//! - [`stmt`] — statement dispatch, functions, events, timers, I/O
//! - [`collections`] — list, map, and numeric target mutations
//! - [`control`] — conditional branches and loops
//! - [`expr`] — expression evaluation (including binary operators)
//! - [`target`] — assignment targets (`key ... of`, `item N of`, ...)
//! - [`functions`] — calls: user functions, host functions, engine verbs
//! - [`modules`] — `use "file.vibe"` loading with cache + cycle detection
//! - [`timers`] — `every` / `after` timers
//! - [`mock`] — mock engine objects (`make rounded_box ...`)
//! - [`helpers`] — shared utilities (line numbers, number math, phrases)

mod collections;
mod control;
mod expr;
mod functions;
mod helpers;
mod mock;
mod modules;
mod stmt;
mod target;
mod timers;

use crate::ast::*;
use crate::env::Scope;
use crate::error::AzraError;
use crate::sandbox::SandboxConfig;
use crate::stdlib::Rng;
use crate::token::Num;
use crate::value::{ObjectData, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

pub use self::modules::resolve_module_path;

/// Safety cap for `repeat` / `while` loops.
const MAX_LOOP_ITERATIONS: u64 = 10_000_000;

/// Mock engine verbs understood in standalone CLI mode.
const ENGINE_VERBS: &[&str] = &[
    "play", "destroy", "jump", "spawn", "move", "rotate", "wait",
];

use self::helpers::normalize_phrase;

/// Which stream a line of output belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// One buffered line produced by `say` / `warn` / `error` or an engine verb.
#[derive(Debug, Clone, PartialEq)]
pub struct OutputLine {
    pub level: OutputLevel,
    /// `text` already contains the `[WARN]` / `[ERROR]` / `[ENGINE]` prefix;
    /// color is applied by the presenter (CLI).
    pub text: String,
}

impl OutputLine {
    pub fn stream(&self) -> OutputStream {
        match self.level {
            OutputLevel::Say => OutputStream::Stdout,
            OutputLevel::Warn | OutputLevel::Error => OutputStream::Stderr,
        }
    }
}

/// A registered `when <event>:` handler.
#[derive(Debug)]
struct EventHandler {
    phrase: String,
    normalized: String,
    params: Vec<String>,
    body: Rc<Vec<Stmt>>,
    closure: Rc<Scope>,
}

/// A scheduled `every N seconds:` / `after N seconds:` timer.
#[derive(Debug, Clone)]
struct Timer {
    /// Seconds between fires (`every`) or delay until fire (`after`).
    interval: f64,
    /// Script-clock time of the next fire.
    next_due: f64,
    repeating: bool,
    body: Rc<Vec<Stmt>>,
    closure: Rc<Scope>,
    line: usize,
}

type HostFn = Box<dyn Fn(&[Value]) -> Result<Value, AzraError>>;

/// Execution effect of a statement list.
enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

/// The Azra virtual machine.
pub struct Interpreter {
    globals: Rc<Scope>,
    scope: Rc<Scope>,
    handlers: Vec<EventHandler>,
    output: Vec<OutputLine>,
    host: HashMap<String, HostFn>,
    call_depth: usize,
    in_function: usize,
    /// Security / resource configuration.
    sandbox: SandboxConfig,
    /// Random source for `random number ...` / `random decimal`.
    rng: Rng,
    /// Script clock (seconds). Advanced by `tick` and by `wait`.
    now: f64,
    /// `true` while `start()` / `tick()` is running (affects `wait`).
    in_tick: bool,
    /// Scheduled `every` / `after` timers.
    timers: Vec<Timer>,
    /// Unique id source for `make` objects.
    next_id: u64,
    /// Objects created with `make` (host world queries).
    world: Vec<Value>,
    /// Loaded modules keyed by canonical path (import cache).
    modules: HashMap<PathBuf, Vec<(String, Value)>>,
    /// Canonical paths currently being loaded (circular import detection).
    module_stack: Vec<PathBuf>,
    /// Directory of the file currently being executed (for `use`).
    current_dir: Option<PathBuf>,
    /// Instructions executed since the last public entry point.
    instructions: usize,
}

impl Interpreter {
    /// Fresh VM with mock engine types and a default `player` entity.
    pub fn new() -> Self {
        Interpreter::with_sandbox(SandboxConfig::permissive())
    }

    /// Fresh VM with a specific sandbox configuration.
    pub fn with_sandbox(sandbox: SandboxConfig) -> Self {
        let globals = Scope::root();

        // Default player so game scripts (see docs example) run standalone.
        let mut player = ObjectData::new("player");
        player.id = 1;
        player.set("name", Value::Str("player".to_string()));
        player.set("health", Value::Number(Num::Int(100)));
        player.set(
            "position",
            Value::list(vec![
                Value::Number(Num::Int(0)),
                Value::Number(Num::Int(0)),
                Value::Number(Num::Int(0)),
            ]),
        );
        globals.define("player", Value::object(player));

        Interpreter {
            globals: globals.clone(),
            scope: globals,
            handlers: Vec::new(),
            output: Vec::new(),
            host: HashMap::new(),
            call_depth: 0,
            in_function: 0,
            sandbox,
            rng: Rng::from_clock(),
            now: 0.0,
            in_tick: false,
            timers: Vec::new(),
            next_id: 1,
            world: Vec::new(),
            modules: HashMap::new(),
            module_stack: Vec::new(),
            current_dir: None,
            instructions: 0,
        }
    }

    // ------------------------------------------------------------ public API

    /// Execute a parsed program in the global scope.
    pub fn run(&mut self, program: &[Stmt]) -> Result<(), AzraError> {
        self.begin_tick_budget();
        self.scope = self.globals.clone();
        match self.exec_block(program)? {
            Flow::Return(_) => Err(AzraError::runtime(
                "`give back` outside of a function",
            )),
            Flow::Break | Flow::Continue => Err(AzraError::runtime(
                "`stop loop` / `skip next` outside of a loop",
            )),
            Flow::Normal => Ok(()),
        }
    }

    /// Register a native host function callable from Azra scripts as
    /// `my_func with arg1, arg2`.
    pub fn register_function<F>(&mut self, name: &str, func: F)
    where
        F: Fn(&[Value]) -> Result<Value, AzraError> + 'static,
    {
        self.host.insert(name.to_string(), Box::new(func));
    }

    /// Drain buffered output (`say` / `warn` / `error` / engine logs).
    pub fn take_output(&mut self) -> Vec<OutputLine> {
        std::mem::take(&mut self.output)
    }

    /// Read a variable from the global scope.
    pub fn get_global(&self, name: &str) -> Option<Value> {
        self.globals.get(name)
    }

    /// Create or overwrite a global variable.
    pub fn set_global(&mut self, name: &str, value: Value) {
        self.globals.define(name, value);
    }

    /// Names of all global variables (REPL `:vars`).
    pub fn global_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.globals.entries().into_iter().map(|(k, _)| k).collect();
        names.sort();
        names
    }

    /// Phrases of all registered `when` handlers.
    pub fn registered_events(&self) -> Vec<String> {
        self.handlers.iter().map(|h| h.phrase.clone()).collect()
    }

    /// Current value of the script clock (seconds).
    pub fn now(&self) -> f64 {
        self.now
    }

    /// Objects created with `make`, in creation order.
    pub fn get_world_objects(&self) -> Vec<Value> {
        self.world.clone()
    }

    /// Set the directory used to resolve `use "..."` imports.
    pub fn set_current_dir(&mut self, dir: Option<PathBuf>) {
        self.current_dir = dir;
    }

    /// Run every `when` handler whose phrase matches `phrase`
    /// (no arguments — see [`Interpreter::trigger_event_with_args`]).
    pub fn trigger_event(&mut self, phrase: &str) -> Result<(), AzraError> {
        self.trigger_event_with_args(phrase, &[])
    }

    /// Run every `when` handler whose phrase matches `phrase`, binding
    /// `args` to the handler's declared `with` parameters (missing arguments
    /// become `null`, extra arguments are ignored).
    pub fn trigger_event_with_args(
        &mut self,
        phrase: &str,
        args: &[Value],
    ) -> Result<(), AzraError> {
        self.begin_tick_budget();
        self.trigger_event_inner(phrase, args)
    }

    fn trigger_event_inner(
        &mut self,
        phrase: &str,
        args: &[Value],
    ) -> Result<(), AzraError> {
        let normalized = normalize_phrase(phrase);
        let matching: Vec<usize> = self
            .handlers
            .iter()
            .enumerate()
            .filter(|(_, h)| h.normalized == normalized)
            .map(|(i, _)| i)
            .collect();
        if matching.is_empty() {
            let known = self.registered_events();
            let hint = if known.is_empty() {
                "no `when` handlers are registered".to_string()
            } else {
                format!("registered events: {}", known.join(" | "))
            };
            return Err(AzraError::runtime(format!(
                "no event handler matches `{}` ({})",
                phrase, hint
            )));
        }
        for idx in matching {
            let (body, closure, params) = {
                let h = &self.handlers[idx];
                (h.body.clone(), h.closure.clone(), h.params.clone())
            };
            // Bind trigger arguments in a child of the handler's closure.
            let saved = self.scope.clone();
            let scope = Scope::new(&closure);
            for (i, param) in params.iter().enumerate() {
                let value = args.get(i).cloned().unwrap_or(Value::Null);
                scope.define(param, value);
            }
            self.scope = scope;
            let flow = self.exec_block(&body);
            self.scope = saved;
            match flow? {
                Flow::Normal => {}
                Flow::Return(_) => {
                    return Err(AzraError::runtime(
                        "`give back` outside of a function",
                    ))
                }
                Flow::Break | Flow::Continue => {
                    return Err(AzraError::runtime(
                        "`stop loop` / `skip next` outside of a loop",
                    ))
                }
            }
        }
        Ok(())
    }

    /// Engine start: run every `when game starts:` handler.
    /// Scripts without such a handler are fine (nothing runs).
    pub fn start(&mut self) -> Result<(), AzraError> {
        self.begin_tick_budget();
        self.in_tick = true;
        let result = self.trigger_event_optional("game starts", &[]);
        self.in_tick = false;
        result
    }

    /// Advance the engine by `delta_time` seconds: run
    /// `when game updates with delta_time:` handlers, then fire due timers.
    pub fn tick(&mut self, delta_time: f64) -> Result<(), AzraError> {
        if delta_time.is_nan() || delta_time < 0.0 || delta_time.is_infinite() {
            return Err(AzraError::runtime(format!(
                "delta_time must be a finite, non-negative number, got {}",
                delta_time
            )));
        }
        self.begin_tick_budget();
        self.in_tick = true;
        self.now += delta_time;
        let update = self.trigger_event_optional(
            "game updates",
            &[Value::Number(Num::Float(delta_time))],
        );
        let result = match update {
            Ok(()) => self.fire_due_timers(),
            Err(e) => Err(e),
        };
        self.in_tick = false;
        result
    }

    /// Like [`Interpreter::trigger_event_with_args`] but a missing handler
    /// is not an error — used for the engine lifecycle events so plain
    /// scripts can run without `when game starts:` / `game updates:`.
    fn trigger_event_optional(
        &mut self,
        phrase: &str,
        args: &[Value],
    ) -> Result<(), AzraError> {
        let normalized = normalize_phrase(phrase);
        let has_handler = self.handlers.iter().any(|h| h.normalized == normalized);
        if !has_handler {
            return Ok(());
        }
        self.trigger_event_inner(phrase, args)
    }

    // ---------------------------------------------------------------- budget

    fn begin_tick_budget(&mut self) {
        self.instructions = 0;
    }

    fn count_instruction(&mut self) -> Result<(), AzraError> {
        self.instructions += 1;
        if self.instructions > self.sandbox.max_instructions_per_tick {
            return Err(AzraError::fatal_runtime(format!(
                "instruction budget exceeded (max {} per tick)",
                self.sandbox.max_instructions_per_tick
            )));
        }
        Ok(())
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Interpreter::new()
    }
}
