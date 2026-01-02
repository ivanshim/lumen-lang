// Arithmetic operations
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_mini_apple_pascal::numeric;

pub const PLUS: &str = "+";
pub const MINUS: &str = "-";
pub const STAR: &str = "*";
pub const SLASH: &str = "/";
pub const PERCENT: &str = "%";

#[derive(Debug)]
struct UnaryMinusExpr { expr: Box<dyn ExprNode> }
impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        match self.expr.eval(env)? {
            Value::Number(s) => {
                let result = numeric::negate(&s)?;
                Ok(Value::Number(result))
            }
            _ => Err("Invalid operand for unary '-'".into()),
        }
    }
}

pub struct UnaryMinusPrefix;
impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == MINUS
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
        Ok(Box::new(UnaryMinusExpr { expr }))
    }
}

#[derive(Debug)]
struct ArithmeticExpr {
    left: Box<dyn ExprNode>,
    op: &'static str,
    right: Box<dyn ExprNode>,
}
impl ExprNode for ArithmeticExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;
        match (l, r) {
            (Value::Number(a), Value::Number(b)) => {
                let result = match self.op {
                    PLUS => numeric::add(&a, &b)?,
                    MINUS => numeric::subtract(&a, &b)?,
                    STAR => numeric::multiply(&a, &b)?,
                    SLASH => numeric::divide(&a, &b)?,
                    PERCENT => numeric::modulo(&a, &b)?,
                    _ => return Err("Invalid arithmetic operator".into()),
                };
                Ok(Value::Number(result))
            }
            _ => Err("Invalid operands for arithmetic operation".into()),
        }
    }
}

pub struct ArithmeticInfix {
    op: &'static str,
    prec: Precedence,
}
impl ArithmeticInfix {
    pub fn new(op: &'static str, prec: Precedence) -> Self {
        Self { op, prec }
    }
}
impl ExprInfix for ArithmeticInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.op
    }
    fn precedence(&self) -> Precedence { self.prec }
    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op, right }))
    }
}

pub fn register(reg: &mut Registry) {    reg.register_prefix(Box::new(UnaryMinusPrefix));
    reg.register_infix(Box::new(ArithmeticInfix::new(PLUS, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(MINUS, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(STAR, Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new(SLASH, Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new(PERCENT, Precedence::Factor)));
}
