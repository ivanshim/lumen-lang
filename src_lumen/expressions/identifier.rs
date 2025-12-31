// Variable reference expression

use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{LumenResult, ExprPrefix};
use crate::framework::runtime::{Env, Value};

#[derive(Debug)]
struct IdentExpr {
    name: String,
}

impl ExprNode for IdentExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

pub struct IdentPrefix;

impl ExprPrefix for IdentPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(IdentExpr { name })),
            _ => unreachable!(),
        }
    }
}
