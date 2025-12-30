# Lumen-Lang

Lumen is a minimal, experimental programming language and interpreter.
This repository contains **Lumen v0.0.1**, a working proof-of-concept implemented in Rust.

The goal of Lumen is not to be a production language, but to explore language design, parsing, and execution semantics in the smallest possible form that still feels *real*.

---

## Features

* Indentation-based syntax (Python-style blocks)
* `while` loops
* `if / else` conditionals
* Comparison operators: `==`, `<`, `>`, `!=`
* Variables and numeric arithmetic
* Simple `print()` statements
* AST-based interpreter (parse → evaluate)

---

## Example

```lm
x = 0
while x < 5:
    print(x)
    x = x + 1
```

Output:

```
0
1
2
3
4
```

---

## Getting Started

### Option A: Run with GitHub Codespaces (Recommended)

This option requires **no local setup** and runs entirely in your browser.

#### Step 1: Open the repository

```
https://github.com/ivanshim/lumen-lang
```

#### Step 2: Start a Codespace

1. Click **Code**
2. Select **Codespaces**
3. Click **Create codespace on main**
4. Wait for the VS Code environment to load

#### Step 3: Install Rust (if not already installed)

```bash
curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
```

Verify installation:

```bash
rustc --version
```

#### Step 4: Build the interpreter

```bash
cargo build
```

#### Step 5: Run the example program

```bash
cargo run examples/loop.lm
```

---

### Option B: Run Locally

#### Requirements

* Git
* Rust ([https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install))

#### Steps

```bash
git clone https://github.com/ivanshim/lumen-lang.git
cd lumen-lang
cargo build
cargo run examples/loop.lm
```

---

## Repository Structure

```
lumen-lang/
├── examples/
│   └── loop.lm        # Sample Lumen program
├── src/
│   ├── ast.rs         # Abstract Syntax Tree definitions
│   ├── parser.rs      # Indentation-aware parser
│   ├── eval.rs        # Interpreter / evaluator
│   └── main.rs        # Entry point
├── Cargo.toml
└── README.md
```

---

## Version History & Contributors

This section records each public milestone of Lumen in chronological order.
Each entry is intentionally self-contained so that it remains meaningful even if surrounding detail is trimmed in the future.

---

### v0.0.1 · 2025-12-30 · Ivan Shim & GPT-5.2 · Initial working interpreter

**Ivan Shim**

* Language design, architecture, and implementation

**GPT-5.2**

* Pair programming, debugging, parser design, documentation

**Key changes**

* Implemented a full parse → AST → evaluate execution pipeline
* Added indentation-based block parsing
* Implemented `while` loops and `if / else` conditionals
* Added variables, arithmetic, comparisons, and `print()`
* Delivered the first complete, executable Lumen program

---

## Design Notes

* Indentation is significant (4 spaces per level)
* Tabs are not currently supported
* Error handling is intentionally minimal in v0.0.1
* The AST is the single source of truth for language semantics

---

## License

MIT License.
This project is intended for learning, experimentation, and exploration.
