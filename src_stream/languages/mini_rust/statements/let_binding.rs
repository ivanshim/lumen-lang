// let binding statement for mini-rust

use crate::src_stream::src_stream::kernel::ast::{Control, ExprNode, StmtNode};
use crate::src_stream::src_stream::kernel::parser::Parser;
use crate::src_stream::src_stream::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::src_stream::src_stream::kernel::runtime::{Env, Value};

// --------------------
// Token definitions
// --------------------

pub const LET: &str = "let";
pub const EQUALS: &str = "=";

#[derive(Debug)]
struct LetStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for LetStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val: Value = self.expr.eval(env)?;
        env.set(self.name.clone(), val);
        Ok(Control::None)
    }
}

pub struct LetStmtHandler;

impl StmtHandler for LetStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == LET
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'let'

        let name = parser.advance().lexeme;

        if parser.advance().lexeme != EQUALS {
            return Err(err_at(parser, "Expected '=' after identifier"));
        }

        let expr = parser.parse_expr()?;
        Ok(Box::new(LetStmt { name, expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register tokens    // EQUALS is registered in assignment.rs

    // Register handlers
    reg.register_stmt(Box::new(LetStmtHandler));
}
