// Logical operators: and / or / not

use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};

// --------------------
// Token definitions
// --------------------

pub const AND: &str = "AND";
pub const OR: &str = "OR";
pub const NOT: &str = "NOT";

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

        match (&l, &r) {
            (Value::Bool(a), Value::Bool(b)) => {
                let result = match self.op {
                    AND => *a && *b,
                    OR => *a || *b,
                    _ => return Err(format!("Invalid logical operator: {}", self.op)),
                };
                Ok(Value::Bool(result))
            }
            _ => Err(format!("Invalid logical operation: {:?} {} {:?}", l, self.op, r)),
        }
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
        matches!(parser.peek(), Token::Feature(kind) if *kind == self.op)
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
        match self.expr.eval(env)? {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => Err("Invalid operand for 'not'".into()),
        }
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(NOT))
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
    // Register tokens
    reg.tokens.add_keyword("and", AND);
    reg.tokens.add_keyword("or", OR);
    reg.tokens.add_keyword("not", NOT);

    // Register handlers
    reg.register_infix(Box::new(LogicInfix::new(AND)));
    reg.register_infix(Box::new(LogicInfix::new(OR)));
    reg.register_prefix(Box::new(NotPrefix));
}
