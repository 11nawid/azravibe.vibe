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
- Round 5: one-command installers for Windows/Linux/macOS with PATH setup and
  automatic VS Code extension installation; all stdlib modules embedded in the
  executable; `azra build` now creates a self-contained exe by appending the
  script to the interpreter (no Rust/C++ toolchain needed); official icon in
  the exe; GitHub Actions release workflow for all platforms.
- Round 4: set, stack, priority, url, jwt, ledger, backup, audit, stringfmt,
  cli_table, retry and token modules with self-hosted tests. Official high-res
  logo and ICO; `azra build` now embeds the icon in Windows executables and the
  playground uses it as its favicon.
- Round 3: native Win32 GUI core (`azra gui-native`, `std/nativegui`) with no
  browser, plus stat, pagination, cookie, tween, geo, matrix, graph, astar and
  physics modules and their self-hosted test suite.
- Round 2 application frameworks: `std/session`, `middleware`, `ratelimit`,
  `cache`, `worker`, `mail`, `report`, `chart`, `search`, `schedule`, `state`,
  `eventbus`, `httpclient`, `security`; restyled README with logo, badges,
  Mermaid architecture diagram and Instagram support section.
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
