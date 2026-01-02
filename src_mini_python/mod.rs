// src/src_mini_python/mod.rs
// Mini-Python language module
// Python-like language implementation with indentation-based blocks

pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod structure;

pub fn register_all(registry: &mut crate::kernel::registry::Registry) {
    // Register multi-character lexemes for maximal-munch segmentation
    registry.tokens.set_multichar_lexemes(vec![
        // Two-char operators
        "==", "!=", "<=", ">=",
        // Keywords (multi-char word sequences)
        "and", "or", "not",
        "if", "else", "while", "break", "continue", "print",
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

    // Statement features
    statements::print::register(registry);         // print() statement
    statements::assignment::register(registry);    // Assignment
    statements::if_else::register(registry);       // if/else statements
    statements::while_loop::register(registry);    // while loops
    statements::break_stmt::register(registry);    // break statement
    statements::continue_stmt::register(registry); // continue statement
}
