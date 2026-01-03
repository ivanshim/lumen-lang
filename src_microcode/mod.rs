// Microcode execution model for lumen-lang
//
// ARCHITECTURE:
// The microcode kernel is a data-driven execution engine that:
// - Takes source code and a declarative language schema as input
// - Executes the program entirely by following the schema
// - Contains NO semantic assumptions about language features
// - Interprets all language behavior from pure data
//
// ISOLATION:
// This module is completely independent from src/ (stream model).
// They share the schema system but no parsing or execution logic.

pub mod kernel;
pub mod languages;
pub mod runtime;

pub use kernel::Microcode;
