// Comparison operators: == != < > <= >=

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprInfix, LumenResult, Precedence};
use crate::runtime::{Env, Value};

#[derive(Debug)]
struct ComparisonExpr {
    left: Box<dyn ExprNode>,
    op: Token,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ComparisonExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (l, r, &self.op) {
            (Value::Number(a), Value::Number(b), Token::EqEq) => Ok(Value::Bool(a == b)),
            (Value::Number(a), Value::Number(b), Token::NotEq) => Ok(Value::Bool(a != b)),
            (Value::Number(a), Value::Number(b), Token::Lt) => Ok(Value::Bool(a < b)),
            (Value::Number(a), Value::Number(b), Token::Gt) => Ok(Value::Bool(a > b)),
            (Value::Number(a), Value::Number(b), Token::LtEq) => Ok(Value::Bool(a <= b)),
            (Value::Number(a), Value::Number(b), Token::GtEq) => Ok(Value::Bool(a >= b)),
            _ => Err("Invalid comparison".into()),
        }
    }
}

pub struct ComparisonInfix {
    op: Token,
}

impl ComparisonInfix {
    pub fn new(op: Token) -> Self {
        Self { op }
    }
}

impl ExprInfix for ComparisonInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek() == &self.op
    }

    fn precedence(&self) -> Precedence {
        Precedence::Comparison
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
    ) -> LumenResult<Box<dyn ExprNode>> {
        let op = parser.advance();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ComparisonExpr { left, op, right }))
    }
}
