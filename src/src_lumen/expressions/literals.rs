// Number and boolean literals

use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{ExprPrefix, LumenResult, Registry};
use crate::framework::runtime::{Env, Value};

// --------------------
// Token definitions
// --------------------

pub const TRUE: &str = "TRUE";
pub const FALSE: &str = "FALSE";

#[derive(Debug)]
pub struct NumberLiteral {
    pub value: f64,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
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

// Boolean literals

#[derive(Debug)]
struct BoolLiteral {
    value: bool,
}

impl ExprNode for BoolLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Value::Bool(self.value))
    }
}

pub struct BoolLiteralPrefix;

impl ExprPrefix for BoolLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(TRUE) | Token::Feature(FALSE))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        match parser.advance() {
            Token::Feature(TRUE) => Ok(Box::new(BoolLiteral { value: true })),
            Token::Feature(FALSE) => Ok(Box::new(BoolLiteral { value: false })),
            _ => unreachable!(),
        }
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_keyword("true", TRUE);
    reg.tokens.add_keyword("false", FALSE);

    // Register handlers
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
}
