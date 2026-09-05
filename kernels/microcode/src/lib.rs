// Microcode kernel: a data-driven execution engine.
//
// The kernel owns every algorithm; each language is a YAML document in
// `schemas/` that the kernel reads as data. The four stages are:
//
//   1. Ingest    source text  → tokens          (kernel/_1_ingest.rs)
//   2. Structure tokens       → block structure (kernel/_2_structure.rs)
//   3. Reduce    tokens       → instruction tree (kernel/_3_reduce.rs)
//   4. Execute   instructions → values          (kernel/_4_execute.rs)
//
// This crate never imports the stream kernel; the two meet only in the host.

pub mod schema;
pub mod kernel;

pub use kernel::value::Value;
pub use schema::LanguageSchema;

/// Language definitions embedded at build time. Each is pure data.
const SCHEMAS: &[(&str, &str)] = &[
    ("lumen", include_str!("../schemas/lumen.yaml")),
    ("rust_core", include_str!("../schemas/rust_core.yaml")),
    ("python_core", include_str!("../schemas/python_core.yaml")),
];

/// Load the schema for a language name.
pub fn schema_for(language: &str) -> Result<LanguageSchema, String> {
    let text = SCHEMAS
        .iter()
        .find(|(name, _)| *name == language)
        .map(|(_, text)| *text)
        .ok_or_else(|| format!("Error: Unknown language '{}'", language))?;
    LanguageSchema::from_yaml(text)
}

/// Run `source` as `language`; `program_args` become the program's arguments.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let schema = schema_for(language)?;
    kernel::run(source, &schema, program_args)
        .map(|_| ())
        .map_err(|e| format!("{}: {}", schema.error_prefix, e))
}
