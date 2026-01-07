# EBNF Grammar Quick Reference

## What's Here?

This directory contains **EBNF (Extended Backus-Naur Form) grammars** for 5 programming languages, derived mechanically from YAML specifications.

```
grammar/
├── lumen.ebnf              ← Start here for Lumen language
├── mini-python.ebnf        ← Minimal Python variant
├── mini-rust.ebnf          ← Minimal Rust variant
├── python.ebnf             ← Full Python 3.14 specification
├── rust.ebnf               ← Full Rust 1.75+ specification
├── README.md               ← Comprehensive documentation
├── GENERATION_REPORT.md    ← Detailed analysis
└── QUICK_REFERENCE.md      ← This file
```

## Quick Start

### For Reading EBNF

1. **Open any `.ebnf` file**
2. **Each section explains part of the syntax**:
   - Keywords, operators, literals
   - Statements and control flow
   - Expressions and operators (ordered by precedence)
   - Type annotations

### Example: Reading `lumen.ebnf`

```ebnf
(* Line 1-12: Header explaining file purpose *)

(* Line 14-77: Lexical Tokens *)
keyword = "let" | "mut" | "if" | ... ;

(* Line 80-88: Structural Rules *)
program = statement* EOF ;
block = COLON INDENT statement+ DEDENT ;

(* Line 90-125: Statements *)
variable_binding = "let" mutability? identifier ... ;
if_statement = "if" expression block ... ;

(* Line 127-179: Operators (with precedence comments) *)

(* Line 181-237: Expressions (lowest to highest precedence) *)
expression = pipe_expression ;
pipe_expression = assignment_expression ... ;

(* Line 239-264: Type Annotations *)

(* Line 266+: Special Rules and Notes *)
```

## Understanding EBNF Syntax

### Symbols

| Symbol | Meaning | Example |
|--------|---------|---------|
| `=` | Rule definition | `statement = ...` |
| `\|` | Alternatives (OR) | `"if" \| "else"` |
| `?` | Optional (0 or 1) | `mutability?` |
| `*` | Repetition (0+) | `statement*` |
| `+` | Repetition (1+) | `statement+` |
| `( )` | Grouping | `("a" \| "b")*` |
| `;` | End of rule | `rule = ... ;` |

### Conventions

- **Nonterminals**: `lowercase_with_underscores`
- **Terminals**: `"quoted"` or `UPPERCASE`
- **Comments**: `(* comment *)`

## Three Key Patterns

### 1. Keywords
```ebnf
keyword = "let" | "mut" | "if" | "else" | ... ;
```

### 2. Operators (by precedence)
```ebnf
(* Lowest *)
expression = assignment_expression ;
assignment_expression = logical_or_expression ("=" ...)? ;
logical_or_expression = logical_and_expression ("or" ...)* ;
(* Highest *)
primary_expression = literal | identifier | ... ;
```

### 3. Block Structure

**Indentation-based** (Lumen, Mini-Python, Python):
```ebnf
block = ":" INDENT statement+ DEDENT ;
```

**Brace-based** (Mini-Rust, Rust):
```ebnf
block = "{" statement* "}" ;
```

## File Guide

### Grammar Files (EBNF)

| File | Language | Block Style | Lines | Complexity |
|------|----------|-------------|-------|-----------|
| `lumen.ebnf` | Lumen | Indentation | 259 | Medium |
| `mini-python.ebnf` | Minimal Python | Indentation | 199 | Low |
| `mini-rust.ebnf` | Minimal Rust | Braces | 326 | Medium |
| `python.ebnf` | Full Python | Indentation | 313 | High |
| `rust.ebnf` | Full Rust | Braces | 467 | Very High |

### Documentation Files

| File | Purpose | Audience |
|------|---------|----------|
| `README.md` | Complete guide to grammars | Everyone |
| `GENERATION_REPORT.md` | Detailed analysis & statistics | Technical |
| `QUICK_REFERENCE.md` | This file - getting started | New users |

## Common Tasks

### Task: Find a keyword

1. Open the language's `.ebnf` file
2. Search for `keyword =` section
3. Scan the list (A-Z)

**Example**: In `lumen.ebnf` line 20:
```ebnf
keyword = "let" | "mut" | "if" | "else" | "while" | "for" | "until" | "in"
         | "break" | "continue" | "return" | "fn" | "extern" | ... ;
```

### Task: Understand operator precedence

1. Open language file, search for `expression =`
2. Follow the chain downward (each step = lower precedence)
3. `primary_expression` is highest precedence

**Example**: In `lumen.ebnf`:
```
expression                    ← Assignment lowest
  → pipe_expression           ← Pipe operator (0.5)
    → assignment_expression   ← Assignment (1)
      → logical_or_expression ← OR (2)
        → logical_and_expr    ← AND (3)
          → comparison        ← Comparisons (4)
            → ...
              → primary       ← Literals, identifiers (highest)
```

### Task: Check what statements exist

1. Open language file
2. Find `statement =` rule
3. Lists all statement types available

