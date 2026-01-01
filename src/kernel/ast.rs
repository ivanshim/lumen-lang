// src/framework/ast.rs
//
// Minimal AST spine.
// No feature enums. No syntax knowledge.

use std::fmt;
use crate::kernel::runtime::{Env, Value};

pub struct Program {
    pub statements: Vec<Box<dyn StmtNode>>,
}

impl Program {
    pub fn new(statements: Vec<Box<dyn StmtNode>>) -> Self {
        Self { statements }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    None,
    Break,
    Continue,
}

pub trait ExprNode: fmt::Debug {
    fn eval(&self, env: &mut Env) -> Result<Value, String>;
}

pub trait StmtNode: fmt::Debug {
    fn exec(&self, env: &mut Env) -> Result<Control, String>;
}
