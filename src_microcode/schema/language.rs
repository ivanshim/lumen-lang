// Language schema type definitions
//
// LanguageSchema is purely declarative and contains NO executable logic.
// It specifies lexical rules, structural rules, token roles, operators, and statements.

use super::structure::StatementPattern;
use super::operator::{OperatorInfo, UnaryOperatorInfo};
use std::collections::HashMap;

/// Complete declarative schema for a language
///
/// Each language is defined entirely as data: no parsing logic, no semantic interpretation,
/// just tables and rules that the kernel uses to understand the language.
///
/// INVARIANT: This struct contains ONLY data. All logic is in the kernel.
#[derive(Debug, Clone)]
pub struct LanguageSchema {
    /// Maps statement keyword to its pattern (e.g., "if" -> if_pattern)
    pub statements: HashMap<String, StatementPattern>,

    /// Maps binary operator lexeme to its properties (precedence, associativity)
    pub binary_ops: HashMap<String, OperatorInfo>,

    /// Maps unary operator lexeme to its properties (precedence, position)
    pub unary_ops: HashMap<String, UnaryOperatorInfo>,

    /// All keywords in the language (e.g., ["if", "while", "for", "class"])
    pub keywords: Vec<String>,

    /// Multi-character lexemes (e.g., ["==", "!=", "<=", ">=", "->", ":="])
    /// Must be sorted by length descending for maximal-munch lexing
    pub multichar_lexemes: Vec<&'static str>,

    /// Lexemes that terminate statements (e.g., [";", "\n"])
    pub statement_terminators: Vec<String>,

    /// Character(s) that open a block (e.g., "{", "begin")
    pub block_open: String,

    /// Character(s) that close a block (e.g., "}", "end")
    pub block_close: String,

    /// Optional syntax for external/foreign calls
    pub extern_syntax: Option<ExternSyntax>,
}

/// Syntax for external/foreign function calls
///
/// Example for Lumen: extern foo:capability(arg1, arg2)
/// - keyword: "extern"
/// - selector_start: " "
/// - selector_end: ":"
/// - args_open: "("
/// - args_close: ")"
/// - arg_separator: ","
#[derive(Debug, Clone)]
pub struct ExternSyntax {
    /// Keyword that introduces extern calls (e.g., "extern", "ffi")
    pub keyword: String,

    /// Character(s) before the capability selector
    pub selector_start: String,

    /// Character(s) separating capability name from arguments
    pub selector_end: String,

    /// Character(s) that open the argument list
    pub args_open: String,

    /// Character(s) that close the argument list
    pub args_close: String,

    /// Character(s) that separate arguments
    pub arg_separator: String,
}

impl LanguageSchema {
    /// Check if a lexeme is a statement keyword (e.g., "if", "while", "print")
    pub fn is_statement_keyword(&self, lexeme: &str) -> bool {
        self.statements.contains_key(lexeme)
    }

    /// Check if a lexeme is any keyword
    pub fn is_keyword(&self, lexeme: &str) -> bool {
        self.keywords.contains(&lexeme.to_string())
    }

    /// Check if a lexeme terminates a statement
    pub fn is_terminator(&self, lexeme: &str) -> bool {
        self.statement_terminators.contains(&lexeme.to_string())
    }

    /// Get binary operator info by lexeme
    pub fn get_binary_op(&self, lexeme: &str) -> Option<&OperatorInfo> {
        self.binary_ops.get(lexeme)
    }

    /// Get unary operator info by lexeme
    pub fn get_unary_op(&self, lexeme: &str) -> Option<&UnaryOperatorInfo> {
        self.unary_ops.get(lexeme)
    }
}
