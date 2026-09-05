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
// Token skipping (whitespace, comments) is handled by language-specific extension traits.

use crate::kernel::lexer::{SpannedToken, Token};
use crate::kernel::registry::{LumenResult, TokenRegistry};

pub struct Parser<'a> {
    pub toks: Vec<SpannedToken>,
    pub i: usize,
    _token_registry: std::marker::PhantomData<&'a TokenRegistry>,
}

impl<'a> Parser<'a> {
    /// Create parser with pre-tokenized token stream.
    /// Used when language-specific token processing is needed.
    pub fn new_with_tokens(toks: Vec<SpannedToken>, _token_registry: &'a TokenRegistry) -> LumenResult<Self> {
        Ok(Self {
            toks,
            i: 0,
            _token_registry: std::marker::PhantomData,
        })
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
}

