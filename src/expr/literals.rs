use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::registry::{LumenResult, PrefixHandler};
use crate::runtime::Value;
use crate::parser::Parser;

#[derive(Debug)]
pub struct NumberLiteral {
    value: f64,
}

impl ExprNode for NumberLiteral {
    fn eval(&self, _env: &mut crate::runtime::Env) -> Result<Value, String> {
        Ok(Value::Number(self.value))
    }
}

pub struct NumberLiteralPrefix;

impl PrefixHandler for NumberLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Number(_))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        if let Token::Number(n) = parser.advance() {
            Ok(Box::new(NumberLiteral { value: n }))
        } else {
            unreachable!()
        }
    }
}
