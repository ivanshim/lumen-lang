use crate::language::prelude::*;
// Array literals, with the brackets and separator the definition spells.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};
use crate::language::values::LumenArray;

#[derive(Debug)]
pub struct ArrayLiteral {
    pub elements: Vec<Box<dyn ExprNode>>,
}

impl ExprNode for ArrayLiteral {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let mut values = Vec::new();
        for elem in &self.elements {
            values.push(elem.eval(env)?);
        }
        Ok(Box::new(LumenArray::new(values)))
    }
}

pub struct ArrayLiteralPrefix;

impl ExprPrefix for ArrayLiteralPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("syntax.array.open", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let d = def();
        parser.advance(); // opening bracket
        parser.skip_tokens();

        let mut elements = Vec::new();

        // Parse array elements until the closing bracket
        while !d.is("syntax.array.close", &parser.peek().lexeme) {
            let elem = parser.parse_expr(registry)?;
            elements.push(elem);
            parser.skip_tokens();

            // Check for separator or closing bracket
            if d.is("syntax.array.separator", &parser.peek().lexeme) {
                parser.advance();
                parser.skip_tokens();

                // Allow a trailing separator before the closing bracket
                if d.is("syntax.array.close", &parser.peek().lexeme) {
                    break;
                }
            } else if !d.is("syntax.array.close", &parser.peek().lexeme) {
                return Err(format!(
                    "Expected '{}' or '{}' in array literal, got '{}'",
                    d.first("syntax.array.separator"),
                    d.first("syntax.array.close"),
                    parser.peek().lexeme
                ));
            }
        }

        if !d.is("syntax.array.close", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' to close array literal", d.first("syntax.array.close")));
        }

        Ok(Box::new(ArrayLiteral { elements }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(ArrayLiteralPrefix));
}