**Example**: In `python.ebnf`:
```ebnf
statement = simple_statement | compound_statement ;
simple_statement = pass_statement | break_statement | ... ;
compound_statement = if_statement | while_loop | ... ;
```

### Task: Learn how to write a function

1. Search for `function_definition =`
2. Read the pattern

**Lumen example** (line 113):
```ebnf
function_definition = "fn" identifier "(" parameter_list? ")"
                     type_annotation? block ;
```

This means: `fn name(params) : type: block`

**Rust example**:
```ebnf
function_definition = "fn" identifier generic_params?
                     "(" parameter_list? ")" return_type? block ;
```

This means: `fn name<T>(params) -> RetType { block }`

## Precedence Quick Reference

### Lumen (7 levels)
0.5 - Pipe (`|>`)
1 - Assignment (`=`)
2 - OR (`or`)
3 - AND (`and`)
4 - Comparison (`==`, `<`, etc.)
5 - Additive (`+`, `-`)
6 - Multiplicative (`*`, `/`, `%`)
7 - Unary (`-`, `not`), Exponentiation (`**`)

### Python (15+ levels)
1 - Assignment (`=`)
2 - Lambda
3 - Conditional (`if`/`else`)
4 - OR
5 - AND
6 - NOT
7 - Comparisons
8 - Bitwise OR
9 - Bitwise XOR
10 - Bitwise AND
11 - Shifts
12 - Additive
13 - Multiplicative
14 - Exponentiation
15+ - Unary, Primary

### Rust (14 levels)
Similar to Python but with assignment, logical operators, bitwise ops

## Block Structure Comparison

### Lumen (Indentation)
```
if x > 5:
    print(x)
else:
    print("small")
```

**EBNF**: `block = ":" INDENT statement+ DEDENT ;`

### Rust (Braces)
```
if x > 5 {
    println!("{}", x);
} else {
    println!("small");
}
```

**EBNF**: `block = "{" statement* "}" ;`

## Type Annotations

### Lumen (Optional)
```ebnf
type_annotation = ":" type_expression ;
type_expression = primitive_type | composite_type | function_type ;
primitive_type = "number" | "string" | "boolean" | "none" ;
```

### Rust (Required)
```ebnf
let x: i32 = 5;
fn add(a: i32, b: i32) -> i32 { a + b }
```

### Python (Optional)
```ebnf
x: int = 5
def add(a: int, b: int) -> int: return a + b
```

## Key Differences

| Aspect | Lumen | Python | Rust |
|--------|-------|--------|------|
| **Block Syntax** | `:` then indent | `:` then indent | `{ }` |
| **Terminator** | `;` or newline | newline | `;` |
| **Types** | Optional | Optional | Required |
| **Operators** | 7 levels | 15+ levels | 14 levels |
| **Safety** | Language enforced | Runtime | Borrow checker |

## File Locations

```
lumen-lang/
├── yaml/                   ← Source YAML specifications
│   ├── lumen.yaml
│   ├── python.yaml
│   └── rust.yaml
└── grammar/                ← Generated EBNF grammars (you are here)
    ├── lumen.ebnf
    ├── python.ebnf
    ├── rust.ebnf
    ├── README.md           ← Full documentation
    └── QUICK_REFERENCE.md  ← This file
```

## Using EBNF Grammars

### 1. Parser Generation
```bash
# With ANTLR
antlr4 grammar/lumen.ebnf -Dlanguage=Python

# With other tools
yacc grammar/lumen.ebnf
```

### 2. Documentation
```bash
# Convert to diagrams
# Use online EBNF diagram generators
# Export to formal specifications
```

### 3. Language Analysis
```bash
# Compare languages
# Extract operator precedence
# Analyze grammar complexity
```

## Troubleshooting

**Q: How do I know if a syntax is allowed?**
A: Search the `.ebnf` file for the relevant rule. Follow the chain of definitions until you reach terminals (quoted/UPPERCASE).

**Q: Why does my parser fail on valid code?**
A: The grammar defines *syntax*, not semantics. Check:
- Your tokenizer produces correct tokens
- Whitespace is handled correctly (for indentation-based languages)
- Keywords are recognized as terminals, not identifiers

**Q: What about type checking?**
A: EBNF only covers syntax. Type checking is handled separately (see YAML specs and language references).

## Further Reading

1. **README.md** - Comprehensive grammar documentation
2. **GENERATION_REPORT.md** - Detailed analysis and statistics
3. **yaml/lumen.yaml** - Semantic details for Lumen
4. **[ISO/IEC 14977](https://en.wikipedia.org/wiki/Extended_Backus%E2%80%93Naur_form)** - EBNF standard

## Summary

✅ **These EBNF grammars are:**
- Complete syntactic specifications
- Mechanically derived from YAML
- Ready for parser generation
- Suitable for documentation
- Useful for language analysis

📖 **To learn more:**
- Start with README.md for full documentation
- Use GENERATION_REPORT.md for statistics
- Consult specific .ebnf files for language details
- Refer to yaml/ directory for semantic specifications

---

**Quick Tip**: Use your editor's Find function (Ctrl+F / Cmd+F) to search for keywords, operators, and rules in the EBNF files!
