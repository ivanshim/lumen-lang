// Number and boolean literals

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};

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
        // Check if lexeme starts with a digit
        parser.peek().lexeme.chars().next().map_or(false, |c| c.is_ascii_digit())
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        let lexeme = parser.advance().lexeme;
        Ok(Box::new(NumberLiteral { value: lexeme }))
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
        let lex = &parser.peek().lexeme;
        lex == "true" || lex == "false"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        let lexeme = parser.advance().lexeme;
        let value = lexeme == "true";
        Ok(Box::new(BoolLiteral { value }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_prefix(Box::new(NumberLiteralPrefix));
    reg.register_prefix(Box::new(BoolLiteralPrefix));
}
