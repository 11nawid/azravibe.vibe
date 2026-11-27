# Azra Web Playground

A browser IDE for Azra: Monaco editor (with a textarea fallback), live
diagnostics-ready source editing, shareable URLs and an interpreter compiled to
WebAssembly. Nothing is sent to a server — execution happens locally.

Live site: **https://azravibe.com/vibe**

## Build the interpreter for WebAssembly

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --lib
cp target/wasm32-unknown-unknown/release/azra.wasm playground/azra.wasm
```

## Serve locally

Because `fetch()` is used to load the `.wasm` file, open the folder through a
local HTTP server (not `file://`):

```bash
cd playground
python -m http.server 8000
# visit http://localhost:8000
```

## Features

- Run with the button or **Ctrl+Enter**; output and exit code appear on the right.
- Example programs: hello, math/vectors, recursion and tests.
- **Share** copies a URL with the source encoded in the hash (`#code=...`).
- Source is kept in `localStorage` between visits.
- Monaco is loaded from a CDN; if it is unavailable the textarea still works.

The WASM bridge (`src/wasm.rs`) exports `azra_input_ptr`, `azra_input_capacity`,
`azra_run`, `azra_output_ptr` and `azra_output_len` — no JavaScript glue crates.
