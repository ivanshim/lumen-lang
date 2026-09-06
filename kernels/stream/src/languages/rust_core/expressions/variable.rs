// Variable reference expressions

use crate::languages::rust_core::{word_char, word_start};
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::KernelResult as LumenResult;
use crate::languages::rust_core::registry::{ExprPrefix, Registry};
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
pub struct VariableExpr {
    pub name: String,
}

impl ExprNode for VariableExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name).map_err(|_| format!("Undefined variable: {}", self.name))
    }
}

pub struct VariablePrefix;

impl ExprPrefix for VariablePrefix {
    fn matches(&self, parser: &Parser) -> bool {
        // Check if lexeme is a valid identifier (starts with letter or underscore)
        // But exclude reserved keywords
        let lex = &parser.peek().lexeme;
        let is_identifier = lex.chars().next().map_or(false, word_start);
        let is_reserved = matches!(lex.as_str(), "let" | "if" | "else" | "while" | "break" | "continue" | "print" | "true" | "false");
        is_identifier && !is_reserved
    }

    fn parse(&self, parser: &mut Parser, _registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        // Consume the first character of the identifier
        let mut name = parser.advance().lexeme;

        // Since the kernel lexer is agnostic, multi-character identifiers are split into single chars
        // Continue consuming identifier characters
        loop {
            if parser.peek().lexeme.chars().count() == 1 {
                let ch = parser.peek().lexeme.chars().next().unwrap();
                if word_char(ch) {
                    name.push_str(&parser.advance().lexeme);
                    continue;
                }
            }
            break;
        }

        Ok(Box::new(VariableExpr { name }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    // No tokens to register (identifiers are lexer primitives)

    // Register handlers
    reg.register_prefix(Box::new(VariablePrefix));
}
