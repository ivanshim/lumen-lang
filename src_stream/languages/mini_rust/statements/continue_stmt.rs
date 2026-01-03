// continue statement for mini-rust

use crate::src_stream::src_stream::kernel::ast::{Control, StmtNode};
use crate::src_stream::src_stream::kernel::parser::Parser;
use crate::src_stream::src_stream::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::src_stream::src_stream::kernel::runtime::Env;

pub const CONTINUE: &str = "continue";

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
        parser.peek().lexeme == CONTINUE
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
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_stmt(Box::new(ContinueStmtHandler));
}
