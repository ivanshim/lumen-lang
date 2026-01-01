# Lumen Version History

This document records each public milestone of Lumen in chronological order.
Each entry is intentionally self-contained so that it remains meaningful even if surrounding detail is trimmed in the future.

---

### v0.0.2 - 2025-12-31 - Ivan Shim & Claude Code Haiku 4.5 - Language-agnostic framework architecture

**Ivan Shim**

* Language design, architecture vision, direction, and project management

**Claude Code Haiku 4.5**

* Full refactoring to language-agnostic framework, code reorganization, testing, documentation

**Key changes**

* **Framework/Language Separation**: Split monolithic codebase into language-agnostic `src/framework/` and language-specific `src_lumen/` modules
* **Removed Structural Concepts from Framework**: Eliminated all hardcoded token logic (NEWLINE, INDENT, DEDENT, EOF) from framework parser, lexer, and registry
* **Language-Specific Structural Parsing**: Moved all indentation, newline, and block parsing logic to `src_lumen/structure/structural.rs`
* **Generic Parser**: Framework parser now purely generic—delegates all parsing decisions to registered handlers via trait-based dispatch
* **Plugin Architecture**: Languages can now define custom syntax, tokens, and operators by implementing and registering handlers
* **Documentation Consolidation**: Reorganized docs; BNF.md is now the authoritative grammar specification
* **Verified Functionality**: All examples (loop.lm, fibonacci.lm, demo_v0_1.lm) tested and working

**Architectural Achievement**: Framework is now completely language-agnostic. Can support multiple languages (Lumen, mini-Python, mini-C, etc.) with completely different syntax and semantics using the same framework core.

---

### v0.0.1 - 2025-12-30 - Ivan Shim & GPT-5.2 - Initial working interpreter

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
