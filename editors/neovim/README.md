# Azra for Neovim

Attach the `azra-lsp` binary to `.vibe` files. Build it first:

```bash
cargo build --release --bin azra-lsp
```

Then add this to `init.lua` (adjust the path to your checkout):

```lua
vim.filetype.add({ extension = { vibe = "vibe" } })

vim.api.nvim_create_autocmd("FileType", {
  pattern = "vibe",
  callback = function()
    vim.lsp.start({
      name = "azra-lsp",
      cmd = { "/path/to/azravibe.vibe/target/release/azra-lsp" },
      root_dir = vim.fs.dirname(vim.fs.find({ "azra.toml", ".git" }, { upward = true })[1]),
    })
  end,
})
```

You now get diagnostics, completion, hover, go-to-definition and `:lua vim.lsp.buf.format()`.
Run the current file with `:!azra run %`.

To install the VS Code extension instead, see `editors/vscode/README.md`.
