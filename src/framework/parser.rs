// src/framework/parser.rs
//
// Syntax delegation only.
// Knows NOTHING about operators, keywords, or language features.

use crate::framework::ast::{ExprNode, Program, StmtNode};
use crate::framework::lexer::{lex, SpannedToken, Token};
use crate::framework::registry::{err_at, LumenResult, Precedence, Registry};

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
        let newline = self.reg.tokens.newline();
        while matches!(self.peek(), Token::Feature(k) if *k == newline) {
            self.advance();
        }
    }

    pub fn parse_program(&mut self) -> LumenResult<Program> {
        let mut stmts = Vec::new();
        self.consume_newlines();

        let eof = self.reg.tokens.eof();
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

        let indent = self.reg.tokens.indent();
        match self.advance() {
            Token::Feature(k) if k == indent => {}
            _ => return Err(err_at(self, "Expected INDENT")),
        }

        self.consume_newlines();

        let mut stmts = Vec::new();

        let dedent = self.reg.tokens.dedent();
        let eof = self.reg.tokens.eof();
        while !matches!(self.peek(), Token::Feature(k) if k == &dedent || k == &eof) {
            let s = self
                .reg
                .find_stmt(self)
                .ok_or_else(|| err_at(self, "Unknown statement in block"))?
                .parse(self)?;

            stmts.push(s);
            self.consume_newlines();
        }

        match self.advance() {
            Token::Feature(k) if k == dedent => Ok(stmts),
            _ => Err(err_at(self, "Expected DEDENT")),
        }
    }
}
