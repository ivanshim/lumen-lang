use crate::language::prelude::*;
// Array indexing expression: arr[i], with the brackets the definition spells.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};
use crate::language::values::{as_array, as_string, LumenString};

#[derive(Debug)]
pub struct ArrayIndex {
    pub array_expr: Box<dyn ExprNode>,
    pub index_expr: Box<dyn ExprNode>,
}

impl ExprNode for ArrayIndex {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let array_val = self.array_expr.eval(env)?;
        let index_val = self.index_expr.eval(env)?;

        // Get the index as an integer
        let index_bigint = crate::language::values::as_number(index_val.as_ref())?;
        let (sign, digits) = index_bigint.value.to_u32_digits();

        // Check for negative index
        use num_bigint::Sign;
        if sign == Sign::Minus {
            return Err("Array index cannot be negative".to_string());
        }

        // Get the index value (0 if digits is empty, otherwise digits[0])
        let idx = if digits.is_empty() {
            0usize
        } else if digits.len() == 1 {
            digits[0] as usize
        } else {
            return Err("Array index out of bounds".to_string());
        };

        // A string, where the definition lets strings be indexed, yields its character
        if def().index_strings {
            if let Ok(s) = as_string(array_val.as_ref()) {
                return match s.value.chars().nth(idx) {
                    Some(c) => Ok(Box::new(LumenString::new(c.to_string()))),
                    None => Err("String index out of bounds".to_string()),
                };
            }
        }

        let arr = as_array(array_val.as_ref())?;
        if idx >= arr.elements.len() {
            return Err(format!("Array index out of bounds"));
        }

        Ok(arr.elements[idx].clone_boxed())
    }
}

pub struct ArrayIndexInfix;

impl ExprInfix for ArrayIndexInfix {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("op.index.open", &parser.peek().lexeme)
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '['
        parser.skip_tokens();

        let index_expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        if !def().is("op.index.close", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after array index", def().first("op.index.close")));
        }

        Ok(Box::new(ArrayIndex {
            array_expr: left,
            index_expr,
        }))
    }

    fn precedence(&self) -> Precedence {
        Precedence::postfix()
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_infix(Box::new(ArrayIndexInfix));
}
