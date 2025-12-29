# Lumen-Lang

This repository contains a minimal proof-of-concept interpreter for **Lumen**, an experimental programming language. The goal of this v0.0.1 release is to demonstrate simple control flow with if/else statements, while loops, assignments, arithmetic, and printing.

## Features

- Indentation-based syntax inspired by Python.
- Supports `if`/`else` conditionals with comparison operators (`==`, `<`, `>`).
- Supports `while` loops.
- Variables and integer arithmetic.
- Simple `print()` statements.

## Building

You need Rust installed. To build the interpreter:

```bash
git clone https://github.com/ivanshim/lumen-lang.git
cd lumen-lang
cargo build --release
```

## Running

Run a `.lm` program file with:

```bash
cargo run examples/loop.lm
```

The included example prints numbers 0 through 4.
