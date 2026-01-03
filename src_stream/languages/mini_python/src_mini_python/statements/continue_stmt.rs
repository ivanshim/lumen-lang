// src/stmt/continue_stmt.rs
//
// continue statement

use crate::src_stream::kernel::ast::{Control, StmtNode};
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::src_stream::kernel::runtime::Env;

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

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
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
