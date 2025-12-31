# Lumen Grammar (v0.1)

## Overview

Lumen uses a Python-style indentation-based grammar.
Blocks are introduced by colons and defined by increased indentation.

Statements are line-oriented: each statement occupies exactly one line unless it introduces a block.

## Core Grammar Rules

```
Program       → Stmt*

SimpleStmt    → Assignment | Print | Break | Continue
CompoundStmt  → IfStmt | WhileStmt

Assignment    → IDENT "=" Expr
Print         → "print" "(" Expr ")"
Break         → "break"
Continue      → "continue"

IfStmt        → "if" Expr NEWLINE INDENT Stmt+ DEDENT
              | "else" NEWLINE INDENT Stmt+ DEDENT

WhileStmt     → "while" Expr NEWLINE INDENT Stmt+ DEDENT

Expr          → OrExpr
OrExpr        → AndExpr ("or" AndExpr)*
AndExpr       → EqualityExpr ("and" EqualityExpr)*
EqualityExpr  → CompareExpr (("==" | "!=") CompareExpr)*
CompareExpr   → AddExpr (("<" | "<=" | ">" | ">=") AddExpr)*
AddExpr       → MulExpr (("+" | "-") MulExpr)*
MulExpr       → UnaryExpr (("*" | "/") UnaryExpr)*
UnaryExpr     → ("-" | "not") UnaryExpr | Primary
Primary       → NUMBER | "true" | "false" | IDENT | "(" Expr ")"
```

## Language Characteristics

**Syntax:**
- No semicolons required (newlines are statement separators)
- No user-defined functions (v0.1 limitation)
- Exactly one builtin function: `print()`

**Semantics:**
- Control flow is explicit and minimal
- No implicit truthiness: only boolean values are true/false
- break and continue apply only to the nearest enclosing while loop

**Indentation:**
- Exactly 4 spaces per indentation level
- Tabs are not supported
- Mixed indentation is an error

**Execution:**
- Programs execute top-to-bottom
- while and if statements introduce lexical scopes
- Variables persist across block boundaries (no shadowing)

## Stability

This grammar is intentionally small and stable.
Future versions may extend the grammar but will not change v0.1 semantics.
