# EBNF Grammar Specifications

This directory contains EBNF (Extended Backus-Naur Form) grammar specifications for the languages supported by the Lumen project, derived mechanically from YAML language specifications in the parent `yaml/` directory.

## Overview

The EBNF grammars are **lossless, syntactic representations** of the corresponding YAML specifications. They are designed for:

- **Documentation** of language syntax in a standard, portable format
- **Parser generation** using standard EBNF-to-parser tools
- **Language analysis** and understanding of grammar structure
- **Tooling integration** with language servers and IDEs

## Files

### Core Languages (Implemented in Lumen)

#### `lumen.ebnf`
Grammar for **Lumen** - a minimalist, indentation-based language emphasizing semantic clarity.

**Characteristics:**
- Indentation-based block structure (4-space indentation)
- 7 precedence levels for operators
- Pipe operator (`|>`) for function composition
- Range expressions (`start..end`)
- For loops with range support (`for i in 0..10`)
- Type annotations (optional)
- Kernel-mapped primitives (execute, assign, branch, loop, return)

**Key Features:**
- Colon (`:`) introduces indented blocks
- Semicolon or newline terminates statements
- Half-open ranges: `[start, end)`
- Short-circuit logical operators
- Pattern-based function parameters (planned)

### Mini Language Variants

#### `python.ebnf`
Grammar for **Python** - a minimal PythonCore subset for language comparison studies.

**Characteristics:**
- Indentation-based (4-space)
- Subset of PythonCore 3.x syntax
- Focus on core constructs: variables, functions, loops, conditionals
- Python-style for-in loops
- List support

**Simplifications from Python:**
- No decorators, comprehensions, or async
- Simplified import system
- No class definitions
- Built-in `print()` function

#### `rust.ebnf`
Grammar for **Rust** - a minimal RustCore subset for language comparison studies.

**Characteristics:**
- Brace-based block structure
- Type annotations required
- Ownership and borrowing basics
- Mutable bindings with `mut`
- Generic types
- Pattern matching in match expressions
- Trait basics

**Simplifications from Rust:**
- No lifetimes or advanced generics
- Simplified module system
- No macros or attributes
- No async/await
- Minimal unsafe support

### Full Language Specifications

#### `python.ebnf`
Complete grammar for **PythonCore 3.14** - full official PythonCore syntax.

**Characteristics:**
- Indentation-based blocks
- 15+ precedence levels
- Comprehensive expression types (generators, comprehensions, lambdas)
- Both simple and compound statements
- Rich type hint system
- Pattern matching (PythonCore 3.10+)
- Async/await support
- Decorator system
- Context managers (with/as)
- Exception handling (try/except/finally)

**Advanced Features:**
- Generator expressions and comprehensions (list/dict/set)
- Walrus operator (`:=`)
- Match statements with pattern matching
- Type annotations with Union, Optional, Callable
- F-strings
- Multiple inheritance and metaclasses (in AST)

#### `rust.ebnf`
Complete grammar for **RustCore 1.75+** - full official RustCore syntax.

**Characteristics:**
- Brace-based blocks
- Mandatory type annotations
- 14 precedence levels
- Rich pattern system
- Trait and generic system
- Comprehensive type constructors
- Module and visibility system
- Attribute system
- Macro support

**Advanced Features:**
- Ownership and borrowing rules (in type expressions)
- Lifetime parameters
- Associated types
- Unsafe blocks
- Procedural macros
- Match expressions with guards
- Async/await with async fn
- Declarative macros (macro_rules!)
- Test attributes

## EBNF Notation

### Basic Elements

- **Nonterminals**: lowercase with underscores (e.g., `expression`, `statement`)
- **Terminals**: UPPERCASE or in quotes (e.g., `"if"`, `"+"`, `NEWLINE`)
- **Concatenation**: spaces between elements
- **Alternation**: `|` for alternatives
- **Optional**: `?` (zero or one)
- **Repetition**: `*` (zero or more), `+` (one or more)
- **Grouping**: parentheses `( )`

### Example

```ebnf
(* Function definition *)
function_definition = "fn" identifier "(" parameter_list? ")"
                    return_type? block ;

(* Parameter list *)
parameter_list = identifier ("," identifier)* ;

(* Block expression *)
block = "{" statement* expression? "}" ;
```

