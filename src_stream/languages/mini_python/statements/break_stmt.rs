use crate::languages::mini_python::prelude::*;
// src/stmt/break_stmt.rs
//
// break statement

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_python::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct BreakStmt;

impl StmtNode for BreakStmt {
    fn exec(&self, _env: &mut Env) -> LumenResult<Control> {
        Ok(Control::Break)
    }
}

pub struct BreakStmtHandler;

impl StmtHandler for BreakStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "break"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'break'
        Ok(Box::new(BreakStmt))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "break" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(BreakStmtHandler));
}
