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

use crate::kernel::ast::{ExprNode};
use crate::kernel::lexer::{lex, SpannedToken, Token, Span};
use crate::kernel::registry::{err_at, LumenResult, TokenRegistry};

pub struct Parser<'a> {
    pub toks: Vec<SpannedToken>,
    pub i: usize,
    pub token_registry: &'a TokenRegistry,
}

impl<'a> Parser<'a> {
    /// Create parser from source and token registry
    pub fn new(source: &str, token_registry: &'a TokenRegistry) -> LumenResult<Self> {
        Ok(Self {
            toks: lex(source, token_registry)?,
            i: 0,
            token_registry,
        })
    }

    /// Create parser with pre-tokenized token stream.
    /// Used when language-specific token processing is needed.
    pub fn new_with_tokens(toks: Vec<SpannedToken>, token_registry: &'a TokenRegistry) -> LumenResult<Self> {
        Ok(Self {
            toks,
            i: 0,
            token_registry,
        })
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

    pub fn prev_span(&self) -> Span {
        if self.i > 0 {
            let t = &self.toks[self.i - 1];
            t.tok.span
        } else {
            Span::new(0, 0)
        }
    }
}

