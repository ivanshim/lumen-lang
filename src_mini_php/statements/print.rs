// Mini-PHP: echo statement

use crate::framework::ast::{Control, ExprNode, StmtNode};
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::framework::runtime::{Env, Value};
use crate::src_mini_php::structure::structural::{LPAREN, RPAREN};

pub const ECHO: &str = "ECHO";

#[derive(Debug)]
struct EchoStmt {
    expr: Box<dyn ExprNode>,
}

impl StmtNode for EchoStmt {
    fn exec(&self, env: &mut Env) -> LumenResult<Control> {
        let value = self.expr.eval(env)?;
        match value {
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    println!("{}", n as i64);
                } else {
                    println!("{}", n);
                }
            }
            Value::Bool(b) => println!("{}", b),
        }
        Ok(Control::None)
    }
}

pub struct EchoStmtHandler;

impl StmtHandler for EchoStmtHandler {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(ECHO))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>> {
        parser.advance(); // consume 'echo'
        match parser.advance() {
            Token::Feature(LPAREN) => {}
            _ => return Err(err_at(parser, "Expected '(' after 'echo'")),
        }
        let expr = parser.parse_expr()?;
        match parser.advance() {
            Token::Feature(RPAREN) => {}
            _ => return Err(err_at(parser, "Expected ')'")),
        }
        Ok(Box::new(EchoStmt { expr }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("echo", ECHO);
    reg.register_stmt(Box::new(EchoStmtHandler));
}
