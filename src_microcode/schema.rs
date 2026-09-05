// Language schema: declarative syntax and semantics
//
// LanguageSchema contains ONLY data loaded from YAML specifications.
// All interpretation is done by the kernel stages.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct OperatorInfo {
    pub precedence: f32,
    pub associativity: Associativity,
    pub short_circuit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Associativity {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct UnaryOperatorInfo {
    pub precedence: f32,
    pub position: UnaryPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryPosition {
    Prefix,
    Postfix,
}

/// Comprehensive language schema loaded from YAML
#[derive(Debug, Clone)]
pub struct LanguageSchema {
    /// Multi-character lexemes (keywords, operators) that should be recognized as units
    pub multichar_lexemes: Vec<&'static str>,

    /// Keywords that require word boundaries
    pub word_boundary_keywords: Vec<&'static str>,

    /// Statement terminators (e.g., ";", "\n")
    pub terminators: Vec<&'static str>,

    /// Binary operators with precedence and associativity
    pub binary_operators: HashMap<String, OperatorInfo>,

    /// Unary operators with precedence and position
    pub unary_operators: HashMap<String, UnaryOperatorInfo>,

    /// All keywords in the language
    pub keywords: Vec<String>,

    /// Indentation settings
    pub indentation_size: usize,
    pub indentation_char: char,

    /// Block structure markers (e.g., ":" for Lumen)
    pub block_open_marker: String,
    pub block_close_marker: String,
}

impl LanguageSchema {
    /// Create a new, empty schema
    pub fn new() -> Self {
        LanguageSchema {
            multichar_lexemes: Vec::new(),
            word_boundary_keywords: Vec::new(),
            terminators: Vec::new(),
            binary_operators: HashMap::new(),
            unary_operators: HashMap::new(),
            keywords: Vec::new(),
            indentation_size: 4,
            indentation_char: ' ',
            block_open_marker: ":".to_string(),
            block_close_marker: "DEDENT".to_string(),
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

    /// Get binary operator info
    pub fn get_binary_operator(&self, op: &str) -> Option<&OperatorInfo> {
        self.binary_operators.get(op)
    }

    /// Get unary operator info
    pub fn get_unary_operator(&self, op: &str) -> Option<&UnaryOperatorInfo> {
        self.unary_operators.get(op)
    }

    /// Check if operator is left-associative
    pub fn is_left_associative(&self, op: &str) -> bool {
        self.binary_operators
            .get(op)
            .map(|info| info.associativity == Associativity::Left)
            .unwrap_or(false)
    }

    /// Check if operator has short-circuit evaluation
    pub fn is_short_circuit(&self, op: &str) -> bool {
        self.binary_operators
            .get(op)
            .map(|info| info.short_circuit)
            .unwrap_or(false)
    }
}

impl Default for LanguageSchema {
    fn default() -> Self {
        Self::new()
    }
}
