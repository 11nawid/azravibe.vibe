# Azra (`.vibe`)

**Website: https://azravibe.com/vibe** · **Playground: https://azravibe.com/vibe** · **Source: https://github.com/11nawid/azravibe.vibe**

**Azra** is the English-language, general-purpose & game-scripting language of the
**Azravibe** ecosystem. Azra scripts run through a standalone interpreter (this
repository) either from the command line or embedded inside the *Azravibe Studio*
3D rounded-voxel builder.

> Ecosystem naming:
> - **Azravibe (`.azr`)** — Persian programming language & RTL IDE
> - **Azra (`.vibe`)** — General-purpose & game scripting language *(this repo)*
> - **Azravibe Studio** — 3D rounded-voxel builder, physics & IDE
> - **Azravibe Player** — Multiplayer game launcher & runtime

## Language highlights

- **Zero brackets** — no `{}`, `[]` or `()`. Blocks start with `:` and use
  exactly **4-space indentation** (`INDENT` / `DEDENT` tokens).
- **Comments** — `note:`, `#` and `/* ... */` (multi-line). An optional trailing
  period `.` at the end of a statement is ignored.
- **English operators** — `equals`, `not_equals`, `less_than`, `less_than=`,
  `greater_than`, `greater_than=`, plus `is`, `is not`, `contains`,
  `starts with`, `has key`, `and`, `or`, `not`, math `+ - * / modulo power`.
- **Variables** — `set health to 100`; lists — `set items to "sword", "shield"`;
  arithmetic — `add 10 to health`, `subtract 25 from health`,
  `multiply health by 2`, `divide health by 2`, `add "key" to items`,
  `remove "key" from items`. Integer division truncates; use a decimal operand
  (`7 / 2.0`) for a fractional result.
- **Objects** — `make rounded_box as lava_block` creates a dynamic object whose
  properties are read/written with dot notation: `set lava_block.color to "red"`.
  Mock engine types (`rounded_box`, `sound`, `player`, …) are registered so game
  scripts run and log actions cleanly in standalone CLI mode. `player` is
  pre-created with `health` 100.
- **Maps** — `set m to empty map`, `set key "hp" of m to 100`,
  `say key "hp" of m`, `keys of m`, `m has key "hp"`.
- **Output** — `say` prints values joined by spaces, `warn` prints `[WARN] …`,
  `error` prints `[ERROR] …` (both to stderr).
- **Control flow** — `if …:` / `otherwise if …:` / `otherwise:`,
  `repeat N times:`, `repeat with i from 1 to 10 by 2:`,
  `for each item in list:`, `while cond:`, `stop loop` / `break`,
  `skip next` / `continue`.
- **Functions** — `define heal_player with target, amount:` … `give back value`,
  called **without parentheses**: `heal_player with player, 25`.
- **Modules & errors** — `use "weapons.vibe" as w`,
  `try:` … `if error as err:` … to catch `fail with "message"`.
- **Timers & files** — `after 2 seconds:`, `every 1 seconds:`,
  `wait 0.5 seconds`, `write data to file path`, `read file path`,
  `parse json text`, `to json value`.
- **Engine events** — `when player touches lava_block:` blocks are registered as
  event handlers and can be triggered from the CLI or tests.
- **String interpolation** — `say "Welcome, {player.name}!"`.
- **Advanced math** — `pi`, `e`, `tan`/`asin`/`acos`/`atan`/`atan2`,
  `log`/`log10`/`exp`, `lerp`, degree/radian conversions, and 3D vector
  `dot`/`cross`/`magnitude`/`normalize`/`distance`/`angle between`.
- **Built-ins** — math, text, time, RNG, vectors/colors and a JSON codec.

The full language reference is [`docs/language.vibetxt`](docs/language.vibetxt).

### Math stress suites

Runnable `.vibe` scripts under `tests/vibe-test/` cover recursion
(Fibonacci, factorial, Ackermann), a 1..10000 prime sieve, Newton-Raphson,
quadratics, 3D projectile physics, precision/overflow and every math built-in:

```bash
cargo run -- run tests/vibe-test/00_math_stress.vibe
cargo test --test vibe_scripts
```

Also included: `09_euler500.vibe` solves Project Euler #500 in pure Azra
(answer `35407281`, modulo 500500507) and `10_three_body.vibe` simulates the
3-4-5 three-star system with adaptive RK4, stopping on ejection or a close
binary while keeping relative energy drift below 10^-9 (`cargo test -- --ignored`
runs the slower Euler suite).

### Example

```vibe
note: variables, objects, loops and functions — no brackets anywhere
set inventory to "sword", "shield", "potion"
make rounded_box as lava_block
set lava_block.color to "red"

define heal_player with target, amount:
    add amount to target.health
    give back target.health

if first item of inventory equals "sword":
    say "Equipped the sword!"
otherwise:
    say "Nothing to equip."

when player touches lava_block:
    say "Game Over!"
```

## Building

```bash
cargo build --release
```

## CLI usage

Run a script (the `run` keyword is optional for `.vibe` files):

```bash
cargo run -- run examples/hello.vibe
cargo run -- examples/hello.vibe
```

Check a script without executing it, or evaluate a snippet directly:

```bash
cargo run -- check examples/game_demo.vibe
cargo run -- eval 'say "hi from Azra"'
```

Run the game demo and trigger an engine event or simulate engine frames:

