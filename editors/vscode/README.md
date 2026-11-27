# Azra VS Code extension

Provides `.vibe` syntax highlighting, an `azra-lsp` language client (diagnostics,
completion, hover, go-to-definition, formatting) and **Azra: Run Current File**.

## Install from source

```bash
cargo build --release --bin azra-lsp
cd editors/vscode
npm install
npm run compile   # optional: the extension is plain JS
code --install-extension .
```

Set `azra.languageServer` to the full path of `azra-lsp` if it is not on `PATH`.

Website: https://azravibe.com/vibe
