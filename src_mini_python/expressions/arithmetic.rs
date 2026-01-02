// src/expr/arithmetic.rs
//
// + - * / % and unary minus

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_mini_python::numeric;

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

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
        parser.peek().lexeme == "-"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '-'
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
        Ok(Box::new(UnaryMinusExpr { expr }))
    }
}

#[derive(Debug)]
struct ArithmeticExpr {
    left: Box<dyn ExprNode>,
    op: String,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ArithmeticExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (l, r) {
            (Value::Number(a), Value::Number(b)) => {
                let result = match self.op.as_str() {
                    "+" => numeric::add(&a, &b)?,
                    "-" => numeric::subtract(&a, &b)?,
                    "*" => numeric::multiply(&a, &b)?,
                    "/" => numeric::divide(&a, &b)?,
                    "%" => numeric::modulo(&a, &b)?,
                    _ => return Err("Invalid arithmetic operator".into()),
                };
                Ok(Value::Number(result))
            }
            _ => Err("Invalid operands for arithmetic operation".into()),
        }
    }
}

pub struct ArithmeticInfix {
    op: String,
    prec: Precedence,
}

impl ArithmeticInfix {
    pub fn new(op: &str, prec: Precedence) -> Self {
        Self { op: op.to_string(), prec }
    }
}

impl ExprInfix for ArithmeticInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.op
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op.clone(), right }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_prefix(Box::new(UnaryMinusPrefix));
    reg.register_infix(Box::new(ArithmeticInfix::new("+", Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new("-", Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new("*", Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new("/", Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new("%", Precedence::Factor)));
}
