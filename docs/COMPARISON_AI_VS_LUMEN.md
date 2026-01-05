# Comparison: ai.yaml vs lumen.yaml v2.0

## Overview

| Aspect | ai.yaml | lumen.yaml |
|--------|---------|-----------|
| **Purpose** | AI/ML-specific workflows | General-purpose language semantics |
| **Target Domain** | Machine learning, data science | Language design exploration |
| **Scope** | Production ML systems | Semantic clarity and composition |
| **Philosophy** | 10 AI-specific principles | 15 universal language design principles |
| **Type System** | Rich, ML-focused | Minimal, extensible |
| **Backward Compat** | No (greenfield v0.1) | Yes (v0.1 compatible) |

---

## Design Principles Comparison

### ai.yaml Principles (10)
1. **Semantic clarity** - Every operation's data flow must be visible in syntax
2. **No implicit broadcasting** - Type mismatches fail early with clear error messages (AI-specific!)
3. **Immutable by default** - Values are immutable unless explicitly marked mutable
4. **Explicit function types** - Function signatures show inputs, outputs, and transformations
5. **Tensor as first-class** - Tensors/arrays are native types with clear semantics (ML domain!)
6. **Pattern matching first** - Control flow uses pattern matching for ML workflows
7. **Fail fast, fail clearly** - Type errors caught at parse time, runtime errors descriptive
8. **Composable pipelines** - Functions compose naturally without wrapper overhead
9. **Transparent computation** - All operations are traceable and explainable (ML explainability!)
10. **Minimal syntax** - Every syntactic element serves semantic purpose

### lumen.yaml Principles (15 from LUMEN_LANGUAGE_DESIGN.md)
1. **Semantic clarity** - Every operation's meaning visible in syntax
2. **Minimalism with intent** - No decorative syntax (foundational!)
3. **Syntax hierarchy** - Clear categories (grouping, blocks, identifiers)
4. **Statements vs expressions** - Distinction visible in code
5. **Readability over uniformity** - Asymmetry acceptable if clearer
6. **Semantics first - AST is truth** - AST is single source of truth for meaning
7. **Small, honest semantics** - Do few things correctly rather than many approximately
8. **Explicit over clever** - No implicit conversions or hidden control flow
9. **Failure is a feature** - Fail early, clearly, without guessing user intent
10. **Evolution constraint** - New features only if they don't invalidate existing mental models
11. **No feature without pressure** - Features only added when real examples demand them (evidence-driven!)
12. **Boring is good** - Prefer well-understood techniques over novel ones
13. **Portability of thought** - Design decisions must survive re-implementation
14. **Growth discipline** - Language must remain inspectable by one person (!)
15. **Final test: Clarity, not brevity** - Changes only if they make programs clearer

**Key Difference**:
- **ai.yaml** focuses on ML correctness and safety
- **lumen.yaml** focuses on language design principles and disciplined growth

---

## Keywords Comparison

### ai.yaml Keywords (33 total)
```
Type definitions:       type, struct, enum
Control flow:           if, else, match, case (pattern matching!)
Functions:              fn, return
Loops:                  for, while, break, continue
Binding:                let, mut
Error handling:         try, catch, throw (exceptions!)
Values:                 true, false, null, none
Visibility:             pub, private (module system!)
Operators:              as, in
```

### lumen.yaml Keywords (20 total, 6 reserved)
```
Binding:                let
Control flow:           if, else, while, break, continue, return
Functions:              fn
Type annotations:       type
Values:                 true, false, none
Logical operators:      and, or, not (as keywords!)
External interface:     extern
Mutability:             mut

RESERVED (not implemented):
                        match, case, for, struct, enum, trait
```

