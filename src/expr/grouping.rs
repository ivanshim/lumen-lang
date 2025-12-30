use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{LumenResult, PrefixHandler};

pub struct GroupingPrefix;

impl PrefixHandler for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::LParen)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '('
        let expr = parser.parse_expr()?;
        match parser.advance() {
            Token::RParen => Ok(expr),
            _ => Err("Expected ')'".into()),
        }
    }
}
