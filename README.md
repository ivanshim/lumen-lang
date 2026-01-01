For language philosophy and evolution constraints, see DESIGN.md.

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
while x < 5
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
cargo run src_lumen/examples/loop.lm
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
cargo run src_lumen/examples/loop.lm
```

---

## Repository Structure

```
lumen-lang/
├── src/                           # Language-agnostic framework
│   ├── framework/
│   │   ├── lexer.rs              # Pure tokenization
│   │   ├── parser.rs             # Generic dispatch parser
│   │   ├── registry.rs           # Token/operator registration
│   │   ├── ast.rs                # Generic AST traits
│   │   └── runtime/              # Evaluation engine
│   │       ├── env.rs            # Scoping and variables
│   │       └── value.rs          # Runtime values
│   └── main.rs                   # Entry point
├── src_lumen/                     # Lumen language implementation
│   ├── dispatcher.rs             # Lumen handler registration
│   ├── structure/
│   │   └── structural.rs         # Indentation, newlines, block parsing
│   ├── statements/               # Lumen statement implementations
│   │   ├── assignment.rs
│   │   ├── if_else.rs
│   │   ├── print.rs
│   │   └── while_loop.rs
│   ├── expressions/              # Lumen expression implementations
│   │   ├── arithmetic.rs
│   │   ├── comparison.rs
│   │   ├── grouping.rs
│   │   └── identifier.rs
│   ├── examples/
│   │   ├── loop.lm
│   │   ├── fibonacci.lm
│   │   └── demo_v0_1.lm
│   └── docs/                     # Lumen design documentation
│       ├── DESIGN.md
│       ├── BNF.md
│       ├── ROADMAP.md
├── Cargo.toml
└── README.md
```

### Architecture Philosophy

The codebase is split into two layers:

1. **Framework (`src/framework/`)**: Language-agnostic infrastructure
   - Pure tokenization, AST representation, evaluation
   - Zero knowledge of language-specific syntax (colons, indentation, newlines, etc.)
   - Provides trait-based dispatch for extensible parsing and evaluation

2. **Language Module (`src_lumen/`)**: Lumen-specific implementation
   - Defines tokens, operators, and syntax rules
   - Implements indentation-based block parsing
   - Registers handlers for all Lumen statements and expressions

This design allows the framework to support multiple languages with different syntaxes and semantics.

---

## Version History

For a detailed version history and contributor credits, see [VERSION_HISTORY.md](VERSION_HISTORY.md).

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
