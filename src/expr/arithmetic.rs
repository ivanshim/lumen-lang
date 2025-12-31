// src/expr/arithmetic.rs
//
// + - * / % and unary minus

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprInfix, ExprPrefix, LumenResult, Precedence, Registry};
use crate::runtime::{Env, Value};

#[derive(Debug)]
struct UnaryMinusExpr {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryMinusExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        match self.expr.eval(env)? {
            Value::Number(n) => Ok(Value::Number(-n)),
            _ => Err("Invalid operand for unary '-'".into()),
        }
    }
}

pub struct UnaryMinusPrefix;

impl ExprPrefix for UnaryMinusPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Minus)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '-'
        let expr = parser.parse_expr_prec(Precedence::Unary)?;
        Ok(Box::new(UnaryMinusExpr { expr }))
    }
}

#[derive(Debug)]
struct ArithmeticExpr {
    left: Box<dyn ExprNode>,
    op: Token,
    right: Box<dyn ExprNode>,
}

impl ExprNode for ArithmeticExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (l, r, &self.op) {
            (Value::Number(a), Value::Number(b), Token::Plus) => Ok(Value::Number(a + b)),
            (Value::Number(a), Value::Number(b), Token::Minus) => Ok(Value::Number(a - b)),
            (Value::Number(a), Value::Number(b), Token::Star) => Ok(Value::Number(a * b)),
            (Value::Number(a), Value::Number(b), Token::Slash) => Ok(Value::Number(a / b)),
            (Value::Number(a), Value::Number(b), Token::Percent) => Ok(Value::Number(a % b)),
            _ => Err("Invalid arithmetic operation".into()),
        }
    }
}

pub struct ArithmeticInfix {
    op: Token,
    prec: Precedence,
}

impl ArithmeticInfix {
    pub fn new(op: Token, prec: Precedence) -> Self {
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

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>> {
        let op = parser.advance();
        let right = parser.parse_expr_prec(self.precedence() + 1)?;
        Ok(Box::new(ArithmeticExpr { left, op, right }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register tokens
    reg.tokens.add_single_char('+', Token::Plus);
    reg.tokens.add_single_char('-', Token::Minus);
    reg.tokens.add_single_char('*', Token::Star);
    reg.tokens.add_single_char('/', Token::Slash);
    reg.tokens.add_single_char('%', Token::Percent);

    // Register handlers
    reg.register_prefix(Box::new(UnaryMinusPrefix));
    reg.register_infix(Box::new(ArithmeticInfix::new(Token::Plus, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(Token::Minus, Precedence::Term)));
    reg.register_infix(Box::new(ArithmeticInfix::new(Token::Star, Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new(Token::Slash, Precedence::Factor)));
    reg.register_infix(Box::new(ArithmeticInfix::new(Token::Percent, Precedence::Factor)));
}
