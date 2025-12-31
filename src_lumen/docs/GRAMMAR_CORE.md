# Lumen – Grammar Core

This document captures the minimal, non-negotiable grammar of Lumen.
It is a checksum for the language: if this breaks, the language has drifted.

## Core Structure

program ::= statement*

## Statements

statement ::= assignment
            | print_statement
            | while_statement
            | if_statement

assignment ::= identifier "=" expression

print_statement ::= "print" "(" expression ")"

while_statement ::= "while" expression ":" block

if_statement ::= "if" expression ":" block
                 ( "else" ":" block )?

block ::= INDENT statement+ DEDENT

## Expressions

expression ::= comparison

comparison ::= arithmetic ( ("==" | "!=" | "<" | ">") arithmetic )?

arithmetic ::= term ( ("+" | "-") term )*

term ::= number
       | identifier
       | "(" expression ")"

## Lexical Rules (Summary)

- Indentation defines blocks (4 spaces per level)
- Parentheses group expressions only
- Control flow constructs are statements, not expressions
- Identifiers are bare; no sigils
- Whitespace is insignificant except for indentation

End of grammar core.
