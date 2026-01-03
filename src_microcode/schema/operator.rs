// Operator definitions
//
// All operators in a language are described as data: precedence, associativity, and arity.
// The kernel parser uses these to construct operator precedence trees.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Associativity {
    /// Left-associative: a + b + c = (a + b) + c
    Left,

    /// Right-associative: a = b = c = (a = (b = c))
    Right,

    /// Non-associative: a < b < c is an error
    None,
}

/// Binary operator information
#[derive(Debug, Clone)]
pub struct OperatorInfo {
    /// Operator lexeme (e.g., "+", "-", "*", "/")
    pub lexeme: String,

    /// Precedence level (higher = binds tighter)
    /// Common precedence levels:
    ///   1 = assignment (=)
    ///   2 = logical or (or)
    ///   3 = logical and (and)
    ///   4 = comparison (==, !=, <, >, <=, >=)
    ///   5 = addition/subtraction (+, -)
    ///   6 = multiplication/division (*, /)
    pub precedence: u32,

    /// How the operator associates
    pub associativity: Associativity,
}

impl OperatorInfo {
    pub fn new(lexeme: &str, precedence: u32, associativity: Associativity) -> Self {
        Self {
            lexeme: lexeme.to_string(),
            precedence,
            associativity,
        }
    }

    /// Left-associative operator of given precedence
    pub fn left(lexeme: &str, precedence: u32) -> Self {
        Self::new(lexeme, precedence, Associativity::Left)
    }

    /// Right-associative operator of given precedence
    pub fn right(lexeme: &str, precedence: u32) -> Self {
        Self::new(lexeme, precedence, Associativity::Right)
    }

    /// Non-associative operator of given precedence
    pub fn none(lexeme: &str, precedence: u32) -> Self {
        Self::new(lexeme, precedence, Associativity::None)
    }
}

/// Unary operator information
#[derive(Debug, Clone)]
pub struct UnaryOperatorInfo {
    /// Operator lexeme (e.g., "not", "-", "!")
    pub lexeme: String,

    /// Precedence level (all unary ops bind tighter than binary)
    /// Typically in the 7+ range
    pub precedence: u32,

    /// Is this prefix (before operand) or postfix (after operand)?
    pub is_prefix: bool,
}

impl UnaryOperatorInfo {
    pub fn new(lexeme: &str, precedence: u32, is_prefix: bool) -> Self {
        Self {
            lexeme: lexeme.to_string(),
            precedence,
            is_prefix,
        }
    }

    /// Create a prefix unary operator
    pub fn prefix(lexeme: &str, precedence: u32) -> Self {
        Self::new(lexeme, precedence, true)
    }

    /// Create a postfix unary operator
    pub fn postfix(lexeme: &str, precedence: u32) -> Self {
        Self::new(lexeme, precedence, false)
    }
}
