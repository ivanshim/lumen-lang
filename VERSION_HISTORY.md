# Lumen-Lang Version History

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

## v0.0.1 - 2025-12-30
**Contributors:** Ivan Shim & GPT-5.2
**Release:** Initial working interpreter

### What was done:
- Implemented Lumen language parser and interpreter
- Indentation-based syntax (Python-style blocks)
- Support for `while` loops, `if/else` conditionals
- Variables, arithmetic operators, comparison operators
- `print()` statements
- AST-based evaluation engine
