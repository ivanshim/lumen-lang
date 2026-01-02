// Mini-PHP: Number and boolean literals

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};

pub const TRUE: &str = "true";
pub const FALSE: &str = "false";

#[derive(Debug)]
pub struct NumberLiteral {
    pub value: String,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Value::Number(self.value.clone()))
    }
}

pub struct NumberLiteralPrefix;

impl ExprPrefix for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        let value = parser.advance().lexeme;
        Ok(Box::new(NumberLiteral { value }))
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
        (parser.peek().lexeme == "true" || parser.peek().lexeme == "false")
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        { let value = parser.advance().lexeme == "true"; Ok(Box::new(BoolLiteral { value })) }
    }
}

pub fn register(reg: &mut Registry) {    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
}
