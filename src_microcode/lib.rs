// Microcode Kernel v2: Semantic normalization engine
//
// A complete redesign of the microcode kernel to optimize for:
// - Clarity: Simple, understandable code
// - Minimality: Only what's needed for the 7 primitives
// - Transformability: Easy to analyze and rewrite
// - Semantic normalization: Kernel as genotype, languages as phenotype
//
// 4-stage pipeline:
// 1. Ingest: source → tokens (lossless tokenization)
// 2. Structure: tokens → structured tokens (indentation/blocks)
// 3. Reduce: tokens → instructions (Pratt parsing)
// 4. Execute: instructions → values (faithful evaluation)

pub mod schema;
pub mod kernel;
pub mod languages;

pub use kernel::run;
pub use kernel::Value;
