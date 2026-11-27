# Changelog

All notable changes to Azra are documented here. This project follows
Semantic Versioning where practical (pre-1.0 versions may break syntax).

## [Unreleased]

### Added

- Package tooling: `azra.toml` manifests, local registry, `azra pkg add/remove/
  install/tree/publish/list`, lockfile and package-aware `use` resolution.
- Standard library modules in `std/`: math, collections, string, vec3, random,
  io, json and testing.
- In-language tests: `test "name":` blocks, `assert`, `assert equals` and
  `assert not_equals`; `azra test [file-or-dir]` runner.
- `azra fmt [--check]` and a shared source formatter.
- `azra-lsp` language server with diagnostics, completion, hover,
  go-to-definition and formatting, plus VS Code and Neovim clients.
- WebAssembly bridge and browser playground in `playground/`.
- Round 1 application frameworks: `std/config`, `log`, `template`, `auth`,
  `orm`, `queue`, `cli`, `metrics`, `i18n`, `upload`, `plugin`, `validate`,
  `gui`; common crypto/encoding/environment host functions; browser GUI via
  `azra gui`; standalone executable builder via `azra build`.
- HTTP server framework: C++ networking core, Rust FFI bridge, `azra serve`,
  `respond`/`http_fetch`/`http_get` host functions, and `std/http`,
  `std/query`, `std/db` modules with `examples/server.vibe`.
- Open-source governance files: LICENSE, CONTRIBUTING, CODE_OF_CONDUCT,
  SECURITY, GOVERNANCE, issue/PR templates and CI.

### Changed

- CLI help and documentation now describe packages, tests, formatting and the
  hosted playground at https://azravibe.com/vibe.
- `use "std/..."` resolves bundled modules; registry dependencies resolve from
  `.azra/packages/`.

### Fixed

- WASM builds no longer panic on startup (wall-clock RNG is replaced by a fixed
  seed on `wasm32`).
- Deep recursion no longer overflows the CLI stack.

## [0.0.1]

- Initial Azra interpreter, CLI, REPL, mock engine objects, math built-ins,
  modules, timers, events, sandbox and `.vibe` stress suites.
- Earlier language audit fixes: `#` comment indentation, NaN handling, modulo
  consistency, self-equality and parser chain limits.
