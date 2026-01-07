use crate::languages::python::prelude::*;
// Parenthesized expressions: ( ... )

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use crate::languages::python::registry::{ExprPrefix, Registry};
use crate::languages::python::structure::structural::{LPAREN, RPAREN};

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == LPAREN
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '('
        let expr = parser.parse_expr(registry)?;

        if parser.advance().lexeme != RPAREN {
            return Err("Expected ')'".into());
        }

        Ok(expr)
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (parentheses are single-char lexemes emitted automatically)
    // Register handlers
    reg.register_prefix(Box::new(GroupingPrefix));
}
