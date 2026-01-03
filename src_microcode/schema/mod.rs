// Declarative schema system for microcode kernel
// Allows languages to be specified entirely as data

pub mod statements;
pub mod operators;

pub use statements::{StatementPattern, StatementRole, PatternBuilder};
pub use operators::{OperatorInfo, Associativity, UnaryOperatorInfo};

use std::collections::HashMap;

/// Complete declarative schema for a language
#[derive(Debug, Clone)]
pub struct LanguageSchema {
    pub statements: HashMap<String, StatementPattern>,
    pub binary_ops: HashMap<String, OperatorInfo>,
    pub unary_ops: HashMap<String, UnaryOperatorInfo>,
    pub keywords: Vec<String>,
    pub multichar_lexemes: Vec<&'static str>,
    pub statement_terminators: Vec<String>,
    pub block_open: String,
    pub block_close: String,
    pub extern_syntax: Option<ExternSyntax>,
}

#[derive(Debug, Clone)]
pub struct ExternSyntax {
    pub keyword: String,
    pub selector_start: String,
    pub selector_end: String,
    pub args_open: String,
    pub args_close: String,
    pub arg_separator: String,
}

impl LanguageSchema {
    pub fn is_statement_keyword(&self, lexeme: &str) -> bool {
        self.statements.contains_key(lexeme)
    }

    pub fn is_keyword(&self, lexeme: &str) -> bool {
        self.keywords.contains(&lexeme.to_string())
    }

    pub fn is_terminator(&self, lexeme: &str) -> bool {
        self.statement_terminators.contains(&lexeme.to_string())
    }

    pub fn get_binary_op(&self, lexeme: &str) -> Option<&OperatorInfo> {
        self.binary_ops.get(lexeme)
    }

    pub fn get_unary_op(&self, lexeme: &str) -> Option<&UnaryOperatorInfo> {
        self.unary_ops.get(lexeme)
    }
}
