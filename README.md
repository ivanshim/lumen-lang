For language philosophy and evolution constraints, see DESIGN.md.

# Lumen-Lang

Lumen is a minimal, experimental programming language and interpreter.
This repository contains **Lumen v0.0.3**, a multi-language kernel with support for 7 different language implementations.

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
├── src/                                # Lumen kernel (language-agnostic framework)
│   ├── kernel/                         # Core framework
│   │   ├── lexer.rs                    # Pure tokenization
│   │   ├── parser.rs                   # Generic dispatch parser
│   │   ├── registry.rs                 # Token/operator registration
│   │   ├── ast.rs                      # Generic AST traits
│   │   └── runtime/                    # Evaluation engine
│   │       ├── env.rs                  # Scoping and variables
│   │       └── value.rs                # Runtime values
│   └── main.rs                         # Entry point & language dispatcher
├── src_lumen/                          # Lumen language implementation
│   ├── src_lumen.rs                    # Dispatcher
│   ├── structure/
│   │   └── structural.rs               # Indentation, newlines, block parsing
│   ├── statements/ & expressions/      # Lumen statement/expression handlers
│   └── examples/                       # Example programs
├── src_mini_rust/                      # Mini-Rust implementation
├── src_mini_php/                       # Mini-PHP implementation
├── src_mini_sh/                        # Mini-Shell implementation
├── src_mini_c/                         # Mini-C implementation
├── src_mini_apple_pascal/              # Mini-Pascal implementation
├── src_mini_apple_basic/               # Mini-Basic implementation
├── test_all.sh                         # Comprehensive test suite
├── Cargo.toml
└── README.md
```

### Architecture Philosophy

The codebase is split into three layers:

1. **Kernel (`src/kernel/`)**: Language-agnostic infrastructure
   - Pure tokenization, AST representation, evaluation
   - Zero knowledge of language-specific syntax
   - Provides trait-based dispatch for extensible parsing and evaluation

2. **Language Modules (`src_*/`)**: Language-specific implementations
   - Each module implements one language variant
   - Defines tokens, operators, and syntax rules
   - Registers handlers for all statements and expressions
   - Includes language-specific example programs

3. **Dispatcher (`src/main.rs`)**: Multi-language runtime
   - Detects language from `--lang` parameter (priority 1) or file extension (priority 2)
   - Routes program to appropriate language module
   - Supports simultaneous operation of all 7 languages

This design allows the kernel to support multiple languages with different syntaxes and semantics, making it easy to add new language implementations without modifying the core framework.

---

## Version History
