// src/framework/eval.rs
//
// Core execution loop.
// No language semantics live here.

use crate::kernel::ast::{Control, Program};
use crate::kernel::runtime::env::Env;
use crate::kernel::exec_options::StreamExecutionOptions;

/// Execute a program with default execution options (all optimizations disabled).
/// This is the default entry point - behavior is unchanged from original stream kernel.
pub fn eval(program: &Program) -> Result<(), String> {
    eval_with_options(program, &StreamExecutionOptions::new())
}

/// Execute a program with explicit execution options.
/// This allows enabling optional optimizations like memoization.
///
/// # Parameters
/// - `program`: The parsed program to execute
/// - `options`: Execution options controlling optional features
///
/// # Example
/// ```ignore
/// let options = StreamExecutionOptions::with_memoization();
/// eval_with_options(program, &options)?;
/// ```
pub fn eval_with_options(program: &Program, options: &StreamExecutionOptions) -> Result<(), String> {
    // Create environment with appropriate configuration based on options
    let mut env = if options.enable_memoization {
        Env::with_memoization()
    } else {
        Env::new()
    };

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
