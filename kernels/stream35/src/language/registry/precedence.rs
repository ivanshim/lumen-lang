// Operator precedence: the tier an operator sits in, read from the
// definition's op.precedence table. Higher binds tighter.

use crate::language::definition::def;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Precedence(pub u32);

impl Precedence {
    pub fn lowest() -> Self {
        Precedence(0)
    }

    /// The tier of a binary operator lexeme.
    pub fn binary(lexeme: &str) -> Self {
        Precedence(def().binary_precedence(lexeme))
    }

    /// The tier of a unary operator lexeme.
    pub fn unary(lexeme: &str) -> Self {
        Precedence(def().unary_precedence(lexeme))
    }

    /// Above every operator: postfix forms such as indexing.
    pub fn postfix() -> Self {
        Precedence(def().postfix_precedence())
    }

    /// The next tighter level.
    pub fn next(self) -> Self {
        Precedence(self.0 + 1)
    }

    /// The minimum precedence for the right operand of `lexeme`: the same
    /// tier for a right-associative operator, one tighter otherwise.
    pub fn right_operand(self, lexeme: &str) -> Self {
        if def().is_right_associative(lexeme) {
            self
        } else {
            self.next()
        }
    }
}
