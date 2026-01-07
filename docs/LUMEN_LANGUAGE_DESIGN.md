# Lumen – Design Philosophy

This document defines the non-negotiable design principles of **Lumen**.
It is intended to be portable across time, contributors, implementations, and AI collaborators.

This document outranks syntax sketches, implementation details, and convenience-driven changes.

---

## 1. Purpose

Lumen exists to explore **language semantics**.

It does not prioritize performance, novelty, feature breadth, or production readiness.

---

## 2. Minimalism with Intent

Every syntactic element must carry **semantic information that cannot be inferred otherwise**.

If a character, keyword, or rule adds no meaning, it does not belong.

---

## 3. Syntax Hierarchy

- Parentheses exist only for **expression grouping and function calls**
- Indentation introduces **control-flow blocks**
- Bare identifiers represent variables; **no sigils or prefixes**
- Colons exist only for **type annotations**

These categories must never overlap.

---

## 4. Statements vs Expressions

Control structures (`if`, `while`, future `for`) are **statements**, not expressions.

Expressions produce values.  
Statements alter control flow.

This distinction must remain visible in the syntax.

---

## 5. Readability over Uniformity

Consistency is valued only when it improves clarity.

Asymmetry is acceptable—and encouraged—when it makes intent more obvious at a glance.

---

## 6. Semantics First

The **AST is the single source of truth** for language meaning.

Parsing, evaluation, and any future compilation stages must align with the AST and never bypass it.

---

## 7. Small, Honest Semantics

The language should do **a small number of things correctly**, rather than many things approximately.

Undefined behavior is preferable to silently wrong behavior.

---

## 8. Explicit over Clever

Avoid:
- implicit conversions
- hidden control flow
- magic defaults

If something happens, it must be visible in the code.

---

## 9. Failure Is a Feature

When the language cannot handle a construct, it should:
- fail early
- fail clearly
- fail without guessing user intent

Panics are acceptable in early versions if they expose semantic gaps.

---

## 10. Evolution Constraint

New features may be added **only if they do not invalidate existing mental models**.

If a feature requires retroactive explanation, it is too expensive.

---

## 11. No Feature Without Pressure

A feature exists only because:
- a real example demands it, or
- an existing construct becomes unreasonably awkward without it

Speculative features are rejected by default.

---

## 12. Boring Is Good

Boring code is stable code.

Prefer well-understood techniques over novel ones unless novelty provides a clear semantic win.

---

## 13. Portability of Thought

Design decisions must be explainable without reference to:
- a specific implementation language
- a specific parser
- a specific runtime

If a rule cannot survive re-implementation, it is suspect.

---

## 14. Growth Discipline

At every version:
- the language must remain explainable in one sitting
- the entire interpreter must remain inspectable by one person

When this stops being true, growth has outpaced understanding.

---

## 15. Final Test

A change is acceptable only if it makes programs:
- easier to reason about
- easier to read later
- harder to misinterpret

Not shorter.  
Not faster.  
**Clearer.**
