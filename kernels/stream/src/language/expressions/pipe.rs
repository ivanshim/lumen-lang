// Pipe operator expression: expr |> func(args)
// Passes the left value as the first argument to the right function

use crate::language::prelude::*;
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::language::expressions::calls;
use crate::language::expressions::variable::call_user_function;
use crate::language::registry::{ExprInfix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
struct PipeExpr {
    left: Box<dyn ExprNode>,
    func_name: String,
    args: Vec<Box<dyn ExprNode>>,
}

impl ExprNode for PipeExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // The left value becomes the first argument
        let mut arg_values = vec![self.left.eval(env)?];
        for arg in &self.args {
            arg_values.push(arg.eval(env)?);
        }
        call_user_function(&self.func_name, arg_values, env)
    }
}

/// One spelling of the pipe operator, at the tier the definition gives it.
pub struct PipeInfix {
    lexeme: String,
    prec: Precedence,
}

impl ExprInfix for PipeInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == self.lexeme
    }

    fn precedence(&self) -> Precedence {
        self.prec
    }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume the pipe
        parser.skip_tokens();

        let func_name = parser
            .take_identifier()
            .ok_or_else(|| "Expected function name after pipe operator".to_string())?;
        parser.skip_tokens();

        if !calls::at_call_open(parser) {
            return Err("Expected a call after the function name in a pipe expression".into());
        }
        let args = calls::parse_arguments(parser, registry, "in pipe expression")?;

        Ok(Box::new(PipeExpr {
            left,
            func_name,
            args,
        }))
    }
}

pub fn register(reg: &mut Registry) {
    for lexeme in def().list("op.pipe") {
        let prec = Precedence::binary(lexeme);
        reg.register_infix(Box::new(PipeInfix { lexeme: lexeme.clone(), prec }));
    }
}
