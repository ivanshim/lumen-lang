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

// 4-stage pipeline modules (in execution order)
pub mod _1_ingest;
pub mod _2_structure;
pub mod _3_reduce;
pub mod _4_execute;

use crate::schema::LanguageSchema;
use env::Environment;
use _4_execute::execute;
use _1_ingest as ingest;
use _2_structure as structure;
use _3_reduce as reduce;

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
