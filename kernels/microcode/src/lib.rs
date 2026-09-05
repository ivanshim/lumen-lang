// Microcode kernel: a data-driven execution engine.
//
// Four-stage pipeline:
// 1. Ingest:    source → tokens
// 2. Structure: tokens → tokens with block structure
// 3. Reduce:    tokens → instruction tree
// 4. Execute:   instruction tree → values
//
// This crate never imports the stream kernel; the two kernels only meet in
// the command-line host.

pub mod schema;
pub mod kernel;
pub mod languages;

pub use kernel::Value;

/// Run `source` as `language` on the microcode kernel.
/// `program_args` are exposed to the program as its arguments.
pub fn run(language: &str, source: &str, program_args: &[String]) -> Result<(), String> {
    let (schema, error_prefix) = match language {
        "lumen" => (languages::lumen_schema::get_schema(), "LumenError"),
        "rust_core" => (languages::rust_core_schema::get_schema(), "RustCoreError"),
        "python_core" => (languages::python_core_schema::get_schema(), "PythonCoreError"),
        other => return Err(format!("Error: Unknown language '{}'", other)),
    };
    kernel::run(source, &schema, program_args)
        .map(|_| ())
        .map_err(|e| format!("{}: {}", error_prefix, e))
}
