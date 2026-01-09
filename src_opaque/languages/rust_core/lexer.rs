// Rust-specific lexer for opaque kernel

use crate::kernel::lexer::Token;

/// Rust keywords
pub const RUST_KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "in",
    "fn", "let", "return", "break", "continue",
    "true", "false", "True", "False", "None",
];

/// Lex Rust source code
pub fn lex_rust(source: &str) -> Vec<Token> {
    crate::kernel::lexer::lex(source, RUST_KEYWORDS)
}