**Key Differences**:
- ai.yaml: 33 keywords, everything implemented
- lumen.yaml: 14 implemented, 6 reserved for future
- ai.yaml: Pattern matching as primary feature (`match/case`)
- lumen.yaml: Reserved but deferred (evidence-driven growth)
- ai.yaml: Exception handling (`try/catch/throw`)
- lumen.yaml: Uses `extern` bridge (simpler, defers to kernel)
- ai.yaml: Visibility modifiers (`pub/private`)
- lumen.yaml: No module system yet (future feature)
- lumen.yaml: Logical operators as keywords (`and, or, not`)
- ai.yaml: Single-char operators (`&&, ||, !`)

---

## Type System Comparison

### ai.yaml Type System (Rich, ML-Focused)

**Primitive Types:**
```
number      - With distinctions (i32, f32, f64, i64)
string      - UTF-8 text
boolean     - true/false
none        - Unit type
```

**Composite Types:**
```
Tensor<T, Shape>        - First-class! Shape validation, explicit broadcasting
Array<T, N>             - Fixed-size homogeneous
Tuple<T1, T2, ...>      - Heterogeneous
Option<T>               - Some(T) or None
Result<T, E>            - Ok(T) or Err(E)
Function types          - With composition support
```

**Operators:**
```
Arithmetic:     +, -, *, /, %, **
Comparison:     ==, !=, <, >, <=, >=
Logical:        &&, ||, !
Data ops:       @ (matrix multiplication!)
Composition:    |> (pipeline)
```

### lumen.yaml Type System (Minimal, Extensible)

**Primitive Types:**
```
number      - Unified 64-bit float (no distinctions)
string      - UTF-8 text
boolean     - true/false
none        - Unit type
```

**Composite Types (Deferred):**
```
tuple       - Shown in examples, not fully specified
option      - Planned for future versions
result      - Planned for future versions
struct, enum - Reserved keywords, not implemented
```

**Operators:**
```
Arithmetic:     +, -, *, /, %, **
Comparison:     ==, !=, <, >, <=, >=
Logical:        and, or, not (keywords, not symbols!)
Composition:    |> (pipeline)
Special:        -> (type annotation), := (walrus, reserved)
```

**Key Differences:**
- ai.yaml: Tensor as first-class type (ML domain-specific)
- lumen.yaml: No tensor support (general-purpose language)
- ai.yaml: Multiple numeric types for precision control
- lumen.yaml: Single unified `number` type (simpler)
- ai.yaml: Option/Result types built-in and mandatory
- lumen.yaml: Reserved for future (growth discipline)
- ai.yaml: Matrix multiplication operator `@`
- lumen.yaml: No multi-dimensional operators
- ai.yaml: Operators as symbols (`&&`, `||`)
- lumen.yaml: Operators as keywords (`and`, `or`)

---

## Control Flow Comparison

### ai.yaml Control Flow

```
// PATTERN MATCHING (PRIMARY!)
match result {
    Classification(probs) => apply_softmax(probs),
    Regression(value) => postprocess(value),
    Error(msg) => handle_error(msg),
}
// Compiler ensures exhaustiveness

// TRY-CATCH EXCEPTION HANDLING
try {
    data = load_file("model.pth");
} catch(error) {
    log("Failed to load: " + error.message);
}

// RESULT TYPE (ERROR HANDLING)
loss_result = compute_loss(logits, labels);
loss = match loss_result {
    Ok(l) => l,
    Err(e) => { print("Loss error: " + e); return 0.0; }
};

// FOR LOOPS
for item in collection {
    process(item);
}
```

### lumen.yaml Control Flow

```
// IF/ELSE (PRIMARY, NO PATTERN MATCHING)
if condition
    statement1
else
    statement2

// WHILE LOOPS ONLY
while condition
    statement1

// RETURN STATEMENT FOR EARLY EXIT
fn safe_divide(a, b)
    if b == 0
        return none
    a / b

// NO EXCEPTIONS (try/catch reserved)
// NO FOR LOOPS (reserved as 'for')
// NO PATTERN MATCHING (reserved as 'match'/'case')
```

