# Lumen Roadmap

This document captures the long-term direction of the Lumen language.
It is intentionally conceptual rather than technical.

Lumen is not designed to compete with production languages.
It exists to explore how little language is required for meaningful computation,
and how semantics emerge from small, explicit rules.

Once frozen, completed stages are never reinterpreted retroactively.

---

## Status

- **v0.1 — COMPLETE**
  - Minimal, executable language
  - Variables, arithmetic, booleans
  - if / else, while
  - break / continue
  - indentation-based blocks
  - single builtin: `print()`
  - no functions
  - no expression statements
  - no implicit behavior

This version is stable and locked.

---

## Roadmap Overview

Lumen grows in *layers*, not features.
Each version introduces exactly one new semantic axis.

---

## v0.x — Foundations (Completed)

### v0.1 — Imperative Core ✅
The smallest language that:
- parses cleanly
- executes deterministically
- expresses loops and conditionals
- fails honestly

This version answers:
> “What is the minimum executable language that still feels real?”

---

## v0.2 — Functions & Scope

**New concepts**
- `fn` definitions
- parameter binding
- `return`
- lexical scope
- call stack

**Constraints**
- no closures initially
- no recursion limits lifted
- no default arguments
- no overloading

**Goal**
Introduce abstraction *without* magic.

This version answers:
> “How does meaning get reused safely?”

---

## v0.3 — Data Structures

**New concepts**
- lists
- indexing
- iteration patterns
- value vs reference clarity

**Non-goals**
- no classes
- no inheritance
- no mutation-by-aliasing

This version answers:
> “How does structure emerge from repetition?”

---

## v0.4 — Modules & Files

**New concepts**
- `import`
- namespaces
- file boundaries
- visibility rules

**Goal**
Make Lumen programs larger without becoming opaque.

This version answers:
> “How does scale happen without loss of clarity?”

---

## v0.5 — Errors as Values

**New concepts**
- explicit error values
- propagation
- controlled failure

**Non-goals**
- no exceptions
- no hidden stack unwinding

This version answers:
> “How does a language admit fallibility honestly?”

---

## v0.6 — Tooling & Introspection

**New concepts**
- AST inspection
- runtime tracing
- deterministic debugging hooks

**Goal**
Make the language explain itself.

This version answers:
> “Can a language be understood from the inside?”

---

## v0.7 — Self-Description

At this stage:
- the grammar is fully specified
- the AST is stable
- the interpreter is conceptually complete

The language can now **describe its own grammar and semantics**.

This is *not* self-hosting yet.

This version answers:
> “Can the language explain what it is?”

---

## v0.8 — Partial Self-Hosting

At this stage:
- parts of the interpreter (parser, evaluator, or tools)
  can be reimplemented *in Lumen itself*
- Rust remains the execution substrate

This is a **meta-circular** phase.

This version answers:
> “Can the language reason about itself?”

---

## v0.9 — Bootstrap Threshold

This is the critical inflection point.

At this stage:
- a Lumen interpreter exists written in Lumen
- it runs on top of the Rust interpreter
- both interpreters agree on semantics

This is sometimes called:
- **self-hosting**
- **bootstrapping**
- **the reflective threshold**

In GEB terms, this is a **strange loop**:
> the system contains a representation of itself
> that is rich enough to execute itself.

---

## v1.0 — Self-Hosting (Conceptual Completion)

Lumen is now:
- defined in terms of itself
- executable via itself
- fully specified without reference to Rust semantics

Rust becomes an *implementation detail*, not the definition.

This is not “singularity”.
It is **closure**.

The language is now *expressed by itself*.

---

## Notes on the GEB “Singularity” Moment

In *Gödel, Escher, Bach*, this moment is not mystical.
It occurs when:
- a system can encode statements about itself
- those statements are executable
- the execution preserves meaning

For programming languages, this moment is called:
- **bootstrapping**
- **meta-circular interpretation**
- **reflective closure**

It is not intelligence.
It is not consciousness.

It is **semantic self-reference without collapse**.

That is the real achievement.

---

## Final Principle

Lumen will never grow by adding power first.

It grows by:
1. making meaning explicit
2. freezing semantics
3. allowing structure to emerge
4. resisting cleverness

If a feature cannot be explained clearly,
it does not belong.

---

End of roadmap.
