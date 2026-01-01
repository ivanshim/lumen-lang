// Variable reference expression

use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{LumenResult, ExprPrefix};
use crate::kernel::runtime::{Env, Value};

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
