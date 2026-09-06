// kernel/mod.rs
// Language-agnostic kernel.
//
// Nothing in this directory knows any language: no keywords, no comment or
// string syntax, no precedence, no value types, no runtime policy.

pub mod ast;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod registry;
pub mod runtime;


