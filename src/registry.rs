// src/registry.rs
//
// Feature registration API (expr / stmt hooks)
//
// Core rule: the core (ast/parser/eval/registry) stays stable.
// Features live in their own files and register parselets here.
//
// No central match tables per feature. No enum expansion per feature.

use std::fmt;

use crate::ast::{ExprNode, StmtNode};
use crate::parser::{Parser, Token};

pub type LumenResult<T> = Result<T, String>;

/// Pratt / precedence levels for infix operators.
/// Higher number = tighter binding.
pub type Precedence = u8;

/// Prefix expression parselet: parses an expression that starts at the current token.
pub trait PrefixParselet: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, p: &Parser) -> bool;
    fn parse(&self, p: &mut Parser) -> LumenResult<Box<dyn ExprNode>>;
}

/// Infix expression parselet: parses `left <op> right` style expressions.
pub trait InfixParselet: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, p: &Parser) -> bool;
    fn precedence(&self) -> Precedence;

    /// Parse the infix form, given the already-parsed left operand.
    fn parse(&self, p: &mut Parser, left: Box<dyn ExprNode>) -> LumenResult<Box<dyn ExprNode>>;
}

/// Statement parselet: parses a full statement at the current position.
pub trait StmtParselet: Send + Sync {
    fn name(&self) -> &'static str;
    fn matches(&self, p: &Parser) -> bool;
    fn parse(&self, p: &mut Parser) -> LumenResult<Box<dyn StmtNode>>;
}

/// The feature registry holds all parselets.
///
/// The core parser delegates to these parselets to build an AST.
/// The core evaluator dispatches by calling node trait methods (no big matches).
pub struct Registry {
    prefix: Vec<Box<dyn PrefixParselet>>,
    infix: Vec<Box<dyn InfixParselet>>,
    stmt: Vec<Box<dyn StmtParselet>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            prefix: Vec::new(),
            infix: Vec::new(),
            stmt: Vec::new(),
        }
    }

    // --- Registration ---

    pub fn add_prefix(&mut self, p: Box<dyn PrefixParselet>) {
        self.prefix.push(p);
    }

    pub fn add_infix(&mut self, i: Box<dyn InfixParselet>) {
        self.infix.push(i);
        // Keep infix parselets sorted by precedence (high -> low) for quick scans.
        // Not required for correctness, but makes debugging deterministic.
        self.infix.sort_by(|a, b| b.precedence().cmp(&a.precedence()));
    }

    pub fn add_stmt(&mut self, s: Box<dyn StmtParselet>) {
        self.stmt.push(s);
    }

    // --- Lookup used by Parser ---

    pub fn find_prefix(&self, p: &Parser) -> Option<&dyn PrefixParselet> {
        self.prefix
            .iter()
            .map(|x| x.as_ref())
            .find(|pl| pl.matches(p))
    }

    pub fn find_infix(&self, p: &Parser) -> Option<&dyn InfixParselet> {
        self.infix
            .iter()
            .map(|x| x.as_ref())
            .find(|pl| pl.matches(p))
    }

    pub fn find_stmt(&self, p: &Parser) -> Option<&dyn StmtParselet> {
        self.stmt
            .iter()
            .map(|x| x.as_ref())
            .find(|pl| pl.matches(p))
    }

    /// Useful for error messages: show what statement parselets exist.
    pub fn stmt_names(&self) -> Vec<&'static str> {
        self.stmt.iter().map(|s| s.name()).collect()
    }

    /// Useful for error messages: show what expression parselets exist.
    pub fn expr_names(&self) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = Vec::new();
        v.extend(self.prefix.iter().map(|p| p.name()));
        v.extend(self.infix.iter().map(|i| i.name()));
        v
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper: token matching utilities for features.
/// Keeping these in `registry.rs` gives features a common stable vocabulary.
pub fn token_is_keyword(tok: &Token, kw: &str) -> bool {
    matches!(tok, Token::Keyword(s) if s == kw)
}

pub fn token_is_symbol(tok: &Token, sym: &str) -> bool {
    matches!(tok, Token::Symbol(s) if s == sym)
}

pub fn token_is_ident(tok: &Token) -> bool {
    matches!(tok, Token::Ident(_))
}

pub fn token_is_number(tok: &Token) -> bool {
    matches!(tok, Token::Number(_))
}

pub fn token_is_string(tok: &Token) -> bool {
    matches!(tok, Token::String(_))
}

/// Small error helper: adds context about where parsing failed.
pub fn err_at(p: &Parser, msg: impl Into<String>) -> String {
    let msg = msg.into();
    let (line, col) = p.position();
    format!("Parse error at line {line}, col {col}: {msg}")
}

/// Debug view of the registry (names only).
impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("prefix", &self.prefix.iter().map(|p| p.name()).collect::<Vec<_>>())
            .field("infix", &self.infix.iter().map(|i| i.name()).collect::<Vec<_>>())
            .field("stmt", &self.stmt.iter().map(|s| s.name()).collect::<Vec<_>>())
            .finish()
    }
}
