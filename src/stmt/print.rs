// src/stmt/print.rs
//
// print(expr) statement.
// Fully removable language feature.

use crate::ast::{Control, ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, StmtHandler};
use crate::runtime::Env;

#[derive(Debug)]
struct PrintStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for PrintStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let value = self.expr.eval(env)?;
        println!("{value}");
        Ok(Control::None)
    }
}

pub struct PrintStmtHandler;

impl StmtHandler for PrintStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(name) if name == "print")
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        // consume 'print'
        parser.advance();

        // expect '('
        match parser.advance() {
            Token::LParen => {}
            _ => return Err(parser.error("Expected '(' after print")),
        }

        let expr = parser.parse_expr()?;

        // expect ')'
        match parser.advance() {
            Token::RParen => {}
            _ => return Err(parser.error("Expected ')' after expression")),
        }

        Ok(Box::new(PrintStmt { expr }))
    }
}
