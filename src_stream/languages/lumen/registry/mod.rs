// Lumen language registry
// Manages all Lumen-specific parsing handlers and features

pub mod precedence;
pub mod traits;

use crate::kernel::parser::Parser;
use self::traits::{ExprPrefix, ExprInfix, StmtHandler};

pub use precedence::Precedence;
pub use traits::{ExprPrefix, ExprInfix, StmtHandler};

/// Lumen's feature registry
/// Maintains all registered expression prefix/infix handlers and statement handlers
pub struct Registry {
    prefixes: Vec<Box<dyn ExprPrefix>>,
    infixes: Vec<Box<dyn ExprInfix>>,
    stmts: Vec<Box<dyn StmtHandler>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
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
