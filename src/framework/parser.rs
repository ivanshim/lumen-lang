// src/framework/parser.rs
//
// Syntax delegation only.
// Knows NOTHING about operators, keywords, or language features.

use crate::framework::ast::{ExprNode, Program, StmtNode};
use crate::framework::lexer::{lex, SpannedToken, Token};
use crate::framework::registry::{err_at, LumenResult, Precedence, Registry};

/// Structural tokens configuration for a language.
/// Languages pass these to the parser to define what tokens represent structure.
#[derive(Clone, Copy)]
pub struct StructuralTokens {
    pub newline: &'static str,
    pub indent: &'static str,
    pub dedent: &'static str,
    pub eof: &'static str,
}

pub struct Parser<'a> {
    pub reg: &'a Registry,
    pub toks: Vec<SpannedToken>,
    pub i: usize,
    pub structural: Option<StructuralTokens>,
}

impl<'a> Parser<'a> {
    pub fn new(reg: &'a Registry, source: &str) -> LumenResult<Self> {
        Ok(Self {
            reg,
            toks: lex(source, &reg.tokens)?,
            i: 0,
            structural: None,
        })
    }

    /// Create parser with pre-tokenized token stream and structural token config.
    /// Used when language-specific token processing is needed (e.g., indentation).
    pub fn new_with_tokens(reg: &'a Registry, toks: Vec<SpannedToken>, structural: StructuralTokens) -> LumenResult<Self> {
        Ok(Self { reg, toks, i: 0, structural: Some(structural) })
    }

    pub fn position(&self) -> (usize, usize) {
        let t = self.toks.get(self.i).unwrap();
        (t.line, t.col)
    }

    pub fn peek(&self) -> &Token {
        &self.toks[self.i].tok
    }

    pub fn peek_n(&self, n: usize) -> Option<&Token> {
        self.toks.get(self.i + n).map(|t| &t.tok)
    }

    pub fn advance(&mut self) -> Token {
        let t = self.toks[self.i].tok.clone();
        self.i += 1;
        t
    }

    pub fn consume_newlines(&mut self) {
        if let Some(st) = self.structural {
            while matches!(self.peek(), Token::Feature(k) if *k == st.newline) {
                self.advance();
            }
        }
    }

    pub fn parse_program(&mut self) -> LumenResult<Program> {
        let mut stmts = Vec::new();
        self.consume_newlines();

        let eof = self.structural.map(|st| st.eof).unwrap_or("EOF");
        while !matches!(self.peek(), Token::Feature(k) if *k == eof) {
            let stmt = self
                .reg
                .find_stmt(self)
                .ok_or_else(|| err_at(self, "Unknown statement"))?
                .parse(self)?;

            stmts.push(stmt);
            self.consume_newlines();
        }

        Ok(Program::new(stmts))
    }

    pub fn parse_expr(&mut self) -> LumenResult<Box<dyn ExprNode>> {
        self.parse_expr_prec(Precedence::Lowest)
    }

    pub fn parse_expr_prec(&mut self, min_prec: Precedence) -> LumenResult<Box<dyn ExprNode>> {
        let prefix = self
            .reg
            .find_prefix(self)
            .ok_or_else(|| err_at(self, "Unknown expression"))?;

        let mut left = prefix.parse(self)?;

        loop {
            let infix = match self.reg.find_infix(self) {
                Some(i) => i,
                None => break,
            };

            if infix.precedence() < min_prec {
                break;
            }

            left = infix.parse(self, left)?;
        }

        Ok(left)
    }

    pub fn parse_block(&mut self) -> LumenResult<Vec<Box<dyn StmtNode>>> {
        self.consume_newlines();

        let st = self.structural.ok_or_else(|| err_at(self, "Structural tokens not configured for language"))?;

        match self.advance() {
            Token::Feature(k) if k == st.indent => {}
            _ => return Err(err_at(self, "Expected INDENT")),
        }

        self.consume_newlines();

        let mut stmts = Vec::new();

        while !matches!(self.peek(), Token::Feature(k) if *k == st.dedent || *k == st.eof) {
            let s = self
                .reg
                .find_stmt(self)
                .ok_or_else(|| err_at(self, "Unknown statement in block"))?
                .parse(self)?;

            stmts.push(s);
            self.consume_newlines();
        }

        match self.advance() {
            Token::Feature(k) if k == st.dedent => Ok(stmts),
            _ => Err(err_at(self, "Expected DEDENT")),
        }
    }
}
