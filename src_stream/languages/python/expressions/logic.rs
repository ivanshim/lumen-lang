use crate::languages::python::prelude::*;
// Logical operators: and / or / not

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::python::registry::{ExprInfix, ExprPrefix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::python::values::{PythonBool, as_bool};

#[derive(Debug)]
struct LogicExpr {
    left: Box<dyn ExprNode>,
    op: String,
    right: Box<dyn ExprNode>,
}

impl ExprNode for LogicExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        let left_bool = as_bool(l.as_ref())?;
        let right_bool = as_bool(r.as_ref())?;

        let result = match self.op.as_str() {
            "and" => left_bool.value && right_bool.value,
            "or" => left_bool.value || right_bool.value,
            _ => return Err(format!("Invalid logical operator: {}", self.op)),
        };
        Ok(Box::new(PythonBool::new(result)))
    }
}

pub struct LogicInfix {
    op: String,
}

impl LogicInfix {
    pub fn new(op: &str) -> Self {
        Self { op: op.to_string() }
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
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        let right = parser.parse_expr_prec(registry, self.precedence() + 1)?;
        Ok(Box::new(LogicExpr { left, op: self.op.clone(), right }))
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
        Ok(Box::new(PythonBool::new(!b.value)))
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "not"
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr_prec(registry, Precedence::Unary)?;
        Ok(Box::new(NotExpr { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // Register handlers
    reg.register_infix(Box::new(LogicInfix::new("and")));
    reg.register_infix(Box::new(LogicInfix::new("or")));
    reg.register_prefix(Box::new(NotPrefix));
}