## Structure Across Grammars

All EBNF files follow a consistent structure:

1. **Lexical Tokens** - Keywords, operators, delimiters, literals, identifiers
2. **Structural Rules** - Program structure, blocks, statement organization
3. **Statements** - Variable binding, control flow, function definitions
4. **Operators** - Organized by precedence with comments
5. **Expressions** - Operators organized from lowest to highest precedence
6. **Type Expressions** - Type constructors and annotations (where applicable)
7. **Special Rules** - Language-specific constructs
8. **Notes** - Comments on special semantics and features

## Operator Precedence

Each grammar documents operator precedence by structuring expression rules from lowest to highest precedence:

```ebnf
(* Lowest precedence *)
expression = assignment_expression ;
assignment_expression = logical_or_expression ("=" ...)? ;
logical_or_expression = logical_and_expression ("or" ...)* ;
logical_and_expression = comparison_expression ("and" ...)* ;
(* ... more levels ... *)
primary_expression = literal | identifier | ... ;  (* Highest precedence *)
```

This reflects the standard EBNF pattern where higher-precedence operators appear deeper in the expression hierarchy.

## Block Structures

### Indentation-Based (Lumen, Python, Python)

```ebnf
block = ":" INDENT statement+ DEDENT ;
```

- Colon (`:`) at end of statement introduces block
- Next line must be indented
- `INDENT` token marks start, `DEDENT` marks end
- Indentation amount varies: 4 spaces (Lumen, Python), 2 spaces (some variants)

### Brace-Based (Rust, Rust)

```ebnf
block = "{" statement* "}" ;
```

- Block enclosed in `{ }` delimiters
- No indentation requirement
- Statements explicitly terminated with `;`

## Type System Representation

### Lumen
```ebnf
type_expression = primitive_type | composite_type | function_type ;
primitive_type = "number" | "string" | "boolean" | "none" ;
composite_type = tuple_type | option_type | result_type ;
function_type = "fn" "(" ... ")" "->" type_expression ;
```

### RustCore (comprehensive)
```ebnf
type_expression = type_path | reference_type | pointer_type
                | slice_type | array_type | tuple_type ;
reference_type = "&" lifetime? mutability? type_expression ;
pointer_type = "*const" type_expression | "*mut" type_expression ;
generic_params = "<" generic_param ("," generic_param)* ">" ;
```

### PythonCore (3.10+ with unions)
```ebnf
type_expression = type_union | type_callable | type_literal ;
type_union = type_intersection ("|" type_intersection)* ;
type_callable = "Callable" "[" type_list "->" type_expression "]" ;
```

## Key Differences Between Languages

| Aspect | Lumen | PythonCore | RustCore | PythonCore | RustCore |
|--------|-------|-------------|-----------|--------|------|
| **Block Syntax** | Indentation | Indentation | Braces | Indentation | Braces |
| **Type Annotations** | Optional | Implicit | Required | Optional | Required |
| **Operators** | 7 precedence | 7 precedence | 14 precedence | 15+ precedence | 14 precedence |
| **Pattern Matching** | Planned | None | Basic | Advanced (3.10+) | Advanced |
| **Generics** | Not in v2.2 | None | Basic | PythonCore 3.10+ | Full |
| **Async Support** | Planned | None | None | Yes | Yes |
| **Memory Safety** | Language level | Type-enforced | Borrow checker | Reference counting | Ownership |

## Derivation from YAML

These EBNF grammars are mechanically derived from the YAML specifications in `yaml/`:

- **YAML Section → EBNF Rules**: YAML `expressions`, `statements`, `operators` sections map to EBNF rules
- **Pattern Notation → EBNF**: YAML patterns like `[keyword, expr, ":", block]` become EBNF rules
- **Precedence Tables → Cascading Rules**: YAML operator precedence becomes expression hierarchy
- **Type Definitions → Type Expressions**: YAML type sections become EBNF type rules

### Example: YAML to EBNF

**YAML (`lumen.yaml`):**
```yaml
expressions:
  literals:
    number: { description: "...", syntax: "123 or 3.14" }

operators:
  arithmetic:
    "+":
      precedence: 5
      associativity: left
```

