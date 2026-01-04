// src/expr/arithmetic.rs
//
// + - * / % and unary minus

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::patterns::PatternSet;
use crate::kernel::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::numeric;
use crate::languages::lumen::values::{LumenNumber, as_number};

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let val = self.expr.eval(env)?;
        let num = as_number(val.as_ref())?;
        let result = numeric::negate(&num.value)?;
        Ok(Box::new(LumenNumber::new(result)))
    }
}

pub struct UnaryMinusPrefix;

impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "-"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '-'
        parser.skip_whitespace();
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

        let left_num = as_number(l.as_ref())?;
        let right_num = as_number(r.as_ref())?;

        let result = match self.op.as_str() {
            "+" => numeric::add(&left_num.value, &right_num.value)?,
            "-" => numeric::subtract(&left_num.value, &right_num.value)?,
            "*" => numeric::multiply(&left_num.value, &right_num.value)?,
            "/" => numeric::divide(&left_num.value, &right_num.value)?,
            "%" => numeric::modulo(&left_num.value, &right_num.value)?,
            _ => return Err("Invalid arithmetic operator".into()),
        };
        Ok(Box::new(LumenNumber::new(result)))
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
        parser.skip_whitespace();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op: self.op.clone(), right }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["+", "-", "*", "/", "%"])
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
