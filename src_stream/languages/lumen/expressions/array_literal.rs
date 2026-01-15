use crate::languages::lumen::prelude::*;
// Array literals: [ ... ]

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::languages::lumen::structure::structural::{LBRACKET, RBRACKET};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::LumenArray;

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
        parser.peek().lexeme == LBRACKET
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '['
        parser.skip_tokens();

        let mut elements = Vec::new();

        // Parse array elements until we hit ']'
        while parser.peek().lexeme != RBRACKET {
            // Parse one element
            let elem = parser.parse_expr(registry)?;
            elements.push(elem);
            parser.skip_tokens();

            // Check for comma separator or closing bracket
            if parser.peek().lexeme == "," {
                parser.advance(); // consume ','
                parser.skip_tokens();

                // Allow trailing comma before closing bracket
                if parser.peek().lexeme == RBRACKET {
                    break;
                }
            } else if parser.peek().lexeme != RBRACKET {
                return Err(format!(
                    "Expected ',' or ']' in array literal, got '{}'",
                    parser.peek().lexeme
                ));
            }
        }

        if parser.advance().lexeme != RBRACKET {
            return Err("Expected ']' to close array literal".into());
        }

        Ok(Box::new(ArrayLiteral { elements }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["[", "]", ","])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed (brackets are single-char lexemes emitted automatically)
    // Register handler
    reg.register_prefix(Box::new(ArrayLiteralPrefix));
}
