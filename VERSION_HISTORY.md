# Lumen Version History

This document records each public milestone of Lumen in chronological order.
Each entry is intentionally self-contained so that it remains meaningful even if surrounding detail is trimmed in the future.

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

