# Lumen – Grammar & Syntax

This document defines the syntactic grammar of the Lumen programming language.

It is intentionally minimal and aligned with the Abstract Syntax Tree (AST).
When discrepancies arise, the AST and DESIGN.md take precedence over parser behavior.

This document specifies what is valid, not how it is implemented.

## 1. Lexical Conventions

### 1.1 Indentation

Lumen is an indentation-sensitive language.

Blocks are introduced by a colon (:).
Increased indentation begins a block.
Decreased indentation ends a block.
One indentation level equals 4 spaces.
Tabs are not supported.

Indentation may be modeled internally using INDENT and DEDENT tokens.

### 1.2 Whitespace

Blank lines are ignored.
Leading and trailing whitespace is insignificant except for indentation.

### 1.3 Identifiers

Grammar:
identifier ::= letter ( letter | digit | "_" )*

Identifiers are case-sensitive.
Identifiers must not begin with a digit.

Examples:
x
count
next_value

### 1.4 Literals

Grammar:
number ::= digit+ ( "." digit+ )?

All numeric literals are treated as numeric scalars.

Examples:
0
1
3.14

## 2. Program Structure

Grammar:
program ::= statement*

A Lumen program is a sequence of zero or more statements.

## 3. Statements

Statements control execution or mutate program state.
Statements do not produce values.

Grammar:
statement ::= assignment
            | print_statement
            | while_statement
            | if_statement

### 3.1 Assignment Statement

Grammar:
assignment ::= identifier "=" expression

Example:
x = 10

### 3.2 Print Statement

Grammar:
print_statement ::= "print" "(" expression ")"

Example:
print(x)

### 3.3 While Statement

Grammar:
while_statement ::= "while" expression ":" block

Example:
while x < 5:
    print(x)
    x = x + 1

### 3.4 If Statement

Grammar:
if_statement ::= "if" expression ":" block
                 ( "else" ":" block )?

Example:
if x > 0:
    print(x)
else:
    print(0)

## 4. Blocks

Blocks group one or more statements under a control structure.

Conceptual grammar:
block ::= INDENT statement+ DEDENT

Blocks are defined by indentation and are not explicitly written in source code.

## 5. Expressions

Expressions compute values and may appear in assignments, conditions, and print statements.

Grammar:
expression ::= comparison

### 5.1 Comparison Expressions

Grammar:
comparison ::= arithmetic ( comparison_op arithmetic )?

comparison_op ::= "==" | "!=" | "<" | ">"

Examples:
x < 5
a == b

### 5.2 Arithmetic Expressions

Grammar:
arithmetic ::= term ( ("+" | "-") term )*

Examples:
x + 1
a - b + c

### 5.3 Terms

Grammar:
term ::= number
       | identifier
       | "(" expression ")"

Examples:
42
x
(x + 1)

## 6. Statements vs Expressions

Expressions compute values.
Statements control execution.

Control structures (if, while) are statements, not expressions.
They do not return values.

This distinction is intentional and visible in the syntax.

## 7. Version Scope

This grammar describes Lumen v0.0.1.

Future versions may introduce functions, return statements, additional operators, and richer literal types.

All changes must preserve existing grammar where possible and comply with DESIGN.md.

## 8. Diagram Compatibility

This grammar is suitable for EBNF notation, railroad / syntax diagrams, and AST-driven documentation.

Indentation rules are lexical and should not be diagrammed directly.

End of grammar.
