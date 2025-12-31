// Logical operators: and / or / not

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence};
use crate::runtime::{Env, Value};

#[derive(Debug)]
struct LogicExpr {
    left: Box<dyn ExprNode>,
    op: Token,
    right: Box<dyn ExprNode>,
}

impl ExprNode for LogicExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (&l, &r, &self.op) {
            (Value::Bool(a), Value::Bool(b), Token::And) => Ok(Value::Bool(*a && *b)),
            (Value::Bool(a), Value::Bool(b), Token::Or) => Ok(Value::Bool(*a || *b)),
            _ => Err(format!("Invalid logical operation: {:?} {:?} {:?}", l, self.op, r)),
        }
    }
}

pub struct LogicInfix {
    op: Token,
}

impl LogicInfix {
    pub fn new(op: Token) -> Self {
        Self { op }
    }
}

impl ExprInfix for LogicInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek() == &self.op
    }

    fn precedence(&self) -> Precedence {
        Precedence::Logic
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
    ) -> LumenResult<Box<dyn ExprNode>> {
        let op = parser.advance();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(LogicExpr { left, op, right }))
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
        matches!(parser.peek(), Token::Not)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
        Ok(Box::new(NotExpr { expr }))
    }
}
