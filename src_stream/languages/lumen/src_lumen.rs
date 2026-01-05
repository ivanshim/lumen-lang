// src/src-lumen/src-lumen.rs
// Lumen language dispatcher
// This module registers all Lumen language features with the Lumen registry

use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::registry::TokenDefinition;
use super::registry::Registry;

// Import all feature modules
use super::expressions;
use super::statements;
use super::structure;

/// Aggregate all patterns from all Lumen modules
pub fn aggregate_patterns() -> PatternSet {
    let patterns_list = vec![
        // Structural patterns
        structure::structural::patterns(),

        // Expression patterns
        expressions::literals::patterns(),
        expressions::variable::patterns(),
        expressions::identifier::patterns(),
        expressions::grouping::patterns(),
        expressions::arithmetic::patterns(),
        expressions::comparison::patterns(),
        expressions::logic::patterns(),
        expressions::extern_expr::patterns(),
        expressions::pipe::patterns(),

        // Statement patterns
        statements::print::patterns(),
        statements::let_mut_binding::patterns(),
        statements::let_binding::patterns(),
        statements::assignment::patterns(),
        statements::if_else::patterns(),
        statements::while_loop::patterns(),
        statements::break_stmt::patterns(),
        statements::continue_stmt::patterns(),
        statements::return_stmt::patterns(),
        statements::fn_definition::patterns(),
    ];

    PatternSet::merge(patterns_list)
}

/// Register all Lumen language features
pub fn register_all(registry: &mut Registry) {
    // Define all tokens with unified TokenDefinition API
    // Each token specifies whether it should be skipped during parsing
    let tokens = vec![
        // Two-char operators (not skipped)
        TokenDefinition::recognize("=="),
        TokenDefinition::recognize("!="),
        TokenDefinition::recognize("<="),
        TokenDefinition::recognize(">="),
        TokenDefinition::recognize("**"),

        // Keywords (not skipped)
        TokenDefinition::recognize("let"),
        TokenDefinition::recognize("mut"),
        TokenDefinition::recognize("and"),
        TokenDefinition::recognize("or"),
        TokenDefinition::recognize("not"),
        TokenDefinition::recognize("if"),
        TokenDefinition::recognize("else"),
        TokenDefinition::recognize("while"),
        TokenDefinition::recognize("break"),
        TokenDefinition::recognize("continue"),
        TokenDefinition::recognize("return"),
        TokenDefinition::recognize("fn"),
        TokenDefinition::recognize("|>"),
        TokenDefinition::recognize("print"),
        TokenDefinition::recognize("extern"),  // Impurity boundary marker
        TokenDefinition::recognize("true"),
        TokenDefinition::recognize("false"),
        TokenDefinition::recognize("none"),
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
    expressions::extern_expr::register(registry);   // extern impurity boundary
    expressions::pipe::register(registry);          // Pipe operator

    // Statement features
    statements::print::register(registry);         // print() statement
    statements::let_mut_binding::register(registry); // let mut binding
    statements::let_binding::register(registry);   // let binding
    statements::assignment::register(registry);    // Assignment
    statements::if_else::register(registry);       // if/else statements
    statements::while_loop::register(registry);    // while loops
    statements::break_stmt::register(registry);    // break statement
    statements::continue_stmt::register(registry); // continue statement
    statements::return_stmt::register(registry);   // return statement
    statements::fn_definition::register(registry); // function definition
}
