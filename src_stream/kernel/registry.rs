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
use std::collections::HashSet;

pub type LumenResult<T> = Result<T, String>;

/// Format a parse error with diagnostic position information.
/// DIAGNOSTIC FUNCTION: Uses line/col (derived from source) only for human-readable error messages.
/// line/col are NOT used by parsing logic - all core logic uses Span.
pub fn err_at(parser: &Parser, msg: &str) -> String {
    let (line, col) = parser.position();
    format!("ParseError at {line}:{col}: {msg}")
}

// --------------------
// Token Definition (Unified Token Metadata)
// --------------------

/// Complete definition of a token with all its properties.
/// Unifies multichar lexemes and skip behavior under one structure.
#[derive(Debug, Clone)]
pub struct TokenDefinition {
    /// The lexeme string to recognize
    pub lexeme: &'static str,
    /// Whether this token should be skipped during parsing
    pub skip_during_parsing: bool,
    /// Whether this token requires word boundaries (identifier-safe keywords)
    /// Whether this token requires word boundaries (e.g., keywords shouldn't match inside identifiers)
    pub requires_word_boundary: bool,
}

impl TokenDefinition {
    pub fn new(lexeme: &'static str, skip_during_parsing: bool) -> Self {
        Self {
            lexeme,
            skip_during_parsing,
            requires_word_boundary: false,
        }
    }

    /// Create a token that should be recognized but not skipped
    pub fn recognize(lexeme: &'static str) -> Self {
        Self {
            lexeme,
            skip_during_parsing: false,
            requires_word_boundary: false,
        }
    }

    /// Create a token that should be skipped during parsing
    pub fn skip(lexeme: &'static str) -> Self {
        Self {
            lexeme,
            skip_during_parsing: true,
            requires_word_boundary: false,
        }
    }

    /// Create a keyword token that must respect identifier word boundaries
    /// Create a keyword token that requires word boundaries
    pub fn keyword(lexeme: &'static str) -> Self {
        Self {
            lexeme,
            skip_during_parsing: false,
            requires_word_boundary: true,
        }
    }
}

// --------------------
// Token Registry (Pure Transport Layer)
// --------------------

/// Registry of token definitions for lexical analysis and parsing.
/// Single source of truth for token behavior.
pub struct TokenRegistry {
    // All token definitions with their properties
    token_defs: Vec<TokenDefinition>,
    // Cached: Multi-character lexeme sequences for maximal-munch segmentation
    // Stored in descending length order for proper maximal-munch
    multichar_lexemes: Vec<&'static str>,
    // Cached: Tokens that should be skipped during parsing
    skip_tokens: Vec<&'static str>,
    // Cached: Tokens that require word boundaries (keywords that shouldn't match inside identifiers)
    word_boundary_lexemes: Vec<&'static str>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            token_defs: Vec::new(),
            multichar_lexemes: Vec::new(),
            skip_tokens: Vec::new(),
            word_boundary_lexemes: Vec::new(),
        }
    }

    /// Set tokens using unified TokenDefinition approach.
    /// Languages call this during initialization to register all their tokens.
    /// Internally extracts and caches multichar lexemes and skip tokens for efficiency.
    pub fn set_token_definitions(&mut self, defs: Vec<TokenDefinition>) {
        self.token_defs = defs;
        self.rebuild_caches();
    }

    /// Get the multi-character lexemes in descending length order.
    /// Used by the lexer for maximal-munch segmentation.
    pub fn multichar_lexemes(&self) -> &[&'static str] {
        &self.multichar_lexemes
    }

    /// Get the tokens that should be skipped during parsing.
    /// Used by language-specific parser extension traits.
    pub fn skip_tokens(&self) -> &[&'static str] {
        &self.skip_tokens
    }

    /// Check if a specific lexeme should be skipped during parsing.
    pub fn is_skip_token(&self, lexeme: &str) -> bool {
        self.skip_tokens.contains(&lexeme)
    }

    /// Get the tokens that require word boundaries.
    /// Used by the lexer to prevent keyword matching inside identifiers.
    pub fn word_boundary_lexemes(&self) -> &[&'static str] {
        &self.word_boundary_lexemes
    }

    /// Check if the lexeme requires surrounding word boundaries.
    /// Used by the lexer to avoid splitting identifiers that contain keywords.
    pub fn requires_word_boundary(&self, lexeme: &str) -> bool {
        self.word_boundary_lexemes.iter().any(|&wb| wb == lexeme)
    }

    /// Get all token definitions.
    /// Used for inspection and debugging.
    pub fn token_definitions(&self) -> &[TokenDefinition] {
        &self.token_defs
    }

    /// Rebuild internal caches from token definitions
    fn rebuild_caches(&mut self) {
        let mut multichar = Vec::new();
        let mut skip = Vec::new();
        let mut word_boundary = Vec::new();

        for def in &self.token_defs {
            // Extract multichar lexemes (lexer concern)
            if def.lexeme.len() > 1 {
                multichar.push(def.lexeme);
            }

            // Extract skip tokens (parser concern)
            if def.skip_during_parsing {
                skip.push(def.lexeme);
            }

            // Extract word boundary tokens (lexer concern)
            if def.requires_word_boundary {
                word_boundary.push(def.lexeme);
            }
        }

        // Sort by descending length for proper maximal-munch
        multichar.sort_by(|a, b| b.len().cmp(&a.len()));

        self.multichar_lexemes = multichar;
        self.skip_tokens = skip;
        self.word_boundary_lexemes = word_boundary;
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

