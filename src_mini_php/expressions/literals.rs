// Mini-PHP: Number and boolean literals

use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};

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

pub fn register(reg: &mut Registry) {
    reg.tokens.add_keyword("true", TRUE);
    reg.tokens.add_keyword("false", FALSE);
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
}
