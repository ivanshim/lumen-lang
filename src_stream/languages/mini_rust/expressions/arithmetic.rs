use crate::languages::mini_rust::prelude::*;
// Arithmetic operators: + - * / % and unary minus

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_rust::registry::{ExprInfix, ExprPrefix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::mini_rust::numeric;
use crate::languages::mini_rust::values::{MiniRustNumber, as_number};

// --------------------
// Token definitions
// --------------------

pub const PLUS: &str = "+";
pub const MINUS: &str = "-";
pub const STAR: &str = "*";
pub const SLASH: &str = "/";
pub const PERCENT: &str = "%";

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let val = self.expr.eval(env)?;
        let num = as_number(val.as_ref())?;
        let result = numeric::negate(&num.value)?;
        Ok(Box::new(MiniRustNumber::new(result)))
    }
}

pub struct UnaryMinusPrefix;

impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == MINUS
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '-'
        let expr = parser.parse_expr_prec(registry, Precedence::Unary)?;
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

        let left_num = as_number(l.as_ref())?;
        let right_num = as_number(r.as_ref())?;

        let result = match self.op {
            PLUS => numeric::add(&left_num.value, &right_num.value)?,
            MINUS => numeric::subtract(&left_num.value, &right_num.value)?,
            STAR => numeric::multiply(&left_num.value, &right_num.value)?,
            SLASH => numeric::divide(&left_num.value, &right_num.value)?,
            PERCENT => numeric::modulo(&left_num.value, &right_num.value)?,
            _ => return Err("Invalid arithmetic operator".into()),
        };
        Ok(Box::new(MiniRustNumber::new(result)))
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

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        let right = parser.parse_expr_prec(registry, self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op, right }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register tokens
    // Register handlers
    reg.register_prefix(Box::new(UnaryMinusPrefix));
    reg.register_infix(Box::new(ArithmeticInfix::new(PLUS, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(MINUS, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(STAR, Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new(SLASH, Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new(PERCENT, Precedence::Factor)));
}
