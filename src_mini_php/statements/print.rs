// Mini-PHP: echo statement

use crate::kernel::ast::{Control, ExprNode, StmtNode};
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, LumenResult, Registry, StmtHandler};
use crate::kernel::runtime::{Env, Value};
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
            Value::Number(s) => println!("{}", s),
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
