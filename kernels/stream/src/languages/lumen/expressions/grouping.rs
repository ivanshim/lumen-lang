use crate::languages::lumen::prelude::*;
// Parenthesized expressions, with the brackets the definition spells.

use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        def().is("syntax.group.open", &parser.peek().lexeme)
    }

    fn parse(&self, parser: &mut Parser, registry: &super::super::registry::Registry) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // opening bracket
        parser.skip_tokens();
        let expr = parser.parse_expr(registry)?;
        parser.skip_tokens();

        if !def().is("syntax.group.close", &parser.advance().lexeme) {
            return Err(format!("Expected '{}'", def().first("syntax.group.close")));
        }

        Ok(expr)
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(GroupingPrefix));
}
