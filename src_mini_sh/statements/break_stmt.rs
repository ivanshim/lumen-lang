// break statement
use crate::kernel::ast::{{Control, StmtNode}};
use crate::kernel::parser::Parser;
use crate::kernel::registry::{{LumenResult, Registry, StmtHandler}};
use crate::kernel::runtime::Env;

pub const BREAK: &str = "break";

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
        parser.peek().lexeme == BREAK
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance();
        Ok(Box::new(BreakStmt))
    }
}

pub fn register(reg: &mut Registry) {    reg.register_stmt(Box::new(BreakStmtHandler));
}
