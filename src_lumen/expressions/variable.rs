// src/expr/variable.rs
//
// Variable reference expression: `x`

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{ExprPrefix, LumenResult, Registry};
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
        // But exclude reserved keywords
        let lex = &parser.peek().lexeme;
        let is_identifier = lex.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
        let is_reserved = matches!(lex.as_str(), "true" | "false" | "not" | "and" | "or" | "if" | "else" | "while" | "print" | "break" | "continue" | "extern");
        is_identifier && !is_reserved
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        let name = parser.advance().lexeme;
        Ok(Box::new(VarExpr { name }))
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (identifiers are recognized by lexer)
    // Register handlers
    reg.register_prefix(Box::new(VariablePrefix));
}
