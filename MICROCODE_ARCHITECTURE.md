# Microcode Kernel Architecture

## Overview

The microcode kernel is a pure algorithmic execution engine that makes NO semantic assumptions about what it's executing. All language-specific behavior is table-driven via **declarative schemas**.

## Core Principle

**INVARIANT: The kernel owns ALL algorithms. All language-specific behavior is table-driven via declarative schemas.**

## Directory Structure

```
src_microcode/
├── kernel/                     # Pure algorithmic engine (no logic crosses these files)
│   ├── ingest.rs              # Stage 1: Lex source using schema tables
│   ├── structure.rs           # Stage 2: Process structural tokens (indentation, newlines)
│   ├── reduce.rs              # Stage 3: Convert token stream to instruction tree
│   ├── execute.rs             # Stage 4: Execute instruction tree via primitive dispatch
│   ├── env.rs                 # Execution environment with lexical scoping
│   ├── eval.rs                # Value evaluation and primitive utilities
│   ├── primitives.rs          # Closed primitive set (8 primitives only)
│   └── mod.rs                 # Kernel entry point (4-stage pipeline)
│
├── schema/                     # Declarative schema system (DATA ONLY)
│   ├── language.rs            # LanguageSchema and ExternSyntax types
│   ├── structure.rs           # StatementPattern and StatementRole types
│   ├── operator.rs            # OperatorInfo and Associativity types
│   ├── validation.rs          # Schema validation logic
│   └── mod.rs                 # Exports and re-exports
│
├── languages/                  # Language definitions (DATA ONLY - NO LOGIC)
│   ├── lumen/
│   │   ├── language.rs        # Lumen schema (pure data)
│   │   └── values.rs          # Lumen value types
│   │
│   ├── mini_python/
│   │   ├── language.yaml      # Mini-Python schema (YAML format - extensible)
│   │   └── values.rs          # Mini-Python value types
│   │
│   └── alien_lang/
│       ├── language.yaml      # Example extensible language
│       └── mod.rs             # Loader for YAML-based languages
│
└── runtime/                    # External function dispatch
    └── extern/
        └── mod.rs             # Execute extern calls per schema definition
```

## Four-Stage Pipeline

### Stage 1: Ingest
- **File**: `kernel/ingest.rs`
- **Input**: Source code string
- **Process**: Lex using schema tables (multi-character lexemes, no interpretation)
- **Output**: Token stream
- **Key Principle**: No whitespace rules, just maximal-munch lexing

### Stage 2: Structure
- **File**: `kernel/structure.rs`
- **Input**: Token stream
- **Process**: Process structural tokens per schema (indentation, newlines, EOF)
- **Output**: Structured token stream
- **Key Principle**: Schema defines indentation rules (fixed/none), newline behavior

### Stage 3: Reduce
- **File**: `kernel/reduce.rs`
- **Input**: Structured token stream
- **Process**: Convert to instruction tree using schema patterns and operator precedence
- **Output**: Instruction tree (Instructions with Primitives)
- **Key Principle**: Pratt parsing driven by schema operator tables

### Stage 4: Execute
- **File**: `kernel/execute.rs`
- **Input**: Instruction tree, Environment
- **Process**: Execute via primitive dispatch (8-primitive closed set)
- **Output**: Value (result) and ControlFlow
- **Key Principle**: All semantics come from primitive definitions, never from code

## Closed Primitive Set

The kernel can ONLY execute these 8 primitives (no others):

1. **Sequence** - Execute instructions in order, return last value
2. **Block** - Push/pop scope, execute sequence
3. **Conditional** - if-then-else branching
4. **Loop** - while looping with break/continue support
5. **Jump** - break/continue signals
6. **Assign** - Set variable in current scope
7. **Call** - Call external/foreign function (via schema registry)
8. **UnaryOp/BinaryOp** - Operator dispatch (precedence from schema)

## Schema Responsibilities

Each language schema MUST define (and can ONLY define):

### Lexical Rules
- Multi-character lexemes (sorted by length descending)
- Split characters (whitespace, punctuation)
- NO implicit whitespace rules

### Structural Rules
- EOF handling
- Indentation mode (fixed indent / none)
- Newline behavior (terminator / ignored)

### Token Roles
- Map lexemes → symbolic roles (keyword, identifier, operator, literal, etc.)

### Operators
- Precedence levels
- Associativity (left/right/none)
- Primitive mapping

### Statements
- Expected substructures (patterns)
- Mapped execution primitive

### Extern Calls
- Selector string syntax
- Arity (if checked)
- Primitive dispatch

**INVARIANT: Schemas contain NO executable logic. Only data.**

## Language Definitions

Languages can be defined in two ways:

### Option 1: Rust Data (Compiled)
```rust
// languages/lumen/language.rs
pub fn get_schema() -> LanguageSchema {
    let mut statements = HashMap::new();
    statements.insert("print".to_string(), PatternBuilder::new("print")...);
    // ... all data
    LanguageSchema { statements, binary_ops, ... }
}
```

### Option 2: YAML Data (Runtime Loadable)
```yaml
# languages/mini_python/language.yaml
name: mini_python
keywords: [if, while, for, def, class]
statements:
  print:
    pattern: [keyword, "(", expression, ")"]
operators:
  binary:
    "+": {precedence: 5, associativity: left}
```

## Separation of Concerns

### Kernel Does NOT Know:
- What any language looks like
- What operators mean
- What statements do
- How values behave
- What extern calls are

### Kernel DOES Know:
- How to tokenize using tables
- How to handle indentation per schema
- How to parse using operator precedence
- How to execute primitives
- How to dispatch to extern via schema

### Language Does NOT Have:
- Any parsing logic
- Any execution logic
- Any control flow
- Any value operations

### Language DOES Have:
- Lexical tables
- Structural rules
- Pattern definitions
- Operator definitions
- Statement mappings

## No Cross-Dependencies

**CRITICAL**: There are NO imports between `src_stream` and `src_microcode`.

- `src_stream` is the original procedural kernel (unmodified)
- `src_microcode` is the data-driven kernel (completely independent)
- Both use `src/schema/` for shared type definitions only
- The top-level `src/main.rs` routes between them

This separation ensures:
- Each kernel can evolve independently
- No accidental coupling
- Clear architectural boundaries
- Easy to add new kernels

## Testing

The `test_all` script tests both kernels with all examples:
- Stream kernel: Fully tested with Lumen, Mini-Python, Mini-Rust
- Microcode kernel: Lumen support (basic)

**Current Results**: 21 passed, 5 failed (mini_rust parsing issues), 33 skipped

## Future Work

1. **Complete Microcode Lumen**: Add all language features
2. **Mini-Python/Mini-Rust in Microcode**: Implement schemas and parsing
3. **YAML Schema Loading**: Runtime language loading from files
4. **Extern System**: Full dispatch implementation with capability registry
5. **Performance**: Optimize tokenization and instruction execution
6. **Error Messages**: Better diagnostics with source locations
