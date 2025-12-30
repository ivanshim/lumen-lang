// src/expr/literals.rs
//
// Numeric literal expressions.
// This file is a fully removable language feature.

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprPrefix, LumenResult};
use crate::runtime::Value;

#[derive(Debug)]
struct NumberLiteral {
    value: f64,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut crate::runtime::Env) -> LumenResult<Value> {
        Ok(Value::Number(self.value))
    }
}

pub struct NumberLiteralPrefix;

impl ExprPrefix for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Number(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Number(n) => Ok(Box::new(NumberLiteral { value: n })),
            _ => unreachable!(),
        }
    }
}
