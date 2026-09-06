// Kernel registry: the token registry and the shared error type.
//
// The kernel owns no parsing algorithms and no handler traits. Languages
// register the token definitions the lexer should segment on, and supply
// the character class used for keyword word-boundary checks. Everything else
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
    pub lexeme: String,
    /// Whether this token must be surrounded by non-word characters.
    /// Which characters count as word characters is supplied by the
    /// language through `TokenRegistry::set_word_chars`.
    pub requires_word_boundary: bool,
}

impl TokenDefinition {
    /// A token recognised wherever it appears.
    pub fn recognize(lexeme: impl Into<String>) -> Self {
        Self { lexeme: lexeme.into(), requires_word_boundary: false }
    }

    /// A keyword-like token that must not match inside a longer word.
    pub fn keyword(lexeme: impl Into<String>) -> Self {
        Self { lexeme: lexeme.into(), requires_word_boundary: true }
    }
}

// --------------------
// Token Registry
// --------------------

/// Registry of token definitions used by the lexer.
pub struct TokenRegistry {
    token_defs: Vec<TokenDefinition>,
    /// Multi-character lexemes in descending length order (maximal munch).
    multichar_lexemes: Vec<String>,
    /// Lexemes that require word boundaries.
    word_boundary_lexemes: Vec<String>,
    /// Language-supplied predicate: which characters belong to a word.
    /// Absent means none does, so boundary checks never suppress a match.
    word_chars: Option<fn(char) -> bool>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            token_defs: Vec::new(),
            multichar_lexemes: Vec::new(),
            word_boundary_lexemes: Vec::new(),
            word_chars: None,
        }
    }

    /// Replace all token definitions and rebuild the lexer caches.
    pub fn set_token_definitions(&mut self, defs: Vec<TokenDefinition>) {
        self.token_defs = defs;
        self.rebuild_caches();
    }

    /// Tell the lexer which characters form words, for keyword boundary checks.
    pub fn set_word_chars(&mut self, pred: fn(char) -> bool) {
        self.word_chars = Some(pred);
    }

    /// Multi-character lexemes in descending length order.
    pub fn multichar_lexemes(&self) -> &[String] {
        &self.multichar_lexemes
    }

    /// Whether the lexeme must be surrounded by non-word characters.
    pub fn requires_word_boundary(&self, lexeme: &str) -> bool {
        self.word_boundary_lexemes.iter().any(|wb| wb == lexeme)
    }

    /// Whether a character belongs to a word, per the language's definition.
    pub fn is_word_char(&self, c: char) -> bool {
        self.word_chars.map_or(false, |pred| pred(c))
    }

    fn rebuild_caches(&mut self) {
        let mut multichar = Vec::new();
        let mut word_boundary = Vec::new();
        for def in &self.token_defs {
            if def.lexeme.len() > 1 {
                multichar.push(def.lexeme.clone());
            }
            if def.requires_word_boundary {
                word_boundary.push(def.lexeme.clone());
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
