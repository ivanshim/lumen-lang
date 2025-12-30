// Parenthesized expressions: ( expr )

use crate::ast::ExprNode;
use crate::lexer::Token;
use crate::parser::Parser;
use crate::registry::{ExprPrefix, LumenResult};

#[derive(Debug)]
struct GroupExpr {
    inner: Box<dyn ExprNode>,
}

impl ExprNode for GroupExpr {
    fn eval(&self, env: &mut crate::runtime::Env) -> LumenResult<crate::runtime::Value> {
        self.inner.eval(env)
    }
}

pub struct GroupingPrefix;

impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        matches!(parser.peek(), Token::LParen)
    }

    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance(); // '('
        let expr = parser.parse_expr()?;
        match parser.advance() {
            Token::RParen => {}
            _ => return Err(parser.error("Expected ')'")),
        }
        Ok(Box::new(GroupExpr { inner: expr }))
    }
}
