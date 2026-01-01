// break statement
use crate::framework::ast::{{Control, StmtNode}};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{{LumenResult, Registry, StmtHandler}};
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
        parser.advance();
        Ok(Box::new(BreakStmt))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("break", BREAK);
    reg.register_stmt(Box::new(BreakStmtHandler));
}
