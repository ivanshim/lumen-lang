// src_mini_python/src_mini_python.rs
// Mini-Python language dispatcher
// Python-like: indentation-based blocks, no braces

use crate::kernel::registry::{LumenResult, TokenDefinition};
use crate::languages::mini_python::registry::Registry;

// Import all feature modules
use super::expressions;
use super::statements;
use super::structure;

/// Register all Mini-Python language features
pub fn register_all(registry: &mut Registry) {
    // Define all tokens with unified TokenDefinition API
    // Each token specifies whether it should be skipped during parsing
    let tokens = vec![
        // Two-char operators (not skipped)
        TokenDefinition::recognize("=="),
        TokenDefinition::recognize("!="),
        TokenDefinition::recognize("<="),
        TokenDefinition::recognize(">="),

        // Keywords (not skipped)
        TokenDefinition::recognize("and"),
        TokenDefinition::recognize("or"),
        TokenDefinition::recognize("not"),
        TokenDefinition::recognize("if"),
        TokenDefinition::recognize("else"),
        TokenDefinition::recognize("while"),
        TokenDefinition::recognize("break"),
        TokenDefinition::recognize("continue"),
        TokenDefinition::recognize("print"),
        TokenDefinition::recognize("true"),
        TokenDefinition::recognize("false"),
    ];

    registry.tokens.set_token_definitions(tokens);

    // Core syntax (structural tokens - parentheses, indentation, etc.)
    structure::structural::register(registry);

    // Expression features
    expressions::literals::register(registry);      // Number and boolean literals
    expressions::variable::register(registry);      // Variable references
    expressions::grouping::register(registry);      // Parenthesized expressions
    expressions::arithmetic::register(registry);    // Arithmetic operators
    expressions::comparison::register(registry);    // Comparison operators
    expressions::logic::register(registry);         // Logical operators

    // Statement features
    statements::print::register(registry);         // print() statement
    statements::assignment::register(registry);    // Assignment
    statements::if_else::register(registry);       // if/else statements
    statements::while_loop::register(registry);    // while loops
    statements::break_stmt::register(registry);    // break statement
    statements::continue_stmt::register(registry); // continue statement
}
