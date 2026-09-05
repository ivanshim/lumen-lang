// Mini-PythonCore prelude - convenient import for handler modules
// All expression, statement, and structure modules can use:
// use crate::languages::python_core::prelude::*;

pub use crate::kernel::ast::{ExprNode, StmtNode};
pub use crate::kernel::parser::Parser;
pub use crate::kernel::registry::{LumenResult, err_at};
pub use crate::languages::python_core::registry::{
    ExprPrefix, ExprInfix, StmtHandler, Registry, Precedence, parse_expr_with_prec,
};

// Extension trait for Parser to support Mini-PythonCore expression parsing
pub trait PythonCoreParserExt {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>>;
    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>>;
    fn skip_tokens(&mut self);
}

impl PythonCoreParserExt for Parser<'_> {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, min_prec)
    }

    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, Precedence::Lowest)
    }

    fn skip_tokens(&mut self) {
        while self.i < self.toks.len() {
            let lexeme = &self.toks[self.i].tok.lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.i += 1;
                    continue;
                }
            }
            break;
        }
    }
}
