// src/registry.rs
//
// Feature registration + lookup.
// Parser knows nothing about language features; it consults the Registry.

use crate::ast::{ExprNode, StmtNode};
use crate::lexer::Token;
use crate::parser::Parser;
use std::collections::HashMap;

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

pub fn err_at(parser: &Parser, msg: &str) -> String {
    let (line, col) = parser.position();
    format!("ParseError at {line}:{col}: {msg}")
}

// --------------------
// Token Registry
// --------------------

pub struct TokenRegistry {
    keywords: HashMap<String, Token>,
    single_char: HashMap<char, Token>,
    two_char: HashMap<String, Token>,

    // Structural tokens (set by syntax::structural module)
    lparen: Option<&'static str>,
    rparen: Option<&'static str>,
    newline: Option<&'static str>,
    indent: Option<&'static str>,
    dedent: Option<&'static str>,
    eof: Option<&'static str>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            keywords: HashMap::new(),
            single_char: HashMap::new(),
            two_char: HashMap::new(),
            lparen: None,
            rparen: None,
            newline: None,
            indent: None,
            dedent: None,
            eof: None,
        }
    }

    pub fn add_keyword(&mut self, word: &str, token_kind: &'static str) {
        self.keywords.insert(word.to_string(), Token::Feature(token_kind));
    }

    pub fn add_single_char(&mut self, ch: char, token_kind: &'static str) {
        self.single_char.insert(ch, Token::Feature(token_kind));
    }

    pub fn add_two_char(&mut self, chars: &str, token_kind: &'static str) {
        self.two_char.insert(chars.to_string(), Token::Feature(token_kind));
    }

    pub fn lookup_keyword(&self, word: &str) -> Option<Token> {
        self.keywords.get(word).cloned()
    }

    pub fn lookup_single_char(&self, ch: char) -> Option<Token> {
        self.single_char.get(&ch).cloned()
    }

    pub fn lookup_two_char(&self, chars: &str) -> Option<Token> {
        self.two_char.get(chars).cloned()
    }

    // Structural token setters (called by syntax::structural module)
    pub fn set_lparen(&mut self, token_kind: &'static str) {
        self.lparen = Some(token_kind);
    }

    pub fn set_rparen(&mut self, token_kind: &'static str) {
        self.rparen = Some(token_kind);
    }

    pub fn set_newline(&mut self, token_kind: &'static str) {
        self.newline = Some(token_kind);
    }

    pub fn set_indent(&mut self, token_kind: &'static str) {
        self.indent = Some(token_kind);
    }

    pub fn set_dedent(&mut self, token_kind: &'static str) {
        self.dedent = Some(token_kind);
    }

    pub fn set_eof(&mut self, token_kind: &'static str) {
        self.eof = Some(token_kind);
    }

    // Structural token getters (used by lexer and parser)
    pub fn lparen(&self) -> &'static str {
        self.lparen.expect("LPAREN token not registered")
    }

    pub fn rparen(&self) -> &'static str {
        self.rparen.expect("RPAREN token not registered")
    }

    pub fn newline(&self) -> &'static str {
        self.newline.expect("NEWLINE token not registered")
    }

    pub fn indent(&self) -> &'static str {
        self.indent.expect("INDENT token not registered")
    }

    pub fn dedent(&self) -> &'static str {
        self.dedent.expect("DEDENT token not registered")
    }

    pub fn eof(&self) -> &'static str {
        self.eof.expect("EOF token not registered")
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

