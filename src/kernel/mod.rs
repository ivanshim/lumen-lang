// src/kernel/mod.rs
// Language-agnostic kernel for language implementation

pub mod ast;
pub mod eval;
pub mod lexer;
pub mod numeric;      // Utilities for language modules to work with numeric strings
pub mod parser;
pub mod registry;
pub mod runtime;

