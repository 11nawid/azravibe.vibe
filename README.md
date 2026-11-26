# Azra (`.vibe`)

**Azra** is the English-language, general-purpose & game-scripting language of the
**Azravibe** ecosystem. Azra scripts run through a standalone interpreter/VM (this
repository) either from the command line or embedded inside the *Azravibe Studio*
3D rounded-voxel builder.

> Ecosystem naming:
> - **Azravibe (`.azr`)** — Persian programming language & RTL IDE
> - **Azra (`.vibe`)** — General-purpose & game scripting language *(this repo)*
> - **Azravibe Studio** — 3D rounded-voxel builder, physics & IDE
> - **Azravibe Player** — Multiplayer game launcher & runtime

## Language highlights

- **Zero brackets** — no `{}`, `[]` or `()`. Blocks start with `:` and use
  **4-space indentation** (`INDENT` / `DEDENT` tokens).
- **Comments** start with `note:` and run to the end of the line. An optional
  trailing period `.` at the end of a statement is gracefully ignored.
- **English operators** — comparisons are single tokens:
  `equals`, `not_equals`, `less_than`, `less_than=`, `greater_than`,
  `greater_than=`, plus `and`, `or`, `not` and standard math `+ - * /`.
- **Variables** — `set health to 100`, lists — `set items to "sword", "shield"`,
  arithmetic — `add 10 to health`, `subtract 25 from health`,
  `multiply health by 2`, `divide health by 2`, lists — `add "key" to items`,
  `remove "key" from items`.
- **Objects** — `make rounded_box as lava_block` creates a dynamic object whose
  properties are read/written with dot notation: `set lava_block.color to "red"`.
  Mock engine types (`rounded_box`, `sound`, `player`, …) are registered so game
  scripts run and log actions cleanly in standalone CLI mode.
- **Output** — `say` prints values joined by spaces, `warn` prints a yellow
  `[WARN] …`, `error` prints a red `[ERROR] …`.
- **Control flow** — `if …:` / `otherwise if …:` / `otherwise:`, `repeat N times:`,
  `for each item in list:`, `while cond:`.
- **Functions** — `define heal_player with target, amount:` … `give back value`,
  called **without parentheses**: `heal_player with player, 25`.
- **Engine events** — `when player touches lava_block:` blocks are registered as
  event handlers and can be triggered from the CLI or tests.

### Example

```vibe
note: variables, objects, loops and functions — no brackets anywhere
set inventory to "sword", "shield", "potion"
make rounded_box as lava_block
set lava_block.color to "red"

define heal_player with target, amount:
    add amount to target.health
    give back target.health

if inventory equals "sword":
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

Run a script:

```bash
cargo run -- run examples/hello.vibe
```

Run the game demo and trigger an engine event:

```bash
cargo run -- run examples/game_demo.vibe
cargo run -- run examples/game_demo.vibe --trigger "player touches lava_block"
```

Interactive REPL:

```bash
cargo run -- repl
```

## Embedding as a library

```rust
use azra::AzraEngine;

let mut engine = AzraEngine::new();
engine.eval_str(r#"say "hello from Rust!""#).unwrap();
engine.register_function("spawn_enemy", |args| { /* ... */ Ok(Value::Null) });
let output = engine.take_output();
```

## Repository layout

| Path                    | Purpose                                          |
|-------------------------|--------------------------------------------------|
| `src/lib.rs`            | Public embedding API (`AzraEngine`)             |
| `src/main.rs`           | CLI: `azra run <file>` and `azra repl`          |
| `src/token.rs`          | Token definitions                                |
| `src/lexer.rs`          | Tokenizer with line/column tracking & INDENT/DEDENT |
| `src/ast.rs`            | AST node definitions                             |
| `src/parser.rs`         | Recursive descent parser with line-numbered errors |
| `src/value.rs`          | Runtime `Value` enum                             |
| `src/interpreter.rs`    | Scoped environment, call stack, built-ins        |
| `docs/langauge.vibetxt` | The Azra language specification                  |
| `examples/`             | `.vibe` sample scripts                           |
| `tests/`                | Integration tests covering every language feature |

## License

MIT (see repository metadata).
