// src_rust/src_rust.rs
// Mini-RustCore language dispatcher
// RustCore-like: let bindings, curly braces, print! macro

use crate::kernel::registry::{KernelResult as LumenResult, TokenDefinition};
use crate::languages::rust_core::registry::Registry;

// Import all feature modules
use super::expressions;
use super::statements;
use super::structure;

/// Register all Mini-RustCore language features
pub fn register_all(registry: &mut Registry) {
    // Define all tokens with unified TokenDefinition API
    // Each token specifies whether it should be skipped during parsing
    let tokens = vec![
        // Two-char operators (not skipped)
        TokenDefinition::recognize("=="),
        TokenDefinition::recognize("!="),
        TokenDefinition::recognize("<="),
        TokenDefinition::recognize(">="),
        TokenDefinition::recognize("&&"),
        TokenDefinition::recognize("||"),
        TokenDefinition::recognize(":="),

        // Keywords (not skipped)
        TokenDefinition::keyword("let"),
        TokenDefinition::keyword("if"),
        TokenDefinition::keyword("else"),
        TokenDefinition::keyword("while"),
        TokenDefinition::keyword("break"),
        TokenDefinition::keyword("continue"),
        TokenDefinition::keyword("print"),
        TokenDefinition::keyword("write"),
        TokenDefinition::keyword("true"),
        TokenDefinition::keyword("false"),
    ];

    registry.tokens.set_token_definitions(tokens);
    registry.tokens.set_identifier_bytes(crate::languages::ascii_word_byte);

    // Core syntax (structural tokens - braces, parens, semicolons)
    structure::structural::register(registry);

    // Expression features
    expressions::literals::register(registry);      // Number and boolean literals
    expressions::variable::register(registry);      // Variable references
    expressions::identifier::register(registry);    // Identifier handling
    expressions::grouping::register(registry);      // Parenthesized expressions
    expressions::arithmetic::register(registry);    // Arithmetic operators
    expressions::comparison::register(registry);    // Comparison operators
    expressions::logic::register(registry);         // Logical operators

    // Statement features
    statements::print::register(registry);          // print! statement
    statements::write::register(registry);          // write! statement (print without newline)
    statements::let_binding::register(registry);    // let x = expr; (must be before assignment)
    statements::assignment::register(registry);     // var = expr;
    statements::if_else::register(registry);        // if/else statements
    statements::while_loop::register(registry);     // while loops
    statements::break_stmt::register(registry);     // break statement
    statements::continue_stmt::register(registry);  // continue statement
}
