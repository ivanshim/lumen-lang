# Lumen v0.1 — Core Language Design

This document defines the minimal, non-negotiable structure of Lumen.
It is the authoritative specification: if this breaks, the language has drifted.

## Structure

A Lumen program is a sequence of statements executed top-to-bottom.

## Statements

Simple statements occupy a single line and end with a newline:
- `Assignment` - bind a value to an identifier
- `Print` - output a value
- `Break` - exit a loop
- `Continue` - skip to next loop iteration

Compound statements introduce indented blocks:
- `IfStmt` - conditional execution with optional else
- `WhileStmt` - loop while condition is true

```
assignment  → IDENT "=" Expr
print       → "print" "(" Expr ")"
break       → "break"
continue    → "continue"
if          → "if" Expr NEWLINE INDENT Stmt+ DEDENT ("else" NEWLINE INDENT Stmt+ DEDENT)?
while       → "while" Expr NEWLINE INDENT Stmt+ DEDENT
```

## Expressions

Expressions evaluate to values and follow operator precedence.

```
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

## Lexical Rules

- Indentation defines blocks (exactly 4 spaces per level)
- Newlines separate statements
- Identifiers are sequences of alphanumeric characters and underscores
- Comments are not supported in v0.1
- Whitespace is insignificant except for indentation
