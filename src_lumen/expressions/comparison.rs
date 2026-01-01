// Comparison operators: == != < > <= >=

use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_lumen::numeric;

// --------------------
// Token definitions
// --------------------

pub const EQ_EQ: &str = "EQ_EQ";
pub const NOT_EQ: &str = "NOT_EQ";
pub const LT: &str = "LT";
pub const GT: &str = "GT";
pub const LT_EQ: &str = "LT_EQ";
pub const GT_EQ: &str = "GT_EQ";

#[derive(Debug)]
struct ComparisonExpr {
    left: Box<dyn ExprNode>,
    op: &'static str,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ComparisonExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (l, r) {
            (Value::Number(a), Value::Number(b)) => {
                let result = match self.op {
                    EQ_EQ => {
                        let an = numeric::parse_number(&a)?;
                        let bn = numeric::parse_number(&b)?;
                        an == bn
                    }
                    NOT_EQ => {
                        let an = numeric::parse_number(&a)?;
                        let bn = numeric::parse_number(&b)?;
                        an != bn
                    }
                    LT => numeric::compare_lt(&a, &b)?,
                    GT => numeric::compare_gt(&a, &b)?,
                    LT_EQ => numeric::compare_le(&a, &b)?,
                    GT_EQ => numeric::compare_ge(&a, &b)?,
                    _ => return Err("Invalid comparison operator".into()),
                };
                Ok(Value::Bool(result))
            }
            _ => Err("Invalid comparison operands".into()),
        }
    }
}

pub struct ComparisonInfix {
    op: &'static str,
}

impl ComparisonInfix {
    pub fn new(op: &'static str) -> Self {
        Self { op }
    }
}

impl ExprInfix for ComparisonInfix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(kind) if *kind == self.op)
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
        Ok(Box::new(ComparisonExpr { left, op: self.op, right }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_two_char("==", EQ_EQ);
    reg.tokens.add_two_char("!=", NOT_EQ);
    reg.tokens.add_two_char("<=", LT_EQ);
    reg.tokens.add_two_char(">=", GT_EQ);
    reg.tokens.add_single_char('<', LT);
    reg.tokens.add_single_char('>', GT);

    // Register handlers
    reg.register_infix(Box::new(ComparisonInfix::new(EQ_EQ)));
    reg.register_infix(Box::new(ComparisonInfix::new(NOT_EQ)));
    reg.register_infix(Box::new(ComparisonInfix::new(LT)));
    reg.register_infix(Box::new(ComparisonInfix::new(GT)));
    reg.register_infix(Box::new(ComparisonInfix::new(LT_EQ)));
    reg.register_infix(Box::new(ComparisonInfix::new(GT_EQ)));
}
