// Pipe operator expression: expr |> func(args)
// Passes the left value as the first argument to the right function

use std::rc::Rc;
use std::cell::RefCell;
use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::expressions::calls;
use crate::languages::lumen::registry::{ExprInfix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::statements::functions;

#[derive(Debug)]
struct PipeExpr {
    left: Box<dyn ExprNode>,
    func_name: String,
    args: Vec<Box<dyn ExprNode>>,
}

impl ExprNode for PipeExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // Evaluate the left side
        let left_value = self.left.eval(env)?;

        // Get function definition
        let (params, body) = functions::get_function(&self.func_name)
            .ok_or_else(|| format!("Undefined function '{}'", self.func_name))?;

        // Evaluate other arguments
        let mut arg_values = vec![left_value];
        for arg in &self.args {
            arg_values.push(arg.eval(env)?);
        }

        // Check argument count
        if arg_values.len() != params.len() {
            return Err(format!(
                "Function '{}' expects {} arguments, got {}",
                self.func_name,
                params.len(),
                arg_values.len()
            ));
        }

        // Memoization is a Lumen feature layered on the environment; see memo.rs.
        if let Some(cached) = crate::languages::lumen::memo::lookup(env, &self.func_name, &arg_values) {
            return Ok(cached);
        }

        let result = self.execute_function(&params, &body, &arg_values, env)?;
        crate::languages::lumen::memo::store(env, &self.func_name, &arg_values, &result);
        Ok(result)
    }
}

impl PipeExpr {
    /// Execute the function body in a fresh scope that is popped on every exit path.
    fn execute_function(
        &self,
        params: &[String],
        body: &Rc<RefCell<Vec<Box<dyn StmtNode>>>>,
        arg_values: &[Value],
        env: &mut Env,
    ) -> LumenResult<Value> {
        env.with_scope(|env| {
            for (param, arg_val) in params.iter().zip(arg_values) {
                env.define(param.clone(), arg_val.clone());
            }

            let mut result = Box::new(crate::languages::lumen::values::LumenNull) as Value;
            let body_ref = body.borrow();
            for stmt in body_ref.iter() {
                match stmt.exec(env)? {
                    crate::kernel::ast::Control::ExprValue(val) => result = val,
                    crate::kernel::ast::Control::Return(val) => {
                        result = val;
                        break;
                    }
                    crate::kernel::ast::Control::Break | crate::kernel::ast::Control::Continue => {
                        return Err("break/continue outside of loop".into());
                    }
                    crate::kernel::ast::Control::None => {}
                }
            }
            Ok(result)
        })
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
