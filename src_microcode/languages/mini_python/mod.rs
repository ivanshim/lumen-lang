// Mini-Python language for microcode kernel
//
// This module contains ONLY declarative data about the Mini-Python language.
// All parsing and execution logic is in the kernel.

pub mod values;

use crate::schema::{
    LanguageSchema, StatementPattern, StatementRole, OperatorInfo, UnaryOperatorInfo,
    Associativity, ExternSyntax, PatternBuilder,
};
use std::collections::HashMap;

/// Build the complete Mini-Python language schema
pub fn get_schema() -> LanguageSchema {
    // TODO: Implement Mini-Python schema (declarative data only)
    LanguageSchema {
        statements: HashMap::new(),
        binary_ops: HashMap::new(),
        unary_ops: HashMap::new(),
        keywords: vec![],
        multichar_lexemes: vec![],
        statement_terminators: vec![],
        block_open: "{".to_string(),
        block_close: "}".to_string(),
        extern_syntax: None,
    }
}
