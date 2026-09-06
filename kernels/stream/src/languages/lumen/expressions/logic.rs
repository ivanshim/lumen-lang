use crate::languages::lumen::prelude::*;
// Logical operators: conjunction, disjunction and negation, with
// short-circuit evaluation. The words that spell them come from the
// definition and are lexed whole, as reserved words.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::values::{LumenBool, as_bool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logic {
    And,
    Or,
}

#[derive(Debug)]
struct LogicExpr {
    left: Box<dyn ExprNode>,
    op: Logic,
    right: Box<dyn ExprNode>,
}

impl ExprNode for LogicExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // Implement short-circuit evaluation
        let l = self.left.eval(env)?;
        let left_bool = as_bool(l.as_ref())?;

        match self.op {
            Logic::And => {
                // Short-circuit: if left is false, don't evaluate right
                if !left_bool.value {
                    return Ok(Box::new(LumenBool::new(false)));
                }
            }
            Logic::Or => {
                // Short-circuit: if left is true, don't evaluate right
                if left_bool.value {
                    return Ok(Box::new(LumenBool::new(true)));
                }
            }
        }
        let r = self.right.eval(env)?;
        let right_bool = as_bool(r.as_ref())?;
        Ok(Box::new(LumenBool::new(right_bool.value)))
    }
}

/// One spelling of one logical operator, at the tier the definition gives it.
pub struct LogicInfix {
    lexeme: String,
    op: Logic,
    prec: Precedence,
}

impl ExprInfix for LogicInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.lexeme
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
        registry: &super::super::registry::Registry,
    ) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume the operator
        parser.skip_tokens();
        let right = parser.parse_expr_prec(registry, self.prec.right_operand(&self.lexeme))?;
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
        let val = self.expr.eval(env)?;
        let b = as_bool(val.as_ref())?;
        Ok(Box::new(LumenBool::new(!b.value)))
    }
}

pub struct NotPrefix;

impl ExprPrefix for NotPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("op.not", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let lexeme = parser.advance().lexeme; // consume the negation word
        parser.skip_tokens();
        let expr = parser.parse_expr_prec(registry, Precedence::unary(&lexeme))?;
        Ok(Box::new(NotExpr { expr }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    for (label, op) in [("op.and", Logic::And), ("op.or", Logic::Or)] {
        for lexeme in def().list(label) {
            let prec = Precedence::binary(lexeme);
            reg.register_infix(Box::new(LogicInfix { lexeme: lexeme.clone(), op, prec }));
        }
    }
    reg.register_prefix(Box::new(NotPrefix));
}
