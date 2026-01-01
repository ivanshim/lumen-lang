// break statement for mini-rust

use crate::framework::ast::{Control, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{LumenResult, Registry, StmtHandler};
use crate::framework::runtime::Env;

pub const BREAK: &str = "BREAK";

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
        matches!(parser.peek(), Token::Feature(BREAK))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'break'
        Ok(Box::new(BreakStmt))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_keyword("break", BREAK);

    // Register handlers
    reg.register_stmt(Box::new(BreakStmtHandler));
}
