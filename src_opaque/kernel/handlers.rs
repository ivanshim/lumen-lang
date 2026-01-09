/// Statement Handler Trait System for Opaque Kernel
///
/// This system allows language modules to register their own statement handlers
/// without embedding language-specific logic in the kernel parser.
/// The kernel remains completely language-agnostic.

use crate::kernel::ast::StmtNode;
use crate::kernel::parser::Parser;
use std::cell::RefCell;

// Thread-local storage for the current handler registry
// This allows handlers to recursively parse nested statements with the full registry
thread_local! {
    static CURRENT_REGISTRY: RefCell<Option<*const HandlerRegistry>> = RefCell::new(None);
}

/// Trait for language-specific statement handlers
/// Each handler is responsible for recognizing and parsing one type of statement
pub trait StatementHandler: Send + Sync {
    /// Check if this handler can parse the current token
    /// The parser position is NOT advanced by this check
    fn can_handle(&self, parser: &Parser) -> bool;

    /// Parse a statement and return the AST node
    /// The parser position will be advanced as needed
    fn parse(&self, parser: &mut Parser) -> Result<StmtNode, String>;
}

/// Registry of statement handlers
/// Handlers are tried in order - first match wins
pub struct HandlerRegistry {
    handlers: Vec<Box<dyn StatementHandler>>,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        HandlerRegistry {
            handlers: Vec::new(),
        }
    }

    /// Register a statement handler
    pub fn register<H: StatementHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }

    /// Register a pre-boxed statement handler
    pub fn register_boxed(&mut self, handler: Box<dyn StatementHandler>) {
        self.handlers.push(handler);
    }

    /// Try to parse a statement using registered handlers
    pub fn parse_statement(&self, parser: &mut Parser) -> Option<Result<StmtNode, String>> {
        for handler in &self.handlers {
            if handler.can_handle(parser) {
                return Some(handler.parse(parser));
            }
        }
        None
    }

    /// Set this registry as the current thread-local registry
    /// This allows handlers to recursively access the registry
    pub fn set_as_current(&self) {
        CURRENT_REGISTRY.with(|reg| {
            *reg.borrow_mut() = Some(self as *const _);
        });
    }

    /// Clear the current thread-local registry
    pub fn clear_current() {
        CURRENT_REGISTRY.with(|reg| {
            *reg.borrow_mut() = None;
        });
    }

    /// Get immutable access to all handlers
    pub fn handlers(&self) -> &[Box<dyn StatementHandler>] {
        &self.handlers
    }
}

/// Get a reference to the current registry from thread-local storage
/// Used by handler implementations to recursively parse nested statements
pub fn get_current_registry() -> Option<&'static HandlerRegistry> {
    CURRENT_REGISTRY.with(|reg| {
        reg.borrow().map(|ptr| unsafe { &*ptr })
    })
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
