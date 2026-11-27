# Install Azra

One command, no Rust/cargo/Visual Studio needed. The interpreter ships with
the entire standard library embedded, plus the VS Code extension.

## Windows

Open PowerShell and paste:

```powershell
irm https://raw.githubusercontent.com/11nawid/azravibe.vibe/main/install/install.ps1 | iex
```

This installs `azra.exe` and `azra-lsp.exe` to `%LOCALAPPDATA%\Azra\bin`,
adds that folder to your user PATH (new terminals), and installs the `.vibe`
VS Code extension if `code` is available.

## Linux / macOS

```sh
curl -fsSL https://raw.githubusercontent.com/11nawid/azravibe.vibe/main/install/install.sh | sh
```

This installs to `~/.azra/bin`, updates your shell profile, and installs the
VS Code extension if `code` is on PATH. Restart your shell afterwards.

## What you get

- `azra` — run `/ check / eval / repl / test / fmt / serve / gui / gui-native / build / pkg`
- `azra-lsp` — language server used by the editor extensions
- `azra-vibe.vsix` — VS Code syntax highlighting + LSP client (installed automatically)
- Embedded `std/` modules — `use "std/math"` works from any folder, no files needed
- `azra build app.vibe --out app.exe` — the resulting exe is self-contained and
  carries the Azra icon; it needs no installed interpreter

## Build a standalone app

```bash
azra build app.vibe --out app.exe
./app.exe
```

The output exe is a copy of the interpreter with your script appended, so it
contains everything: stdlib, host functions, crypto and networking.
