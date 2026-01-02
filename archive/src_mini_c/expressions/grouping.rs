// Grouping with parentheses
use crate::kernel::ast::ExprNode;
use crate::kernel::lexer::Token;
use crate::kernel::parser::Parser;
use crate::kernel::registry::{err_at, ExprPrefix, LumenResult, Registry};
use crate::src_mini_c::structure::structural::{LPAREN, RPAREN};

pub struct GroupingPrefix;
impl ExprPrefix for GroupingPrefix {
    fn matches(&self, parser: &Parser) -> bool {
        parser.peek().lexeme == LPAREN
    }
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>> {
        parser.advance();
        let expr = parser.parse_expr()?;
        if parser.advance().lexeme != RPAREN {
            return Err(err_at(parser, "Expected ')'"));
        }
        Ok(expr)
    }
}

pub fn register(reg: &mut Registry) {
    // No token registration needed - kernel handles all segmentation
    reg.register_prefix(Box::new(GroupingPrefix));
}
