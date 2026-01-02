// src/stmt/assignment.rs
//
// x = expr

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
struct AssignStmt {
    name: String,
    expr: Box<dyn ExprNode>,
}

impl StmtNode for AssignStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val: Value = self.expr.eval(env)?;
        env.assign(&self.name, val)?;
        Ok(Control::None)
    }
}

pub struct AssignStmtHandler;

impl StmtHandler for AssignStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if current token is an identifier and next token is '='
        let curr = &parser.peek().lexeme;
        let is_ident = curr.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
        let next_is_eq = parser.peek_n(1).map_or(false, |t| t.lexeme == "=");
        is_ident && next_is_eq
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        let name = parser.advance().lexeme;

        if parser.advance().lexeme != "=" {
            return Err(err_at(parser, "Expected '=' in assignment"));
        }

        let expr = parser.parse_expr()?;
        Ok(Box::new(AssignStmt { name, expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses '=' single-char operator emitted automatically)
    // Register handlers
    reg.register_stmt(Box::new(AssignStmtHandler));
}
