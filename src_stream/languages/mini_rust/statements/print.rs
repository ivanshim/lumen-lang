// print!() statement for mini-rust

use crate::src_stream::kernel::ast::{Control, ExprNode, StmtNode};
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::src_stream::kernel::runtime::Env;
use crate::src_stream::src_mini_rust::structure::structural::{LPAREN, RPAREN};

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

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'print!'

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after print!".into());
        }

        let expr = parser.parse_expr()?;

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
