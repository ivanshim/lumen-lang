// Lumen prelude: the imports every handler module needs.
// use crate::languages::lumen::prelude::*;

pub use crate::kernel::ast::{ExprNode, StmtNode};
pub use crate::kernel::parser::Parser;
pub use crate::kernel::registry::{err_at, KernelResult as LumenResult};
pub use crate::languages::lumen::definition::def;
pub use crate::languages::lumen::registry::{
    parse_expr_with_prec, ExprInfix, ExprPrefix, Precedence, Registry, StmtHandler,
};
pub use crate::languages::lumen::{word_char, word_start};

/// Lumen's view of the kernel parser: expression entry points, the tokens
/// Lumen treats as insignificant between other tokens, and identifiers,
/// which the kernel lexer leaves as one token per character.
pub trait LumenParserExt {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>>;
    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>>;
    fn skip_tokens(&mut self);
    fn at_identifier(&self) -> bool;
    fn take_identifier(&mut self) -> Option<String>;
}

impl LumenParserExt for Parser<'_> {
    fn parse_expr_prec(&mut self, registry: &Registry, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, min_prec)
    }

    fn parse_expr(&mut self, registry: &Registry) -> LumenResult<Box<dyn ExprNode>> {
        parse_expr_with_prec(self, registry, Precedence::lowest())
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

    /// Whether the current token can begin an identifier.
    fn at_identifier(&self) -> bool {
        self.peek().lexeme.chars().next().map_or(false, word_start)
    }

    /// Consume an identifier: the current token, which must begin one, and
    /// every following single-character token that continues it.
    fn take_identifier(&mut self) -> Option<String> {
        if !self.at_identifier() {
            return None;
        }
        let mut name = self.advance().lexeme;
        loop {
            let lexeme = &self.peek().lexeme;
            let mut chars = lexeme.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if word_char(c) => name.push_str(&self.advance().lexeme),
                _ => break,
            }
        }
        Some(name)
    }
}
