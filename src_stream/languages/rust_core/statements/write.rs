use crate::languages::rust_core::prelude::*;
// write!() statement for mini-rust - like print but without newline

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::rust_core::registry::{Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::languages::rust_core::structure::structural::{LPAREN, RPAREN};

pub const WRITE: &str = "write";

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
        matches!(
            parser.peek(),
            _ if parser.peek().lexeme == WRITE
        )
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'write!'
        parser.skip_tokens();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after write!".into());
        }
        parser.skip_tokens();

        let expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

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
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_stmt(Box::new(WriteStmtHandler));
}
