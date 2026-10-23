use crate::language::prelude::*;
// src/stmt/while_loop.rs
//
// while <expr>
//     <block>

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::runtime::Env;
use crate::language::structure::structural;
use crate::language::values::as_bool;

#[derive(Debug)]
struct WhileStmt {
    condition: Box<dyn ExprNode>,
    body: Vec<Box<dyn StmtNode>>,
}

impl StmtNode for WhileStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        while as_bool(self.condition.eval(env)?.as_ref())?.value {
            match crate::kernel::eval::block(&self.body, env)? {
                Control::Break => break,
                value @ Control::Return(_) => return Ok(value),
                _ => {}
            }
        }
        Ok(Control::None)
    }
}

pub struct WhileStmtHandler;

impl StmtHandler for WhileStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("stmt.while", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'while'
        parser.skip_tokens();

        // parse condition expression
        let condition = parser.parse_expr(registry)?;

        // parse indented body
        let body = structural::parse_block(parser, registry)?;

        Ok(Box::new(WhileStmt { condition, body }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "while" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(WhileStmtHandler));
}
