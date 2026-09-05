use crate::languages::python_core::prelude::*;
// src/stmt/print.rs
//
// print(expr)

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::python_core::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::python_core::structure::structural::{LPAREN, RPAREN};

#[derive(Debug)]
struct PrintStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for PrintStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let val = self.expr.eval(env)?;
        println!("{val}");
        Ok(Control::None)
    }
}

pub struct PrintStmtHandler;

impl StmtHandler for PrintStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "print"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        // consume `print`
        parser.advance();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after print".into());
        }

        let expr = parser.parse_expr(registry)?;

        // expect ')'
        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after expression".into());
        }

        Ok(Box::new(PrintStmt { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "print" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(PrintStmtHandler));
}
