# Lumen Language Modules - Comprehensive Guide

Complete reference for all 7 language implementations in the Lumen interpreter framework.

---

## Overview

The Lumen project implements a kernel-based multi-language interpreter with 7 different language variants:
- **Core Lumen** - Python-like syntax with indentation-based blocks
- **Mini-Rust** - Rust-like syntax with `let` bindings
- **Mini-PHP** - PHP-like syntax with `$` variables
- **Mini-Shell** - Shell script syntax
- **Mini-C** - C-like syntax
- **Mini-Pascal** - Pascal syntax with `BEGIN`/`END`
- **Mini-BASIC** - Classic BASIC syntax

Each language runs on the same kernel architecture with language-specific modules for syntax and semantics.

---

## Architecture Overview

### Three-Layer Design

1. **Kernel Layer** (`src/kernel/`)
   - Language-agnostic lexer, parser, AST, and runtime
   - Pure ASCII tokenization via maximal-munch segmentation
   - Generic trait-based expression and statement handling
   - Byte span tracking for authoritative source locations

2. **Language Modules** (`src_*/`)
   - Language-specific token definitions and registration
   - Unique syntax implementations for variables, operators, and statements
   - Language-specific block and structural parsing
   - ~19-20 files per language module

3. **Dispatcher** (`src/main.rs`)
   - Detects language via `--lang` flag or file extension
   - Routes to appropriate language module
   - Manages multi-language execution

### Core Execution Pipeline

```
Source Code
    ↓
[Lexer] → Tokens { lexeme, span }
    ↓
[Language-Specific Processing] → Structured Tokens
    ↓
[Generic Parser] → AST (via trait dispatch)
    ↓
[Evaluator] → Results
```

### Key Architectural Principles

- **Span is Authoritative**: Byte offsets (start, end) are the primary source location mechanism
- **line/col is Diagnostic-Only**: Line and column info used only for error messages
- **Trait-Based Dispatch**: ExprPrefix, ExprInfix, StmtHandler traits enable extensibility
- **Pure ASCII Segmentation**: No semantic knowledge in the lexer
- **Language-Agnostic Kernel**: All semantic interpretation in language modules
- **Maximal-Munch Lexing**: Multi-character operators handled correctly

---

## Language Module Structure

Each language implements this file organization:

```
src_<language>/
├── mod.rs                          # Module root
├── src_<language>.rs               # Dispatcher (registers all features)
├── structure/
│   ├── mod.rs                      # Exports
│   └── structural.rs               # Token/block definitions & parsing
├── expressions/
│   ├── mod.rs                      # Exports
│   ├── literals.rs                 # Numbers & booleans
│   ├── arithmetic.rs               # +, -, *, /, %
│   ├── comparison.rs               # ==, !=, <, >, <=, >=
│   ├── logic.rs                    # Logical operators
│   ├── variable.rs                 # Variable access (LANGUAGE-SPECIFIC)
│   ├── identifier.rs               # Identifier handling
│   └── grouping.rs                 # Parentheses
└── statements/
    ├── mod.rs                      # Exports
    ├── assignment.rs               # Assignment (LANGUAGE-SPECIFIC)
    ├── print.rs                    # Output (LANGUAGE-SPECIFIC)
    ├── if_else.rs                  # Conditionals
    ├── while_loop.rs               # Loops
    ├── break_stmt.rs               # Break
    ├── continue_stmt.rs            # Continue
    └── let_binding.rs              # Variable binding (if applicable)
```

**Language-Specific Files**: `variable.rs`, `assignment.rs`, `print.rs`, and `let_binding.rs` (if used)

**Shared Files**: `literals.rs`, `arithmetic.rs`, `comparison.rs`, `logic.rs`, `grouping.rs`, `if_else.rs`, `while_loop.rs`, `break_stmt.rs`, `continue_stmt.rs`

---

## Language Reference

### 1. Lumen (Core Language)

**File Extension**: `.lm`
**Style**: Python-like with indentation-based blocks

**Features**:
- ✅ Indentation-based block structure (4-space indents)
- ✅ INDENT/DEDENT structural tokens
- ✅ Simple assignment: `x = 10`
- ✅ Newline-sensitive parsing
- ✅ All core operators and control flow

