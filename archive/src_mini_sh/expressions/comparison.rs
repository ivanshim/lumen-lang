// Comparison operators: == != < > <= >=

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_mini_sh::numeric;

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

        match (l, r) {
            (Value::Number(a), Value::Number(b)) => {
                let result = match self.op.as_str() {
                    "==" => {
                        let an = numeric::parse_number(&a)?;
                        let bn = numeric::parse_number(&b)?;
                        an == bn
                    }
                    "!=" => {
                        let an = numeric::parse_number(&a)?;
                        let bn = numeric::parse_number(&b)?;
                        an != bn
                    }
                    "<" => numeric::compare_lt(&a, &b)?,
                    ">" => numeric::compare_gt(&a, &b)?,
                    "<=" => numeric::compare_le(&a, &b)?,
                    ">=" => numeric::compare_ge(&a, &b)?,
                    _ => return Err("Invalid comparison operator".into()),
                };
                Ok(Value::Bool(result))
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
