// Lumen prelude - convenient import for handler modules
// All expression, statement, and structure modules can use:
// use crate::languages::lumen::prelude::*;

pub use crate::kernel::ast::{ExprNode, StmtNode};
pub use crate::kernel::parser::Parser;
pub use crate::kernel::registry::{LumenResult, err_at};
pub use crate::languages::lumen::registry::{
    ExprPrefix, ExprInfix, StmtHandler, Registry, Precedence, parse_expr_with_prec,
};

// Extension trait for Parser to support Lumen expression parsing
pub trait LumenParserExt {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>>;
    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>>;
}

impl LumenParserExt for Parser<'_> {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, min_prec)
    }

    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, Precedence::Lowest)
    }
}
