// A bare block as a statement, for languages whose main program is one
// (Pascal's `begin ... end.`). Matches only in the braces style, where a
// block opener can begin a statement; runs its statements in a new scope.

use crate::language::prelude::*;
use crate::kernel::ast::{Control, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;
use crate::language::definition::BlockStyle;
use crate::language::structure::structural;

#[derive(Debug)]
struct BlockStmt {
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for BlockStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        env.with_scope(|env| {
            for stmt in &self.body {
                match stmt.exec(env)? {
                    Control::None | Control::ExprValue(_) => {}
                    other => return Ok(other),
                }
            }
            Ok(Control::None)
        })
    }
}

pub struct BlockStmtHandler;

impl StmtHandler for BlockStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().block_style == BlockStyle::Braces && def().is("block.open", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        let body = structural::parse_block(parser, registry)?;
        Ok(Box::new(BlockStmt { body }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(BlockStmtHandler));
}
