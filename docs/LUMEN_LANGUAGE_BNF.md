# Lumen v0.1 — Backus-Naur Form Grammar

This is the authoritative BNF specification of Lumen v0.1.
If this grammar breaks, the language has drifted.

```bnf
Program        ::= Statement*

Statement      ::= SimpleStatement NEWLINE
                 | CompoundStatement

SimpleStatement ::= Assignment
                  | PrintStatement
                  | Break
                  | Continue

CompoundStatement ::= IfStatement
                    | WhileStatement

Assignment     ::= IDENTIFIER "=" Expression

PrintStatement ::= "print" "(" Expression ")"

Break          ::= "break"

Continue       ::= "continue"

IfStatement    ::= "if" Expression NEWLINE INDENT Statement+ DEDENT
                 | "if" Expression NEWLINE INDENT Statement+ DEDENT "else" NEWLINE INDENT Statement+ DEDENT

WhileStatement ::= "while" Expression NEWLINE INDENT Statement+ DEDENT

Expression     ::= OrExpression

OrExpression   ::= AndExpression ( "or" AndExpression )*

AndExpression  ::= EqualityExpression ( "and" EqualityExpression )*

EqualityExpression ::= ComparisonExpression ( ( "==" | "!=" ) ComparisonExpression )*

ComparisonExpression ::= AdditiveExpression ( ( "<" | "<=" | ">" | ">=" ) AdditiveExpression )*

AdditiveExpression ::= MultiplicativeExpression ( ( "+" | "-" ) MultiplicativeExpression )*

MultiplicativeExpression ::= UnaryExpression ( ( "*" | "/" ) UnaryExpression )*

UnaryExpression ::= ( "-" | "not" ) UnaryExpression
                  | PrimaryExpression

PrimaryExpression ::= NUMBER
                    | "true"
                    | "false"
                    | IDENTIFIER
                    | "(" Expression ")"

IDENTIFIER     ::= [a-zA-Z_][a-zA-Z0-9_]*

NUMBER         ::= [0-9]+

NEWLINE        ::= <actual newline character>

INDENT         ::= <4 consecutive spaces>

DEDENT         ::= <end of indented block>
```

## Notes

- All whitespace is insignificant except INDENT and DEDENT
- Identifiers are case-sensitive
- Literals are immutable
- Blocks must be indented exactly 4 spaces per level
- Tabs are not permitted in indentation
- Comments are not supported in v0.1
