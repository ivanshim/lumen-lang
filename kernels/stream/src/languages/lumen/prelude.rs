// Lumen prelude: the imports every handler module needs.
// use crate::languages::lumen::prelude::*;

pub use crate::kernel::ast::{ExprNode, StmtNode};
pub use crate::kernel::parser::Parser;
pub use crate::kernel::registry::{err_at, KernelResult as LumenResult};
pub use crate::languages::lumen::registry::{
    parse_expr_with_prec, ExprInfix, ExprPrefix, Precedence, Registry, StmtHandler,
};

/// Lumen's view of the kernel parser: expression entry points and the
/// tokens Lumen treats as insignificant between other tokens.
pub trait LumenParserExt {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>>;
    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>>;
    fn skip_tokens(&mut self);
}

impl LumenParserExt for Parser<'_> {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, min_prec)
    }

    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, Precedence::Lowest)
    }

    /// Skip whitespace and line breaks. Comments were removed before lexing.
    fn skip_tokens(&mut self) {
        while self.i < self.toks.len() {
            let lexeme = &self.toks[self.i].tok.lexeme;
            if lexeme.len() == 1 && matches!(lexeme.as_bytes()[0], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
                continue;
            }
            break;
        }
    }
}

/// Identifier character classes for this language.
pub use crate::languages::lumen::{word_char, word_start};