**Key Differences:**
- ai.yaml: Pattern matching as PRIMARY control flow (data-driven!)
- lumen.yaml: if/while as primary (simple, familiar)
- ai.yaml: Full exception handling (try/catch/throw)
- lumen.yaml: Uses extern bridge for errors (simpler, delegates)
- ai.yaml: Exhaustive matching enforced by compiler
- lumen.yaml: if/else sufficient for current needs
- ai.yaml: For loops over iterables
- lumen.yaml: While loops with manual increment (growth discipline)

---

## Scope & Mutability Comparison

### ai.yaml Scope (Not Discussed)
- Focused on operations, not scoping rules
- Assumes functional programming model
- Implicit immutability (no discussion of `mut`)
- All examples use immutable values
- No explicit documentation of variable scoping

### lumen.yaml Scope (Explicitly Documented)

```
// IMMUTABILITY BY DEFAULT
let x = 10        // Immutable
x = 20            // ERROR: cannot reassign immutable binding

// EXPLICIT MUTABILITY
let mut y = 10    // Mutable
y = 20            // OK: reassigns y

// FLAT SCOPE (NO BLOCK-LOCAL ISOLATION)
x = 10
if true
    x = 20        // Updates outer x (not local copy!)
print(x)          // Prints 20

// SHADOWING WITH NEW LET
let x = 10
if true
    let x = 20    // New binding shadows outer x
print(x)          // Prints 10 (outer x unchanged)
```

**Key Differences:**
- ai.yaml: Assumes immutability, doesn't specify scoping rules
- lumen.yaml: Explicitly documents flat scope semantics (from v0.1)
- ai.yaml: No discussion of mutable variables
- lumen.yaml: Detailed mutability markers (`let` vs `let mut`)
- ai.yaml: Implicit functional style
- lumen.yaml: Explicit mutability for clarity

---

## Data Transformation Comparison

### ai.yaml: Explicit Broadcasting (ML-Focused)

```
// THIS FAILS AT COMPILE TIME
result = tensor[N, M] + vector[M]
// ERROR: Shape mismatch! Cannot add [N,M] to [M]

// CORRECT APPROACH WITH EXPLICIT BROADCAST
result = tensor[N, M] + vector
    .reshape([1, M])
    .broadcast([N, M])

// No silent dimension errors (ML bug prevention!)
```

### lumen.yaml: No Broadcasting Concept

```
// ONLY SCALARS AND STRINGS
x = 10 + 5                      // Arithmetic addition
s = "hello" + "world"           // String concatenation

// NO MULTI-DIMENSIONAL DATA STRUCTURES
// (Tensors deferred as future feature)
```

**Key Difference:**
- ai.yaml: Prevents common ML bugs (broadcasting dimension errors)
- lumen.yaml: Doesn't address multi-dimensional data (different domain)

---

## Error Handling Comparison

### ai.yaml: Rich Error Handling

```
// RESULT TYPE (TYPE-DRIVEN ERROR HANDLING)
result: Result<Model, Error> = try_load_model("weights.pth");
model = match result {
    Ok(m) => m,
    Err(e) => {
        log("Error: " + e);
        get_pretrained()  // Fallback
    },
}

// EXCEPTIONS (ALTERNATIVE APPROACH)
try {
    data = load_file("model.pth");
} catch(error) {
    log("Failed to load: " + error.message);
}

// EXHAUSTIVE ERROR HANDLING (compiler enforces)
```

### lumen.yaml: Simple Kernel Bridge

```
// EXTERN INTERFACE FOR ERRORS
extern("print_native", value)      // Output a value
extern("error", "error message")   // Signal error

// NO BUILT-IN EXCEPTION HANDLING
// (try/catch reserved for future)

// SIMPLE ERROR SIGNALING
if result == none
    extern("error", "computation failed")
```

