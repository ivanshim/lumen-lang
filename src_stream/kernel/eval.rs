// src/framework/eval.rs
//
// Core execution loop.
// No language semantics live here.

use crate::src_stream::kernel::ast::{Control, Program};
use crate::src_stream::kernel::runtime::env::Env;

pub fn eval(program: &Program) -> Result<(), String> {
    let mut env = Env::new();

    for stmt in &program.statements {
        match stmt.exec(&mut env)? {
            Control::None => {}
            Control::Break => break,
            Control::Continue => continue,
        }
    }

    Ok(())
}
