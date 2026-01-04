// src/kernel/mod.rs
// Language-agnostic kernel for language implementation
//
// Pure kernel with ZERO language-specific code.
// All language features (patterns, handler traits, whitespace handling) are in language modules.

pub mod ast;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod registry;
pub mod runtime;


