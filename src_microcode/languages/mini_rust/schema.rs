// Mini-Rust language schema (declarative data only)
//
// This file defines the Mini-Rust language entirely as data.
// The kernel interprets these tables to parse and execute Mini-Rust code.

use crate::schema::{
    LanguageSchema, StatementPattern, StatementRole, OperatorInfo, UnaryOperatorInfo,
    Associativity, ExternSyntax, PatternBuilder,
};
use std::collections::HashMap;

/// Build the complete Mini-Rust language schema
pub fn get_schema() -> LanguageSchema {
    let mut statements = HashMap::new();

    // print! macro: print!(expression)
    statements.insert(
        "print".to_string(),
        PatternBuilder::new("print")
            .keyword("print_keyword")
            .literal("!", "bang")
            .literal("(", "lparen")
            .expression("value")
            .literal(")", "rparen")
            .build(),
    );

    // let binding: let name = expression;
    statements.insert(
        "let".to_string(),
        PatternBuilder::new("let")
            .keyword("let_keyword")
            .expression("name")  // identifier parsed as variable expr
            .literal("=", "assign_op")
            .expression("value")
            .literal(";", "semicolon")
            .build(),
    );

    // let mut binding: let mut name = expression;
    statements.insert(
        "let_mut".to_string(),
        PatternBuilder::new("let_mut")
            .keyword("let_keyword")
            .keyword("mut_keyword")
            .expression("name")
            .literal("=", "assign_op")
            .expression("value")
            .literal(";", "semicolon")
            .build(),
    );

    // assignment: name = expression;
    statements.insert(
        "assign".to_string(),
        PatternBuilder::new("assign")
            .expression("name")  // identifier
            .literal("=", "assign_op")
            .expression("value")
            .literal(";", "semicolon")
            .build(),
    );

    // if statement: if condition { block } [else { block }]
    statements.insert(
        "if".to_string(),
        PatternBuilder::new("if")
            .keyword("if_keyword")
            .expression("condition")
            .block("then_block")
            .optional_literal("else", "else_keyword")
            .build(),
    );

    // while loop: while condition { block }
    statements.insert(
        "while".to_string(),
        PatternBuilder::new("while")
            .keyword("while_keyword")
            .expression("condition")
            .block("body")
            .build(),
    );

    // for loop: for name in expression { block }
    statements.insert(
        "for".to_string(),
        PatternBuilder::new("for")
            .keyword("for_keyword")
            .expression("name")  // loop variable
            .keyword("in_keyword")
            .expression("iterable")
            .block("body")
            .build(),
    );

    // loop statement: loop { block } (infinite loop)
    statements.insert(
        "loop".to_string(),
        PatternBuilder::new("loop")
            .keyword("loop_keyword")
            .block("body")
            .build(),
    );

    // break statement: break;
    statements.insert(
        "break".to_string(),
        PatternBuilder::new("break")
            .keyword("break_keyword")
            .literal(";", "semicolon")
            .build(),
    );

    // continue statement: continue;
    statements.insert(
        "continue".to_string(),
        PatternBuilder::new("continue")
            .keyword("continue_keyword")
            .literal(";", "semicolon")
            .build(),
    );

    // return statement: return [expression];
    statements.insert(
        "return".to_string(),
        PatternBuilder::new("return")
            .keyword("return_keyword")
            .expression("value")  // optional return value
            .literal(";", "semicolon")
            .build(),
    );

    // Binary operators with precedence and associativity
    let mut binary_ops = HashMap::new();

    // Assignment (precedence 0, right-associative)
    binary_ops.insert(
        "=".to_string(),
        OperatorInfo::right("=", 0),
    );

    // Logical OR (precedence 1, left-associative)
    binary_ops.insert(
        "||".to_string(),
        OperatorInfo::left("||", 1),
    );

    // Logical AND (precedence 2, left-associative)
    binary_ops.insert(
        "&&".to_string(),
        OperatorInfo::left("&&", 2),
    );

    // Bitwise OR (precedence 3, left-associative)
    binary_ops.insert(
        "|".to_string(),
        OperatorInfo::left("|", 3),
    );

    // Bitwise XOR (precedence 4, left-associative)
    binary_ops.insert(
        "^".to_string(),
        OperatorInfo::left("^", 4),
    );

    // Bitwise AND (precedence 5, left-associative)
    binary_ops.insert(
        "&".to_string(),
        OperatorInfo::left("&", 5),
    );

    // Comparison: ==, !=, <, >, <=, >= (precedence 6, left-associative, non-chaining)
    binary_ops.insert(
        "==".to_string(),
        OperatorInfo::none("==", 6),
    );

    binary_ops.insert(
        "!=".to_string(),
        OperatorInfo::none("!=", 6),
    );

    binary_ops.insert(
        "<".to_string(),
        OperatorInfo::none("<", 6),
    );

    binary_ops.insert(
        ">".to_string(),
        OperatorInfo::none(">", 6),
    );

    binary_ops.insert(
        "<=".to_string(),
        OperatorInfo::none("<=", 6),
    );

    binary_ops.insert(
        ">=".to_string(),
        OperatorInfo::none(">=", 6),
    );

    // Range (precedence 7, right-associative)
    binary_ops.insert(
        "..".to_string(),
        OperatorInfo::right("..", 7),
    );

    // Shift (precedence 8, left-associative)
    binary_ops.insert(
        "<<".to_string(),
        OperatorInfo::left("<<", 8),
    );

    binary_ops.insert(
        ">>".to_string(),
        OperatorInfo::left(">>", 8),
    );

    // Additive: + - (precedence 9, left-associative)
    binary_ops.insert(
        "+".to_string(),
        OperatorInfo::left("+", 9),
    );

    binary_ops.insert(
        "-".to_string(),
        OperatorInfo::left("-", 9),
    );

    // Multiplicative: * / % (precedence 10, left-associative)
    binary_ops.insert(
        "*".to_string(),
        OperatorInfo::left("*", 10),
    );

    binary_ops.insert(
        "/".to_string(),
        OperatorInfo::left("/", 10),
    );

    binary_ops.insert(
        "%".to_string(),
        OperatorInfo::left("%", 10),
    );

    // Unary operators with precedence
    let mut unary_ops = HashMap::new();

    // Logical NOT (precedence 11, prefix)
    unary_ops.insert(
        "!".to_string(),
        UnaryOperatorInfo::prefix("!", 11),
    );

    // Negation (precedence 11, prefix)
    unary_ops.insert(
        "-".to_string(),
        UnaryOperatorInfo::prefix("-", 11),
    );

    // Bitwise NOT (precedence 11, prefix)
    // Note: Rust uses ! for bitwise NOT, but we use it for logical NOT above
    // In a full implementation, we'd need context to disambiguate

    // Reference (precedence 12, prefix)
    unary_ops.insert(
        "&".to_string(),
        UnaryOperatorInfo::prefix("&", 12),
    );

    // Dereference (precedence 12, prefix)
    unary_ops.insert(
        "*".to_string(),
        UnaryOperatorInfo::prefix("*", 12),
    );

    // Keywords list
    let keywords = vec![
        "let".to_string(),
        "mut".to_string(),
        "if".to_string(),
        "else".to_string(),
        "while".to_string(),
        "for".to_string(),
        "in".to_string(),
        "loop".to_string(),
        "break".to_string(),
        "continue".to_string(),
        "return".to_string(),
        "fn".to_string(),
        "true".to_string(),
        "false".to_string(),
        "println".to_string(),
        "print".to_string(),
        "match".to_string(),
        "as".to_string(),
    ];

    // Multi-character lexemes (sorted by length descending for maximal-munch)
    let multichar_lexemes = vec![
        // Keywords (longer first for maximal-munch)
        "println", "continue", "return", "false", "while", "break", "match",
        "let", "mut", "print", "loop", "else", "true", "if", "for", "in", "fn", "as",
        // Operators
        "==", "!=", "<=", ">=", "<<", ">>",
        "&&", "||", "->", "=>", "::", "+=", "-=", "*=", "/=", "..",
    ];

    // Statement terminators
    let statement_terminators = vec![
        ";".to_string(),
    ];

    // Construct and return the complete schema
    LanguageSchema {
        statements,
        binary_ops,
        unary_ops,
        keywords,
        multichar_lexemes,
        statement_terminators,
        block_open: "{".to_string(),
        block_close: "}".to_string(),
        is_indentation_based: false,
        extern_syntax: None,  // Mini-Rust doesn't support extern in this implementation
    }
}
