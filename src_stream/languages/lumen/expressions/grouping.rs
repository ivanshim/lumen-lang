// Parenthesized expressions: ( ... )

use crate::src_stream::src_stream::kernel::ast::ExprNode;
use crate::src_stream::src_stream::kernel::parser::Parser;
use crate::src_stream::src_stream::kernel::registry::{ExprPrefix, LumenResult, Registry};
use crate::src_stream::src_lumen::structure::structural::{LPAREN, RPAREN};

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == LPAREN
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '('
        parser.skip_whitespace();
        let expr = parser.parse_expr()?;
        parser.skip_whitespace();

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
