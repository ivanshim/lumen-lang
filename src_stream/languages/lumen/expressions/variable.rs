use crate::languages::lumen::prelude::*;
// src/expr/variable.rs
//
// Variable reference expression: `x` or function call: `func(args)`

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::{Env, Value};
use crate::languages::lumen::function_registry;
use crate::languages::lumen::structure::structural::{LPAREN, RPAREN};

#[derive(Debug)]
struct VarExpr {
    name: String,
}

impl ExprNode for VarExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

#[derive(Debug)]
struct FunctionCallExpr {
    func_name: String,
    args: Vec<Box<dyn ExprNode>>,
}

impl ExprNode for FunctionCallExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        // Get function definition
        let (params, body) = function_registry::get_function(&self.func_name)
            .ok_or_else(|| format!("Undefined function '{}'", self.func_name))?;

        // Check argument count
        if self.args.len() != params.len() {
            return Err(format!(
                "Function '{}' expects {} arguments, got {}",
                self.func_name,
                params.len(),
                self.args.len()
            ));
        }

        // Evaluate arguments
        let mut arg_values = Vec::new();
        for arg in &self.args {
            arg_values.push(arg.eval(env)?);
        }

        // Create new scope for function
        env.push_scope();

        // Bind parameters to arguments
        for (param, arg_val) in params.iter().zip(arg_values) {
            env.define(param.clone(), arg_val);
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

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme is a valid identifier (starts with letter or underscore)
        // Exclude only the registered statement keywords (if, else, while, break, continue, print, fn, let, mut, return)
        // Allow "and", "or", "not", "true", "false", "extern" to pass through - they'll be handled
        // by their own expression handlers (logic, literals, extern_expr)
        let lex = &parser.peek().lexeme;
        let is_identifier = lex.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
        let is_statement_keyword = matches!(lex.as_str(),
            "if" | "else" | "while" | "break" | "continue" | "print" | "fn" | "let" | "mut" | "return");
        is_identifier && !is_statement_keyword
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Consume the first character of the identifier
        let mut name = parser.advance().lexeme;

        // Since the kernel lexer is agnostic, multi-character identifiers are split into single chars
        // Continue consuming identifier characters
        loop {
            if parser.peek().lexeme.len() == 1 {
                let ch = parser.peek().lexeme.as_bytes()[0];
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    name.push_str(&parser.advance().lexeme);
                    continue;
                }
            }
            break;
        }

        // Keywords like "and", "or", "not", "true", "false", "extern" will be collected as identifiers
        // but their own expression handlers should match first (they're registered with higher priority)
        // If we get here with one of those, it means it wasn't handled by a higher-priority handler,
        // so we try to treat it as a variable name
        // Check if this is a function call (name followed by '(')
        parser.skip_tokens();
        if parser.peek().lexeme == LPAREN {
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
                    return Err("Expected ',' or ')' after argument".into());
                }
            }

            if parser.advance().lexeme != RPAREN {
                return Err("Expected ')' after arguments".into());
            }

            return Ok(Box::new(FunctionCallExpr {
                func_name: name,
                args,
            }));
        }

        Ok(Box::new(VarExpr { name }))
    }
}

// --------------------
// Pattern Declaration
// --------------------

/// Declare what patterns this module recognizes
pub fn patterns() -> PatternSet {
    PatternSet::new()
        .with_char_classes(vec!["ident_start", "ident_char"])
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (identifiers are recognized by lexer)
    // Register handlers
    reg.register_prefix(Box::new(VariablePrefix));
}
