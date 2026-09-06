// Microcode kernel: a data-driven execution engine.
//
// The kernel owns every algorithm; each language is a JSON definition in
// `langs/` that the kernel reads as data. The four stages are:
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

/// Language definitions embedded at build time. Each is pure data; a
/// definition given on the command line is read the same way.
const CONFIGS: &[&str] = &[
    include_str!("../../../langs/lumen.json"),
    include_str!("../../../langs/rplumen.json"),
    include_str!("../../../langs/python.json"),
    include_str!("../../../langs/rust.json"),
];

/// The embedded languages: each name with its file extensions.
pub fn languages() -> Result<Vec<(String, Vec<String>)>, String> {
    CONFIGS.iter().map(|text| LanguageSchema::describe(text)).collect()
}

/// The name of the language a definition describes.
pub fn language_of(definition: &str) -> Result<String, String> {
    LanguageSchema::describe(definition).map(|(name, _)| name)
}

/// Load the embedded definition for a language name.
pub fn schema_for(language: &str) -> Result<LanguageSchema, String> {
    for text in CONFIGS {
        let (name, _) = LanguageSchema::describe(text)?;
        if name == language {
            return LanguageSchema::from_json(text).map_err(|e| format!("Error: definition of '{language}': {e}"));
        }
    }
    Err(format!("Error: Unknown language '{}'", language))
}

/// Run `source` as `language`; `program_args` become the program's arguments.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let schema = schema_for(language)?;
    run_schema(&schema, source, program_args)
}

/// Run `source` under a definition given as JSON text.
pub fn run_definition(definition: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let schema = LanguageSchema::from_json(definition).map_err(|e| format!("Error: language definition: {e}"))?;
    run_schema(&schema, source, program_args)
}

fn run_schema(schema: &LanguageSchema, source: &str, program_args: &[String]) -> Result<(), String> {
    kernel::run(source, schema, program_args)
        .map(|_| ())
        .map_err(|e| format!("{}: {}", schema.error_prefix, e))
}
