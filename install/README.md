# Install Azra

One command, no Rust/cargo/Visual Studio needed. Each release contains exactly
**one executable**: `azra.exe` on Windows, `azra` on Linux/macOS. It includes the
language server (`azra lsp`) and the VS Code extension (`azra setup-vscode`), so
there are no extra files to download or configure.

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

- `azra` — run / check / eval / repl / test / fmt / serve / gui / gui-native / build / pkg / lsp / setup-vscode
- `azra lsp` — language server (stdio JSON-RPC) built into the same exe
- `azra setup-vscode` — installs the embedded VS Code extension directly into `~/.vscode/extensions`, so it works even when the `code` CLI is not on PATH
- `azra setup-path` — adds the exe's own folder to PATH; installers call this command, so the exe and terminal behave identically
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
