// src/expr/variable.rs
//
// Variable reference expression: `x`

use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{ExprPrefix, LumenResult, Registry};
use crate::framework::runtime::{Env, Value};

#[derive(Debug)]
struct VarExpr {
    name: String,
}

impl ExprNode for VarExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Ident(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Ident(name) => Ok(Box::new(VarExpr { name })),
            _ => unreachable!(),
        }
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (uses Token::Ident from lexer)

    // Register handlers
    reg.register_prefix(Box::new(VariablePrefix));
}
