// Logical operators: and / or / not

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};

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

        match (&l, &r) {
            (Value::Bool(a), Value::Bool(b)) => {
                let result = match self.op.as_str() {
                    "and" => *a && *b,
                    "or" => *a || *b,
                    _ => return Err(format!("Invalid logical operator: {}", self.op)),
                };
                Ok(Value::Bool(result))
            }
            _ => Err(format!("Invalid logical operation: {:?} {} {:?}", l, self.op, r)),
        }
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
        match self.expr.eval(env)? {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => Err("Invalid operand for 'not'".into()),
        }
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "not"
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
    // Register handlers
    reg.register_infix(Box::new(LogicInfix::new("and")));
    reg.register_infix(Box::new(LogicInfix::new("or")));
    reg.register_prefix(Box::new(NotPrefix));
}
