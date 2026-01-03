// Language schema definitions
//
// The schema system allows languages to be expressed entirely as declarative data.
// All parsing decisions in the kernel are driven by these tables.
// No semantic assumptions - the kernel is pure data interpreter.

pub mod statements;
pub mod operators;

pub use statements::{StatementPattern, StatementRole, PatternBuilder};
pub use operators::{OperatorInfo, Associativity, UnaryOperatorInfo};

use std::collections::HashMap;

/// Complete declarative schema for a language
#[derive(Debug, Clone)]
pub struct LanguageSchema {
    /// All statement patterns indexed by keyword
    pub statements: HashMap<String, StatementPattern>,

    /// All binary operators indexed by lexeme
    pub binary_ops: HashMap<String, OperatorInfo>,

    /// All unary operators indexed by lexeme
    pub unary_ops: HashMap<String, UnaryOperatorInfo>,

    /// All reserved keywords (prevents them from being identifiers)
    pub keywords: Vec<String>,

    /// Multi-character sequences to recognize (e.g., "<=", "==", "->")
    /// Lexer uses these for maximal-munch matching
    pub multichar_lexemes: Vec<&'static str>,

    /// Lexemes that terminate statements (e.g., ";", newline)
    pub statement_terminators: Vec<String>,

    /// Opening brace for blocks
    pub block_open: String,

    /// Closing brace for blocks
    pub block_close: String,

    /// How to parse extern calls (if supported)
    pub extern_syntax: Option<ExternSyntax>,
}

/// Extern call syntax if the language supports it
#[derive(Debug, Clone)]
pub struct ExternSyntax {
    /// Keyword that starts extern call (e.g., "extern")
    pub keyword: String,

    /// Character that starts the selector (e.g., "\"")
    pub selector_start: String,

    /// Character that ends the selector (e.g., "\"")
    pub selector_end: String,

    /// Character that opens argument list (e.g., "(")
    pub args_open: String,

    /// Character that closes argument list (e.g., ")")
    pub args_close: String,

    /// Character separating arguments (e.g., ",")
    pub arg_separator: String,
}

impl LanguageSchema {
    /// Check if a lexeme is a statement keyword
    pub fn is_statement_keyword(&self, lexeme: &str) -> bool {
        self.statements.contains_key(lexeme)
    }

    /// Check if a lexeme is a reserved keyword
    pub fn is_keyword(&self, lexeme: &str) -> bool {
        self.keywords.contains(&lexeme.to_string())
    }

    /// Check if a lexeme terminates a statement
    pub fn is_terminator(&self, lexeme: &str) -> bool {
        self.statement_terminators.contains(&lexeme.to_string())
    }

    /// Get binary operator info if it exists
    pub fn get_binary_op(&self, lexeme: &str) -> Option<&OperatorInfo> {
        self.binary_ops.get(lexeme)
    }

    /// Get unary operator info if it exists
    pub fn get_unary_op(&self, lexeme: &str) -> Option<&UnaryOperatorInfo> {
        self.unary_ops.get(lexeme)
    }
}
