// src/stmt/print.rs
//
// print(expr)

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::Env;
use crate::src_lumen::structure::structural::{LPAREN, RPAREN};

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
            Token::Ident(name) if name == "print"
        )
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        // consume `print`
        parser.advance();

        // expect '('
        match parser.advance() {
            Token::Feature(k) if k == LPAREN => {}
            _ => return Err("Expected '(' after print".into()),
        }

        let expr = parser.parse_expr()?;

        // expect ')'
        match parser.advance() {
            Token::Feature(k) if k == RPAREN => {}
            _ => return Err("Expected ')' after expression".into()),
        }

        Ok(Box::new(PrintStmt { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses "print" as identifier)

    // Register handlers
    reg.register_stmt(Box::new(PrintStmtHandler));
}
