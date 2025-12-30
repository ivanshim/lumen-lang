// src/stmt/print.rs

use crate::ast::{Control, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{err_at, LumenResult, StmtHandler};
use crate::runtime::Env;

#[derive(Debug)]
struct PrintStmt {
    expr: Box<dyn crate::ast::ExprNode>,
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
        matches!(parser.peek(), Token::Print)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'print'

        match parser.advance() {
            Token::LParen => {}
            _ => return Err(err_at(parser, "Expected '(' after print")),
        }

        let expr = parser.parse_expr()?;

        match parser.advance() {
            Token::RParen => {}
            _ => return Err(err_at(parser, "Expected ')' after expression")),
        }

        Ok(Box::new(PrintStmt { expr }))
    }
}
