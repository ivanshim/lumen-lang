Lumen v0.1 — Core Grammar

Program        ::= Stmt*

Stmt           ::= SimpleStmt NEWLINE
                 | CompoundStmt

SimpleStmt    ::= Assignment
                 | Print
                 | Break
                 | Continue

CompoundStmt  ::= IfStmt
                 | WhileStmt

Assignment    ::= IDENT "=" Expr

Print         ::= "print" "(" Expr ")"

Break         ::= "break"
Continue      ::= "continue"

IfStmt        ::= "if" Expr ":" NEWLINE INDENT Stmt+ DEDENT
                   ( "else" ":" NEWLINE INDENT Stmt+ DEDENT )?

WhileStmt     ::= "while" Expr ":" NEWLINE INDENT Stmt+ DEDENT

Expr          ::= OrExpr

OrExpr        ::= AndExpr ( "or" AndExpr )*

AndExpr       ::= EqualityExpr ( "and" EqualityExpr )*

EqualityExpr ::= CompareExpr ( ("==" | "!=") CompareExpr )*

CompareExpr  ::= AddExpr ( ("<" | "<=" | ">" | ">=") AddExpr )*

AddExpr      ::= MulExpr ( ("+" | "-") MulExpr )*

MulExpr      ::= UnaryExpr ( ("*" | "/") UnaryExpr )*

UnaryExpr    ::= ("-" | "not") UnaryExpr
                 | Primary

Primary      ::= NUMBER
                 | "true"
                 | "false"
                 | IDENT
                 | "(" Expr ")"
