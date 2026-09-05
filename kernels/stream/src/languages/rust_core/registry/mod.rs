// Mini-RustCore language registry
// Manages all Mini-RustCore-specific parsing handlers and features

pub mod precedence;
pub mod traits;

use crate::kernel::parser::Parser;
use crate::kernel::registry::{TokenRegistry, LumenResult, err_at};
use crate::languages::rust_core::prelude::RustCoreParserExt;

pub use precedence::Precedence;
pub use traits::{ExprPrefix, ExprInfix, StmtHandler};

/// Mini-RustCore's feature registry
/// Maintains all registered expression prefix/infix handlers, statement handlers,
/// and the token registry for lexeme segmentation
pub struct Registry {
    pub tokens: TokenRegistry,
    prefixes: Vec<Box<dyn ExprPrefix>>,
    infixes: Vec<Box<dyn ExprInfix>>,
    stmts: Vec<Box<dyn StmtHandler>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            tokens: TokenRegistry::new(),
            prefixes: Vec::new(),
            infixes: Vec::new(),
            stmts: Vec::new(),
        }
    }

    pub fn register_prefix(&mut self, h: Box<dyn ExprPrefix>) {
        self.prefixes.push(h);
    }

    pub fn register_infix(&mut self, h: Box<dyn ExprInfix>) {
        self.infixes.push(h);
    }

    pub fn register_stmt(&mut self, h: Box<dyn StmtHandler>) {
        self.stmts.push(h);
    }

    pub fn find_prefix(&self, parser: &Parser) -> Option<&dyn ExprPrefix> {
        self.prefixes.iter().map(|b| b.as_ref()).find(|h| h.matches(parser))
    }

    pub fn find_infix(&self, parser: &Parser) -> Option<&dyn ExprInfix> {
        self.infixes.iter().map(|b| b.as_ref()).find(|h| h.matches(parser))
    }

    pub fn find_stmt(&self, parser: &Parser) -> Option<&dyn StmtHandler> {
        self.stmts.iter().map(|b| b.as_ref()).find(|h| h.matches(parser))
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse expression with precedence climbing for Mini-RustCore
pub fn parse_expr_with_prec(
    parser: &mut Parser,
    registry: &Registry,
    min_prec: Precedence,
) -> LumenResult<Box<dyn crate::kernel::ast::ExprNode>> {
    parser.skip_tokens();

    let prefix = registry
        .find_prefix(parser)
        .ok_or_else(|| err_at(parser, "Unknown expression"))?;

    let mut left = prefix.parse(parser, registry)?;

    loop {
        parser.skip_tokens();

        let infix = match registry.find_infix(parser) {
            Some(i) => i,
            None => break,
        };

        if infix.precedence() < min_prec {
            break;
        }

        left = infix.parse(parser, left, registry)?;
    }

    Ok(left)
}
