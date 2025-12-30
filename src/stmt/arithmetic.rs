// src/expr/arithmetic.rs
//
// Arithmetic infix expressions: + - * /
// Fully removable language feature.

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence};
use crate::runtime::{Env, Value};

#[derive(Debug)]
struct BinaryExpr {
    left: Box<dyn ExprNode>,
    op: Token,
    right: Box<dyn ExprNode>,
}

impl ExprNode for BinaryExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (l, r, &self.op) {
            (Value::Number(a), Value::Number(b), Token::Plus) => Ok(Value::Number(a + b)),
            (Value::Number(a), Value::Number(b), Token::Minus) => Ok(Value::Number(a - b)),
            (Value::Number(a), Value::Number(b), Token::Star) => Ok(Value::Number(a * b)),
            (Value::Number(a), Value::Number(b), Token::Slash) => Ok(Value::Number(a / b)),
            _ => Err("Invalid operands for arithmetic".into()),
        }
    }
}

// ---------- Prefix (unary minus) ----------

pub struct UnaryMinusPrefix;

impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Minus)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
        Ok(Box::new(BinaryExpr {
            left: Box::new(crate::expr::literals::NumberLiteral { value: 0.0 }),
            op: Token::Minus,
            right: expr,
        }))
    }
}

// ---------- Infix operators ----------

pub struct ArithmeticInfix {
    op: Token,
    prec: Precedence,
}

impl ArithmeticInfix {
    fn new(op: Token, prec: Precedence) -> Self {
        Self { op, prec }
    }
}

impl ExprInfix for ArithmeticInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek() == &self.op
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
    ) -> LumenResult<Box<dyn ExprNode>> {
        let op = parser.advance();
        let right = parser.parse_expr_prec(self.prec + 1)?;
        Ok(Box::new(BinaryExpr { left, op, right }))
    }
}
