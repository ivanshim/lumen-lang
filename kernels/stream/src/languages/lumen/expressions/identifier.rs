use crate::languages::lumen::prelude::*;
// Variable reference expression

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::languages::lumen::patterns::PatternSet;
use crate::kernel::runtime::{Env, Value};

#[derive(Debug)]
struct IdentExpr {
    name: String,
}

impl ExprNode for IdentExpr {
    fn eval(&self, env: &mut Env) -> LumenResult<Value> {
        env.get(&self.name)
    }
}

pub struct IdentPrefix;

impl ExprPrefix for IdentPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_')
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        let name = parser.advance().lexeme;
        Ok(Box::new(IdentExpr { name }))
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
    reg.register_prefix(Box::new(IdentPrefix));
}
