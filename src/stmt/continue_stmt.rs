// src/stmt/continue_stmt.rs
//
// continue statement

use crate::ast::{Control, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, StmtHandler};
use crate::runtime::Env;

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
        matches!(parser.peek(), Token::Ident(s) if s == "continue")
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'continue'
        Ok(Box::new(ContinueStmt))
    }
}
