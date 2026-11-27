# Introduction to Azra

Welcome to the **Azra Language** tutorial! This guide will take you from zero knowledge to building real programs, step-by-step, using plain English explanations.

## What is Azra?
- A lightweight, interpreted scripting language.
- Designed for teaching programming concepts and for game scripting.
- Syntax looks like natural English, making it easy to read.
- Azra scripts use the `.vibe` file extension (the older `.azra` extension is not used by this interpreter).

## How to run Azra code
1. Build the interpreter from this repository with `cargo build --release`.
2. Run a script:
   ```bash
   azra run my_script.vibe
   ```
   The `run` keyword is optional: `azra my_script.vibe` works too.
3. Start an interactive REPL with:
   ```bash
   azra repl
   ```
   Running `azra` with no arguments also opens the REPL.

## Your first program
Create a file `hello.vibe` with:
```vibe
say "Hello, world!"
```
Run it:
```bash
azra run hello.vibe
```
You should see *Hello, world!* printed.

---
Proceed to the next lesson: **Basics**.
