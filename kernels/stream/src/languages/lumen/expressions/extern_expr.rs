use crate::languages::lumen::prelude::*;
// src_lumen/expressions/extern_expr.rs
//
// extern("selector", arg1, arg2, ...)
//
// Extern marks the boundary where Lumen's semantic guarantees stop.
// It is deliberately uncomfortable, making the impurity explicit. Its name
// comes from the definition and is lexed whole, as a reserved word.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::expressions::literals::take_string_body;
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
        def().is("builtin.extern", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let d = def();
        parser.advance(); // the extern keyword
        parser.skip_tokens();

        if !d.is("syntax.call.open", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after {}", d.first("syntax.call.open"), d.first("builtin.extern")));
        }
        parser.skip_tokens();

        // CRITICAL: The selector MUST be a string literal.
        // This enforces that selectors are data, not identifiers.
        // Lumen must not accept unquoted capability names.
        let quotes = d.chars("lexical.string_quotes");
        let quote = {
            let lexeme = &parser.peek().lexeme;
            let mut chars = lexeme.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if quotes.contains(&c) => c,
                _ => {
                    return Err(
                        "extern selector must be a string literal (e.g., \"print_native\").\n\
                         Selector is data, not an identifier.\n\
                         Use: extern(\"capability\", args...)\n\
                         Not: extern(capability, args...)".into()
                    );
                }
            }
        };
        parser.advance(); // opening quote
        let selector = take_string_body(parser, quote)?;

        if selector.is_empty() {
            return Err("extern selector cannot be empty".into());
        }

        parser.skip_tokens();

        // Parse remaining arguments
        let mut args = Vec::new();

        // Check if there are arguments after the selector
        if !d.is("syntax.call.close", &parser.peek().lexeme) {
            // Expect a separator after the selector
            if !d.is("syntax.call.separator", &parser.advance().lexeme) {
                return Err(format!("Expected '{}' after extern selector", d.first("syntax.call.separator")));
            }
            parser.skip_tokens();

            // Parse argument expressions
            loop {
                args.push(parser.parse_expr(registry)?);
                parser.skip_tokens();

                if d.is("syntax.call.close", &parser.peek().lexeme) {
                    break;
                }

                if !d.is("syntax.call.separator", &parser.advance().lexeme) {
                    return Err(format!("Expected '{}' between extern arguments", d.first("syntax.call.separator")));
                }
                parser.skip_tokens();
            }
        }

        // Expect the closing bracket
        if !d.is("syntax.call.close", &parser.advance().lexeme) {
            return Err(format!("Expected '{}' after extern arguments", d.first("syntax.call.close")));
        }

        Ok(Box::new(ExternExpr { selector, args }))
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(ExternPrefix));
}
