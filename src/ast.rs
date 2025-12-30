// src/ast.rs
//
// Minimal AST spine.
// This file must remain stable as features are added or removed.
//
// There are NO per-feature enums here.
// All semantics live behind trait objects.

use std::fmt;

use crate::runtime::{Env, Value};

/// A complete Lumen program: a linear list of statements.
pub struct Program {
    pub statements: Vec<Box<dyn StmtNode>>,
}

impl Program {
    pub fn new(statements: Vec<Box<dyn StmtNode>>) -> Self {
        Self { statements }
    }
}

/// Control-flow signals used by statement execution.
///
/// This is intentionally minimal and closed.
/// Features may *emit* these, but may not extend them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    None,
    Break,
    Continue,
}

/// Trait implemented by all expression AST nodes.
pub trait ExprNode: fmt::Debug {
    /// Evaluate the expression and return a runtime value.
    fn eval(&self, env: &mut Env) -> Result<Value, String>;
}

/// Trait implemented by all statement AST nodes.
pub trait StmtNode: fmt::Debug {
    /// Execute the statement.
    ///
    /// Returns a control signal so the core executor
    /// can handle loops without knowing feature semantics.
    fn exec(&self, env: &mut Env) -> Result<Control, String>;
}