**Key Differences:**
- ai.yaml: Built-in Result/Option types + exceptions
- lumen.yaml: Delegates errors to kernel (simpler, more flexible)
- ai.yaml: Type-driven error handling (forces handling)
- lumen.yaml: Simpler, relies on extern interface
- ai.yaml: Exhaustive error checking
- lumen.yaml: Programmer responsibility

---

## Immutability Philosophy Comparison

### ai.yaml: Functional Default (Implicit)
```
design_principle: "Immutable by default"
- No explicit discussion of 'mut' keyword
- Assumes functional programming style
- Values transformed via pipelines
- All examples use immutable values
- Immutability is implicit in the design
```

### lumen.yaml: Explicit Markers (Visible)
```
let x = 10              // Immutable (default)
let mut y = 10          // Mutable (explicit)
y = 20                  // Only works if 'mut' declared

benefit: "Intent is visible in code"
- Programmers see mutability decisions
- Can't accidentally reassign immutable bindings
- Prevents bugs from unexpected mutations
```

**Key Difference:**
- ai.yaml: Immutability implicit (assumed from context)
- lumen.yaml: Immutability explicit (visible in syntax)
- This aligns with lumen principle: "Explicit over clever"

---

## Standard Library / Built-Ins Comparison

### ai.yaml: Comprehensive ML Library

```
Tensor Operations:
  reshape, transpose, slice, broadcast, concatenate, stack

Math Functions:
  sin, cos, exp, log, sqrt, max, min, mean, sum, std

Linear Algebra:
  matmul, inv, solve, eigh, svd

Random:
  normal, uniform, shuffle, choice

I/O:
  load, save (for tensors, models, data)

Functional:
  map, filter, reduce, compose, pipe

ML-Specific:
  Loss Functions: mse, crossentropy, mae, huber
  Optimizers: sgd, adam, rmsprop, momentum
  Metrics: accuracy, precision, recall, f1
```

### lumen.yaml: Minimal Kernel Bridge

```
Extern Capabilities:
  extern("print_native", value)        // Output
  extern("value_type", value)          // Type introspection
  extern("debug_info", value)          // Debugging
  extern("error", message)             // Error signaling

No Standard Library Defined:
  Language defers to kernel/implementation
  Minimalism principle (only what's needed)
  Growth discipline (evidence-driven additions)
```

**Key Difference:**
- ai.yaml: Rich ecosystem for ML workflows
- lumen.yaml: Minimal, delegates to kernel
- ai.yaml: Production-ready library
- lumen.yaml: Proof-of-concept system

---

## Backward Compatibility

### ai.yaml
- **No backward compatibility mentioned**
- Version 0.1 (greenfield design)
- Can change freely without constraints
- Designed from scratch for ML

### lumen.yaml
- **Full backward compatibility with v0.1**
- All 24 example programs still work unchanged
- New features are opt-in additions
- Explicitly documented compatibility in schema
- Evolution of existing language

**Key Difference:**
- ai.yaml: Greenfield (no prior constraints)
- lumen.yaml: Evolution (must preserve v0.1)

---

## Growth & Evolution Philosophy

### ai.yaml: Focused on Correctness & Safety
```
Key Driver: "AI systems must be trustworthy and verifiable"

Features Added Because:
- ML correctness requires them
- Safety guarantees demand them
- Explainability needs them
- No feature size limit

Philosophy:
- Static type checking where possible
- No implicit operations
- Explicit broadcasting only
- Exhaustive pattern matching
- Tight safety guarantees
```

### lumen.yaml: Focused on Minimalism & Clarity
```
Key Drivers:
- "No feature without pressure" (evidence-driven)
- "Boring is good" (prefer proven techniques)
- "Growth discipline" (keep inspectable by one person!)
- "Evolution constraint" (don't invalidate mental models)
- "Portability of thought" (survive re-implementation)

Features Added Only When:
- Real examples demonstrate need
- Existing constructs become awkward
- They preserve existing semantics
- They can be implemented simply

Deliberate Deferrals:
- Pattern matching (no real use case yet)
- Custom types (can use functions)
- For loops (while sufficient)
- Module system (not needed yet)
```

