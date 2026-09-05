// Mini-PythonCore language schema
//
// Loads from yaml/python.yaml with PythonCore-like indentation-based syntax

use crate::schema::{LanguageSchema, OperatorInfo, UnaryOperatorInfo, Associativity, UnaryPosition};

pub fn get_schema() -> LanguageSchema {
    let mut schema = LanguageSchema::new();

    // Multi-character lexemes (keywords, operators)
    schema.multichar_lexemes = vec![
        // Two-char operators
        "==", "!=", "<=", ">=", "**",

        // Keywords
        "def", "if", "elif", "else", "while", "for", "break", "continue", "return",
        "and", "or", "not", "print", "True", "False", "None", "in", "pass",
        "true", "false", "none",  // Also support lowercase variants

        // Single-char operators
        ":", "=", "+", "-", "*", "/", "%", "<", ">", "!", "&", "|", "^", "~",
        "(", ")", "[", "]", "{", "}", ",", ".", ";",
    ];

    // Keywords requiring word boundaries
    schema.word_boundary_keywords = vec![
        "def", "if", "elif", "else", "while", "for", "break", "continue", "return",
        "and", "or", "not", "print", "True", "False", "None", "in", "pass",
        "true", "false", "none",  // Also support lowercase variants
    ];

    // Statement terminators
    schema.terminators = vec!["\n", ";"];

    // Binary operators (PythonCore-like precedence)
    schema.binary_operators.insert("=".to_string(), OperatorInfo {
        precedence: 1.0,
        associativity: Associativity::Right,
        short_circuit: false,
    });
    schema.binary_operators.insert("or".to_string(), OperatorInfo {
        precedence: 2.0,
        associativity: Associativity::Left,
        short_circuit: true,
    });
    schema.binary_operators.insert("and".to_string(), OperatorInfo {
        precedence: 3.0,
        associativity: Associativity::Left,
        short_circuit: true,
    });

    for op in &["==", "!=", "<", ">", "<=", ">=", "in"] {
        schema.binary_operators.insert(op.to_string(), OperatorInfo {
            precedence: 4.0,
            associativity: Associativity::Left,
            short_circuit: false,
        });
    }

    for op in &["+", "-"] {
        schema.binary_operators.insert(op.to_string(), OperatorInfo {
            precedence: 5.0,
            associativity: Associativity::Left,
            short_circuit: false,
        });
    }

    for op in &["*", "/", "%", "//"] {
        schema.binary_operators.insert(op.to_string(), OperatorInfo {
            precedence: 6.0,
            associativity: Associativity::Left,
            short_circuit: false,
        });
    }

    schema.binary_operators.insert("**".to_string(), OperatorInfo {
        precedence: 7.0,
        associativity: Associativity::Right,
        short_circuit: false,
    });

    // Unary operators
    schema.unary_operators.insert("not".to_string(), UnaryOperatorInfo {
        precedence: 3.0,
        position: UnaryPosition::Prefix,
    });
    schema.unary_operators.insert("-".to_string(), UnaryOperatorInfo {
        precedence: 7.0,
        position: UnaryPosition::Prefix,
    });

    schema.keywords = vec![
        "def", "if", "elif", "else", "while", "for", "break", "continue", "return",
        "and", "or", "not", "print", "True", "False", "None", "in", "pass",
    ].into_iter().map(|s| s.to_string()).collect();

    // PythonCore-like indentation settings
    schema.indentation_size = 4;
    schema.indentation_char = ' ';
    schema.block_open_marker = ":".to_string();
    schema.block_close_marker = "DEDENT".to_string();

    schema
}
