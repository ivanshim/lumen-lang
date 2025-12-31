# Refactoring Analysis: Framework Agnosticism

## Overview
The framework currently contains language-specific logic that violates the "truly language-agnostic" principle. This document outlines what needs to be refactored.

---

## 1. LEXER.rs - INDENTATION HANDLING ❌

### Current Problem
Lines 36-88 of `src/framework/lexer.rs` contain **hardcoded indentation processing**:
- 4-space indent detection (lines 44-48)
- INDENT token generation (line 66)
- DEDENT token generation (lines 70, 84)
- Indentation validation (lines 62, 73)

This is **purely Lumen/Python-specific**.

### Language Differences
Different languages handle indentation completely differently:
- **Python/Lumen**: Indentation-based blocks (4-space indents)
- **C/Rust/Java**: Brace-based blocks `{ }`
- **Pascal**: `begin...end` blocks
- **BASIC**: Line numbers or no blocks
- **Whitespace handling**: Tabs vs spaces, variable indent sizes

### Solution
**Split lexer into two layers:**

1. **Framework Lexer** (lines 91-182 of current lexer.rs):
   - Pure character → token conversion
   - Handles strings, numbers, identifiers, operators
   - No indentation processing
   - Language-agnostic

2. **Language Lexer** (in `src_lumen/structure/structural.rs`):
   - Takes framework lexer output
   - Does language-specific preprocessing
   - Generates INDENT/DEDENT tokens for indentation-based languages
   - Skipped entirely for brace-based languages

### Refactoring Steps
```
BEFORE:
  Source Code
      ↓
  Framework Lexer (with indentation hardcoded)
      ↓
  Token Stream (with INDENT/DEDENT)

AFTER:
  Source Code
      ↓
  Framework Lexer (pure tokenization)
      ↓
  Token Stream (no indentation tokens)
      ↓
  Language Lexer (src_lumen/structure/structural.rs)
      ↓
  Token Stream (with INDENT/DEDENT for Python-style)
```

---

## 2. PARSER.rs - BLOCK PARSING ⚠️

### Current Situation
`src/framework/parser.rs` has `parse_block()` method (probably around line ~50-80).

### Questions to Investigate
- Does `parse_block()` assume INDENT/DEDENT tokens exist?
- Does it have hardcoded block syntax assumptions?
- Can it work with brace-based blocks?

### Likely Refactoring
If parser assumes INDENT/DEDENT:
- Move block-parsing logic to language modules
- Keep parser fully generic (just expression/statement parsing)
- Let each language define how blocks are parsed

---

## 3. VALUE.rs - PLACEMENT DECISION ❓

### Current: In framework/runtime/value.rs
All languages share the same `Value` enum:
```rust
pub enum Value {
    Number(f64),
    Bool(bool),
}
```

### Arguments for Framework (current):
- ✅ Enables language interop
- ✅ Simplifies framework
- ✅ All languages use same value representation
- ✅ Reduces code duplication

### Arguments for Language-Specific:
- ✅ True language independence
- ✅ Mini-C might need raw pointers/references
- ✅ Mini-Rust might need ownership semantics
- ✅ Mini-Python might need strings/lists/dicts

### Recommendation
**Keep in framework for now**, but:
- Document the assumption
- Design framework to be extensible if languages need custom Value types
- Revisit if adding mini-C or mini-Rust

---

## 4. DOCS & EXAMPLES ✅ (ALREADY DONE)

Moved to:
- `src_lumen/docs/` ✅
- `src_lumen/examples/` ✅

These are language-specific and belong with the language definition.

---

## 5. DIR_STRUCTURE.txt ✅ (ALREADY DONE)

Created and committed with complete architecture documentation.

---

## PRIORITY ROADMAP

### Phase 1 (Required for True Agnosticism):
1. **LEXER REFACTORING** - CRITICAL
   - Extract pure tokenization to framework
   - Move indentation to language modules
   - This unblocks mini-Python, mini-C, etc.

2. **PARSER REVIEW** - IMPORTANT
   - Check for block-parsing assumptions
   - Ensure language modules control syntax

### Phase 2 (Polish):
1. Move parser's block handling to language modules
2. Document framework assumptions
3. Test with mock language

### Phase 3 (Future Languages):
1. Build mini-Python (indentation-based like Lumen)
2. Build mini-C (brace-based, no indentation)
3. Verify framework works with both

---

## Files Affected by Refactoring

```
REFACTOR:
  src/framework/lexer.rs          (Remove indentation logic)
  src/framework/parser.rs         (Review block parsing)
  src_lumen/structure/structural.rs (Add indentation processing)

POTENTIALLY REFACTOR:
  src/framework/registry.rs       (Remove INDENT/DEDENT registration?)

NO CHANGE NEEDED:
  src/framework/ast.rs
  src/framework/eval.rs
  src/framework/runtime/
  src_lumen/expressions/*
  src_lumen/statements/*
```

---

## Current Build Status
✅ All tests pass
✅ Examples run correctly
⚠️ Framework is not truly language-agnostic yet

## Next Steps
1. Decide: Proceed with lexer refactoring? (CRITICAL for true agnosticism)
2. If yes: Extract pure tokenization to framework/lexer.rs
3. Add language-specific lexer to src_lumen/
4. Update parser to work with language-specific tokens
