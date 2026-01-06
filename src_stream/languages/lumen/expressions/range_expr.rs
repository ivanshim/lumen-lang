// src/expr/range_expr.rs
//
// Range expressions: start..end
// Represents a half-open range [start, end)
//
// Returns a special value type that carries range metadata

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value, RuntimeValue};
use crate::languages::lumen::patterns::PatternSet;
use crate::languages::lumen::values::as_number;
use std::any::Any;

#[derive(Debug)]
struct RangeExpr {
    start: Box<dyn ExprNode>,
    end: Box<dyn ExprNode>,
}

/// Represents a half-open range [start, end)
#[derive(Debug, Clone)]
pub struct LumenRange {
    pub start: f64,
    pub end: f64,
}

impl LumenRange {
    pub fn new(start: f64, end: f64) -> Self {
        LumenRange { start, end }
    }
}

impl RuntimeValue for LumenRange {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Range({}, {})", self.start as i64, self.end as i64)
    }

    fn as_display_string(&self) -> String {
        format!("{}..{}", self.start as i64, self.end as i64)
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_range) = other.as_any().downcast_ref::<LumenRange>() {
            Ok(self.start == other_range.start && self.end == other_range.end)
        } else {
            Err("Cannot compare range with non-range".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ExprNode for RangeExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let start_val = self.start.eval(env)?;
        let end_val = self.end.eval(env)?;

        let start_num = as_number(start_val.as_ref())?;
        let end_num = as_number(end_val.as_ref())?;

        let start_f64 = start_num.value.parse::<f64>()
            .map_err(|_| "Failed to parse range start as number".to_string())?;
        let end_f64 = end_num.value.parse::<f64>()
            .map_err(|_| "Failed to parse range end as number".to_string())?;

        Ok(Box::new(LumenRange::new(start_f64, end_f64)))
    }
}

pub struct RangeExprHandler;

impl ExprInfix for RangeExprHandler {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == ".."
    }

    fn precedence(&self) -> Precedence {
        Precedence::Range
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume ".."
        parser.skip_tokens();
        let right = parser.parse_expr_prec(registry, self.precedence())?;
        Ok(Box::new(RangeExpr {
            start: left,
            end: right,
        }))
    }
}

// Helper to downcast a Value to LumenRange
pub fn as_range(val: &dyn RuntimeValue) -> LumenResult<&LumenRange> {
    val.as_any()
        .downcast_ref::<LumenRange>()
        .ok_or_else(|| "Expected range value".to_string())
}

// --------------------
// Pattern Declaration
// --------------------

pub fn patterns() -> PatternSet {
    PatternSet::new().with_literals(vec![".."])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    reg.register_infix(Box::new(RangeExprHandler));
}
