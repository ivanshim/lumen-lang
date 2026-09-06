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

Token registration uses the `TokenDefinition` API:

```rust
let tokens = vec![
    TokenDefinition::recognize("=="),   // lexeme to segment on
    TokenDefinition::keyword("if"),     // lexeme that must stand alone as a word
];
registry.tokens.set_token_definitions(tokens);
registry.tokens.set_word_chars(|c| c.is_alphanumeric() || c == '_');
```

The kernel caches the multi-character lexemes in descending length order for
maximal-munch segmentation. Which characters form a "word", for the keyword
boundary check, is supplied by the language; the kernel has no opinion. Each
stream language declares one flag, `IDENTIFIER_UNICODE`, and derives its
`word_start` and `word_char` predicates from it; every identifier check in
that language goes through those two functions, and the same flag exists as
`identifier.unicode` in the JSON definitions so both kernels read a name
the same way.
Comments are a language concept too. The lexer emits the comment marker like
any other byte, and each language's structure pass drops the tokens from the
marker to the end of the line, the same pass that handles indentation or
braces. Comment removal is a token-stream transformation, never a text
pre-pass.

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
* handle whitespace, comments or skip behavior
* provide generic handler trait types
* cache, memoize or otherwise apply runtime policy

Any such behavior belongs exclusively to the language implementation.

---

## Achieving Kernel Purity

The kernel achieves complete separation of concerns through strategic delegation:

### Language-Specific Artifacts (NOT in Kernel)

All of the following are defined and managed by language modules, not the kernel:

* **Handler traits**: `ExprPrefix`, `ExprInfix`, `StmtHandler` — each language defines its own
* **Spelling**: every keyword, operator, bracket, literal word and builtin name comes from `configs/lumen.json`, read once by `languages/lumen/definition.rs`; handlers ask the definition whether a token spells the construct they parse
* **Precedence**: a numeric tier per operator lexeme, read from the definition's `op.precedence` table, with associativity from `op.right_associative`
* **Skip behavior**: Extension traits like `LumenParserExt::skip_tokens()` for whitespace handling
* **Comment removal**: a token-stream transformation inside each language's structure pass
* **Runtime policy**: Lumen's memoization lives in `languages/lumen/memo.rs`, using the environment's typed extension slot

### Kernel-Only Responsibilities

The kernel provides only:

* **TokenRegistry**: Register tokens via `TokenDefinition` API (language-agnostic)
* **Lexer**: Pure maximal-munch tokenization
* **Parser**: Generic token stream navigation and dispatch
* **AST**: Abstract syntax tree node traits (language-neutral)
* **Evaluator**: Generic evaluation engine
* **Runtime**: Value storage and a scoped environment with `define`, `assign`, `update`, `get`, `get_mut`, `with_scope`, and a typed extension slot for language-owned state

### Result: Zero Language Knowledge in Kernel

A hypothetical audit of kernel source code would find:

* Zero references to "lumen" or "languages"
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
