use crate::languages::mini_rust::prelude::*;
// print!() statement for mini-rust

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_rust::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::mini_rust::structure::structural::{LPAREN, RPAREN};

pub const PRINT: &str = "print";

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
        matches!(
            parser.peek(),
            _ if parser.peek().lexeme == PRINT
        )
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'print!'
        parser.skip_whitespace();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after print!".into());
        }
        parser.skip_whitespace();

        let expr = parser.parse_expr(registry)?;
        parser.skip_whitespace();

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
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_stmt(Box::new(PrintStmtHandler));
}
