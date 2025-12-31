# Dynamic Token System - Architectural Options

## Current System (Enum-Based)

```rust
// lexer.rs
pub enum Token {
    Plus, Minus, If, While,  // All hardcoded
    // ...
}

// Matching
match token {
    Token::Plus => { ... }
}
```

**Pros:**
- ✅ Fast (zero cost pattern matching)
- ✅ Type-safe (compile-time errors)
- ✅ No runtime overhead

**Cons:**
- ❌ All tokens hardcoded in lexer.rs
- ❌ Can't be defined by individual modules
- ❌ Deleting a feature file doesn't remove its token

---

## Option 1: String-Based Tokens

```rust
// lexer.rs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: &'static str,
}

// expr/arithmetic.rs
pub const PLUS: &'static str = "PLUS";
pub const MINUS: &'static str = "MINUS";

pub fn register(reg: &mut Registry) {
    reg.tokens.add_single_char('+', Token { kind: PLUS });
    reg.tokens.add_single_char('-', Token { kind: MINUS });

    // Handler uses string comparison
    reg.register_infix(Box::new(ArithmeticInfix {
        op: PLUS,  // Just a string
    }));
}

// Matching
if token.kind == PLUS {
    ...
}
// or
match token.kind {
    PLUS => { ... }
    MINUS => { ... }
    _ => {}
}
```

**Pros:**
- ✅ Fully modular - modules define their own tokens
- ✅ No central enum needed
- ✅ Easy to understand
- ✅ No ID collision issues

**Cons:**
- ❌ Slower (string comparison vs integer)
- ❌ Slightly more memory (string pointer vs enum discriminant)
- ❌ No exhaustiveness checking in match statements

**Performance Impact:** ~10-20% slower lexing (still very fast for an interpreter)

---

## Option 2: Numeric ID with Auto-Assignment

```rust
// lexer.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    id: u32,
}

impl Token {
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

// registry.rs
pub struct TokenRegistry {
    next_id: u32,
    // ...
}

impl TokenRegistry {
    pub fn add_single_char(&mut self, ch: char) -> Token {
        let token = Token::new(self.next_id);
        self.next_id += 1;
        self.single_char.insert(ch, token);
        token  // Return it so module can use it
    }
}

// expr/arithmetic.rs
pub struct ArithmeticTokens {
    pub plus: Token,
    pub minus: Token,
    // ...
}

pub fn register(reg: &mut Registry) -> ArithmeticTokens {
    let tokens = ArithmeticTokens {
        plus: reg.tokens.add_single_char('+'),
        minus: reg.tokens.add_single_char('-'),
        // ...
    };

    // Handlers store token IDs
    reg.register_infix(Box::new(ArithmeticInfix {
        op_token: tokens.plus,
    }));

    tokens
}

// Matching
if token == self.op_token {
    ...
}
```

**Pros:**
- ✅ Fast (integer comparison)
- ✅ Fully modular
- ✅ No ID collisions (auto-assigned)
- ✅ Compact memory

**Cons:**
- ❌ More complex setup (modules must store returned tokens)
- ❌ Can't pattern match easily
- ❌ Harder to debug (IDs aren't meaningful)

---

## Option 3: Hybrid - Keep Enum, Make it Extensible

```rust
// lexer.rs
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Structural tokens (always present)
    LParen,
    RParen,
    Newline,
    Indent,
    Dedent,
    Eof,

    // Identifiers and literals (always present)
    Ident(String),
    Number(f64),
    String(String),

    // Feature-defined tokens (extensible)
    Feature(&'static str, u32),  // (module_name, token_id)
}

// expr/arithmetic.rs
pub const MOD_NAME: &'static str = "arithmetic";
pub const PLUS: u32 = 0;
pub const MINUS: u32 = 1;

pub fn register(reg: &mut Registry) {
    reg.tokens.add_single_char('+', Token::Feature(MOD_NAME, PLUS));
    reg.tokens.add_single_char('-', Token::Feature(MOD_NAME, MINUS));
}

// Matching
match token {
    Token::Feature("arithmetic", 0) => { /* PLUS */ }
    Token::Feature("arithmetic", 1) => { /* MINUS */ }
    _ => {}
}
```

**Pros:**
- ✅ Keeps structural tokens type-safe
- ✅ Modules can define their own tokens
- ✅ Pattern matching still works
- ✅ Reasonable performance

**Cons:**
- ❌ Still somewhat hardcoded (Feature variant)
- ❌ Module name strings needed
- ❌ Matching is more verbose

---

## Option 4: Trait-Based (Most Flexible)

```rust
// lexer.rs
pub trait TokenType: std::fmt::Debug {
    fn as_any(&self) -> &dyn std::any::Any;
    fn clone_box(&self) -> Box<dyn TokenType>;
}

pub struct Token {
    inner: Box<dyn TokenType>,
}

// expr/arithmetic.rs
#[derive(Debug, Clone, Copy, PartialEq)]
enum ArithOp {
    Plus, Minus, Star, Slash, Percent,
}

impl TokenType for ArithOp {
    fn as_any(&self) -> &dyn Any { self }
    fn clone_box(&self) -> Box<dyn TokenType> { Box::new(*self) }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_single_char('+', Token::new(ArithOp::Plus));
    reg.tokens.add_single_char('-', Token::new(ArithOp::Minus));
}

// Matching
if let Some(op) = token.downcast::<ArithOp>() {
    match op {
        ArithOp::Plus => { ... }
        ArithOp::Minus => { ... }
    }
}
```

**Pros:**
- ✅ Maximum flexibility
- ✅ Each module defines its own token type
- ✅ Type-safe within modules
- ✅ Can have module-specific data

**Cons:**
- ❌ Most complex implementation
- ❌ Runtime type checks (downcasting)
- ❌ Most overhead
- ❌ Harder to understand

---

## Recommendation

For Lumen, I recommend **Option 1 (String-Based)** because:

1. **Simplicity**: Easy to understand and implement
2. **Good enough performance**: For an interpreter, 10-20% slower lexing is negligible
3. **True modularity**: Modules completely own their tokens
4. **Easy debugging**: Token names are readable strings
5. **No coordination needed**: No ID collision concerns

**Implementation effort:** ~2 hours
**Breaking changes:** Moderate (all pattern matching changes)

Would you like me to implement Option 1?
