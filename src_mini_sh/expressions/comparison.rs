// Comparison operations
use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_mini_sh::numeric;

pub const EQ: &str = "EQ";
pub const NE: &str = "NE";
pub const LT: &str = "LT";
pub const GT: &str = "GT";
pub const LE: &str = "LE";
pub const GE: &str = "GE";

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
                    EQ => {
                        let an = numeric::parse_number(&a)?;
                        let bn = numeric::parse_number(&b)?;
                        an == bn
                    }
                    NE => {
                        let an = numeric::parse_number(&a)?;
                        let bn = numeric::parse_number(&b)?;
                        an != bn
                    }
                    LT => numeric::compare_lt(&a, &b)?,
                    GT => numeric::compare_gt(&a, &b)?,
                    LE => numeric::compare_le(&a, &b)?,
                    GE => numeric::compare_ge(&a, &b)?,
                    _ => return Err("Invalid comparison operator".into()),
                };
                Ok(Value::Bool(result))
            }
            (Value::Bool(a), Value::Bool(b)) => {
                let result = match self.op {
                    EQ => a == b,
                    NE => a != b,
                    _ => return Err("Invalid comparison for booleans".into()),
                };
                Ok(Value::Bool(result))
            }
            _ => Err("Type mismatch in comparison".into()),
        }
    }
}

pub struct ComparisonInfix {
    op: &'static str,
}
impl ComparisonInfix {
    pub fn new(op: &'static str) -> Self { Self { op } }
}
impl ExprInfix for ComparisonInfix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(kind) if *kind == self.op)
    }
    fn precedence(&self) -> Precedence { Precedence::Comparison }
    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ComparisonExpr { left, op: self.op, right }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.tokens.add_two_char("==", EQ);
    reg.tokens.add_two_char("!=", NE);
    reg.tokens.add_single_char('<', LT);
    reg.tokens.add_single_char('>', GT);
    reg.tokens.add_two_char("<=", LE);
    reg.tokens.add_two_char(">=", GE);
    reg.register_infix(Box::new(ComparisonInfix::new(EQ)));
    reg.register_infix(Box::new(ComparisonInfix::new(NE)));
    reg.register_infix(Box::new(ComparisonInfix::new(LT)));
    reg.register_infix(Box::new(ComparisonInfix::new(GT)));
    reg.register_infix(Box::new(ComparisonInfix::new(LE)));
    reg.register_infix(Box::new(ComparisonInfix::new(GE)));
}
