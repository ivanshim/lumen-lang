use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{InfixHandler, PrefixHandler, LumenResult, Precedence};
use crate::runtime::{Env, Value};

#[derive(Debug)]
pub struct LogicExpr {
    op: Token,
    left: Box<dyn ExprNode>,
    right: Box<dyn ExprNode>,
}

impl ExprNode for LogicExpr {
    fn eval(&self, env: &mut Env) -> Result<Value, String> {
        let l = self.left.eval(env)?;
        let r = self.right.eval(env)?;

        match (self.op.clone(), l, r) {
            (Token::And, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a && b)),
            (Token::Or, Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(a || b)),
            _ => Err("Invalid logic operation".into()),
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

impl InfixHandler for LogicInfix {
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
        parser.advance();
        let right = parser.parse_expr_prec(self.precedence())?;
        Ok(Box::new(LogicExpr {
            op: self.op.clone(),
            left,
            right,
        }))
    }
}

pub struct NotPrefix;

impl PrefixHandler for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Not)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr_prec(Precedence::Prefix)?;
        Ok(Box::new(UnaryNot { expr }))
    }
}

#[derive(Debug)]
struct UnaryNot {
    expr: Box<dyn ExprNode>,
}

impl ExprNode for UnaryNot {
    fn eval(&self, env: &mut Env) -> Result<Value, String> {
        match self.expr.eval(env)? {
            Value::Bool(b) => Ok(Value::Bool(!b)),
            _ => Err("Expected boolean".into()),
        }
    }
}
