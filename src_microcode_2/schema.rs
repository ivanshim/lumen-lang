// Language schema: declarative syntax and semantics
//
// LanguageSchema contains ONLY data - no executable logic.
// All interpretation is done by the kernel stages.

use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct LanguageSchema {
    /// Multi-character lexemes (keywords, operators) that should be recognized as units
    pub multichar_lexemes: Vec<&'static str>,

    /// Keywords that require word boundaries (can't appear inside identifiers)
    pub word_boundary_keywords: Vec<&'static str>,

    /// Statement terminators (e.g., ";", "\n")
    pub terminators: Vec<&'static str>,
}

impl LanguageSchema {
    /// Create a new, empty schema
    pub fn new() -> Self {
        LanguageSchema {
            multichar_lexemes: Vec::new(),
            word_boundary_keywords: Vec::new(),
            terminators: Vec::new(),
        }
    }

    /// Check if a word is a keyword that requires word boundaries
    pub fn is_word_boundary_keyword(&self, word: &str) -> bool {
        self.word_boundary_keywords.contains(&word)
    }

    /// Check if a token is a terminator
    pub fn is_terminator(&self, lexeme: &str) -> bool {
        self.terminators.contains(&lexeme)
    }

    /// Get the set of multichar lexemes for fast lookup
    pub fn multichar_set(&self) -> HashSet<&'static str> {
        self.multichar_lexemes.iter().copied().collect()
    }
}

impl Default for LanguageSchema {
    fn default() -> Self {
        Self::new()
    }
}
