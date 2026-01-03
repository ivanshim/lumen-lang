// Statement pattern definitions
//
// Each statement pattern describes how to parse a particular statement type.
// Patterns are language-agnostic templates; language-specific data fills the template.

use std::collections::HashMap;

/// Role of a token in a statement pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementRole {
    /// Keyword that starts the statement (e.g., "if", "while", "print")
    Keyword,

    /// Expression to be parsed (e.g., the condition in "if condition")
    Expression,

    /// Block enclosed in braces (e.g., the block in "if ... { ... }")
    Block,

    /// Literal lexeme that must match exactly (e.g., "then", "else")
    Literal(String),

    /// Optional literal (e.g., "else" is optional in if statements)
    OptionalLiteral(String),
}

/// Pattern for parsing a specific statement type
///
/// Each statement is described as a sequence of roles.
/// The parser follows the pattern to know what tokens to expect.
///
/// Example for "if condition then block else block":
/// [Keyword("if"), Expression, OptionalLiteral("else"), Block]
#[derive(Debug, Clone)]
pub struct StatementPattern {
    /// Statement type name (e.g., "if", "while", "print", "assign")
    pub stmt_type: String,

    /// Sequence of roles this statement pattern follows
    pub pattern: Vec<StatementRole>,

    /// Maps role index to field name for instruction building
    /// (e.g., 0 -> "condition", 1 -> "then_block", 2 -> "else_block")
    pub field_names: Vec<String>,
}

impl StatementPattern {
    /// Create a new statement pattern
    pub fn new(stmt_type: String, pattern: Vec<StatementRole>, field_names: Vec<String>) -> Self {
        assert_eq!(
            pattern.len(),
            field_names.len(),
            "Pattern length must match field_names length"
        );
        Self {
            stmt_type,
            pattern,
            field_names,
        }
    }

    /// Get the role at a given index
    pub fn get_role(&self, index: usize) -> Option<&StatementRole> {
        self.pattern.get(index)
    }

    /// Get the field name at a given index
    pub fn get_field_name(&self, index: usize) -> Option<&String> {
        self.field_names.get(index)
    }

    /// Get total pattern length
    pub fn len(&self) -> usize {
        self.pattern.len()
    }

    /// Check if pattern is empty
    pub fn is_empty(&self) -> bool {
        self.pattern.is_empty()
    }
}

/// Builder for creating statement patterns in a readable way
pub struct PatternBuilder {
    stmt_type: String,
    pattern: Vec<StatementRole>,
    field_names: Vec<String>,
}

impl PatternBuilder {
    pub fn new(stmt_type: &str) -> Self {
        Self {
            stmt_type: stmt_type.to_string(),
            pattern: Vec::new(),
            field_names: Vec::new(),
        }
    }

    pub fn keyword(mut self, name: &str) -> Self {
        self.pattern.push(StatementRole::Keyword);
        self.field_names.push(name.to_string());
        self
    }

    pub fn expression(mut self, name: &str) -> Self {
        self.pattern.push(StatementRole::Expression);
        self.field_names.push(name.to_string());
        self
    }

    pub fn block(mut self, name: &str) -> Self {
        self.pattern.push(StatementRole::Block);
        self.field_names.push(name.to_string());
        self
    }

    pub fn literal(mut self, lexeme: &str, name: &str) -> Self {
        self.pattern.push(StatementRole::Literal(lexeme.to_string()));
        self.field_names.push(name.to_string());
        self
    }

    pub fn optional_literal(mut self, lexeme: &str, name: &str) -> Self {
        self.pattern.push(StatementRole::OptionalLiteral(lexeme.to_string()));
        self.field_names.push(name.to_string());
        self
    }

    pub fn build(self) -> StatementPattern {
        StatementPattern::new(self.stmt_type, self.pattern, self.field_names)
    }
}
