// Variable reference expression

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, PrefixHandler};
use crate::runtime::{Env, Value};

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

impl PrefixHandler for IdentPrefix {
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
