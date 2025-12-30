// src/registry.rs
//
// Feature registration and lookup.
// This file is intentionally stable and feature-agnostic.

use crate::ast::{ExprNode, StmtNode};
use crate::parser::Parser;

pub type LumenResult<T> = Result<T, String>;

/// Operator precedence (closed set, minimal)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    Lowest = 0,
    Logic,
    Comparison,
    Term,
    Factor,
    Prefix,
}

/// Prefix expression handler
pub trait PrefixHandler {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn ExprNode>>;
}

/// Infix expression handler
pub trait InfixHandler {
    fn matches(&self, parser: &Parser) -> bool;
    fn precedence(&self) -> Precedence;
    fn parse(
        &self,
        parser: &mut Parser,
        left: Box<dyn ExprNode>,
    ) -> LumenResult<Box<dyn ExprNode>>;
}

/// Statement handler
pub trait StmtHandler {
    fn matches(&self, parser: &Parser) -> bool;
    fn parse(&self, parser: &mut Parser) -> LumenResult<Box<dyn StmtNode>>;
}

/// Central registry
pub struct Registry {
    prefixes: Vec<Box<dyn PrefixHandler>>,
    infixes: Vec<Box<dyn InfixHandler>>,
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

    pub fn register_prefix(&mut self, h: Box<dyn PrefixHandler>) {
        self.prefixes.push(h);
    }

    pub fn register_infix(&mut self, h: Box<dyn InfixHandler>) {
        self.infixes.push(h);
    }

    pub fn register_stmt(&mut self, h: Box<dyn StmtHandler>) {
        self.stmts.push(h);
    }

    pub fn find_prefix(&self, parser: &Parser) -> Option<&Box<dyn PrefixHandler>> {
        self.prefixes.iter().find(|h| h.matches(parser))
    }

    pub fn find_infix(&self, parser: &Parser) -> Option<&Box<dyn InfixHandler>> {
        self.infixes.iter().find(|h| h.matches(parser))
    }

    pub fn find_stmt(&self, parser: &Parser) -> Option<&Box<dyn StmtHandler>> {
        self.stmts.iter().find(|h| h.matches(parser))
    }
}

/// Error helper
pub fn err_at(parser: &Parser, msg: &str) -> String {
    let (line, col) = parser.position();
    format!("{msg} at line {line}, column {col}")
}
