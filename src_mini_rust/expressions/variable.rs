// Variable reference expressions

use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{ExprPrefix, LumenResult, Registry};
use crate::framework::runtime::{Env, Value};

#[derive(Debug)]
pub struct VariableExpr {
    pub name: String,
}

impl ExprNode for VariableExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name).map_err(|_| format!("Undefined variable: {}", self.name))
    }
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VariableExpr { name })),
            _ => unreachable!(),
        }
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (identifiers are lexer primitives)

    // Register handlers
    reg.register_prefix(Box::new(VariablePrefix));
}
