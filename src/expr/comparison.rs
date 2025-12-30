use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{InfixHandler, LumenResult, Precedence};
use crate::runtime::{Env, Value};

#[derive(Debug)]
pub struct ComparisonExpr {
    op: Token,
    left: Box<dyn ExprNode>,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ComparisonExpr {
    fn eval(&self, env: &mut Env) -> Result<Value, String> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        let result = match (&self.op, l, r) {
            (Token::EqEq, Value::Number(a), Value::Number(b)) => a == b,
            (Token::NotEq, Value::Number(a), Value::Number(b)) => a != b,
            (Token::Lt, Value::Number(a), Value::Number(b)) => a < b,
            (Token::Gt, Value::Number(a), Value::Number(b)) => a > b,
            (Token::LtEq, Value::Number(a), Value::Number(b)) => a <= b,
            (Token::GtEq, Value::Number(a), Value::Number(b)) => a >= b,
            _ => return Err("Invalid comparison".into()),
        };

        Ok(Value::Bool(result))
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

impl InfixHandler for ComparisonInfix {
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
        parser.advance();
        let right = parser.parse_expr_prec(self.precedence())?;
        Ok(Box::new(ComparisonExpr {
            op: self.op.clone(),
            left,
            right,
        }))
    }
}
