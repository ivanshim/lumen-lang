# Microcode Kernel Architecture

## Overview

The microcode kernel is a pure algorithmic execution engine that makes NO semantic assumptions about what it's executing. All language-specific behavior is table-driven via **declarative schemas**.

## Core Principle

**INVARIANT: The kernel owns ALL algorithms. All language-specific behavior is table-driven via declarative schemas.**

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
- **Process**: Execute via primitive dispatch (7-primitive canonical set)
- **Output**: Value (result) and ControlFlow
- **Key Principle**: All semantics come from primitive definitions, never from code

## Canonical Primitive Set

The kernel executes only these 7 canonical primitives (single-word verbs):

1. **Sequence** - Execute multiple instructions in order, return last value
2. **Scope** - Push/pop variable binding scope, execute sequence in new scope
3. **Branch** - Conditional execution (if condition then-block else else-block)
4. **Assign** - Variable assignment (update or create in current scope)
5. **Invoke** - Call external/foreign function via schema registry
6. **Operate** - Unary or binary operator dispatch (precedence and associativity from schema)
7. **Transfer** - Control flow signals with tagged variants:
   - `Transfer(Return, value)` - Return from function with optional value
   - `Transfer(Break, None)` - Break from loop
   - `Transfer(Continue, None)` - Continue to next iteration

## Desugaring

Source-level constructs are desugared to primitives during parsing:

- `print(expr)` → `Invoke("print_native", [expr])`
- `return expr` → `Transfer(Return, Some(expr))`
- `break` → `Transfer(Break, None)`
- `continue` → `Transfer(Continue, None)`
- `{ statements }` → `Scope([statements])`

## Internal Non-Canonical Primitive

- **Loop** - While looping construct (internal implementation detail, not part of canonical set)
  - Used by `while` and `loop` statements during parsing
  - Handles repeated condition evaluation with break/continue support
  - Planned for future refactoring to Branch + Transfer patterns

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
# languages/python/language.yaml
name: python
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
- Stream kernel: Fully tested with Lumen, Python, Rust
- Microcode kernel: Lumen support (basic)

**Current Results**: 21 passed, 5 failed (rust parsing issues), 33 skipped

## Future Work

1. **Complete Microcode Lumen**: Add all language features
2. **Python/Rust in Microcode**: Implement schemas and parsing
3. **YAML Schema Loading**: Runtime language loading from files
4. **Extern System**: Full dispatch implementation with capability registry
5. **Performance**: Optimize tokenization and instruction execution
6. **Error Messages**: Better diagnostics with source locations
