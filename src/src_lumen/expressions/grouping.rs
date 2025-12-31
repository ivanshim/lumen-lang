// Parenthesized expressions: ( ... )

use crate::framework::ast::ExprNode;
use crate::framework::lexer::Token;
use crate::framework::parser::Parser;
use crate::framework::registry::{ExprPrefix, LumenResult, Registry};

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        let lparen = parser.reg.tokens.lparen();
        matches!(parser.peek(), Token::Feature(k) if *k == lparen)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume '('
        let expr = parser.parse_expr()?;

        let rparen = parser.reg.tokens.rparen();
        match parser.advance() {
            Token::Feature(k) if k == rparen => Ok(expr),
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
