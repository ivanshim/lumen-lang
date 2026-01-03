// Declarative schema system for microcode kernel
// Allows languages to be specified entirely as data
//
// SCHEMA RESPONSIBILITIES:
// - Define purely declarative types (no logic)
// - Language definitions are data-driven (YAML or Rust const)
// - Schemas must not contain executable logic

pub mod structure;
pub mod operator;
pub mod language;
pub mod validation;

pub use structure::{StatementPattern, StatementRole, PatternBuilder};
pub use operator::{OperatorInfo, Associativity, UnaryOperatorInfo};
pub use language::{LanguageSchema, ExternSyntax};
