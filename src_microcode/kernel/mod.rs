// Microcode kernel: pure algorithmic execution engine
//
// PRINCIPLE: The kernel owns ALL algorithms and makes NO semantic assumptions.
// All language-specific behavior is table-driven via declarative schemas.
//
// FOUR-STAGE PIPELINE:
// 1. Ingest: lex source using schema tables (no interpretation)
// 2. Structure: process structural tokens (indentation, newlines) per schema
// 3. Reduce: convert token stream to instruction tree per schema
// 4. Execute: run instruction tree using primitive dispatch

pub mod primitives;
pub mod ingest;
pub mod structure;
pub mod reduce;
pub mod execute;
pub mod eval;
pub mod env;

use crate::schema::LanguageSchema;
use self::env::Environment;

pub struct Microcode;

impl Microcode {
    /// Execute a program using a declarative language schema.
    /// Takes source code and schema, returns result or error.
    /// No semantic assumptions about what the code means.
    pub fn execute(source: &str, schema: &LanguageSchema) -> Result<(), String> {
        // Stage 1: Ingest - lex source using schema tables
        let tokens = ingest::lex(source, schema)?;

        // Stage 2: Structure - process structural tokens per schema
        // (indentation, newlines, EOF insertion)
        let structured_tokens = structure::process(&tokens, schema)?;

        // Stage 3: Reduce - convert token stream to instruction tree
        let instructions = reduce::parse(&structured_tokens, schema)?;

        // Stage 4: Execute - run instruction tree using primitive dispatch
        let mut env = Environment::new();
        let (_, _) = execute::execute(&instructions, &mut env)?;

        Ok(())
    }
}
