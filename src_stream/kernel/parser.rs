// src/framework/parser.rs
//
// Pure generic syntax parsing.
// Zero language-specific assumptions.
// Delegates all parsing decisions to registered handlers.
//
// ARCHITECTURE:
// Span { start, end } (byte offsets) is the AUTHORITATIVE source-location mechanism.
// All parsing, AST construction, and evaluation logic must use Span.
// line/col are DIAGNOSTIC-ONLY metadata (for error messages).
// This parser is language-agnostic and makes no semantic assumptions.

use crate::kernel::ast::{ExprNode, Program};
use crate::kernel::lexer::{lex, SpannedToken, Token, Span};
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

    /// Get the byte span of the current token (AUTHORITATIVE position).
    /// Span is the canonical coordinate system for all core logic.
    pub fn current_span(&self) -> Span {
        let t = self.toks.get(self.i).unwrap();
        t.tok.span
    }

    /// Get diagnostic line/column position of current token.
    /// DIAGNOSTIC ONLY - derived from source; not used by parsing logic.
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

    /// Skip whitespace tokens (language-level convenience).
    /// Since the kernel lexer is now fully agnostic and emits all characters,
    /// including whitespace, this helper skips over single-character space/tab/newline tokens.
    /// Languages that care about whitespace (e.g., Lumen for indentation) handle it
    /// at their parsing layer, not by using this method.
    pub fn skip_whitespace(&mut self) {
        while self.i < self.toks.len() {
            let lexeme = &self.toks[self.i].tok.lexeme;
            if lexeme.len() == 1 {
                let ch = lexeme.as_bytes()[0];
                if ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r' {
                    self.i += 1;
                    continue;
                }
            }
            break;
        }
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
        self.skip_whitespace();

        let prefix = self
            .reg
            .find_prefix(self)
            .ok_or_else(|| err_at(self, "Unknown expression"))?;

        let mut left = prefix.parse(self)?;

        loop {
            // Skip whitespace before checking for infix operators
            self.skip_whitespace();

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
