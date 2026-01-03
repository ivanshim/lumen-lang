// src/stmt/print.rs
//
// print(expr)

use crate::src_stream::src_stream::kernel::ast::{Control, ExprNode, StmtNode};
use crate::src_stream::src_stream::kernel::parser::Parser;
use crate::src_stream::src_stream::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::src_stream::src_stream::kernel::runtime::Env;
use crate::src_stream::src_lumen::structure::structural::{LPAREN, RPAREN};

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

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        // consume `print`
        parser.advance();
        parser.skip_whitespace();

        // expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after print".into());
        }
        parser.skip_whitespace();

        let expr = parser.parse_expr()?;
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
    // No tokens to register (uses "print" keyword registered in dispatcher)
    // Register handlers
    reg.register_stmt(Box::new(PrintStmtHandler));
}
