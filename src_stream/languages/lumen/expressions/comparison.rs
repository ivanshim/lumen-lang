use crate::languages::lumen::prelude::*;
// Comparison operators: == != < > <= >=

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::patterns::PatternSet;
use crate::kernel::registry::LumenResult;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::registry::{ExprInfix, Precedence, Registry};
use crate::languages::lumen::numeric;
use crate::languages::lumen::values::{as_number, as_string, LumenBool};

#[derive(Debug)]
struct ComparisonExpr {
    left: Box<dyn ExprNode>,
    op: String,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ComparisonExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        // Try numeric comparison first
        if let (Ok(left_num), Ok(right_num)) = (as_number(l.as_ref()), as_number(r.as_ref())) {
            let result = match self.op.as_str() {
                "==" => {
                    let an = numeric::parse_number(&left_num.value)?;
                    let bn = numeric::parse_number(&right_num.value)?;
                    an == bn
                }
                "!=" => {
                    let an = numeric::parse_number(&left_num.value)?;
                    let bn = numeric::parse_number(&right_num.value)?;
                    an != bn
                }
                "<" => numeric::compare_lt(&left_num.value, &right_num.value)?,
                ">" => numeric::compare_gt(&left_num.value, &right_num.value)?,
                "<=" => numeric::compare_le(&left_num.value, &right_num.value)?,
                ">=" => numeric::compare_ge(&left_num.value, &right_num.value)?,
                _ => return Err("Invalid comparison operator".into()),
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Try string comparison
        if let (Ok(left_str), Ok(right_str)) = (as_string(l.as_ref()), as_string(r.as_ref())) {
            let result = match self.op.as_str() {
                "==" => left_str.value == right_str.value,
                "!=" => left_str.value != right_str.value,
                _ => return Err("String comparison only supports == and !=".into()),
            };
            return Ok(Box::new(LumenBool::new(result)));
        }

        // Use the trait method for generic equality (handles Bool-Bool, etc.)
        match self.op.as_str() {
            "==" => {
                let result = l.eq_value(r.as_ref())?;
                Ok(Box::new(LumenBool::new(result)))
            }
            "!=" => {
                let result = l.eq_value(r.as_ref())?;
                Ok(Box::new(LumenBool::new(!result)))
            }
            _ => Err("Invalid comparison operands".into()),
        }
    }
}

pub struct ComparisonInfix {
    op: String,
}

impl ComparisonInfix {
    pub fn new(op: &str) -> Self {
        Self { op: op.to_string() }
    }
}

impl ExprInfix for ComparisonInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.op
    }

    fn precedence(&self) -> Precedence {
        Precedence::Comparison
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        parser.skip_whitespace();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ComparisonExpr { left, op: self.op.clone(), right }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["==", "!=", "<", ">", "<=", ">="])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_infix(Box::new(ComparisonInfix::new("==")));
    reg.register_infix(Box::new(ComparisonInfix::new("!=")));
    reg.register_infix(Box::new(ComparisonInfix::new("<")));
    reg.register_infix(Box::new(ComparisonInfix::new(">")));
    reg.register_infix(Box::new(ComparisonInfix::new("<=")));
    reg.register_infix(Box::new(ComparisonInfix::new(">=")));
}
