use crate::languages::mini_rust::prelude::*;
// Logical operators: && || !

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::mini_rust::registry::{ExprInfix, ExprPrefix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::mini_rust::values::{MiniRustBool, as_bool};

// --------------------
// Token definitions
// --------------------

pub const AND: &str = "&&";
pub const OR: &str = "||";
pub const NOT: &str = "!";

#[derive(Debug)]
struct LogicExpr {
    left: Box<dyn ExprNode>,
    op: &'static str,
    right: Box<dyn ExprNode>,
}

impl ExprNode for LogicExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        let left_bool = as_bool(l.as_ref())?;
        let right_bool = as_bool(r.as_ref())?;

        let result = match self.op {
            AND => left_bool.value && right_bool.value,
            OR => left_bool.value || right_bool.value,
            _ => return Err(format!("Invalid logical operator: {}", self.op)),
        };
        Ok(Box::new(MiniRustBool::new(result)))
    }
}

pub struct LogicInfix {
    op: &'static str,
}

impl LogicInfix {
    pub fn new(op: &'static str) -> Self {
        Self { op }
    }
}

impl ExprInfix for LogicInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.op
    }

    fn precedence(&self) -> Precedence {
        Precedence::Logic
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(LogicExpr { left, op: self.op, right }))
    }
}

// Unary NOT

#[derive(Debug)]
struct NotExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for NotExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let val = self.expr.eval(env)?;
        let b = as_bool(val.as_ref())?;
        Ok(Box::new(MiniRustBool::new(!b.value)))
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == NOT
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
        Ok(Box::new(NotExpr { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register tokens    // Register handlers
    reg.register_infix(Box::new(LogicInfix::new(AND)));
    reg.register_infix(Box::new(LogicInfix::new(OR)));
    reg.register_prefix(Box::new(NotPrefix));
}
