// Mini-Python language schema

use crate::schema::LanguageSchema;

pub fn get_schema() -> LanguageSchema {
    let mut schema = LanguageSchema::new();

    // Multi-character lexemes (keywords, operators)
    schema.multichar_lexemes = vec![
        // Two-char operators
        "==", "!=", "<=", ">=", "**", "->",

        // Keywords
        "def", "if", "elif", "else", "while", "for", "break", "continue", "return",
        "and", "or", "not", "print", "True", "False", "None", "in", "pass",

        // Single-char operators (via schema)
        ":", "=", "+", "-", "*", "/", "%", "<", ">", "!", "&", "|", "^", "~",
        "(", ")", "[", "]", "{", "}", ",", ".", ";",
    ];

    // Keywords requiring word boundaries
    schema.word_boundary_keywords = vec![
        "def", "if", "elif", "else", "while", "for", "break", "continue", "return",
        "and", "or", "not", "print", "True", "False", "None", "in", "pass",
    ];

    // Statement terminators
    schema.terminators = vec!["\n", ";"];

    schema
}
