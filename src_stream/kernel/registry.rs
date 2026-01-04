// src/framework/registry.rs
//
// Kernel registry utilities.
// Provides basic error handling and token registry for stream processing.
//
// AUTHORITY:
// - Span { start, end } (byte offsets) is the AUTHORITATIVE source-location mechanism
// - All parsing and AST construction must use Span
// - line/col are DIAGNOSTIC-ONLY (derived metadata, only for error messages)
//
// ARCHITECTURE:
// - Kernel provides TokenRegistry only (pure token management)
// - Kernel contains ALL parsing algorithms (expression, statement, precedence-climbing)
// - Languages define their own handler trait types (ExprPrefix, ExprInfix, StmtHandler)
// - Languages define their own Precedence types
// - Languages manage all dispatch and handler logic

use crate::kernel::parser::Parser;

pub type LumenResult<T> = Result<T, String>;

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

/// Registry of multi-character lexeme sequences for maximal-munch segmentation.
/// This is the only kernel-level registry - all other registries are language-specific.
pub struct TokenRegistry {
    // Multi-character lexeme sequences for maximal-munch segmentation
    // Stored in descending length order for proper maximal-munch
    multichar_lexemes: Vec<&'static str>,
    // Tokens that should be skipped during parsing (whitespace, comments, etc.)
    skip_tokens: Vec<&'static str>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            multichar_lexemes: Vec::new(),
            skip_tokens: Vec::new(),
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

    /// Set which lexemes should be skipped during parsing.
    /// These are typically whitespace characters or comment tokens.
    pub fn set_skip_tokens(&mut self, tokens: Vec<&'static str>) {
        self.skip_tokens = tokens;
    }

    /// Get the tokens that should be skipped during parsing
    pub fn skip_tokens(&self) -> &[&'static str] {
        &self.skip_tokens
    }

    /// Check if a lexeme should be skipped
    pub fn is_skip_token(&self, lexeme: &str) -> bool {
        self.skip_tokens.contains(&lexeme)
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Handler traits are now COMPLETELY language-specific and defined in language modules.
// The kernel provides NO trait definitions for handlers - each language defines its own
// handler types (ExprPrefix, ExprInfix, StmtHandler) and precedence types.

