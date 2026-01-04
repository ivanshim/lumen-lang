// Mini-Python language registry
// Manages all Mini-Python-specific parsing handlers and features

pub mod precedence;
pub mod traits;

use crate::kernel::parser::Parser;
use self::traits::{ExprPrefix, ExprInfix, StmtHandler};
use crate::kernel::registry::TokenRegistry;

pub use precedence::Precedence;
pub use traits::{ExprPrefix, ExprInfix, StmtHandler};

/// Mini-Python's feature registry
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
