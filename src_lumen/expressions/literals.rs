// Number, boolean, and string literals

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_lumen::values::{LumenNumber, LumenBool, LumenString};

#[derive(Debug)]
pub struct NumberLiteral {
    pub value: String,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        Ok(Box::new(LumenNumber::new(self.value.clone())))
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
        Ok(Box::new(LumenBool::new(self.value)))
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

// String literals

#[derive(Debug)]
struct StringLiteral {
    value: String,
}

impl ExprNode for StringLiteral {
    fn eval(&self, _env: &mut Env) -> LumenResult<Value> {
        // Remove quotes from the tokenized string: "hello" -> hello
        let content = &self.value[1..self.value.len() - 1];
        Ok(Box::new(LumenString::new(content.to_string())))
    }
}

pub struct StringLiteralPrefix;

impl ExprPrefix for StringLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme starts with a double quote
        parser.peek().lexeme.starts_with('"')
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        let lexeme = parser.advance().lexeme;
        Ok(Box::new(StringLiteral { value: lexeme }))
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
    reg.register_prefix(Box::new(StringLiteralPrefix));
}
