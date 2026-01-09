// Lumen-specific lexer for opaque kernel

use crate::kernel::lexer::Token;

/// Lumen keywords
pub const LUMEN_KEYWORDS: &[&str] = &[
    "if", "else", "end", "while", "until", "for", "in",
    "fn", "let", "mut", "return", "break", "continue",
    "print", "true", "false", "none", "and", "or", "not",
];

/// Lex Lumen source code
pub fn lex_lumen(source: &str) -> Vec<Token> {
    crate::kernel::lexer::lex(source, LUMEN_KEYWORDS)
}
