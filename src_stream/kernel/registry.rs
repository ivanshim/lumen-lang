// src/framework/registry.rs
//
// Kernel registry utilities.
// Provides basic error handling and token registry for stream processing.
//
// AUTHORITY:
// - Span { start, end } (byte offsets) is the AUTHORITATIVE source-location mechanism
// - All parsing and AST construction must use Span
// - line/col are DIAGNOSTIC-ONLY (derived metadata, only for error messages)
//
// ARCHITECTURE:
// - Kernel provides only TokenRegistry for lexeme segmentation
// - Handler traits are language-specific (defined in language modules)
// - Parser is generic over language-specific trait types
// - Each language defines its own Precedence, handlers, and Registry

use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;

pub type LumenResult<T> = Result<T, String>;

/// Format a parse error with diagnostic position information.
/// DIAGNOSTIC FUNCTION: Uses line/col (derived from source) only for human-readable error messages.
/// line/col are NOT used by parsing logic - all core logic uses Span.
pub fn err_at(parser: &Parser, msg: &str) -> String {
    let (line, col) = parser.position();
    format!("ParseError at {line}:{col}: {msg}")
}

// --------------------
// Token Registry (Pure Transport Layer)
// --------------------

/// Registry of multi-character lexeme sequences for maximal-munch segmentation.
/// This is the only kernel-level registry - all other registries are language-specific.
pub struct TokenRegistry {
    // Multi-character lexeme sequences for maximal-munch segmentation
    // Stored in descending length order for proper maximal-munch
    multichar_lexemes: Vec<&'static str>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            multichar_lexemes: Vec::new(),
        }
    }

    /// Set the multi-character lexeme sequences that the language uses.
    /// The lexer will use these for maximal-munch segmentation.
    /// Sequences will be sorted by descending length automatically.
    pub fn set_multichar_lexemes(&mut self, mut lexemes: Vec<&'static str>) {
        // Sort by descending length for proper maximal-munch
        lexemes.sort_by(|a, b| b.len().cmp(&a.len()));
        self.multichar_lexemes = lexemes;
    }

    /// Get the multi-character lexemes in descending length order
    pub fn multichar_lexemes(&self) -> &[&'static str] {
        &self.multichar_lexemes
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// --------------------
// Generic Handler Traits (for language implementations to use)
// --------------------
// These traits define the interface for language-specific handlers.
// Actual implementations and concrete trait objects are defined in language modules.

pub trait PrecedenceLevel: std::cmp::Ord + std::cmp::PartialOrd + Copy {
    /// Lowest/base precedence
    fn lowest() -> Self;
}

pub trait ExprPrefixHandler {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>>;
}

pub trait ExprInfixHandler {
    type Prec: PrecedenceLevel;

    fn matches(&self, parser: &Parser) -> bool;
    fn precedence(&self) -> Self::Prec;
    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>>;
}

pub trait StmtHandlerTrait {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>>;
}