**Key Difference:**
- ai.yaml: "Add everything needed for ML correctness"
- lumen.yaml: "Add only what's necessary, reserve rest"

---

## Summary Table: Feature Completeness

| Feature | ai.yaml | lumen.yaml | Note |
|---------|---------|-----------|------|
| Variables | ✓ | ✓ | Both support binding |
| Functions | ✓ | ✓ | Core feature |
| Immutability | ✓ (implicit) | ✓ (explicit) | Different visibility |
| Pattern Matching | ✓ Built-in | ⚠ Reserved | ai.yaml primary |
| If/Else | ✓ | ✓ | Both support |
| Control Flow | match/if/while/for | if/while | ai.yaml richer |
| Error Handling | Result + try/catch | extern bridge | ai.yaml more complete |
| Type System | Rich (tensors, options) | Minimal | ai.yaml domain-specific |
| Type Annotations | ✓ | ✓ (optional) | Both support |
| Scope Rules | Not specified | Flat scope (documented) | lumen.yaml explicit |
| Broadcasting | Explicit only | N/A | ai.yaml ML-specific |
| Operators | Many (incl. @, &&, \|\|) | Basic (and, or as keywords) | ai.yaml more ops |
| Module System | ✓ (pub/private) | ✗ (reserved) | ai.yaml has visibility |
| Backward Compat | N/A | ✓ Full (v0.1) | lumen.yaml constraint |
| Standard Library | Comprehensive (ML) | Minimal (kernel bridge) | ai.yaml production |
| Type Safety | Static/compile-time | Dynamic + optional hints | ai.yaml stronger |
| Keywords | 33 (all implemented) | 14 (+ 6 reserved) | lumen.yaml disciplined |

---

## The Fundamental Difference

### ai.yaml: "What would an ideal ML language look like?"

**Philosophy**: Build everything needed for trustworthy, verifiable AI systems
- Rich types (Tensor, Option, Result)
- Pattern matching for data processing
- Static type checking where possible
- Explicit operations only (no broadcasting surprises)
- Exhaustive error handling
- Complete ML standard library

**Result**: Comprehensive, domain-specific, feature-rich language

### lumen.yaml: "How can we evolve Lumen while staying true to its principles?"

**Philosophy**: Add capabilities while preserving minimalism and clarity
- Keep v0.1 compatible
- Add functions (core abstraction)
- Add type hints (optional clarity)
- Add pipes (composability)
- Defer non-essential features
- Reserve keywords for future with evidence-driven addition

**Result**: Lighter, more focused, preserves Lumen's core identity

---

## Visual Summary

```
                ai.yaml                    lumen.yaml
                  |                           |
        "Ideal ML Language"          "Evolved Lumen v2.0"
                  |                           |
        Complete Feature Set          Minimal + Reserved
        Greenfield Design             Backward Compatible
        Domain-Specific               General-Purpose
        Production-Ready              Proof-of-Concept
        Rich Type System              Minimal Types
        Pattern Matching              If/While
        Result Types                  Extern Bridge
        Tensors First-Class           No Multi-Dim
        ⬇️                              ⬇️
    (Best for ML workflows)      (Best for language design)
```

---

## Learning From Both

**ai.yaml teaches us:**
- How to design for a specific domain (ML correctness)
- The value of explicit operations (no surprises)
- Pattern matching for data processing
- Type-driven error handling

**lumen.yaml teaches us:**
- How to evolve carefully (backward compatibility)
- Growth discipline (no feature creep)
- Minimalism with intent
- The value of explicit syntax over implicit behavior

**Together, they show:**
- Different design goals lead to different languages
- Both approaches are valid for their domains
- Trade-offs between completeness and minimalism
- The importance of clear design philosophy
