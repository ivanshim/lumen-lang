// Mini-Python language for microcode kernel
//
// This module contains ONLY declarative data about the Mini-Python language.
// All parsing and execution logic is in the kernel.

pub mod values;

use crate::schema::{
    LanguageSchema, StatementPattern, StatementRole, OperatorInfo, UnaryOperatorInfo,
    Associativity, ExternSyntax, PatternBuilder,
};
use std::collections::HashMap;

/// Build the complete Mini-Python language schema
pub fn get_schema() -> LanguageSchema {
    // Statement patterns (if, while, for, etc.)
    let mut statements = HashMap::new();

    // print statement
    statements.insert(
        "print".to_string(),
        PatternBuilder::new("print")
            .keyword("print_keyword")
            .literal("(", "lparen")
            .expression("expression")
            .literal(")", "rparen")
            .build(),
    );

    // if statement
    statements.insert(
        "if".to_string(),
        PatternBuilder::new("if")
            .keyword("if_keyword")
            .expression("condition")
            .literal(":", "colon")
            .block("then_block")
            .build(),
    );

    // while statement
    statements.insert(
        "while".to_string(),
        PatternBuilder::new("while")
            .keyword("while_keyword")
            .expression("condition")
            .literal(":", "colon")
            .block("body")
            .build(),
    );

    // for statement
    statements.insert(
        "for".to_string(),
        PatternBuilder::new("for")
            .keyword("for_keyword")
            .expression("variable")
            .keyword("in")
            .expression("iterable")
            .literal(":", "colon")
            .block("body")
            .build(),
    );

    // break statement
    statements.insert(
        "break".to_string(),
        PatternBuilder::new("break")
            .keyword("break_keyword")
            .build(),
    );

    // continue statement
    statements.insert(
        "continue".to_string(),
        PatternBuilder::new("continue")
            .keyword("continue_keyword")
            .build(),
    );

    // Binary operators with precedence and associativity
    let mut binary_ops = HashMap::new();

    // Assignment (precedence 0, right-associative)
    binary_ops.insert(
        "=".to_string(),
        OperatorInfo::right("=", 0),
    );

    // Logical OR (precedence 2, left-associative)
    binary_ops.insert(
        "or".to_string(),
        OperatorInfo::left("or", 2),
    );

    // Logical AND (precedence 3, left-associative)
    binary_ops.insert(
        "and".to_string(),
        OperatorInfo::left("and", 3),
    );

    // Comparison operators (precedence 4, left-associative)
    binary_ops.insert(
        "==".to_string(),
        OperatorInfo::left("==", 4),
    );
    binary_ops.insert(
        "!=".to_string(),
        OperatorInfo::left("!=", 4),
    );
    binary_ops.insert(
        "<".to_string(),
        OperatorInfo::left("<", 4),
    );
    binary_ops.insert(
        ">".to_string(),
        OperatorInfo::left(">", 4),
    );
    binary_ops.insert(
        "<=".to_string(),
        OperatorInfo::left("<=", 4),
    );
    binary_ops.insert(
        ">=".to_string(),
        OperatorInfo::left(">=", 4),
    );
    binary_ops.insert(
        "in".to_string(),
        OperatorInfo::left("in", 4),
    );

    // Additive operators (precedence 5, left-associative)
    binary_ops.insert(
        "+".to_string(),
        OperatorInfo::left("+", 5),
    );
    binary_ops.insert(
        "-".to_string(),
        OperatorInfo::left("-", 5),
    );

    // Multiplicative operators (precedence 6, left-associative)
    binary_ops.insert(
        "*".to_string(),
        OperatorInfo::left("*", 6),
    );
    binary_ops.insert(
        "/".to_string(),
        OperatorInfo::left("/", 6),
    );
    binary_ops.insert(
        "//".to_string(),
        OperatorInfo::left("//", 6),
    );
    binary_ops.insert(
        "%".to_string(),
        OperatorInfo::left("%", 6),
    );

    // Exponentiation (precedence 7, right-associative)
    binary_ops.insert(
        "**".to_string(),
        OperatorInfo::right("**", 7),
    );

    // Unary operators with precedence
    let mut unary_ops = HashMap::new();

    // Logical NOT (prefix)
    unary_ops.insert(
        "not".to_string(),
        UnaryOperatorInfo::prefix("not", 8),
    );

    // Arithmetic negation (prefix)
    unary_ops.insert(
        "-".to_string(),
        UnaryOperatorInfo::prefix("-", 8),
    );

    // Unary plus (prefix)
    unary_ops.insert(
        "+".to_string(),
        UnaryOperatorInfo::prefix("+", 8),
    );

    // Keywords
    let keywords = vec![
        "print".to_string(),
        "if".to_string(),
        "else".to_string(),
        "elif".to_string(),
        "while".to_string(),
        "for".to_string(),
        "in".to_string(),
        "break".to_string(),
        "continue".to_string(),
        "and".to_string(),
        "or".to_string(),
        "not".to_string(),
        "True".to_string(),
        "False".to_string(),
        "true".to_string(),
        "false".to_string(),
        "range".to_string(),
    ];

    // Statement terminators (Python uses newlines)
    let statement_terminators = vec![
        "\n".to_string(),
    ];

    // Multi-character lexemes for maximal-munch
    let multichar_lexemes = vec![
        // Keywords (longer first for maximal-munch)
        "continue", "range",
        "while", "False", "print", "True", "false", "true",
        "break", "False", "else", "elif", "not", "and", "if", "or", "in",
        // Operators
        "==", "!=", "<=", ">=", "//", "**",  // multi-char operators
    ];

    LanguageSchema {
        statements,
        binary_ops,
        unary_ops,
        keywords,
        multichar_lexemes,
        statement_terminators,
        block_open: "".to_string(),  // Python uses indentation, no explicit block open
        block_close: "".to_string(), // Python uses indentation, no explicit block close
        is_indentation_based: true,
        extern_syntax: None,
    }
}
