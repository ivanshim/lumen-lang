# Lumen-Lang

A minimal, experimental programming language interpreter framework with multiple kernel architectures and language implementations.

Lumen v0.0.5 features dual-kernel architecture (Stream & Microcode), support for 3 active languages (Lumen, Mini-Rust, Mini-Python), and comprehensive documentation.

---

## Quick Start

### Requirements
- Git
- Rust ([https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install))

### Installation & Running

```bash
git clone https://github.com/ivanshim/lumen-lang.git
cd lumen-lang
cargo build
cargo run examples/lumen/loop.lm
```

### Example Programs

```bash
# Lumen (Python-style indentation)
cargo run examples/lumen/fibonacci.lm

# Mini-Rust (Rust-style curly braces)
cargo run examples/mini_rust/demo.rs

# Mini-Python (Python-like syntax)
cargo run examples/mini_python/fibonacci.py
```

---

## Language Features

All supported languages include:
- ✅ Variables and arithmetic expressions
- ✅ Comparison and logical operators
- ✅ If/else conditionals
- ✅ While loops with break/continue
- ✅ Proper operator precedence
- ✅ Lexical scoping

### Supported Languages

| Language | File Extension | Style | Status |
|----------|---|---|---|
| **Lumen** | `.lm` | Python-style indentation | ✅ Active |
| **Mini-Rust** | `.rs` | Rust-style curly braces | ✅ Active |
| **Mini-Python** | `.py` | Python-like syntax | ✅ Active |

---

## Architecture

Lumen features a **dual-kernel design**:

### Stream Kernel
- **Style**: Procedural, AST-based execution
- **Approach**: Parse source → build AST → evaluate tree-walking interpreter
- **Status**: Fully tested with all 3 languages
- **Code**: `src_stream/`

### Microcode Kernel
- **Style**: Data-driven, 4-stage pipeline
- **Approach**: Ingest → Structure → Reduce → Execute
- **Principle**: ALL language semantics in declarative schemas, ZERO semantic logic in kernel code
- **Status**: Complete with all 3 languages
- **Code**: `src_microcode/`

Both kernels are **completely independent** with zero cross-imports, allowing independent evolution.

### Design Principles

1. **Kernel Neutrality**: Kernels own only algorithms (tokenization, parsing, execution mechanics)
2. **Language Modularity**: Languages own syntax, semantics, value types, operators
3. **Schema-Driven Semantics**: In the microcode kernel, ALL language behavior is table-driven
4. **Clear Separation**: `src/main.rs` dispatcher routes between kernels and languages

---

## Repository Structure

```
lumen-lang/
├── src/                          # Binary entry point and shared schema
│   ├── main.rs                   # CLI dispatcher (kernels/languages router)
│   └── schema/                   # Common type definitions
├── src_stream/                   # Stream Kernel (procedural, AST-based)
│   ├── kernel/                   # Lexer, parser, registry, evaluator
│   ├── languages/                # Lumen, Mini-Rust, Mini-Python implementations
│   └── schema/                   # Stream-specific AST definitions
├── src_microcode/                # Microcode Kernel (data-driven, 4-stage pipeline)
│   ├── kernel/                   # Ingest → Structure → Reduce → Execute
│   ├── languages/                # Language schemas (Rust & YAML)
│   ├── runtime/                  # Extern function dispatch
│   └── schema/                   # Declarative schema system
├── examples/                     # 35+ example programs
│   ├── lumen/                    # 24 Lumen examples
│   ├── mini_python/              # 5 Mini-Python examples
│   └── mini_rust/                # 5 Mini-Rust examples
├── docs/                         # Comprehensive documentation
│   ├── 00_KERNEL_STREAM.md      # Stream kernel overview
│   ├── 01_KERNEL_MICROCODE.md   # Microcode kernel architecture
│   ├── 02_LANGUAGES.md           # Language-level design guide
│   ├── 03_VERSION_HISTORY.md     # Release notes and changelog
│   ├── 04_LUMEN_BNF.md           # Lumen grammar specification
│   ├── 05_LUMEN_DESIGN.md        # Lumen language design
│   ├── 06_LUMEN_EXTERN_SYSTEM.md # Extern system design
│   └── 07_LUMEN_ROADMAP.md       # Future development roadmap
├── Cargo.toml                    # Rust project configuration
└── test_all.sh                   # Comprehensive test suite
```

---

## Testing

