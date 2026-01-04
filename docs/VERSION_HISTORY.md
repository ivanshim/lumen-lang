# Lumen Version History

This document records each public milestone of Lumen in chronological order.
Each entry is intentionally self-contained so that it remains meaningful even if surrounding detail is trimmed in the future.

---

## v0.0.5 - 2026-01-03
**Contributors:** Ivan Shim orchestrating, GPT-5.2 prompting, Claude Code Haiku 4.5 coding
**Release:** Kernel ontological neutrality, extern correctness, and mathematical proof programs

### What was done:

- **Kernel Refactor to Ontologically Neutral Value System**:
  - Replaced concrete `Value` enum with opaque `RuntimeValue` trait in kernel
  - Kernel now treats all values as abstract types, makes no semantic assumptions
  - Created language-specific value types: `LumenNumber`, `LumenBool`, `LumenString` (Lumen), `MiniRustNumber`, `MiniRustBool` (Mini-Rust), `MiniPythonNumber`, `MiniPythonBool` (Mini-Python)
  - Updated all expressions and statements across all three languages to use language-specific constructors and helpers
  - Implemented safe type downcasting via `as_any()` trait method and language-specific helper functions (`as_number()`, `as_bool()`, `as_string()`)
  - Result: Kernel is now language-independent; all value semantics belong to language modules

- **Extern System Correctness Enforcement**:
  - Fixed design drift: Parser now requires extern selectors to be **string literals**, not identifiers
  - Reject unquoted identifiers (e.g., `extern(print_native, ...)`) with clear error messages
  - Updated all 9 extern example files to use quoted selectors (e.g., `extern("print_native", ...)`)
  - Result: Selectors are now opaque data strings; Lumen makes no assumptions about capability names

- **π and e Examples: Integer-Only, Fixed-Point Implementations**:
  - Replaced all π and e examples with mathematically correct, deterministic integer-only implementations
  - **e (Euler's number)**: Factorial series implementation: e = Σ(1/n!) scaled by SCALE = 10^10
  - **π (Pi)**: Machin's formula with arctangent series: π = 16·arctan(1/5) - 4·arctan(1/239)
  - All arithmetic uses integer operations; decimal point inserted only at output time
  - Updated 6 example files across 3 languages (Lumen, Mini-Python, Mini-Rust)
  - Output format: Separated integer and fractional parts using modulo and division
  - Result: Canonical proof programs demonstrating deterministic integer math for each language

### Key Achievements:
- ✅ Kernel contains zero language-specific assumptions
- ✅ Strings properly implemented as language-level values (not kernel assumptions)
- ✅ Extern system shaped for host-agnostic extensibility
- ✅ Proper abstraction ordering: Kernel → Strings → Extern
- ✅ Clear separation of concerns: Kernel owns mechanics, languages own semantics
- ✅ Canonical proof programs for language correctness

---

## v0.0.4 - 2026-01-02
**Contributors:** Ivan Shim, GPT-5.2 prompting & Claude Code Haiku 4.5 coding
**Release:** Language consolidation and Mini-Python addition

### What was done:
- **Lexical Scoping Implementation**: Added block-scoped environments with proper variable resolution:
  - Each `if/else` block and loop iteration creates a new scope
  - Variable assignments search parent scopes lexically
  - Inner scope variables don't leak to outer scopes
  - All 7 language implementations updated with scoping support
  - 6 new scope test programs demonstrate correct behavior
- **Language Consolidation**: Archived 5 inactive language implementations:
  - `src_mini_php/` → `archive/src_mini_php/`
  - `src_mini_sh/` → `archive/src_mini_sh/`
  - `src_mini_c/` → `archive/src_mini_c/`
  - `src_mini_apple_pascal/` → `archive/src_mini_apple_pascal/`
  - `src_mini_apple_basic/` → `archive/src_mini_apple_basic/`
- **Mini-Python Implementation**: New language module with full feature parity:
  - Indentation-based blocks (Python-like syntax)
  - All expression types: literals, variables, arithmetic, comparison, logical
  - All statement types: assignment, if/else, while, print, break, continue
  - 5 example programs: loop, fibonacci, demo, pi (1000 iterations), e (10 terms)
  - File extensions: `.py` and `.mpy`
- **Project Cleanup**: Updated `src/main.rs` to support only 3 active languages:
  - Lumen (`.lm`)
  - Mini-Rust (`.rs`)
  - Mini-Python (`.py`, `.mpy`)
- **Test Suite Update**: Modified `test_all.sh` for 3-language support (21 total tests)
- **Build Status**: All tests passing, zero compilation errors

---

## v0.0.3 - 2026-01-01
**Contributors:** Ivan Shim & Claude Code Haiku 4.5
**Release:** Lumen multi-language kernel: lumen, rust, php, sh, c, apple pascal, apple basic

### What was done:
- Renamed `framework` module to `kernel` (language-agnostic kernel)
- Implemented 6 additional language modules with full feature parity:
  - **Mini-Rust**: C-style operators (`&&`, `||`, `!`), `let` keyword, semicolons
  - **Mini-PHP**: PHP-style variables (`$var`), `echo` output
  - **Mini-Shell**: Shell-style variables in expressions, shell-like syntax
  - **Mini-C**: C-style syntax, `printf` output
  - **Mini-Pascal**: Pascal-style `:=` assignments, `BEGIN...END` blocks
  - **Mini-Basic**: BASIC-style `LET` and `PRINT` keywords (uppercase)
- Implemented dual language selection system:
  - Priority 1: Explicit `--lang` parameter
  - Priority 2: File extension detection (`.rs`, `.php`, `.sh`, `.c`, `.p`, `.basic`)
- Renamed `demo_v0_1` examples to `demo` across all language modules
- Created mathematical computation examples (pi and e) for all 7 languages
- Updated loop counts: fibonacci (20 → 10), loop (5 → 10)
- Built comprehensive test suite (`test_all.sh`) with auto-discovery
- Fixed EOF token handling for all mini-language modules
- All 35 example programs passing tests

---

## v0.0.2 - 2025-12-31
**Contributors:** Ivan Shim & Claude Code Haiku 4.5
**Release:** Language-agnostic framework architecture

### What was done:
- **Framework/Language Separation**: Split monolithic codebase into language-agnostic `src/framework/` and language-specific `src_lumen/` modules
- **Removed Structural Concepts from Framework**: Eliminated all hardcoded token logic (NEWLINE, INDENT, DEDENT, EOF) from framework parser, lexer, and registry
- **Language-Specific Structural Parsing**: Moved all indentation, newline, and block parsing logic to `src_lumen/structure/structural.rs`
- **Generic Parser**: Framework parser now purely generic—delegates all parsing decisions to registered handlers via trait-based dispatch
- **Plugin Architecture**: Languages can now define custom syntax, tokens, and operators by implementing and registering handlers
- **Documentation Consolidation**: Reorganized docs; BNF.md is now the authoritative grammar specification
- **Verified Functionality**: All examples (loop.lm, fibonacci.lm, demo.lm) tested and working
- **Architectural Achievement**: Framework is now completely language-agnostic. Can support multiple languages with completely different syntax and semantics using the same framework core.

---

## v0.0.1 - 2025-12-30
**Contributors:** Ivan Shim & GPT-5.2
**Release:** Initial working interpreter

### What was done:
- Implemented a full parse → AST → evaluate execution pipeline
- Added indentation-based block parsing
- Implemented `while` loops and `if/else` conditionals
- Added variables, arithmetic, comparisons, and `print()`
- Delivered the first complete, executable Lumen program

---

