# Language Implementation Examples

## Key Code Snippets Demonstrating Unique Features

This document shows the actual implementation code that makes each language unique.

---

## 1. Mini-PHP: Variable Access with $ Prefix

### Variable Expression (`src_mini_php/expressions/variable.rs`)

```rust
// Mini-PHP: Variable reference with $ prefix

use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, ExprPrefix, LumenResult, Registry};
use crate::framework::runtime::{Env, Value};
use crate::src_mini_php::structure::structural::DOLLAR;

#[derive(Debug)]
struct VarExpr {
    name: String,
}

impl ExprNode for VarExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(DOLLAR))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume $
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => Err(err_at(parser, "Expected identifier after '$'")),
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(VariablePrefix));
}
```

### Assignment Statement (`src_mini_php/statements/assignment.rs`)

```rust
pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Matches: $ <ident> =
        matches!(parser.peek(), Token::Feature(DOLLAR))
            && matches!(parser.peek_n(1), Some(Token::Ident(_)))
            && matches!(parser.peek_n(2), Some(Token::Feature(ASSIGN)))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume $
        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => return Err(err_at(parser, "Expected identifier after '$'")),
        };
        match parser.advance() {
            Token::Feature(ASSIGN) => {}
            _ => return Err(err_at(parser, "Expected '=' in assignment")),
        }
        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}
```

**Example Usage:**
```php
$x = 10;          // Assignment with $
$y = $x + 5;      // Variable access with $
echo($y);         // Output: 15
```

---

## 2. Mini-SH: Shell-Style Variable Handling

### Variable Expression (`src_mini_sh/expressions/variable.rs`)

```rust
// Shell script: $ for reading, not for assignment

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(DOLLAR))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // $ for expansion
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => Err(err_at(parser, "Expected identifier after '$'")),
        }
    }
}
```

### Assignment Statement (`src_mini_sh/statements/assignment.rs`)

```rust
// Shell style: x=5 (no $ on left side)

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Matches: <ident> = (NO $ prefix)
        matches!(parser.peek(), Token::Ident(_))
            && matches!(parser.peek_n(1), Some(Token::Feature(ASSIGN)))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => unreachable!(),
        };
        match parser.advance() {
            Token::Feature(ASSIGN) => {}
            _ => return Err(err_at(parser, "Expected '='")),
        }
        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}
```

**Example Usage:**
```sh
x=10              # Assignment without $
y=$x              # Variable expansion with $
print($x + $y)    # Output: 20
```

---

## 3. Mini-C: Standard C-Style Syntax

### Variable Expression (`src_mini_c/expressions/variable.rs`)

```rust
// Standard identifier-based variable access

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => unreachable!(),
        }
    }
}
```

### Print Statement (`src_mini_c/statements/print.rs`)

```rust
// Uses 'printf' keyword

pub const PRINTF: &str = "PRINTF";

pub struct PrintStmtHandler;

impl StmtHandler for PrintStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(PRINTF))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'printf'
        match parser.advance() {
            Token::Feature(LPAREN) => {}
            _ => return Err(err_at(parser, "Expected '(' after 'printf'")),
        }
        let expr = parser.parse_expr()?;
        match parser.advance() {
            Token::Feature(RPAREN) => {}
            _ => return Err(err_at(parser, "Expected ')'")),
        }
        Ok(Box::new(PrintStmt { expr }))
    }
}
```

**Example Usage:**
```c
x = 10;
y = 20;
printf(x + y);    // Output: 30
```

---

## 4. Mini-Apple-Pascal: BEGIN/END and := Operator

### Structural Syntax (`src_mini_apple_pascal/structure/structural.rs`)

```rust
// Pascal uses BEGIN/END instead of braces

pub const BEGIN: &str = "BEGIN";
pub const END: &str = "END";

// Aliases for compatibility with generic block parser
pub const LBRACE: &str = "BEGIN";
pub const RBRACE: &str = "END";

pub fn parse_block(parser: &mut Parser) -> LumenResult<Vec<Box<dyn StmtNode>>> {
    match parser.advance() {
        Token::Feature(k) if k == BEGIN => {}
        _ => return Err(err_at(parser, "Expected 'BEGIN'")),
    }

    let mut statements = Vec::new();
    consume_semicolons(parser);

    while !matches!(parser.peek(), Token::Feature(k) if *k == END || *k == EOF) {
        let stmt = parser.reg.find_stmt(parser)
            .ok_or_else(|| err_at(parser, "Unknown statement"))?
            .parse(parser)?;
        statements.push(stmt);
        consume_semicolons(parser);
    }

    match parser.advance() {
        Token::Feature(k) if k == END => {}
        _ => return Err(err_at(parser, "Expected 'END'")),
    }

    Ok(statements)
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("BEGIN", BEGIN);
    reg.tokens.add_keyword("END", END);
    // ...
}
```

