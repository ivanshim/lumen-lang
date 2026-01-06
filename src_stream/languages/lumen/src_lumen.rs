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
        statements::expr_stmt::patterns(),
    ];

    PatternSet::merge(patterns_list)
}

/// Register all Lumen language features
pub fn register_all(registry: &mut Registry) {
    // Define all tokens with unified TokenDefinition API
    // Keywords use boundary-aware registration to avoid splitting identifiers that contain
    // keyword substrings.
    let tokens = vec![
        // Two-char operators (not skipped)
        TokenDefinition::recognize("=="),
        TokenDefinition::recognize("!="),
        TokenDefinition::recognize("<="),
        TokenDefinition::recognize(">="),
        TokenDefinition::recognize("**"),

        // Keywords that require word boundaries (to prevent matching inside identifiers like "test_let" or "no_return")
        // NOTE: "and", "or", "not" are NOT registered to prevent breaking identifiers like "factorial"
        // NOTE: "true", "false", "extern" are NOT registered as they have their own expression handlers
        TokenDefinition::keyword("let"),
        TokenDefinition::keyword("mut"),
        // "and", "or", "not" - now registered with word boundaries to prevent breaking identifiers like "android", "north", "notation"
        TokenDefinition::keyword("and"),
        TokenDefinition::keyword("or"),
        TokenDefinition::keyword("not"),
        TokenDefinition::keyword("if"),
        TokenDefinition::keyword("else"),
        TokenDefinition::keyword("while"),
        TokenDefinition::keyword("break"),
        TokenDefinition::keyword("continue"),
        TokenDefinition::keyword("return"),
        TokenDefinition::keyword("fn"),
        TokenDefinition::recognize("|>"),  // Pipe operator, not a keyword
        TokenDefinition::keyword("print"),
        // "extern" is NOT registered
        // "true" and "false" are NOT registered
        TokenDefinition::keyword("none"),
    ];

    registry.tokens.set_token_definitions(tokens);

    // Core syntax (structural tokens - parentheses, indentation, etc.)
    structure::structural::register(registry);

    // Expression features
    // NOTE: Registration order matters - earlier registrations have higher priority
    // Special expressions (literals, extern) must come before generic variable matching
    expressions::literals::register(registry);      // Number and boolean literals
    expressions::extern_expr::register(registry);   // extern impurity boundary (before variable)
    expressions::variable::register(registry);      // Variable references (generic identifier matching)
    expressions::grouping::register(registry);      // Parenthesized expressions
    expressions::arithmetic::register(registry);    // Arithmetic operators
    expressions::comparison::register(registry);    // Comparison operators
    expressions::logic::register(registry);         // Logical operators
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
    statements::expr_stmt::register(registry);     // expression statements (fallback handler)
}
