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

## Version History & Contributors

This section records each public milestone of Lumen in chronological order.
Each entry is intentionally self-contained so that it remains meaningful even if surrounding detail is trimmed in the future.

---

### v0.0.3 · 2026-01-01 · Ivan Shim & Claude Code Haiku 4.5 · Lumen multi-language kernel

**Ivan Shim**

* Feature specification and direction

**Claude Code Haiku 4.5**

* Implementation of 6 additional language modules, dual language selection system, comprehensive test suite

**Key changes**

* Renamed framework module to `kernel` (language-agnostic kernel)
* Added 6 new language implementations:
  - Mini-Rust: C-style operators (`&&`, `||`, `!`), `let` keyword, semicolons, braces
  - Mini-PHP: PHP-style variables (`$var`), `echo` output, `and`/`or` keywords
  - Mini-Shell: Shell-style variables (`$var` in expressions), sh-like syntax
  - Mini-C: C-style operators and `printf` output, curly braces
  - Mini-Pascal: Pascal-style `:=` assignments, `BEGIN...END` blocks, `writeln` output
  - Mini-Basic: BASIC-style `LET` and `PRINT` keywords (uppercase)
* Implemented dual language selection:
  - Explicit `--lang` parameter (priority 1)
  - File extension detection (priority 2)
  - Examples: `.rs` → mini-rust, `.php` → mini-php, `.sh` → mini-sh, `.c` → mini-c, `.p` → mini-pascal, `.basic` → mini-basic
* Renamed `demo_v0_1` examples to `demo` across all language modules
* Created mathematical examples (pi and e computation) for all 7 languages
* Updated loop examples: fibonacci (20 → 10 iterations), loop (5 → 10 iterations)
* Built comprehensive test suite (`test_all.sh`) with auto-discovery of examples
* Fixed EOF token handling for all mini-language modules

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
