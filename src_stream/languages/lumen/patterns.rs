// Lumen pattern definitions
//
// Patterns are language-specific declarations of what patterns Lumen recognizes.
// Each module in Lumen provides an array of strings it recognizes.

/// A set of patterns that a Lumen module can recognize.
/// Each module exports the literal strings and character classes it handles.
pub struct PatternSet {
    /// Literal string patterns: "if", "else", "+", "-", "print", etc.
    pub literals: Vec<&'static str>,

    /// Character class names this module handles: "digit", "letter", "quote", etc.
    pub char_classes: Vec<&'static str>,

    /// Structural pattern names: "newline", "indent", "dedent", etc.
    /// Language-specific structural elements.
    pub structural: Vec<&'static str>,
}

impl PatternSet {
    pub fn new() -> Self {
        Self {
            literals: Vec::new(),
            char_classes: Vec::new(),
            structural: Vec::new(),
        }
    }

    pub fn with_literals(mut self, literals: Vec<&'static str>) -> Self {
        self.literals = literals;
        self
    }

    pub fn with_char_classes(mut self, classes: Vec<&'static str>) -> Self {
        self.char_classes = classes;
        self
    }

    pub fn with_structural(mut self, structural: Vec<&'static str>) -> Self {
        self.structural = structural;
        self
    }

    /// Merge multiple pattern sets into one
    pub fn merge(sets: Vec<PatternSet>) -> Self {
        let mut merged = PatternSet::new();
        for set in sets {
            merged.literals.extend(set.literals);
            merged.char_classes.extend(set.char_classes);
            merged.structural.extend(set.structural);
        }
        merged
    }

    /// Check if a literal pattern is registered
    pub fn has_literal(&self, s: &str) -> bool {
        self.literals.contains(&s)
    }

    /// Check if a character class is registered
    pub fn has_char_class(&self, class: &str) -> bool {
        self.char_classes.contains(&class)
    }

    /// Check if a structural pattern is registered
    pub fn has_structural(&self, s: &str) -> bool {
        self.structural.contains(&s)
    }
}

impl Default for PatternSet {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Standard Character Classes (Lumen-defined)
// ============================================================================

pub mod char_classes {
    /// ASCII digit: 0-9
    pub const DIGIT: &str = "digit";

    /// ASCII letter: a-z, A-Z
    pub const LETTER: &str = "letter";

    /// Identifier start: letter or underscore
    pub const IDENT_START: &str = "ident_start";

    /// Identifier character: letter, digit, underscore
    pub const IDENT_CHAR: &str = "ident_char";

    /// Whitespace (space, tab, etc.)
    pub const WHITESPACE: &str = "whitespace";

    /// Quote character for string literals
    pub const QUOTE: &str = "quote";
}
