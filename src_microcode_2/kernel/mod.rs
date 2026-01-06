// Microcode Kernel: Semantic normalization layer
//
// 4-stage pipeline:
// 1. Ingest: source → tokens
// 2. Structure: tokens → tokens (with block structure)
// 3. Reduce: tokens → instructions (7 primitives)
// 4. Execute: instructions → values

pub mod primitives;
pub mod eval;
pub mod env;
pub mod ingest;
pub mod structure;
pub mod reduce;
pub mod execute;

use crate::schema::LanguageSchema;
use env::Environment;
use execute::execute;

pub use primitives::Instruction;
pub use eval::Value;

/// Run a program through the microcode kernel
pub fn run(source: &str, schema: &LanguageSchema) -> Result<Value, String> {
    // Stage 1: Ingest - source → tokens
    let tokens = ingest::lex(source, schema)?;

    // Stage 2: Structure - tokens → structured tokens
    let tokens = structure::process_structure(tokens, schema)?;

    // Stage 3: Reduce - tokens → instructions
    let instr = reduce::parse(tokens, schema)?;

    // Stage 4: Execute - instructions → values
    let mut env = Environment::new();
    let (result, _flow) = execute(&instr, &mut env, schema)?;

    Ok(result)
}
