use crate::languages::lumen::prelude::*;
// src/expr/variable.rs
//
// Variable reference expression: `x`

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
struct VarExpr {
    name: String,
}

impl ExprNode for VarExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme is a valid identifier (starts with letter or underscore)
        // Exclude only the registered statement keywords (if, else, while, break, continue, print)
        // Allow "and", "or", "not", "true", "false", "extern" to pass through - they'll be handled
        // by their own expression handlers (logic, literals, extern_expr)
        let lex = &parser.peek().lexeme;
        let is_identifier = lex.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
        let is_statement_keyword = matches!(lex.as_str(), "if" | "else" | "while" | "break" | "continue" | "print");
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
