# Lumen Language – Roadmap

This roadmap describes the intentional evolution of **Lumen** from a proof-of-concept language into a stable, composable, AI-reasoning-friendly system.

The roadmap is ordered by **semantic dependency**, not convenience.  
Later phases assume the invariants of earlier phases are stable.

---

## Phase 0 — Foundations (Completed / In Progress)

**Goal:** Establish a minimal, coherent execution model.

- Core syntax and grammar
- AST construction
- Interpreter / evaluator
- Basic value types (numbers, strings, booleans)
- Control flow primitives (conditionals, loops)
- Minimal I/O (e.g. print)
- Host implementation in Rust

At this stage, **velocity is prioritised over permanence**.  
No APIs are considered stable.

[Established]

---

## Phase 1 — Semantic Contracts (Critical)

**Goal:** Define what Lumen *means*, independent of syntax or libraries.

This phase freezes the **behavioral invariants** upon which all libraries depend.

### 1.1 Value Semantics
- Value vs reference rules
- Mutability model
- Copy vs move behavior
- Equality and ordering semantics

### 1.2 Error and Failure Semantics
- Error as value vs control-flow
- Optionality (e.g. `Option`-like semantics)
- Recoverable vs unrecoverable failure
- Deterministic error propagation

### 1.3 Iteration and Control Abstractions
- Iteration model (eager vs lazy)
- Canonical looping and traversal semantics
- Termination guarantees

### 1.4 Side-Effect Discipline
- What constitutes a side effect
- How I/O, state mutation, and time are expressed
- Explicit vs implicit effects

Deliverables:
- Formal documentation of contracts
- Executable tests enforcing invariants
- Reference programs exercising edge cases

No standard library stabilization occurs before this phase completes.

[Established]

---

## Phase 2 — Core Library (Rust-Core-Inspired)

**Goal:** Provide a minimal, rigorous core library aligned with frozen semantics.

This layer mirrors the *philosophy* of Rust’s `core`, not its surface API.

### 2.1 Core Data Types
- Numeric utilities
- Strings and string operations
- Core collections
- Ranges and iteration helpers

### 2.2 Core Abstractions
- Optionality and result handling
- Comparison and ordering helpers
- Functional combinators (map, filter, fold)

Constraints:
- No OS assumptions
- No allocation strategy exposure (unless explicit)
- No concurrency or I/O dependencies

The core library is:
- Always present
- Versioned with the language
- Semantically stable once released

[Indicative]

---

## Phase 3 — Standard Library (Environment-Aware)

**Goal:** Introduce practical utilities that interact with the external world.

This layer is **explicitly effectful** and environment-dependent.

### 3.1 I/O and Environment
- File system abstractions
- Paths and directories
- Basic environment queries

### 3.2 Serialization and Parsing
- JSON
- Text-based formats
- Deterministic parsing rules

### 3.3 Time and Randomness
- Clocks and timers
- Random number generation
- Explicit nondeterminism markers

Stability policy:
- APIs stabilize slowly
- Breaking changes are expected early
- Experimental modules are clearly labeled

[Indicative]

---

## Phase 4 — Interoperability and FFI

**Goal:** Treat the external ecosystem as a first-class resource.

Rather than re-implementing large ecosystems, Lumen prioritizes **bridges**.

### 4.1 Rust Interop
- Calling Rust functions from Lumen
- Safe value and error translation
- Ownership and lifetime boundaries

### 4.2 Foreign Libraries
- C ABI compatibility (where applicable)
- Host-language bindings
- Sandboxing and safety boundaries

This phase enables access to:
- High-performance native libraries
- Existing numerical, crypto, and systems code

[Hypothetical]

---

## Phase 5 — Tooling and Ecosystem

**Goal:** Support long-term growth without semantic erosion.

### 5.1 Tooling
- Formatter
- Linter
- Static analysis hooks
- Test frameworks

### 5.2 Documentation and Contracts
- Behavior-driven documentation
- Stability guarantees
- Deprecation policies

### 5.3 Package and Module System
- Dependency resolution
- Versioning semantics
- Reproducible builds

[Hypothetical]

---

## Guiding Principles

- **Semantics before syntax**
- **Contracts before convenience**
- **Libraries are consequences, not prerequisites**
- **Interoperability beats duplication**
- **Stability is earned, not assumed**

Lumen is designed to remain:
- Reasonable for humans
- Legible to AI systems
- Resistant to semantic drift

---

## Non-Goals (Explicit)

- Blind compatibility with Python or other languages
- Premature standard library completeness
- Implicit side effects
- Semantics defined by implementation accident

---

End of roadmap.