```bash
cargo run -- run examples/game_demo.vibe --trigger "player touches lava_block"
cargo run -- run examples/game_demo.vibe --ticks 120
```

Interactive REPL (also starts when `azra` is run with no arguments):

```bash
cargo run -- repl
```

Inside the REPL: `:help`, `:vars`, `:reset`, `:clear`, `:quit` / `:exit`,
and `leave azra` to exit. Multi-line blocks are entered with 4-space indentation.

### Packages & standard library

```bash
azra init my-game                       # create azra.toml + main.vibe
azra pkg publish ./my-math-lib           # publish to your local registry
azra pkg add my-math-lib@1.0.0           # add a dependency
azra pkg install                        # install into .azra/packages + azra.lock
azra pkg tree                           # show the locked dependency tree
azra pkg list                           # list registry packages
```

`use "std/math"`, `use "std/collections"`, `use "std/string"`,
`use "std/vec3"`, `use "std/random"`, `use "std/io"`, `use "std/json"` and
`use "std/testing"` load curated modules shipped with the interpreter.
Installed packages are resolved with `use "package-name/module"`.

### Testing Azra code

Write tests directly in `.vibe` files:

```vibe
test "addition works":
    assert equals 2 + 2, 4
    assert "azra" contains "az", "substring check"

test "vectors":
    set d to dot of vec3 1, 0, 0 and vec3 0, 1, 0
    assert equals d, 0
```

Run them with `azra test tests/` (or pass a single file). Assertions support
`assert CONDITION`, `assert equals A, B` and `assert not_equals A, B`, with an
optional message. See `std/testing.vibe` for `check`, `check_equal` and
`check_close` helpers.

### Editor tooling

`cargo build --release --bin azra-lsp` builds a dependency-free language server
(JSON-RPC over stdio) with diagnostics, completion, hover, go-to-definition and
formatting. Ready-to-use clients live in `editors/vscode/` and
`editors/neovim/`.

### Formatting

`azra fmt main.vibe` rewrites blocks to exact 4-space indentation;
`azra fmt --check main.vibe` only reports whether formatting is needed.

### Browser playground

`playground/` contains a Monaco-based web IDE that runs Azra through
WebAssembly with shareable URLs. Build and serve it with:

```bash
cargo build --release --target wasm32-unknown-unknown --lib
cp target/wasm32-unknown-unknown/release/azra.wasm playground/azra.wasm
cd playground && python -m http.server 8000
```

### Standalone engine verbs

Game scripts can call engine actions directly — in standalone CLI mode they
are logged cleanly instead of touching a 3D scene:

```vibe
play boom_sfx
destroy lava_block
jump player with 15
spawn zombie
move player
rotate lava_block
wait
```

→ `[ENGINE] play <sound file=explosion.wav volume=1>` etc.

`add "gold_key" to loot` also bootstraps `loot` as a list when the variable
does not exist yet (numbers still require `set coins to 0` first).

## Embedding as a library

```rust
use azra::AzraEngine;

let mut engine = AzraEngine::new();
engine.eval_str(r#"say "hello from Rust!""#).unwrap();
engine.register_function("spawn_enemy", |args| { /* ... */ Ok(Value::Null) });
let output = engine.take_output();
```

Use `AzraEngine::with_sandbox(SandboxConfig::strict())` to cap instructions and
disable I/O when running untrusted scripts.

## Repository layout

| Path                    | Purpose                                                  |
|-------------------------|----------------------------------------------------------|
| `src/lib.rs`            | Public embedding API (`AzraEngine`)                      |
| `src/main.rs`           | CLI: `azra run`, `check`, `eval`, `repl`                  |
| `src/cli/`              | REPL and colorized output presentation                   |
| `src/error.rs`          | `AzraError` with line/column diagnostics                 |
| `src/sandbox.rs`        | Security and resource-budget `SandboxConfig`             |
| `src/env.rs`            | Lexical scope chain                                      |
| `src/token.rs`          | Token definitions                                        |
| `src/lexer/`            | Tokenizer with line/column tracking & INDENT/DEDENT      |
| `src/ast/`              | AST node definitions (`stmt`, `expr`, `target`)          |
| `src/parser/`           | Recursive descent parser with line-numbered errors       |
| `src/value.rs`          | Runtime `Value` enum (`Object`, `Map`, `Vec3`, `Color`)   |
| `src/stdlib/`           | Built-in math, text, time, RNG, and JSON codec           |
| `src/interpreter/`      | Tree-walking VM, collections, events, timers, modules    |
| `docs/language.vibetxt` | The Azra language specification                          |
| `examples/`             | `.vibe` sample scripts                                   |
| `tests/`                | Integration, feature, error, CLI and regression suites   |
| `tests/vibe-test/`      | Runnable `.vibe` math/algorithm stress suites            |
| `std/`                  | Standard library modules written in Azra                 |
| `playground/`           | Browser IDE with a WebAssembly interpreter build         |
| `editors/`              | VS Code and Neovim language-server clients               |
| `.github/`              | Issues, PR templates and continuous integration          |

## Open source & contributing

Azra is public, MIT-licensed and developed at
https://github.com/11nawid/azravibe.vibe. Everyone is welcome to use, learn
from and report issues. See [CONTRIBUTING.md](CONTRIBUTING.md) for the current
contributor policy, commit rules and development workflow, and
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community expectations.
The project homepage is https://azravibe.com/vibe.

## License

MIT — see [LICENSE](LICENSE).
