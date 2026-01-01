// Mini-PHP: break statement

use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;

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

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("break", BREAK);
    reg.register_stmt(Box::new(BreakStmtHandler));
}
