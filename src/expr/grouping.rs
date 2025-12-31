// Parenthesized expressions: ( ... )

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprPrefix, LumenResult, Registry};
use crate::syntax::structural::{LPAREN, RPAREN};

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(LPAREN))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '('
        let expr = parser.parse_expr()?;

        match parser.advance() {
            Token::Feature(RPAREN) => Ok(expr),
            _ => Err("Expected ')'".into()),
        }
    }
}

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // No tokens to register (parentheses are structural tokens always recognized by lexer)

    // Register handlers
    reg.register_prefix(Box::new(GroupingPrefix));
}
