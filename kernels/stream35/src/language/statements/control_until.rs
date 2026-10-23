// src/stmt/until_loop.rs
//
// until <expr>
//     <block>
//
// Executes block once, then loops while expr is false
// (Stops looping when expr becomes true)
//
// Desugars into: { block; while not expr { block } }

use crate::language::prelude::*;
use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;
use crate::language::structure::structural;
use crate::language::values::as_bool;

#[derive(Debug)]
struct UntilStmt {
    condition: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for UntilStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        loop {
            match crate::kernel::eval::block(&self.body, env)? {
                Control::Break => break,
                value @ Control::Return(_) => return Ok(value),
                _ => {}
            }
            if as_bool(self.condition.eval(env)?.as_ref())?.value {
                break;
            }
        }
        Ok(Control::None)
    }
}

pub struct UntilStmtHandler;

impl StmtHandler for UntilStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.until", &parser.peek().lexeme)
    }

    fn parse(
        &self,
        parser: &mut Parser,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'until'
        parser.skip_tokens();

        // parse condition expression
        let condition = parser.parse_expr(registry)?;

        // parse indented body
        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(UntilStmt { condition, body }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_stmt(Box::new(UntilStmtHandler));
}
