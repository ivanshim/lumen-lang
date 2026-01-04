// src/framework/registry.rs
//
// Feature registration + lookup.
// Parser knows nothing about language features; it consults the Registry.
//
// AUTHORITY:
// - Span { start, end } (byte offsets) is the AUTHORITATIVE source-location mechanism
// - All parsing and AST construction must use Span
// - line/col are DIAGNOSTIC-ONLY (derived metadata, only for error messages)
//
// ARCHITECTURE:
// - TokenRegistry no longer holds semantic mappings (keywords, operators)
// - Instead, it only stores multi-character lexeme sequences for maximal-munch
// - ALL semantic interpretation happens in language modules

use crate::kernel::ast::{ExprNode, StmtNode};
use crate::kernel::parser::Parser;

pub type LumenResult<T> = Result<T, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Logic = 10,
    Comparison = 20,
    Term = 30,
    Factor = 40,
    Unary = 50,
}

impl std::ops::Add<i32> for Precedence {
    type Output = Precedence;

    fn add(self, rhs: i32) -> Precedence {
        let v = self as i32 + rhs;
        match v {
            v if v <= 0 => Precedence::Lowest,
            v if v < 20 => Precedence::Logic,
            v if v < 30 => Precedence::Comparison,
            v if v < 40 => Precedence::Term,
            v if v < 50 => Precedence::Factor,
            _ => Precedence::Unary,
        }
    }
}

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

// --------------------
// Expression features
// --------------------

pub trait ExprPrefix {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>>;
}

pub trait ExprInfix {
    fn matches(&self, parser: &Parser) -> bool;
    fn precedence(&self) -> Precedence;
    fn parse(&self, parser: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>>;
}

// --------------------
// Statement features
// --------------------

pub trait StmtHandler {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>>;
}

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
