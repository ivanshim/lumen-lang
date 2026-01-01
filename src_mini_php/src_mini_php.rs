// src_mini_php/src_mini_php.rs
// Mini-PHP language dispatcher
// PHP-like: $ variables, echo, semicolons

use crate::framework::registry::Registry;

// Import all feature modules
use super::expressions;
use super::statements;
use super::structure;

/// Register all Mini-PHP language features
pub fn register_all(registry: &mut Registry) {
    // Core syntax (structural tokens - braces, semicolons, etc.)
    structure::structural::register(registry);

    // Expression features
    expressions::literals::register(registry);      // Number and boolean literals
    expressions::variable::register(registry);      // $variable references
    expressions::identifier::register(registry);    // Identifier handling
    expressions::grouping::register(registry);      // Parenthesized expressions
    expressions::arithmetic::register(registry);    // Arithmetic operators
    expressions::comparison::register(registry);    // Comparison operators
    expressions::logic::register(registry);         // Logical operators

    // Statement features
    statements::print::register(registry);         // echo statement
    statements::assignment::register(registry);    // $var = expr;
    statements::if_else::register(registry);       // if/else statements
    statements::while_loop::register(registry);    // while loops
    statements::break_stmt::register(registry);    // break statement
    statements::continue_stmt::register(registry); // continue statement
}
