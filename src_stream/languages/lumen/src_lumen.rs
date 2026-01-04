// src/src-lumen/src-lumen.rs
// Lumen language dispatcher
// This module registers all Lumen language features with the framework registry

use crate::kernel::registry::Registry;

// Import all feature modules
use super::expressions;
use super::statements;
use super::structure;

/// Register all Lumen language features
pub fn register_all(registry: &mut Registry) {
    // Register multi-character lexemes for maximal-munch segmentation
    // The kernel lexer will use these for pure lossless ASCII segmentation
    registry.tokens.set_multichar_lexemes(vec![
        // Two-char operators
        "==", "!=", "<=", ">=",
        // Keywords (multi-char word sequences)
        "and", "or", "not",
        "if", "else", "while", "break", "continue", "print",
        "extern",  // Impurity boundary marker
        "true", "false",
    ]);

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

    // Statement features
    statements::print::register(registry);         // print() statement
    statements::assignment::register(registry);    // Assignment
    statements::if_else::register(registry);       // if/else statements
    statements::while_loop::register(registry);    // while loops
    statements::break_stmt::register(registry);    // break statement
    statements::continue_stmt::register(registry); // continue statement
}
