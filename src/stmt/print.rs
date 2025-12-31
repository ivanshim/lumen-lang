// src/stmt/print.rs
//
// print(expr)

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, Registry, StmtHandler};
use crate::runtime::Env;
use crate::syntax::structural::{LPAREN, RPAREN};

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
            Token::Feature(LPAREN) => {}
            _ => return Err("Expected '(' after print".into()),
        }

        let expr = parser.parse_expr()?;

        // expect ')'
        match parser.advance() {
            Token::Feature(RPAREN) => {}
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
