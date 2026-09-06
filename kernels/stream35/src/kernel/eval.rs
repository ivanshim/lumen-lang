// kernel/eval.rs
//
// Core execution loop.
// No language semantics live here.

use crate::kernel::ast::{Control, Program};
use crate::kernel::runtime::env::Env;

/// Execute a program statement by statement and hand back the environment,
/// so the language may act on what the program defined.
/// `init_fn` lets the language seed the environment (system values such as ARGS).
pub fn eval<F>(program: &Program, init_fn: F) -> Result<Env, String>
where
    F: FnOnce(&mut Env) -> Result<(), String>,
{
    let mut env = Env::new();

    // Initialize system values (ARGS, etc.) via language-specific callback
    init_fn(&mut env)?;

    for stmt in &program.statements {
        match stmt.exec(&mut env)? {
            Control::None => {}
            Control::ExprValue(_) => {
                // Expression statement value - ignore at top level and continue
            }
            Control::Break => break,
            Control::Continue => continue,
            Control::Return(_) => {
                // Explicit return at top level - stop execution
                break;
            }
        }
    }

    Ok(env)
}
