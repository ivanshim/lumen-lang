// Lumen language schema (declarative data only)
//
// This file defines the Lumen language entirely as data.
// The kernel interprets these tables to parse and execute Lumen code.

use crate::schema::{
    LanguageSchema, StatementPattern, StatementRole, OperatorInfo, UnaryOperatorInfo,
    Associativity, ExternSyntax, PatternBuilder,
};
use std::collections::HashMap;

/// Build the complete Lumen language schema
pub fn get_schema() -> LanguageSchema {
    let mut statements = HashMap::new();

    // print statement: print(expression)
    statements.insert(
        "print".to_string(),
        PatternBuilder::new("print")
            .keyword("print_keyword")
            .literal("(", "lparen")
            .expression("value")
            .literal(")", "rparen")
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

    // Note: The pattern system as currently designed handles linear sequences.
    // For "if ... else block" we need an extended pattern that says:
    // if the optional "else" was present, parse another block.
    // For now, we'll handle this in the parser with special logic for conditional blocks.
    // In a full implementation, we'd have a pattern variant for optional paired blocks.

    // while statement: while condition { block }
    statements.insert(
        "while".to_string(),
        PatternBuilder::new("while")
            .keyword("while_keyword")
            .expression("condition")
            .block("body")
            .build(),
    );

    // var statement: var name = expression
    statements.insert(
        "var".to_string(),
        PatternBuilder::new("var")
            .keyword("var_keyword")
            .expression("name")  // identifier will be parsed as variable expr
            .literal("=", "assign_op")
            .expression("value")
            .build(),
    );

    // break statement: break
    statements.insert(
        "break".to_string(),
        PatternBuilder::new("break")
            .keyword("break_keyword")
            .build(),
    );

    // continue statement: continue
    statements.insert(
        "continue".to_string(),
        PatternBuilder::new("continue")
            .keyword("continue_keyword")
            .build(),
    );

    // let statement: let [mut] name [: Type] = expression
    statements.insert(
        "let".to_string(),
        PatternBuilder::new("let")
            .keyword("let_keyword")
            .expression("binding")  // Includes mut and identifier
            .build(),
    );

    // return statement: return [expression]
    statements.insert(
        "return".to_string(),
        PatternBuilder::new("return")
            .keyword("return_keyword")
            .build(),
    );

    // Binary operators with precedence and associativity
    let mut binary_ops = HashMap::new();

    // Assignment (lowest precedence, right-associative)
    binary_ops.insert(
        "=".to_string(),
        OperatorInfo::right("=", 0),
    );

    // Logical OR
    binary_ops.insert(
        "or".to_string(),
        OperatorInfo::left("or", 2),
    );

    // Logical AND
    binary_ops.insert(
        "and".to_string(),
        OperatorInfo::left("and", 3),
    );

    // Comparison operators
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

    // Arithmetic operators
    binary_ops.insert(
        "+".to_string(),
        OperatorInfo::left("+", 5),
    );
    binary_ops.insert(
        "-".to_string(),
        OperatorInfo::left("-", 5),
    );
    binary_ops.insert(
        "*".to_string(),
        OperatorInfo::left("*", 6),
    );
    binary_ops.insert(
        "/".to_string(),
        OperatorInfo::left("/", 6),
    );
    binary_ops.insert(
        "%".to_string(),
        OperatorInfo::left("%", 6),
    );
    binary_ops.insert(
        "**".to_string(),
        OperatorInfo::right("**", 7),  // Exponentiation: right-associative, higher precedence
    );

    // Pipe operator (very low precedence, right-associative)
    binary_ops.insert(
        "|>".to_string(),
        OperatorInfo::right("|>", 1),
    );

    // Unary operators
    let mut unary_ops = HashMap::new();

    // Logical NOT (prefix)
    unary_ops.insert(
        "not".to_string(),
        UnaryOperatorInfo::prefix("not", 7),
    );

    // Arithmetic negation (prefix)
    unary_ops.insert(
        "-".to_string(),
        UnaryOperatorInfo::prefix("-", 7),
    );

    // All keywords
    let keywords = vec![
        "print".to_string(),
        "if".to_string(),
        "else".to_string(),
        "while".to_string(),
        "var".to_string(),
        "let".to_string(),
        "mut".to_string(),
        "break".to_string(),
        "continue".to_string(),
        "return".to_string(),
        "fn".to_string(),
        "true".to_string(),
        "false".to_string(),
        "none".to_string(),
        "and".to_string(),
        "or".to_string(),
        "not".to_string(),
        "extern".to_string(),
    ];

    // Statement terminators
    let statement_terminators = vec![
        ";".to_string(),
        "\n".to_string(),
    ];

    // Multi-character lexemes for maximal-munch
    let multichar_lexemes = vec![
        // Keywords (longer first for maximal-munch)
        "continue", "extern", "return",
        "while", "break", "false", "print",
        "true", "else", "false", "let", "var", "not", "and", "if", "or", "fn", "mut", "none",
        // Operators
        "|>", "**",  // Pipe and exponentiation (2 chars)
        "==", "!=", "<=", ">=",  // comparison operators
    ];

    // Extern call syntax
    let extern_syntax = Some(ExternSyntax {
        keyword: "extern".to_string(),
        selector_start: "\"".to_string(),
        selector_end: "\"".to_string(),
        args_open: "(".to_string(),
        args_close: ")".to_string(),
        arg_separator: ",".to_string(),
    });

    LanguageSchema {
        statements,
        binary_ops,
        unary_ops,
        keywords,
        multichar_lexemes,
        statement_terminators,
        block_open: "{".to_string(),
        block_close: "}".to_string(),
        is_indentation_based: true,
        extern_syntax,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_completeness() {
        let schema = get_schema();

        // Verify all expected statements are present
        assert!(schema.is_statement_keyword("print"));
        assert!(schema.is_statement_keyword("if"));
        assert!(schema.is_statement_keyword("while"));
        assert!(schema.is_statement_keyword("var"));
        assert!(schema.is_statement_keyword("let"));
        assert!(schema.is_statement_keyword("return"));
        assert!(schema.is_statement_keyword("break"));
        assert!(schema.is_statement_keyword("continue"));

        // Verify all expected operators are present
        assert!(schema.get_binary_op("+").is_some());
        assert!(schema.get_binary_op("*").is_some());
        assert!(schema.get_binary_op("**").is_some());
        assert!(schema.get_binary_op("|>").is_some());
        assert!(schema.get_unary_op("not").is_some());

        // Verify precedence ordering
        assert!(schema.get_binary_op("+").unwrap().precedence <
                schema.get_binary_op("*").unwrap().precedence);
        assert!(schema.get_binary_op("**").unwrap().precedence >
                schema.get_binary_op("*").unwrap().precedence);
        assert!(schema.get_binary_op("|>").unwrap().precedence <
                schema.get_binary_op("+").unwrap().precedence);
    }

    #[test]
    fn test_keywords_reserved() {
        let schema = get_schema();
        assert!(schema.is_keyword("if"));
        assert!(schema.is_keyword("print"));
        assert!(!schema.is_keyword("myvar"));
    }
}