**Example**:
```lumen
x = 0
while x < 10:
    if x == 5:
        break
    x = x + 1
print(x)
```

**Key Files**: `src_lumen/structure/structural.rs` (indentation processing)

---

### 2. Mini-Rust

**File Extension**: `.rs`
**Style**: Rust-like with curly braces

**Features**:
- ✅ `let` keyword for variable binding: `let x = 10;`
- ✅ Direct assignments: `x = x + 1;`
- ✅ `print!()` macro-style function
- ✅ Curly brace blocks: `{ ... }`
- ✅ Semicolon-terminated statements
- ✅ `&&`, `||`, `!` logical operators
- ✅ Proper operator precedence

**Example**:
```rust
let x = 0;
let y = 10;
while x < y {
    if x == 5 {
        x = x + 1;
        continue;
    }
    print(x);
    x = x + 1;
}
```

**Key Differences from Rust**:
- No type system
- Single-line execution model
- Limited to basic control flow
- All variables are untyped

**Notable Fixes**:
- Fixed `&&`, `||`, `!` operator token validation
- Fixed `let` statement parser
- Fixed direct assignment parser

---

### 3. Mini-PHP

**File Extension**: `.php`
**Style**: PHP-like with `$` variables

**Features**:
- ✅ Dollar-sign variables: `$x = 10;`
- ✅ `$` required in both assignment and access
- ✅ `echo()` statement: `echo($x);`
- ✅ Curly brace blocks
- ✅ Semicolon-required statements
- ✅ PHP-style variable handling

**Example**:
```php
$x = 0;
$y = 10;
while ($x < $y) {
    if ($x == 5) {
        $x = $x + 1;
        continue;
    }
    echo($x);
    $x = $x + 1;
}
```

**Key Feature**: Dollar sign required for both assignment and access

---

### 4. Mini-Shell

**File Extension**: `.sh`
**Style**: Shell script with variable expansion

**Features**:
- ✅ No `$` in assignments: `x=10`
- ✅ `$` only for variable expansion: `$x`
- ✅ `echo()` statement (like bash)
- ✅ Curly brace blocks
- ✅ Semicolons optional in some contexts
- ✅ Shell-style variable handling
- ✅ `and`/`or`/`not` logical operators

**Example**:
```shell
x = 0;
y = 10;
while ($x < $y) {
    if ($x == 5) {
        x = $x + 1;
        continue;
    }
    echo($x);
    x = $x + 1;
}
```

**Key Feature**: Dollar sign only for variable expansion, not assignment

---

### 5. Mini-C

**File Extension**: `.c`
**Style**: C-like without type system

**Features**:
- ✅ No special variable prefixes: `x = 10;`
- ✅ `printf()` statement
- ✅ Curly brace blocks
- ✅ Semicolon-required statements
- ✅ C-style control flow

**Example**:
```c
x = 0;
y = 10;
while (x < y) {
    if (x == 5) {
        x = x + 1;
        continue;
    }
    printf(x);
    x = x + 1;
}
```

**Key Feature**: Clean variable syntax, no prefix or suffix markers

---

### 6. Mini-Pascal

**File Extension**: `.p`
**Style**: Pascal-like with `BEGIN`/`END`

**Features**:
- ✅ `:=` assignment operator
- ✅ `BEGIN`/`END` blocks (instead of `{}`)
- ✅ `writeln()` output statement
- ✅ Pascal-style syntax
- ✅ Semicolons optional
- ✅ Indentation-aware formatting

**Example**:
```pascal
x := 0;
y := 10;
while (x < y) BEGIN
    if (x == 5) BEGIN
        x := x + 1;
        continue;
    END
    writeln(x);
    x := x + 1;
END
```

**Key Features**: `:=` operator and `BEGIN`/`END` blocks

---

### 7. Mini-BASIC

**File Extension**: `.basic`
**Style**: Classic BASIC with uppercase keywords

**Features**:
- ✅ `LET` keyword for assignment: `LET x = 10`
- ✅ `PRINT()` statement (uppercase)
- ✅ Curly brace blocks
- ✅ No semicolon requirement
- ✅ Traditional BASIC feel
- ✅ Uppercase keywords

