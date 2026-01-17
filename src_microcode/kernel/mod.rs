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
use num_bigint::BigInt;

pub use primitives::Instruction;
pub use eval::Value;

/// Run a program through the microcode kernel
/// program_args: command-line arguments passed to the program
pub fn run(source: &str, schema: &LanguageSchema, program_args: &[String]) -> Result<Value, String> {
    let start = std::time::Instant::now();

    // Stage 1: Ingest - source → tokens
    let t1 = std::time::Instant::now();
    let tokens = ingest::lex(source, schema)?;
    let ingest_time = t1.elapsed();

    // Stage 2: Structure - tokens → structured tokens
    let t2 = std::time::Instant::now();
    let tokens = structure::process_structure(tokens, schema)?;
    let structure_time = t2.elapsed();

    // Stage 3: Reduce - tokens → instructions
    let t3 = std::time::Instant::now();
    let instr = reduce::parse(tokens, schema)?;
    let reduce_time = t3.elapsed();

    // Stage 4: Execute - instructions → values
    let t4 = std::time::Instant::now();
    let mut env = Environment::new();

    // Bind ARGS: system-provided semantic value containing all program arguments as a single string
    // ARGS is immutable and read-only (cannot be reassigned by user code)
    let args_str = if program_args.is_empty() {
        String::new()
    } else {
        program_args.join(" ")
    };
    env.set("ARGS".to_string(), Value::String(args_str));

    // Bind kind meta-value constants: INTEGER, RATIONAL, REAL, STRING, BOOLEAN, ARRAY, NULL
    // These are predefined kernel-level type descriptors that match kind() return values
    env.set("INTEGER".to_string(), Value::Kind(eval::KindValue::INTEGER));
    env.set("RATIONAL".to_string(), Value::Kind(eval::KindValue::RATIONAL));
    env.set("REAL".to_string(), Value::Kind(eval::KindValue::REAL));
    env.set("STRING".to_string(), Value::Kind(eval::KindValue::STRING));
    env.set("BOOLEAN".to_string(), Value::Kind(eval::KindValue::BOOLEAN));
    env.set("ARRAY".to_string(), Value::Kind(eval::KindValue::ARRAY));
    env.set("NULL".to_string(), Value::Kind(eval::KindValue::NULL));

    // Bind kernel constant: REAL_DEFAULT_PRECISION
    env.set("REAL_DEFAULT_PRECISION".to_string(), Value::Number(BigInt::from(15)));

    let (result, _flow) = execute(&instr, &mut env, schema)?;
    let execute_time = t4.elapsed();

    let total_time = start.elapsed();

    // Only print timing if environment variable is set (for debugging)
    if std::env::var("LUMEN_TIMING").is_ok() {
        eprintln!("[TIMING] Ingest:    {:?}", ingest_time);
        eprintln!("[TIMING] Structure: {:?}", structure_time);
        eprintln!("[TIMING] Reduce:    {:?}", reduce_time);
        eprintln!("[TIMING] Execute:   {:?}", execute_time);
        eprintln!("[TIMING] Total:     {:?}", total_time);
    }

    Ok(result)
}
