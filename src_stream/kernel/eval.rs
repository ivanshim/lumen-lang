// src/framework/eval.rs
//
// Core execution loop.
// No language semantics live here.

use crate::kernel::ast::{Control, Program};
use crate::kernel::runtime::env::Env;

/// Execute a program.
/// The environment includes a memoization cache that is always present.
/// Only functions explicitly marked as memoizable use the cache (matching microcode kernel design).
/// init_fn: callback to initialize the environment with language-specific system values (like ARGS)
pub fn eval<F>(program: &Program, init_fn: F) -> Result<(), String>
where
    F: FnOnce(&mut Env) -> Result<(), String>,
{
    let mut env = Env::new();

    // Initialize system values (ARGS, etc.) via language-specific callback
    init_fn(&mut env)?;

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
