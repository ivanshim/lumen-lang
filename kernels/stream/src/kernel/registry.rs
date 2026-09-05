// Kernel registry: the token registry and the shared error type.
//
// The kernel owns no parsing algorithms and no handler traits. Languages
// register the token definitions the lexer should segment on, and supply
// the byte class used for keyword word-boundary checks. Everything else
// (precedence, handlers, dispatch) lives in the language modules.
//
// Span { start, end } (byte offsets) is the authoritative source location.
// line/col are diagnostic-only metadata for error messages.

use crate::kernel::parser::Parser;

pub type KernelResult<T> = Result<T, String>;

/// Format a parse error with diagnostic position information.
pub fn err_at(parser: &Parser, msg: &str) -> String {
    let (line, col) = parser.position();
    format!("ParseError at {line}:{col}: {msg}")
}

// --------------------
// Token Definition
// --------------------

/// One lexeme the lexer should recognise as a unit.
#[derive(Debug, Clone)]
pub struct TokenDefinition {
    /// The lexeme string to recognise.
    pub lexeme: &'static str,
    /// Whether this token must be surrounded by non-identifier bytes.
    /// Which bytes count as identifier bytes is supplied by the language
    /// through `TokenRegistry::set_identifier_bytes`.
    pub requires_word_boundary: bool,
}

impl TokenDefinition {
    /// A token recognised wherever it appears.
    pub fn recognize(lexeme: &'static str) -> Self {
        Self { lexeme, requires_word_boundary: false }
    }

    /// A keyword-like token that must not match inside a longer word.
    pub fn keyword(lexeme: &'static str) -> Self {
        Self { lexeme, requires_word_boundary: true }
    }
}

// --------------------
// Token Registry
// --------------------

/// Registry of token definitions used by the lexer.
pub struct TokenRegistry {
    token_defs: Vec<TokenDefinition>,
    /// Multi-character lexemes in descending length order (maximal munch).
    multichar_lexemes: Vec<&'static str>,
    /// Lexemes that require word boundaries.
    word_boundary_lexemes: Vec<&'static str>,
    /// Language-supplied predicate: which bytes belong to a word.
    /// Absent means no byte does, so boundary checks never suppress a match.
    identifier_bytes: Option<fn(u8) -> bool>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            token_defs: Vec::new(),
            multichar_lexemes: Vec::new(),
            word_boundary_lexemes: Vec::new(),
            identifier_bytes: None,
        }
    }

    /// Replace all token definitions and rebuild the lexer caches.
    pub fn set_token_definitions(&mut self, defs: Vec<TokenDefinition>) {
        self.token_defs = defs;
        self.rebuild_caches();
    }

    /// Tell the lexer which bytes form words, for keyword boundary checks.
    pub fn set_identifier_bytes(&mut self, pred: fn(u8) -> bool) {
        self.identifier_bytes = Some(pred);
    }

    /// Multi-character lexemes in descending length order.
    pub fn multichar_lexemes(&self) -> &[&'static str] {
        &self.multichar_lexemes
    }

    /// Whether the lexeme must be surrounded by non-word bytes.
    pub fn requires_word_boundary(&self, lexeme: &str) -> bool {
        self.word_boundary_lexemes.iter().any(|&wb| wb == lexeme)
    }

    /// Whether a byte belongs to a word, per the language's definition.
    pub fn is_identifier_byte(&self, b: u8) -> bool {
        self.identifier_bytes.map_or(false, |pred| pred(b))
    }

    fn rebuild_caches(&mut self) {
        let mut multichar = Vec::new();
        let mut word_boundary = Vec::new();
        for def in &self.token_defs {
            if def.lexeme.len() > 1 {
                multichar.push(def.lexeme);
            }
            if def.requires_word_boundary {
                word_boundary.push(def.lexeme);
            }
        }
        multichar.sort_by(|a, b| b.len().cmp(&a.len()));
        self.multichar_lexemes = multichar;
        self.word_boundary_lexemes = word_boundary;
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}
