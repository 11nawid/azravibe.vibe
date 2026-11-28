# Self-hosted Azra compiler

This directory is the beginning of a compiler and runtime written **in Azra
itself** (`.vibe`), not in Rust, C, C++ or JavaScript.

## Why this exists

The original `src/` implementation is a Rust interpreter with a C++ native
layer. That was a bootstrap implementation, not the final language. The goal is
that the compiler, VM, standard tooling and apps are Azra programs, with a
one-time bootstrap used only to run the first compiler.

## Files

| File | Purpose |
| --- | --- |
| `lexer.vibe` | Azra-written lexer: indentation, strings, comments, commas |
| `compiler.vibe` | Azra-written parser and bytecode emitter (self-contained) |
| `vm.vibe` | Azra-written stack VM |
| `run.vibe` | Compiles and runs `tests/demo.vibe` |
| `tests/demo.vibe` | Demo program with panels, conditionals and nested loops |
| `tests/self_test.vibe` | Compiler + VM assertions, all in Azra |

`compiler.vibe` deliberately contains its own copy of the lexer helpers so it
can be loaded as a standalone module without depending on module export
behavior.

## Supported subset

The self-hosted compiler currently supports:

- `set NAME to EXPR`
- `say VALUE, VALUE, ...` (space-joined, like the real language)
- `add VALUE to NAME`, `subtract VALUE from NAME`,
  `multiply NAME by VALUE`, `divide NAME by VALUE`
- numbers, strings, `true` / `false`, variables
- `+ - * / modulo`, `equals`, `not_equals`, `less_than`, `less_than=`,
  `greater_than`, `greater_than=`, `and`, `or`, `not`
- `if` / `otherwise` blocks
- `repeat N times` and nested loops
- `give back VALUE`

Not yet in the self-hosted compiler: user functions, lists/maps at runtime,
`use`, events, timers, host functions and native GUI host calls.

## Run it

From the repository root:

```sh
azra run bootstrap/run.vibe
azra run bootstrap/tests/self_test.vibe
```

`azra.exe` here is only the bootstrap host that runs the first Azra compiler.

## Roadmap to a fully native, self-hosted language

1. **Compiler (in progress)** — complete the subset: user functions, lists,
   maps, modules, string helpers.
2. **Self-hosting** — compile `compiler.vibe` with itself and run the resulting
   bytecode on `vm.vibe`; the Rust implementation is no longer needed to build
   the compiler.
3. **Native code emitter** — an Azra-written emitter that turns bytecode into
   a standalone executable (PE/ELF) with a minimal Azra-written runtime, so
   `azra build` no longer depends on a Rust-generated exe.
4. **Native UI** — an Azra-written window/widget layer on top of OS APIs, with
   no browser, HTML or CSS.
5. **Tooling** — formatter, LSP and package manager rewritten in Azra.

The `azra-app/` folder contains a real app already built with the current
self-hosted pipeline: `app.vibe` + `build.vibe` + generated `app.azb`.
