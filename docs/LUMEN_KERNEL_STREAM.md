# Lumen Kernel

## Overview

The Lumen Kernel is **not a programming language**.

It is a **meta-language runtime**: a minimal execution substrate that allows *languages* to be defined, parsed, and executed without the kernel itself knowing their syntax, structure, or semantics.

If a source file is treated as nothing more than a string of ASCII characters, the kernel exists solely to provide the machinery by which *meaning* may be attached to that string by a language implementation.

The kernel has no opinions about what a language should look like.

---

## Design Philosophy

The kernel follows three core principles:

1. **Structural neutrality**
   The kernel does not understand blocks, indentation, braces, newlines, or statements.

2. **Semantic ignorance**
   The kernel does not understand numbers, arithmetic, truth, or control flow meaning.

3. **Delegated meaning**
   All syntax and semantics are supplied by the language layer.

The kernel exists to *host* languages, not to *be* one.

---

## What the Kernel Provides

Starting from raw source text, the kernel provides a small set of meta-language primitives.

### 1. Tokenization Infrastructure

The kernel converts a stream of characters into a stream of tokens.

* Tokens are labeled, ordered, and position-aware.
* The kernel does not assign meaning to tokens.
* Token definitions (keywords, operators, symbols) are registered by the language via `TokenRegistry`.

Token registration uses the **unified `TokenDefinition` API**:

```rust
let tokens = vec![
    TokenDefinition::recognize("=="),      // Token to be lexed
    TokenDefinition::recognize("if"),      // Token to be lexed
    TokenDefinition::skip(" "),            // Token to skip during parsing
];
registry.tokens.set_token_definitions(tokens);
```

The kernel internally extracts and caches:
* Multichar lexemes for the lexer (for maximal-munch segmentation)
* Skip tokens for the parser (for whitespace/comment handling)

The kernel guarantees **stability and order**, not interpretation.

---

### 2. Token Stream Transport

The kernel treats the token stream as data.

* It preserves ordering.
* It preserves source location.
* It does not insert, remove, or reinterpret tokens.

Languages are free to transform the token stream in any way they choose, including:

* inserting structural markers
* collapsing spans
* performing multiple passes
* ignoring structure entirely

The kernel does not participate in these decisions.

---

### 3. Syntactic Assembly Protocol

The kernel provides a generic mechanism for assembling abstract syntax trees (ASTs).

* No grammar is hardcoded.
* No syntax rules are embedded.
* Parsing behavior is delegated to language-registered handlers.

The kernel does not recognize syntax.
It provides a place where syntax can be recognized.

---

### 4. Semantic Scaffolding

The kernel defines:

* what an AST node is
* how nodes are evaluated
* how execution proceeds step by step

It does **not** define:

* what any node means
* how expressions are computed
* how comparisons are interpreted

All semantic meaning is supplied by the language.

---

### 5. Execution Substrate

The kernel provides:

* an execution loop
* an environment for storing values
* propagation of control signals (e.g. normal flow, early exit)

The kernel enforces *process*, not *policy*.

It ensures that execution happens in a well-defined order, but it does not decide what execution *means*.

---

## What the Kernel Explicitly Does *Not* Provide

The kernel intentionally does **not**:

* understand arithmetic or numeric representations
* define truthiness or comparison semantics
* recognize blocks, indentation, or grouping
* privilege any syntactic style
* impose a grammar
* impose a type system
* define operator precedence
* define token patterns or pattern matching
* handle whitespace or skip behavior
* provide generic handler trait types

Any such behavior belongs exclusively to the language implementation.

---

## Achieving Kernel Purity

The kernel achieves complete separation of concerns through strategic delegation:

### Language-Specific Artifacts (NOT in Kernel)

All of the following are defined and managed by language modules, not the kernel:

* **Handler traits**: `ExprPrefix`, `ExprInfix`, `StmtHandler` — each language defines its own
* **Precedence types**: `Precedence` enums with language-specific levels
* **Pattern definitions**: `PatternSet` structures for language syntax patterns
* **Skip behavior**: Extension traits like `LumenParserExt::skip_tokens()` for whitespace handling

### Kernel-Only Responsibilities

The kernel provides only:

* **TokenRegistry**: Register tokens via `TokenDefinition` API (language-agnostic)
* **Lexer**: Pure maximal-munch tokenization
* **Parser**: Generic token stream navigation and dispatch
* **AST**: Abstract syntax tree node traits (language-neutral)
* **Evaluator**: Generic evaluation engine
* **Runtime**: Value storage and execution environment

### Result: Zero Language Knowledge in Kernel

A hypothetical audit of kernel source code would find:

* Zero references to "lumen", "rust", "python", or "languages"
* Zero hardcoded keywords, operators, or syntax rules
* Zero trait definitions for handlers or precedence
* Zero pattern matching or structural assumptions
* Zero semantic interpretations

The kernel contains **pure infrastructure**, nothing else.

---

## The Kernel as a Meta-Language

The kernel can be understood as a **language about languages**.

It defines:

* how symbols flow
* how structure may be constructed
* how meaning may be executed

But it does not define:

* which symbols matter
* which structures exist
* which meanings are valid

In this sense, the kernel is intentionally *empty*.

That emptiness is the feature.

---

## Consequences of This Design

Because of this design:

* Multiple languages can coexist on the same kernel.
* Languages with radically different syntax can share execution infrastructure.
* Numeric models, control semantics, and structure rules can vary freely.
* The kernel remains stable as languages evolve.

The kernel’s job is finished once it has provided a place for meaning to live.

---

## Summary

The Lumen Kernel is a minimal, language-agnostic execution core.

It transforms character streams into token streams, hosts syntax assembly, and drives execution — while remaining deliberately ignorant of what any of it *means*.

It is not a framework.
It is not a language.

It is the ground on which languages stand.
