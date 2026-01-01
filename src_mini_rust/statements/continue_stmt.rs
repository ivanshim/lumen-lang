// continue statement for mini-rust

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;

pub const CONTINUE: &str = "CONTINUE";

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
        matches!(parser.peek(), Token::Feature(CONTINUE))
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
    // Register tokens
    reg.tokens.add_keyword("continue", CONTINUE);

    // Register handlers
    reg.register_stmt(Box::new(ContinueStmtHandler));
}
