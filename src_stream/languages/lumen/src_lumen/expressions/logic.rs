// Logical operators: and / or / not

use crate::src_stream::kernel::ast::ExprNode;
use crate::src_stream::kernel::parser::Parser;
use crate::src_stream::kernel::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::src_stream::kernel::runtime::{Env, Value};
use crate::src_lumen::values::{LumenBool, as_bool};

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
        Ok(Box::new(LumenBool::new(result)))
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
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume operator
        parser.skip_whitespace();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
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
        Ok(Box::new(LumenBool::new(!b.value)))
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "not"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        parser.skip_whitespace();
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
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
