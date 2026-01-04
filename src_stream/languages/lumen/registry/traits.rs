// Lumen-specific handler traits
// Languages define their own trait definitions for parsing

use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;
use crate::kernel::registry::LumenResult;
use super::precedence::Precedence;

/// Prefix expression handler
/// Handles expressions that start with a prefix operator or literal
pub trait ExprPrefix {
    /// Check if this handler matches the current token
    fn matches(&self, parser: &Parser) -> bool;

    /// Parse the prefix expression
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>>;
}

/// Infix expression handler
/// Handles binary operators and expressions that appear between two expressions
pub trait ExprInfix {
    /// Check if this handler matches the current token
    fn matches(&self, parser: &Parser) -> bool;

    /// Get the operator precedence for this infix operator
    fn precedence(&self) -> Precedence;

    /// Parse the infix expression with left-hand side already parsed
    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>>;
}

/// Statement handler
/// Handles parsing of individual statements
pub trait StmtHandler {
    /// Check if this handler matches the current token
    fn matches(&self, parser: &Parser) -> bool;

    /// Parse the statement
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>>;
}
