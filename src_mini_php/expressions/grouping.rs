// Mini-PHP: Grouping with parentheses

use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, ExprPrefix, LumenResult, Registry};
use crate::src_mini_php::structure::structural::{LPAREN, RPAREN};

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::Feature(LPAREN))
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // consume (
        let expr = parser.parse_expr()?;
        match parser.advance() {
            Token::Feature(RPAREN) => Ok(expr),
            _ => Err(err_at(parser, "Expected ')'")),
        }
    }
}

pub fn register(reg: &mut Registry) {
    reg.register_prefix(Box::new(GroupingPrefix));
}
