// Pipe operator expression: expr |> func(args)
// Passes the left value as the first argument to the right function

use std::rc::Rc;
use std::cell::RefCell;
use crate::languages::lumen::prelude::*;
use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::languages::lumen::registry::{ExprInfix, Precedence, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::statements::functions;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};

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

        // ================================================================
        // MEMOIZATION: Gated by execution context (MEMOIZATION = true/false)
        // ================================================================
        // Cache operations are gated by env.memoization_enabled().
        // If MEMOIZATION = false (default): no cache lookup/storage
        // If MEMOIZATION = true: check cache before execution, store after
        //
        let arg_fingerprint = Env::fingerprint_args(&arg_values);
        if let Some(cached_result) = env.get_cached(&self.func_name, &arg_fingerprint) {
            return Ok(cached_result);
        }

        let result = self.execute_function(&params, &body, &arg_values, env)?;
        env.cache_result(&self.func_name, &arg_fingerprint, result.clone());
        Ok(result)
    }
}

impl PipeExpr {
    /// Execute function body and return result.
    fn execute_function(
        &self,
        params: &[String],
        body: &Rc<RefCell<Vec<Box<dyn StmtNode>>>>,
        arg_values: &[Value],
        env: &mut Env,
    ) -> LumenResult<Value> {
        // Create new scope for function
        env.push_scope();

        // Bind parameters to arguments
        for (param, arg_val) in params.iter().zip(arg_values) {
            env.define(param.clone(), arg_val.clone());
        }

        // Execute function body
        let mut result = Box::new(crate::languages::lumen::values::LumenNone) as Value;
        {
            let body_ref = body.borrow();
            for stmt in body_ref.iter() {
                let ctl = stmt.exec(env)?;
                match ctl {
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
        }

        // Exit function scope
        env.pop_scope();

        Ok(result)
    }
}

pub struct PipeInfix;

impl ExprInfix for PipeInfix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "|>"
    }

    fn precedence(&self) -> Precedence {
        Precedence::Pipe
    }

    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '|>'
        parser.skip_tokens();

        // Parse function name
        let mut func_name = String::new();
        if parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_') {
            func_name = parser.advance().lexeme;
            parser.skip_tokens();

            // Handle multi-character identifiers
            loop {
                if parser.peek().lexeme.len() == 1 {
                    let ch = parser.peek().lexeme.as_bytes()[0];
                    if ch.is_ascii_alphanumeric() || ch == b'_' {
                        func_name.push_str(&parser.advance().lexeme);
                        parser.skip_tokens();
                        continue;
                    }
                }
                break;
            }
        } else {
            return Err("Expected function name after pipe operator".into());
        }

        // Expect '('
        if parser.peek().lexeme != LPAREN {
            return Err("Expected '(' after function name in pipe expression".into());
        }
        parser.advance(); // consume '('
        parser.skip_tokens();

        let mut args = Vec::new();

        // Parse arguments
        while parser.peek().lexeme != RPAREN {
            let arg = parser.parse_expr(registry)?;
            args.push(arg);

            parser.skip_tokens();
            if parser.peek().lexeme == "," {
                parser.advance();
                parser.skip_tokens();
            } else if parser.peek().lexeme != RPAREN {
                return Err("Expected ',' or ')' after argument in pipe expression".into());
            }
        }

        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after arguments in pipe expression".into());
        }

        Ok(Box::new(PipeExpr {
            left,
            func_name,
            args,
        }))
    }
}

pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_literals(vec!["|>"])
}

pub fn register(reg: &mut Registry) {
    reg.register_infix(Box::new(PipeInfix));
}
