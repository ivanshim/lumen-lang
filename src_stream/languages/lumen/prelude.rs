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
    fn skip_tokens(&mut self);
}

impl LumenParserExt for Parser<'_> {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, min_prec)
    }

    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, Precedence::Lowest)
    }

    fn skip_tokens(&mut self) {
        while self.i < self.toks.len() {
            let lexeme = &self.toks[self.i].tok.lexeme;

            // Skip whitespace and newlines
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.i += 1;
                    continue;
                }

                // Handle comments: # ... until newline
                if ch == b'#' {
                    // Skip the # and all following characters until newline
                    self.i += 1;
                    while self.i < self.toks.len() {
                        let comment_lexeme = &self.toks[self.i].tok.lexeme;
                        if comment_lexeme == "\n" {
                            self.i += 1; // skip the newline too
                            break;
                        }
                        self.i += 1;
                    }
                    continue;
                }
            }
            break;
        }
    }
}
