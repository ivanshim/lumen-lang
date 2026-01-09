// Python-specific lexer for opaque kernel

use crate::kernel::lexer::Token;

/// Python keywords
pub const PYTHON_KEYWORDS: &[&str] = &[
    "if", "else", "while", "for", "in",
    "def", "return", "break", "continue",
    "print", "True", "False", "None", "and", "or", "not",
];

/// Lex Python source code
pub fn lex_python(source: &str) -> Vec<Token> {
    crate::kernel::lexer::lex(source, PYTHON_KEYWORDS)
}