Run the full test suite (68 tests across all kernels and languages):

```bash
./test_all.sh
```

**Current Status**: ✅ **All 68 tests passing** (48 Lumen, 10 Mini-Python, 10 Mini-Rust)

---

## Documentation

Comprehensive documentation is organized in the `docs/` directory:

### Getting Started
- Start with the main README (this file)
- Review **docs/00_KERNEL_STREAM.md** for kernel philosophy
- Check **docs/02_LANGUAGES.md** for language-level overview

### Architecture Deep Dives
- **docs/01_KERNEL_MICROCODE.md** - Microcode kernel design and 4-stage pipeline
- **docs/02_LANGUAGES.md** - Complete language module reference
- **docs/00_KERNEL_STREAM_REFERENCE.md** - Stream kernel design philosophy

### Lumen Language Documentation
- **docs/04_LUMEN_BNF.md** - Lumen grammar specification
- **docs/05_LUMEN_DESIGN.md** - Language design and semantics
- **docs/06_LUMEN_EXTERN_SYSTEM.md** - External function system design
- **docs/07_LUMEN_ROADMAP.md** - Planned features and improvements

### Release Information
- **docs/03_VERSION_HISTORY.md** - Complete version history and release notes

---

## Key Features (v0.0.5)

### Dual-Kernel Architecture
- Stream kernel: Immediate AST-based execution
- Microcode kernel: Data-driven schema-based execution
- Both kernels fully tested and independent

### Multi-Language Support
- **Lumen**: Python-style indentation-based syntax
- **Mini-Rust**: Rust-style curly brace syntax
- **Mini-Python**: Python-like syntax with indentation

### Language Neutrality
- Kernel contains zero language-specific assumptions
- Values are opaque `RuntimeValue` traits (not kernel enums)
- All semantics belong to language modules

### Proper Abstractions
- Strings implemented at language level (not kernel)
- Extern system designed for host-agnostic extensibility
- Clear separation: Kernel owns mechanics, languages own semantics

### Mathematical Proof Programs
- Canonical π (Pi) computation using Machin's formula
- Canonical e (Euler's number) using factorial series
- Integer-only, fixed-point implementations with deterministic output
- Available in all 3 languages

---

## v0.0.5 Release Highlights

**Date:** 2026-01-03
**Key Achievement:** Kernel ontological neutrality, extern system correctness, mathematical proof programs

### What's New
1. **Kernel Refactored to Ontological Neutrality**: Values are now opaque traits, not kernel enums
2. **Extern System Correctness**: String literal selectors for host-agnostic extensibility
3. **Canonical Proof Programs**: Integer-only, deterministic math implementations
4. **Mini-Rust Stream Kernel**: Fixed statement registration order and whitespace handling
5. **Full Test Coverage**: 68/68 tests passing (all kernels, all languages)

For detailed version history, see **docs/03_VERSION_HISTORY.md**.

---

## Project Philosophy

Lumen is **not a production language** but an exploration of:
- Language design and parsing mechanics
- Multi-kernel execution architectures
- Separation of concerns (kernel vs. language semantics)
- Data-driven language design patterns
- Canonical implementations of complex algorithms

---

## Usage Examples

### Run with Automatic Language Detection

```bash
# Detect language from file extension
cargo run examples/lumen/fibonacci.lm
cargo run examples/mini_rust/demo.rs
cargo run examples/mini_python/fibonacci.py
```

### Run with Explicit Kernel Selection

```bash
# Stream kernel (default for most)
cargo run -- --kernel stream examples/lumen/pi.lm

# Microcode kernel
cargo run -- --kernel microcode examples/lumen/pi.lm
```

### Example Output

```bash
$ cargo run examples/lumen/loop.lm
0
1
2
3
4
```

---

## Contributing

This is an educational project. Contributions for:
- New language implementations
- Additional kernel architectures
- Improved documentation
- Bug fixes and optimizations

are welcome!

---

## License

This project is provided as-is for educational and experimental purposes.

---

## Contact & Attribution

**Project Lead**: Ivan Shim
**Contributors**: GPT-5.2 (prompting), Claude Code Haiku 4.5 (implementation)
**Repository**: https://github.com/ivanshim/lumen-lang

---

**Last Updated**: 2026-01-04
**Status**: ✅ All tests passing, production-quality implementation
**Next Steps**: See docs/07_LUMEN_ROADMAP.md for planned enhancements
