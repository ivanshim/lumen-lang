use crate::languages::mini_python::prelude::*;
// Comparison operators: == != < > <= >=

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_python::registry::{ExprInfix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::mini_python::numeric;
use crate::languages::mini_python::values::{MiniPythonNumber, MiniPythonBool, as_number, as_bool};

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
            return Ok(Box::new(MiniPythonBool::new(result)));
        }

        match self.op.as_str() {
            "==" => {
                let result = l.eq_value(r.as_ref())?;
                Ok(Box::new(MiniPythonBool::new(result)))
            }
            "!=" => {
                let result = l.eq_value(r.as_ref())?;
                Ok(Box::new(MiniPythonBool::new(!result)))
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
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ComparisonExpr { left, op: self.op.clone(), right }))
    }
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
