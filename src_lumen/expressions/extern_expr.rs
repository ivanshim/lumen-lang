// src_lumen/expressions/extern_expr.rs
//
// extern "selector" (arg1, arg2, ...)
//
// Extern marks the boundary where Lumen's semantic guarantees stop.
// It is deliberately uncomfortable, making the impurity explicit.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::src_lumen::structure::structural::{LPAREN, RPAREN};
use crate::src_lumen::extern_system;

#[derive(Debug)]
struct ExternExpr {
    selector: String,
    args: Vec<Box<dyn ExprNode>>,
}

impl ExprNode for ExternExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // Evaluate all arguments
        let mut eval_args = Vec::new();
        for arg in &self.args {
            eval_args.push(arg.eval(env)?);
        }

        // Call the extern function
        extern_system::call_extern(&self.selector, eval_args)
    }
}

pub struct ExternPrefix;

impl ExprPrefix for ExternPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == "extern"
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        // Consume 'extern'
        parser.advance();

        // Expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after extern".into());
        }

        // Parse selector - built from consecutive identifier/operator tokens
        // Allowed in selector: word characters, colon, pipe, parentheses
        let mut selector = String::new();

        loop {
            let next_token = parser.peek().lexeme.clone();

            // Check if we've reached the end of the selector
            if next_token == "," || next_token == RPAREN || next_token == "" {
                break;
            }

            // Accumulate tokens that look like selector parts
            // Allow: identifiers, operators like :, |, (, )
            let is_selector_char = next_token.chars().all(|c| {
                c.is_alphanumeric() || c == '_' || c == ':' || c == '|' || c == '(' || c == ')'
            });

            if is_selector_char {
                selector.push_str(&next_token);
                parser.advance();
            } else {
                break;
            }
        }

        if selector.is_empty() {
            return Err("extern selector cannot be empty".into());
        }

        // Parse remaining arguments
        let mut args = Vec::new();

        // Check if there are arguments after the selector
        if parser.peek().lexeme != RPAREN {
            // Expect a comma after selector
            if parser.advance().lexeme != "," {
                return Err("Expected ',' after extern selector".into());
            }

            // Parse argument expressions
            loop {
                args.push(parser.parse_expr()?);

                if parser.peek().lexeme == RPAREN {
                    break;
                }

                if parser.advance().lexeme != "," {
                    return Err("Expected ',' between extern arguments".into());
                }
            }
        }

        // Expect ')'
        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')' after extern arguments".into());
        }

        Ok(Box::new(ExternExpr { selector, args }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // extern is a keyword - needs to be in multichar_lexemes
    // (handled in dispatcher)
    reg.register_prefix(Box::new(ExternPrefix));
}