**Example**:
```basic
LET x = 0
LET y = 10
WHILE (x < y) {
    IF (x == 5) {
        LET x = x + 1
        CONTINUE
    }
    PRINT(x)
    LET x = x + 1
}
```

**Key Feature**: `LET` keyword and uppercase `PRINT` and `WHILE`

---

## Feature Comparison Matrix

### Variable Declaration & Access

| Language | Assignment | Access | Example |
|----------|-----------|--------|---------|
| Lumen | `x = 5` | `x` | `y = x + 1` |
| Mini-Rust | `let x = 5;` or `x = 5;` | `x` | `y = x + 1;` |
| Mini-PHP | `$x = 5;` | `$x` | `$y = $x + 1;` |
| Mini-Shell | `x = 5;` | `$x` | `y = $x + 1;` |
| Mini-C | `x = 5;` | `x` | `y = x + 1;` |
| Mini-Pascal | `x := 5;` | `x` | `y := x + 1;` |
| Mini-BASIC | `LET x = 5` | `x` | `LET y = x + 1` |

### Output Statement

| Language | Keyword | Syntax |
|----------|---------|--------|
| Lumen | `print` | `print(value)` |
| Mini-Rust | `print` | `print(value);` |
| Mini-PHP | `echo` | `echo($value);` |
| Mini-Shell | `echo` | `echo($value);` |
| Mini-C | `printf` | `printf(value);` |
| Mini-Pascal | `writeln` | `writeln(value);` |
| Mini-BASIC | `PRINT` | `PRINT(value)` |

### Block Delimiters

| Language | Start | End |
|----------|-------|-----|
| Lumen | Indentation | Dedentation |
| Mini-Rust | `{` | `}` |
| Mini-PHP | `{` | `}` |
| Mini-Shell | `{` | `}` |
| Mini-C | `{` | `}` |
| Mini-Pascal | `BEGIN` | `END` |
| Mini-BASIC | `{` | `}` |

### Operators

| Category | Supported | Syntax |
|----------|-----------|--------|
| Arithmetic | `+`, `-`, `*`, `/`, `%` | All languages |
| Comparison | `==`, `!=`, `<`, `>`, `<=`, `>=` | All languages |
| Logical | See below | Language-specific |

**Logical Operators**:
- Lumen, Mini-Rust, Mini-PHP, Mini-Shell: `and`/`or`/`not` or `&&`/`||`/`!`
- Mini-C: `&&`, `||`, `!`
- Mini-Pascal: `and`/`or`/`not`
- Mini-BASIC: `and`/`or`/`not`

---

## All Supported Features

### Expressions (All Languages)
- ✅ Number literals (integers and floats): `42`, `3.14`
- ✅ Boolean literals: `true`, `false`
- ✅ Arithmetic: `+`, `-`, `*`, `/`, `%`
- ✅ Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
- ✅ Logical: `and`/`or`/`not` (language-specific syntax)
- ✅ Variables: language-specific syntax
- ✅ Grouping: `(expr)`
- ✅ Proper operator precedence

### Statements (All Languages)
- ✅ Variable assignment (language-specific syntax)
- ✅ Output statements (language-specific keyword)
- ✅ If/else conditionals with proper block handling
- ✅ While loops
- ✅ Break statement
- ✅ Continue statement
- ✅ Block scoping and nesting

### Runtime
- ✅ Variable storage and lookup
- ✅ Expression evaluation
- ✅ Control flow (if/else, while, break, continue)
- ✅ Proper operator semantics

---

## Implementation Statistics

| Metric | Value |
|--------|-------|
| Languages | 7 |
| Total Source Files | 156+ |
| Lines of Code | ~10,000+ |
| Shared Expressions | 6 (arithmetic, comparison, logic, grouping, literals, identifiers) |
| Shared Statements | 4 (if/else, while, break, continue) |
| Language-Specific | 3+ per language (variable, assignment, output) |

---

## Usage

### Running a Program

```bash
# Auto-detect language by file extension
cargo run src_mini_rust/examples/demo.rs

# Explicit language selection
cargo run -- --lang mini-php src_mini_php/examples/loop.php

# All supported languages
cargo run -- --lang lumen src_lumen/examples/pi.lm
cargo run -- --lang mini-rust src_mini_rust/examples/demo.rs
cargo run -- --lang mini-php src_mini_php/examples/fibonacci.php
cargo run -- --lang mini-shell src_mini_sh/examples/loop.sh
cargo run -- --lang mini-c src_mini_c/examples/pi.c
cargo run -- --lang mini-pascal src_mini_apple_pascal/examples/e.p
cargo run -- --lang mini-basic src_mini_apple_basic/examples/fibonacci.basic
```

