// src_mini_rust/src_mini_rust.rs
// Mini-Rust language dispatcher
// Rust-like: let bindings, curly braces, print! macro

use crate::kernel::registry::LumenResult;
use crate::languages::mini_rust::registry::Registry;

// Import all feature modules
use super::expressions;
use super::statements;
use super::structure;

/// Register all Mini-Rust language features
pub fn register_all(registry: &mut Registry) {
    // Register multi-character lexemes for maximal-munch segmentation
    // The kernel lexer will use these for pure lossless ASCII segmentation
    registry.tokens.set_multichar_lexemes(vec![
        // Two-char operators
        "==", "!=", "<=", ">=", "&&", "||", ":=",
        // Keywords (multi-char word sequences)
        "let", "if", "else", "while", "break", "continue", "print",
        "true", "false",
    ]);

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
    statements::let_binding::register(registry);    // let x = expr; (must be before assignment)
    statements::assignment::register(registry);     // var = expr;
    statements::if_else::register(registry);        // if/else statements
    statements::while_loop::register(registry);     // while loops
    statements::break_stmt::register(registry);     // break statement
    statements::continue_stmt::register(registry);  // continue statement
}
