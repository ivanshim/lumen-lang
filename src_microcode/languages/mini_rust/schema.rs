use crate::schema::{LanguageSchema, OperatorInfo, UnaryOperatorInfo, Associativity, UnaryPosition};

pub fn get_schema() -> LanguageSchema {
    let mut schema = LanguageSchema::new();

    // Multichar lexemes (Mini-Rust with brace syntax, not indentation)
    schema.multichar_lexemes = vec![
        "==", "!=", "<=", ">=", "**", "->", "&&", "||",
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none",
        ":", "=", "+", "-", "*", "/", "%", "<", ">", "!", "&", "|", "^", "~",
        "(", ")", "{", "}", "[", "]", ",", ".", ";",
    ];

    schema.word_boundary_keywords = vec![
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none",
    ];

    schema.terminators = vec!["\n", ";"];

    // Binary operators (similar to Lumen but no pipe operator)
    schema.binary_operators.insert("=".to_string(), OperatorInfo {
        precedence: 1.0,
        associativity: Associativity::Right,
        short_circuit: false,
    });
    // Logical operators: both || and 'or', && and 'and'
    schema.binary_operators.insert("||".to_string(), OperatorInfo {
        precedence: 2.0,
        associativity: Associativity::Left,
        short_circuit: true,
    });
    schema.binary_operators.insert("or".to_string(), OperatorInfo {
        precedence: 2.0,
        associativity: Associativity::Left,
        short_circuit: true,
    });
    schema.binary_operators.insert("&&".to_string(), OperatorInfo {
        precedence: 3.0,
        associativity: Associativity::Left,
        short_circuit: true,
    });
    schema.binary_operators.insert("and".to_string(), OperatorInfo {
        precedence: 3.0,
        associativity: Associativity::Left,
        short_circuit: true,
    });

    for op in &["==", "!=", "<", ">", "<=", ">="] {
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

    for op in &["*", "/", "%"] {
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
        precedence: 7.0,
        position: UnaryPosition::Prefix,
    });
    schema.unary_operators.insert("!".to_string(), UnaryOperatorInfo {
        precedence: 7.0,
        position: UnaryPosition::Prefix,
    });
    schema.unary_operators.insert("-".to_string(), UnaryOperatorInfo {
        precedence: 7.0,
        position: UnaryPosition::Prefix,
    });

    schema.keywords = vec![
        "let", "mut", "if", "else", "while", "for", "break", "continue", "return", "fn",
        "and", "or", "not", "print", "true", "false", "none",
    ].into_iter().map(|s| s.to_string()).collect();

    // Mini-Rust uses braces, not indentation
    schema.indentation_size = 0;
    schema.block_open_marker = "{".to_string();
    schema.block_close_marker = "}".to_string();

    schema
}
