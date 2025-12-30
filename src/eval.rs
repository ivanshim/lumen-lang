// src/eval.rs
//
// Core execution loop.
// No language semantics live here.

use crate::ast::{Control, Program};
use crate::runtime::env::Env;

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
