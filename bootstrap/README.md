# Self-hosted Azra compiler

This directory is the beginning of a compiler and runtime written **in Azra
itself** (`.vibe`), not in Rust, C, C++ or JavaScript.

## Why this exists

The original `src/` implementation is a Rust interpreter with a C++ native
layer. That was a bootstrap implementation, not the final language. The goal is
that the compiler, VM, standard tooling and apps are Azra programs, with a
one-time bootstrap used only to run the first compiler.

## Files (all under 500 lines)

| File | Lines | Purpose |
| --- | --- | --- |
| `core.vibe` | 230 | lexer helpers, tokens, AST builders |
| `parser.vibe` | 182 | expression parser and precedence |
| `emit.vibe` | 117 | bytecode instruction emitters |
| `compile_block.vibe` | 302 | statement compiler, loops, patches |
| `compiler.vibe` | 91 | program entry point, functions, prologue |
| `vm.vibe` | 347 | Azra stack VM |
| `pe.vibe` | 212 | byte primitives, PE headers, imports |
| `codegen.vibe` | 373 | bytecode to x64 machine code |
| `protect.vibe` | 17 | HMAC integrity and obfuscation |
| `run.vibe` | 9 | compiles and runs the demo |
| `tests/*.vibe` | 1–16 | self-host, feature and native tests |

The compiler is split into small modules for maintainability. The self-host
test concatenates them into a generated source, so no maintained file exceeds
500 lines.

## Supported subset (Round 1)

The self-hosted compiler currently supports:

- `set NAME to EXPR` and comma-separated list literals
- `set key "k" of MAP to VALUE`
- `say VALUE, VALUE, ...` (space-joined, like the real language)
- `add` / `subtract` / `multiply` / `divide`
- numbers, strings, booleans, lists, maps, variables
- `+ - * / modulo`, all comparisons, `and`, `or`, `not`
- `if` / `otherwise`, `while`, `repeat`, nested loops
- `define NAME with a, b:` and user function calls (`NAME with a, b`)
- builtins: `length`, `uppercase`, `lowercase`, `trim`, `round`, `floor`,
  `ceil`, `abs`, `sqrt`, `sin`, `cos`, `tan`, `exp`, `log`, `log10`, `min`,
  `max`, `magnitude`, `normalize`, `to json`, `parse json`, `read file`,
  `file exists`, `item`, `key`, `join`, `split`, `current time`,
  `random decimal`, `empty list`, `empty map`

Also supported: `for each ... in ...`, `repeat with i from a to b [by n]`,
`skip next`, `stop loop`, bare function-call statements, escape sequences in
string literals and the `replace` builtin.

Not yet supported: `use` modules, events, timers, `try`, host functions and
native GUI calls.

## Honest status

**Round 1 is complete.** The compiler compiles its own source into bytecode,
and running that bytecode on the Azra VM produces **byte-identical** output.
Verified by `bootstrap/tests/selfhost.vibe`
(`self-host fixed point passed: 3775 instructions`). All regression tests pass.

Key fixes that closed the gap: `contains`/`starts with` operators, zero-arg
function calls in the VM, `remove item` statements, correct `strip_colon` for
quoted strings containing colons, shared global state across VM calls, and
proper break/continue patching.

## Round 2 progress

`codegen.vibe` now emits a complete **1536-byte PE32+ console executable**
entirely in Azra: DOS/PE/COFF headers, optional header, import directory for
`kernel32.dll` (`GetStdHandle`, `WriteFile`, `ExitProcess`), `.text` and
`.idata` sections, and x64 machine code that prints a message and exits 0.
Verified by `bootstrap/tests/native.vibe` and by running the generated EXE:

```
AZRA NATIVE HELLO
exit:0
```

`bootstrap/tests/bytecode_native.vibe` closes the loop: `.vibe` source is
compiled to bytecode by the self-hosted compiler, then translated to x64
machine code and emitted as a PE. The generated EXE prints
`AZRA NATIVE BYTECODE` and exits 0.

`protect.vibe` adds HMAC-SHA256 integrity + base64url obfuscation to bytecode
bundles. See `SECURITY.md` for the honest limits.

Round 3 (native Windows GUI, Studio, Player) has not started yet.

Rounds 2 and 3 (native code emitter / PE / native Windows GUI) have not
started.

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
