# Lumen-Lang

A minimal, experimental programming language interpreter framework exploring language design, multi-kernel architectures, and separation of concerns between kernel logic and language semantics.

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

# Rust (Rust-style curly braces)
cargo run examples/rust/demo.rs

# Python (Python-like syntax)
cargo run examples/python/fibonacci.py
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
| **Rust** | `.rs` | Rust-style curly braces | ✅ Active |
| **Python** | `.py` | Python-like syntax | ✅ Active |

---

## Development Tracks

Lumen has **4 independent development tracks**, each with its own architecture and purpose:

### Track 1: Microcode Kernel (`src_microcode/kernel/`)
- **Design**: Data-driven, table-based execution engine
- **Pipeline**: 4-stage (Ingest → Structure → Reduce → Execute)
- **Principle**: ALL language semantics in declarative schemas, ZERO semantic logic in kernel code
- **Language-Agnostic**: Kernel makes no assumptions about syntax or semantics

### Track 2: Stream Kernel (`src_stream/kernel/`)
- **Design**: Procedural, AST-based execution engine
- **Pipeline**: Parse → AST → Tree-Walking Interpreter
- **Principle**: Language-agnostic core with trait-based handler dispatch
- **Language-Agnostic**: Generic parser delegates all decisions to registered handlers

### Track 3: Lumen Language (Primary Language)
- **Stream Implementation**: `src_stream/languages/lumen/`
- **Microcode Implementation**: `src_microcode/languages/lumen/`
- **Style**: Python-style indentation-based syntax
- **Role**: Reference implementation demonstrating both kernels

### Track 4: Other Language Examples
- **Rust**: `src_stream/languages/rust/` + `src_microcode/languages/rust/`
  - Rust-style curly braces and `let` bindings
- **Python**: `src_stream/languages/python/` + `src_microcode/languages/python/`
  - Python-like syntax with indentation

**Key Design Principle**: Each kernel is **completely independent** with zero cross-imports, allowing independent evolution. The `src/main.rs` dispatcher routes between kernels and languages.

---

## Testing

Run the full test suite (68 tests across all kernels and languages):

```bash
./test_all.sh
```

**Current Status**: ✅ **All 68 tests passing** (48 Lumen, 10 Python, 10 Rust)

---

## Documentation

Comprehensive documentation is organized in the `docs/` directory:

### Project Structure
- [**DIRECTORY_STRUCTURE.txt**](docs/DIRECTORY_STRUCTURE.txt) - Complete directory and file organization

### Kernel Architecture
- [**LUMEN_KERNEL_STREAM.md**](docs/LUMEN_KERNEL_STREAM.md) - Stream kernel design and philosophy
- [**LUMEN_KERNEL_MICROCODE.md**](docs/LUMEN_KERNEL_MICROCODE.md) - Microcode kernel design and 4-stage pipeline

### Lumen Language Documentation
- [**LUMEN_LANGUAGE_BNF.md**](docs/LUMEN_LANGUAGE_BNF.md) - Lumen grammar specification
- [**LUMEN_LANGUAGE_DESIGN.md**](docs/LUMEN_LANGUAGE_DESIGN.md) - Language design and semantics
- [**LUMEN_LANGUAGE_EXTERN_SYSTEM.md**](docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md) - External function system design
- [**LUMEN_LANGUAGE_ROADMAP.md**](docs/LUMEN_LANGUAGE_ROADMAP.md) - Planned features and improvements

### Release Information
- [**VERSION_HISTORY.md**](docs/VERSION_HISTORY.md) - Complete version history and release notes

---

## Key Features

### Dual-Kernel Architecture
- **Stream Kernel**: Immediate AST-based execution
- **Microcode Kernel**: Data-driven schema-based execution
- **Independent Evolution**: Both kernels fully tested, zero cross-imports

### Multi-Language Support
- **Lumen**: Python-style indentation-based syntax
- **Rust**: Rust-style curly brace syntax
- **Python**: Python-like syntax with indentation

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
cargo run examples/rust/demo.rs
cargo run examples/python/fibonacci.py
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

**Status**: ✅ All 68 tests passing (48 Lumen, 10 Python, 10 Rust)

For release notes and version history, see **docs/VERSION_HISTORY.md**

For planned enhancements, see **docs/LUMEN_LANGUAGE_ROADMAP.md**
