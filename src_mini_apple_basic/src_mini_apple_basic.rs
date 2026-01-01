// src_mini_apple_basic/src_mini_apple_basic.rs
// mini-apple-basic language dispatcher

use crate::framework::registry::Registry;

use super::expressions;
use super::statements;
use super::structure;

/// Register all mini-apple-basic language features
pub fn register_all(registry: &mut Registry) {
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
