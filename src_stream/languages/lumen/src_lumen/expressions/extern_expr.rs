// src_lumen/expressions/extern_expr.rs
//
// extern "selector" (arg1, arg2, ...)
//
// Extern marks the boundary where Lumen's semantic guarantees stop.
// It is deliberately uncomfortable, making the impurity explicit.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::registry::{ExprPrefix, LumenResult, Registry};
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};
use crate::languages::lumen::extern_system;

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
        parser.skip_whitespace();

        // Expect '('
        if parser.advance().lexeme != LPAREN {
            return Err("Expected '(' after extern".into());
        }
        parser.skip_whitespace();

        // CRITICAL: The selector MUST be a string literal.
        // This enforces that selectors are data, not identifiers.
        // Lumen must not accept unquoted capability names.
        if parser.peek().lexeme != "\"" {
            return Err(
                "extern selector must be a string literal (e.g., \"print_native\").\n\
                 Selector is data, not an identifier.\n\
                 Use: extern(\"capability\", args...)\n\
                 Not: extern(capability, args...)".into()
            );
        }

        // Consume opening quote
        let mut selector_lexeme = parser.advance().lexeme;

        // Since the kernel lexer is agnostic, assemble the full string from individual characters
        loop {
            let ch = parser.peek().lexeme.clone();

            // Check for closing quote
            if ch == "\"" {
                selector_lexeme.push_str(&parser.advance().lexeme);
                break;
            }

            // Add character to selector (including whitespace, etc.)
            selector_lexeme.push_str(&parser.advance().lexeme);

            // Protect against unterminated strings
            if parser.i >= parser.toks.len() {
                return Err("Unterminated string literal in extern selector".into());
            }
        }

        // Remove the surrounding quotes: "selector" -> selector
        if selector_lexeme.len() < 2 || !selector_lexeme.ends_with('"') {
            return Err("Invalid string literal in extern selector".into());
        }
        let selector = selector_lexeme[1..selector_lexeme.len() - 1].to_string();

        if selector.is_empty() {
            return Err("extern selector cannot be empty".into());
        }

        parser.skip_whitespace();

        // Parse remaining arguments
        let mut args = Vec::new();

        // Check if there are arguments after the selector
        if parser.peek().lexeme != RPAREN {
            // Expect a comma after selector
            if parser.advance().lexeme != "," {
                return Err("Expected ',' after extern selector".into());
            }
            parser.skip_whitespace();

            // Parse argument expressions
            loop {
                args.push(parser.parse_expr()?);
                parser.skip_whitespace();

                if parser.peek().lexeme == RPAREN {
                    break;
                }

                if parser.advance().lexeme != "," {
                    return Err("Expected ',' between extern arguments".into());
                }
                parser.skip_whitespace();
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
