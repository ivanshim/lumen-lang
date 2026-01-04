use crate::languages::mini_python::prelude::*;
// src/stmt/continue_stmt.rs
//
// continue statement

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_python::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct ContinueStmt;

impl StmtNode for ContinueStmt {
    fn exec(&self, _env: &mut Env) -> LumenResult<Control> {
        Ok(Control::Continue)
    }
}

pub struct ContinueStmtHandler;

impl StmtHandler for ContinueStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "continue"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'continue'
        Ok(Box::new(ContinueStmt))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "continue" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(ContinueStmtHandler));
}
