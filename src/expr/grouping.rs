// Parenthesized expressions: ( ... )

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprPrefix, LumenResult, Registry};

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::LParen)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '('
        let expr = parser.parse_expr()?;

        match parser.advance() {
            Token::RParen => Ok(expr),
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
