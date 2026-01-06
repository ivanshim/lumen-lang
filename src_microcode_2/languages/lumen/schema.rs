// Lumen language schema
//
// All Lumen-specific syntax and semantics are defined here.
// The kernel knows nothing about Lumen - only about the primitives.

use crate::schema::LanguageSchema;

pub fn get_schema() -> LanguageSchema {
    let mut schema = LanguageSchema::new();

    // Multichar lexemes (operators, keywords)
    schema.multichar_lexemes = vec![
        // Two-char operators
        "==", "!=", "<=", ">=", "**", "->",

        // Keywords
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn", "and", "or", "not",
        "print", "true", "false", "none", "extern",

        // Single-char operators (via schema)
        ":", "=", "+", "-", "*", "/", "%", "<", ">", "!", "&", "|", "^", "~",
        "(", ")", "{", "}", "[", "]", ",", ".", ";",
    ];

    // Keywords that require word boundaries
    schema.word_boundary_keywords = vec![
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none", "extern",
    ];

    // Terminators (statement boundaries)
    schema.terminators = vec!["\n", ";"];

    schema
}
