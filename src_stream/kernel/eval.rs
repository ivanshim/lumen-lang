// src/framework/eval.rs
//
// Core execution loop.
// No language semantics live here.

use crate::kernel::ast::{Control, Program};
use crate::kernel::runtime::env::Env;

/// Execute a program.
/// The environment includes a memoization cache that is always present.
/// Only functions explicitly marked as memoizable use the cache (matching microcode kernel design).
pub fn eval(program: &Program) -> Result<(), String> {
    let mut env = Env::new();

    for stmt in &program.statements {
        match stmt.exec(&mut env)? {
            Control::None => {}
            Control::Break => break,
            Control::Continue => continue,
            Control::Return(_) => {
                // Return at top level (outside function) - just stop execution
                break;
            }
        }
    }

    Ok(())
}
