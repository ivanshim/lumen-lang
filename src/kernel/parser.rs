// src/framework/parser.rs
//
// Pure generic syntax parsing.
// Zero language-specific assumptions.
// Delegates all parsing decisions to registered handlers.

use crate::kernel::ast::{ExprNode, Program};
use crate::kernel::lexer::{lex, SpannedToken, Token};
use crate::kernel::registry::{err_at, LumenResult, Precedence, Registry};

pub struct Parser<'a> {
    pub reg: &'a Registry,
    pub toks: Vec<SpannedToken>,
    pub i: usize,
}

impl<'a> Parser<'a> {
    pub fn new(reg: &'a Registry, source: &str) -> LumenResult<Self> {
        Ok(Self {
            reg,
            toks: lex(source, &reg.tokens)?,
            i: 0,
        })
    }

    /// Create parser with pre-tokenized token stream.
    /// Used when language-specific token processing is needed.
    pub fn new_with_tokens(reg: &'a Registry, toks: Vec<SpannedToken>) -> LumenResult<Self> {
        Ok(Self { reg, toks, i: 0 })
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

    /// Generic program parsing - just dispatches to handlers.
    /// Languages define their own parse_program() if they need custom behavior.
    pub fn parse_program(&mut self) -> LumenResult<Program> {
        let mut stmts = Vec::new();

        while self.i < self.toks.len() {
            let stmt = self
                .reg
                .find_stmt(self)
                .ok_or_else(|| err_at(self, "Unknown statement"))?
                .parse(self)?;

            stmts.push(stmt);
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
}
