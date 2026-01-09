use crate::languages::python_core::prelude::*;
// src/stmt/write.rs
//
// write(expr) - like print but without newline

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::python_core::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::python_core::structure::structural::{LPAREN, RPAREN};

#[derive(Debug)]
struct WriteStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for WriteStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        print!("{val}");
        Ok(Control::None)
    }
}

pub struct WriteStmtHandler;

impl StmtHandler for WriteStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "write"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // consume `write`
        parser.advance();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after write".into());
        }

        let expr = parser.parse_expr(registry)?;

        // expect ')'
        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after expression".into());
        }

        Ok(Box::new(WriteStmt { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "write" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(WriteStmtHandler));
}