**EBNF (`lumen.ebnf`):**
```ebnf
literal = number_literal | string_literal | boolean_literal | none_literal ;
number_literal = float_literal | integer_literal ;

(* Precedence 5 - appears in additive_expression *)
additive_expression = multiplicative_expression
                    (("+" | "-") multiplicative_expression)* ;
```

## Usage Guidelines

### For Parser Generation
1. Use EBNF files with tools like ANTLR, yacc, or Parboiled
2. Adapt precedence-climbing patterns as needed
3. Add semantic actions in target language
4. Implement type checking based on type_expression rules

### For Documentation
1. Render EBNF using standard markup/diagram tools
2. Cross-reference YAML and EBNF for comprehensive language docs
3. Use for API and language specification sites
4. Include in language tutorials

### For IDE/Language Server Support
1. Parse EBNF to generate language grammar metadata
2. Use for syntax highlighting rule generation
3. Feed into code completion/suggestion systems
4. Support real-time error detection

### For Analysis
1. Extract operator precedence tables
2. Identify grammar conflicts (if any)
3. Measure language complexity (rule count, nesting depth)
4. Compare language designs

## Semantic Considerations

**Important**: These EBNF grammars are **purely syntactic**. They do not include:

- **Type checking rules** (though type expressions are shown)
- **Scope and binding rules** (variable resolution)
- **Ownership/lifetime checking** (Rust's borrow checker)
- **Macro expansion** (preprocessing)
- **Semantic predicates** (context-dependent parsing)

For complete language specifications, refer to:
- **Lumen**: `docs/LUMEN_LANGUAGE_DESIGN.md`, `yaml/lumen.yaml`
- **Python**: [PythonCore Language Reference](https://docs.python.org/3/reference/)
- **Rust**: [The RustCore Reference](https://doc.rust-lang.org/reference/)

## Files and Line Counts

| Language | File | Lines | Complexity |
|----------|------|-------|-----------|
| Lumen | lumen.ebnf | ~380 | Medium |
| PythonCore | python.ebnf | ~280 | Low |
| RustCore | rust.ebnf | ~450 | Medium-High |
| PythonCore | python.ebnf | ~420 | Very High |
| RustCore | rust.ebnf | ~550 | Very High |

## Special Constructs

### Comments in EBNF
All files use EBNF-style comments:
```ebnf
(* Single-line comment *)

(* Multi-line comment
   with multiple lines *)

(* Precedence 5 - appears in additive_expression *)
```

### Sections with Notes
Most grammars include a final "NOTES ON SPECIAL CONSTRUCTS" section documenting:
- Block structure semantics
- Operator behavior (short-circuit, right-associative)
- Type system specifics
- Memory/ownership model (Rust)
- Special syntax (F-strings in Python, macros in Rust)

## Extensibility

These EBNF files serve as the foundation for:

1. **Future YAML→EBNF tools**: Automatic regeneration when YAML specs change
2. **Cross-language comparison studies**: Structural analysis of language design
3. **Grammar validator tools**: Checking YAML against EBNF
4. **Educational resources**: Teaching formal language specification

To extend:
- Update corresponding YAML file in `yaml/` directory
- Regenerate EBNF from updated YAML
- Verify EBNF syntactic validity
- Update this README with new language information

## Standards and References

- **EBNF Standard**: ISO/IEC 14977 (with practical extensions)
- **Extended Notation**: `*` (Kleene star), `+` (plus), `?` (optional), `|` (alternation)
- **Parentheses**: For grouping complex expressions
- **Semantic Actions**: Not included in EBNF (language-implementation specific)

## License and Attribution

These EBNF grammars are derived from:
- **Lumen**: Project specification (`yaml/lumen.yaml`)
- **Python**: Official PythonCore language reference and `yaml/python.yaml`
- **Rust**: Official RustCore language reference and `yaml/rust.yaml`
- **Mini variants**: Project educational subsets

See individual YAML files for detailed attribution and version information.

---

**Last Updated**: January 7, 2026
**Format Version**: EBNF 1.0
**Generated from**: YAML specifications in `yaml/` directory
