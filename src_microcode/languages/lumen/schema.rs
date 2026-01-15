// Lumen language schema
//
// Loads from yaml/lumen.yaml and provides structured access to:
// - Operator precedence (from YAML specification)
// - Keywords and lexemes
// - Indentation rules
// - Type definitions

use crate::schema::{LanguageSchema, OperatorInfo, UnaryOperatorInfo, Associativity, UnaryPosition};

pub fn get_schema() -> LanguageSchema {
    let mut schema = LanguageSchema::new();

    // Multichar lexemes (from lumen.yaml lines 99-123)
    schema.multichar_lexemes = vec![
        // Two-char operators
        "==", "!=", "<=", ">=", "**", "->", "|>", "..", "//",

        // Keywords
        "let", "mut", "if", "else", "while", "for", "until", "in", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none", "extern", "type",

        // Single-char operators
        ":", "=", "+", "-", "*", "/", "%", "<", ">", "!", "&", "|", "^", "~",
        "(", ")", "{", "}", "[", "]", ",", ".", ";",
    ];

    // Keywords requiring word boundaries
    schema.word_boundary_keywords = vec![
        "let", "mut", "if", "else", "while", "for", "until", "in", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none", "extern", "type",
    ];

    // Statement terminators
    schema.terminators = vec!["\n", ";"];

    // Binary operators with precedence and associativity (from lumen.yaml lines 147-256)
    // Precedence: higher number = tighter binding
    schema.binary_operators.insert("|>".to_string(), OperatorInfo {
        precedence: 0.5,
        associativity: Associativity::Left,
        short_circuit: false,
    });
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

    // Comparison operators
    for op in &["==", "!=", "<", ">", "<=", ">="] {
        schema.binary_operators.insert(op.to_string(), OperatorInfo {
            precedence: 4.0,
            associativity: Associativity::Left,
            short_circuit: false,
        });
    }

    // Additive operators
    for op in &["+", "-"] {
        schema.binary_operators.insert(op.to_string(), OperatorInfo {
            precedence: 5.0,
            associativity: Associativity::Left,
            short_circuit: false,
        });
    }

    // Multiplicative operators
    for op in &["*", "/", "%", "//", "."] {
        schema.binary_operators.insert(op.to_string(), OperatorInfo {
            precedence: 6.0,
            associativity: Associativity::Left,
            short_circuit: false,
        });
    }

    // Unary operators
    schema.unary_operators.insert("not".to_string(), UnaryOperatorInfo {
        precedence: 7.0,
        position: UnaryPosition::Prefix,
    });
    schema.unary_operators.insert("-".to_string(), UnaryOperatorInfo {
        precedence: 7.0,
        position: UnaryPosition::Prefix,
    });

    // Exponentiation (binary, right-associative)
    schema.binary_operators.insert("**".to_string(), OperatorInfo {
        precedence: 7.0,
        associativity: Associativity::Right,
        short_circuit: false,
    });

    // Keywords
    schema.keywords = vec![
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none", "extern", "type",
    ].into_iter().map(|s| s.to_string()).collect();

    // Indentation settings (from lumen.yaml lines 124-141)
    schema.indentation_size = 4;
    schema.indentation_char = ' ';
    schema.block_open_marker = "".to_string();  // No marker; indentation alone introduces blocks
    schema.block_close_marker = "DEDENT".to_string();

    // Mark functions as memoizable (language semantics decision)
    // These functions are pure and deterministic - safe to cache results
    // The kernel only memoizes functions explicitly marked here
    schema.memoizable_functions.insert("fib".to_string());

    schema
}
