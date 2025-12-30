// Comparison operators: == != < > <= >=

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprInfix, LumenResult, Precedence};
use crate::runtime::{Env, Value};

#[derive(Debug)]
struct CompareExpr {
    left: Box<dyn ExprNode>,
    op: Token,
    right: Box<dyn ExprNode>,
}

impl ExprNode for CompareExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        use Token::*;
        use Value::*;

        let result = match (l, r, &self.op) {
            (Number(a), Number(b), EqEq) => a == b,
            (Number(a), Number(b), NotEq) => a != b,
            (Number(a), Number(b), Lt) => a < b,
            (Number(a), Number(b), Gt) => a > b,
            (Number(a), Number(b), LtEq) => a <= b,
            (Number(a), Number(b), GtEq) => a >= b,
            _ => return Err("Invalid comparison".into()),
        };

        Ok(Bool(result))
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
        Ok(Box::new(CompareExpr { left, op, right }))
    }
}
