use crate::languages::lumen::prelude::*;
// src/src-lumen/statements/mod.rs
// Lumen statement features

pub mod assignment;
pub mod break_stmt;
pub mod continue_stmt;
pub mod if_else;
pub mod print;
pub mod write;
pub mod while_loop;
pub mod for_loop;
pub mod until_loop;
pub mod return_stmt;
pub mod let_binding;
pub mod let_mut_binding;
pub mod functions;
pub mod memoization;
pub mod expr_stmt;

// Re-export function utilities for use by expressions
pub use functions::get_function;
