// A no-op statement, Python's `pass`. Registered only when the definition
// spells one.

use crate::language::prelude::*;
use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;

#[derive(Debug)]
struct PassStmt;

impl StmtNode for PassStmt {
    fn exec(&self, _env: &mut Env) -> LumenResult<Control> {
        Ok(Control::None)
    }
}

pub struct PassStmtHandler;

impl StmtHandler for PassStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.pass", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance();
        Ok(Box::new(PassStmt))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(PassStmtHandler));
}
