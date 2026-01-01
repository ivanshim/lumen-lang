// src_mini_apple_pascal/src_mini_apple_pascal.rs
// Mini-Apple-Pascal language dispatcher
// Pascal-like: BEGIN/END blocks, := assignment, writeln

use crate::kernel::registry::Registry;

use super::expressions;
use super::statements;
use super::structure;

/// Register all mini-apple-pascal language features
pub fn register_all(registry: &mut Registry) {
    // Register multi-character lexemes for maximal-munch segmentation
    // The kernel lexer will use these for pure lossless ASCII segmentation
    registry.tokens.set_multichar_lexemes(vec![
        // Two-char operators (note: := is important for Pascal)
        ":=", "==", "!=", "<=", ">=",
        // Keywords (multi-char word sequences)
        "and", "or", "not", "if", "else", "while", "break", "continue", "writeln",
        "true", "false",
    ]);

    structure::structural::register(registry);
    expressions::literals::register(registry);
    expressions::variable::register(registry);
    expressions::identifier::register(registry);
    expressions::grouping::register(registry);
    expressions::arithmetic::register(registry);
    expressions::comparison::register(registry);
    expressions::logic::register(registry);
    statements::print::register(registry);
    statements::assignment::register(registry);
    statements::if_else::register(registry);
    statements::while_loop::register(registry);
    statements::break_stmt::register(registry);
    statements::continue_stmt::register(registry);
}
