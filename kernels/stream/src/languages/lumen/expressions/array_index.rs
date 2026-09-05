use crate::languages::lumen::prelude::*;
// Array indexing expression: arr[i]

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::languages::lumen::structure::structural::LBRACKET;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::as_array;

#[derive(Debug)]
pub struct ArrayIndex {
    pub array_expr: Box<dyn ExprNode>,
    pub index_expr: Box<dyn ExprNode>,
}

impl ExprNode for ArrayIndex {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let array_val = self.array_expr.eval(env)?;
        let index_val = self.index_expr.eval(env)?;

        // Get the array
        let arr = as_array(array_val.as_ref())?;

        // Get the index as an integer
        let index_bigint = crate::languages::lumen::values::as_number(index_val.as_ref())?;
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

        if idx >= arr.elements.len() {
            return Err(format!("Array index out of bounds"));
        }

        Ok(arr.elements[idx].clone_boxed())
    }
}

pub struct ArrayIndexInfix;

impl ExprInfix for ArrayIndexInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == LBRACKET
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

        if parser.advance().lexeme != "]" {
            return Err("Expected ']' after array index".into());
        }

        Ok(Box::new(ArrayIndex {
            array_expr: left,
            index_expr,
        }))
    }

    fn precedence(&self) -> Precedence {
        Precedence::Call
    }
}

// --------------------
// Pattern Declaration
// --------------------

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["[", "]"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_infix(Box::new(ArrayIndexInfix));
}