### Assignment with := (`src_mini_apple_pascal/statements/assignment.rs`)

```rust
// Pascal uses := for assignment

pub const ASSIGN: &str = "ASSIGN";

pub fn register(reg: &mut Registry) {
    reg.tokens.add_two_char(":=", ASSIGN);  // Two-char token!
    reg.register_stmt(Box::new(AssignStmtHandler));
}
```

### Print Statement (`src_mini_apple_pascal/statements/print.rs`)

```rust
// Pascal uses 'writeln'

pub const WRITELN: &str = "WRITELN";

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("writeln", WRITELN);
    reg.register_stmt(Box::new(PrintStmtHandler));
}
```

**Example Usage:**
```pascal
x := 10;
y := 20;
writeln(x + y);

if (x < y) BEGIN
    writeln(x);
END else BEGIN
    writeln(y);
END
```

---

## 5. Mini-Apple-BASIC: LET Keyword

### Assignment with LET (`src_mini_apple_basic/statements/assignment.rs`)

```rust
// BASIC requires LET keyword for assignment

pub const LET: &str = "LET";
pub const ASSIGN: &str = "ASSIGN";

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(LET))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume LET

        let name = match parser.advance() {
            Token::Ident(s) => s,
            _ => return Err(err_at(parser, "Expected identifier after LET")),
        };

        match parser.advance() {
            Token::Feature(ASSIGN) => {}
            _ => return Err(err_at(parser, "Expected '=' after variable")),
        }

        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("LET", LET);
    reg.tokens.add_single_char('=', ASSIGN);
    reg.register_stmt(Box::new(AssignStmtHandler));
}
```

### Print Statement (`src_mini_apple_basic/statements/print.rs`)

```rust
// BASIC uses uppercase PRINT

pub const PRINT: &str = "PRINT";

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("PRINT", PRINT);
    reg.register_stmt(Box::new(PrintStmtHandler));
}
```

**Example Usage:**
```basic
LET x = 10
LET y = 20
PRINT(x + y)

if (x < y) {
    PRINT(x)
} else {
    PRINT(y)
}
```

---

## Common Patterns Across All Languages

### Arithmetic Operations (Shared Implementation)

All 5 languages share the same arithmetic implementation:

```rust
pub struct ArithmeticInfix {
    op: &'static str,
    prec: Precedence,
}

impl ExprInfix for ArithmeticInfix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(kind) if *kind == self.op)
    }

    fn precedence(&self) -> Precedence { self.prec }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op, right }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_infix(Box::new(ArithmeticInfix::new(PLUS, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(STAR, Precedence::Factor)));
    // ...
}
```

### While Loops (Shared Structure, Language-Specific Parsing)

All languages use similar while loop implementation with language-specific block parsing:

```rust
impl StmtNode for WhileStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            let cond = self.condition.eval(env)?;
            match cond {
                Value::Bool(true) => {
                    for stmt in &self.body {
                        match stmt.exec(env)? {
                            Control::Break => return Ok(Control::None),
                            Control::Continue => break,
                            Control::None => {}
                        }
                    }
                }
                Value::Bool(false) => break,
                _ => return Err("while condition must be boolean".into()),
            }
        }
        Ok(Control::None)
    }
}
```

---

## Summary of Unique Features by Language

| Language | Unique Feature | Implementation Location |
|----------|---------------|------------------------|
| **mini-php** | `$` prefix for variables | `expressions/variable.rs` |
| | `echo()` statement | `statements/print.rs` |
| | `$` in assignment | `statements/assignment.rs` |
| **mini-sh** | `$` only for expansion | `expressions/variable.rs` |
| | No `$` in assignment | `statements/assignment.rs` |
| | `print()` statement | `statements/print.rs` |
| **mini-c** | `printf()` statement | `statements/print.rs` |
| | Standard C syntax | All files |
| **mini-pascal** | `BEGIN`/`END` blocks | `structure/structural.rs` |
| | `:=` assignment | `statements/assignment.rs` |
| | `writeln()` statement | `statements/print.rs` |
| **mini-basic** | `LET` keyword | `statements/assignment.rs` |
| | `PRINT()` uppercase | `statements/print.rs` |
| | Line number ready | `structure/structural.rs` |

---

## Framework Pattern Used

All implementations follow this consistent pattern:

```rust
// 1. Define the AST node
#[derive(Debug)]
struct MyNode { /* fields */ }

// 2. Implement the trait (ExprNode or StmtNode)
impl ExprNode for MyNode {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // implementation
    }
}

// 3. Define the handler
pub struct MyHandler;

// 4. Implement the handler trait (ExprPrefix, ExprInfix, or StmtHandler)
impl ExprPrefix for MyHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // token matching logic
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        // parsing logic
    }
}

// 5. Register the handler
pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("keyword", TOKEN_CONST);
    reg.register_prefix(Box::new(MyHandler));
}
```

This pattern is used consistently across all 95 files in all 5 language modules!