### Example Programs

Each language has 5 example programs:
1. `demo.<ext>` - Comprehensive feature demonstration
2. `loop.<ext>` - Simple loop example
3. `fibonacci.<ext>` - Fibonacci sequence
4. `pi.<ext>` - Pi computation using series
5. `e.<ext>` - Euler's number computation

Location: `src_<language>/examples/`

---

## Testing

Run all 35 tests (5 examples × 7 languages):

```bash
./test_all.sh
```

Current status: ✅ **All 35 tests passing**

---

## Key Design Decisions

### 1. Trait-Based Expression Parsing

Expression handlers implement two traits:
- `ExprPrefix`: Handles prefix/primary expressions (literals, variables, grouping)
- `ExprInfix`: Handles binary operators with precedence

This allows languages to define their own operators without modifying the parser.

### 2. Statement Handler Dispatch

Each statement type implements `StmtHandler`:
```rust
pub trait StmtHandler {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>>;
}
```

Languages register their statement handlers, and the parser tries each in order.

### 3. Span as Primary Source Location

- **Authoritative**: All parsing, AST construction, and evaluation use byte spans
- **Diagnostic**: Line/column computed from source for error messages only
- **Benefit**: Precise source attribution, future IDE integration

### 4. Pure ASCII Lexer

The kernel lexer:
- Takes raw ASCII/UTF-8 source
- Performs maximal-munch segmentation
- Emits tokens with byte spans
- **No semantic knowledge** - all interpretation in language modules

### 5. Modular Language Definition

Each language:
- Registers tokens via `set_multichar_lexemes()`
- Implements unique variable/assignment/output handling
- Reuses shared arithmetic, comparison, logic, control flow
- Defines block structure (braces vs `BEGIN`/`END` vs indentation)

---

## Future Extensions

### Potential Language Additions
- Mini-Python (true indentation, list/dict support)
- Mini-Go (goroutines, channels)
- Mini-JavaScript (dynamic objects, closures)
- Mini-TypeScript (type annotations)

### Feature Extensions per Language
- Strings and concatenation
- Arrays and indexing
- Functions and procedures
- Type systems
- More sophisticated control flow

---

## Technical Details

### Kernel Components Used

1. **Lexer** (`src/kernel/lexer.rs`):
   - Byte-by-byte tokenization
   - Span tracking (start, end byte offsets)
   - No semantic classification

2. **Parser** (`src/kernel/parser.rs`):
   - Generic token stream consumer
   - Pratt parsing for expressions
   - Trait dispatch for handlers

3. **Registry** (`src/kernel/registry.rs`):
   - Feature registration system
   - Operator precedence management
   - Error formatting with diagnostic position

4. **AST** (`src/kernel/ast.rs`):
   - Generic expression and statement nodes
   - Trait-based evaluation and execution
   - Control flow primitives (Break, Continue)

5. **Runtime** (`src/kernel/runtime/`):
   - Value representation
   - Variable environment/scoping
   - Expression evaluation and statement execution

---

## Notes on Recent Fixes

### Mini-Rust Updates
- Fixed token validation bug in `let_binding.rs`
- Fixed token validation bug in `assignment.rs`
- Corrected logical operators from `and`/`or` to `&&`/`||`/`!`

### Mini-Shell Updates
- Corrected all examples to use `echo` instead of `print`
- Ensured proper shell-style variable expansion syntax

### Span Refactoring
- Made Span the authoritative source-location mechanism
- Demoted line/col to diagnostic-only (error messages)
- Added `current_span()` method to parser
- Updated `err_at()` documentation

---

## Resources

- **Main Entry Point**: `src/main.rs`
- **Test Suite**: `test_all.sh`
- **Examples**: `src_*/examples/`
- **Documentation**: This file

---

**Last Updated**: 2026-01-01
**Status**: ✅ Production Ready - All 35 tests passing
**Project**: Lumen Language Interpreter Framework
