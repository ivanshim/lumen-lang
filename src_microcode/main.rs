// Microcode Kernel v2 - Main Entry Point
// Handles language detection and routing for the new microcode kernel
// Usage: microcode_2 <file> [--lang <language>]

use std::env;
use std::fs;
use std::path::Path;
use std::process;

// Import the microcode_2 library
use microcode_2::kernel::run;
use microcode_2::languages::{lumen_schema, rust_core_schema, python_core_schema};

// Build-time packaging: embedded .lm file list from lib_lumen/prelude.rs
mod embedded_files {
    include!("../lib_lumen/prelude.rs");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse arguments: [binary] <file> [--lang <language>] [program_args...]
    let (filepath, language, program_args) = parse_args(&args);

    // Read source file
    let source = match fs::read_to_string(&filepath) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Failed to read {}: {}", filepath, e);
            process::exit(1);
        }
    };

    // Route to appropriate language
    match language.as_str() {
        "lumen" => {
            let schema = lumen_schema::get_schema();
            // Load bootstrap file (prelude.lm) before user code
            // The kernel has no semantic knowledge of what this file does or contains
            let bootstrap_source = include_str!("../lib_lumen/prelude.lm");

            // Process include directives in bootstrap file
            let expanded_bootstrap = match process_includes(bootstrap_source) {
                Ok(expanded) => expanded,
                Err(e) => {
                    eprintln!("Include error: {}", e);
                    process::exit(1);
                }
            };

            let full_source = format!("{}\n{}", expanded_bootstrap, source);
            if let Err(e) = run(&full_source, &schema, &program_args) {
                eprintln!("LumenError: {}", e);
                process::exit(1);
            }
        }
        "rust_core" => {
            let schema = rust_core_schema::get_schema();
            if let Err(e) = run(&source, &schema, &program_args) {
                eprintln!("RustCoreError: {}", e);
                process::exit(1);
            }
        }
        "python_core" => {
            let schema = python_core_schema::get_schema();
            if let Err(e) = run(&source, &schema, &program_args) {
                eprintln!("PythonCoreError: {}", e);
                process::exit(1);
            }
        }
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> (String, String, Vec<String>) {
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <file> [--lang <language>] [program_args...]",
            args.get(0).unwrap_or(&"microcode_2".to_string())
        );
        process::exit(1);
    }

    let filepath = args[1].clone();
    let mut language = String::new();
    let mut program_args = Vec::new();

    // Parse --lang flag
    let mut consumed_until = 2;
    if args.len() > 2 && args[2] == "--lang" {
        if args.len() < 4 {
            eprintln!("Error: --lang requires an argument");
            process::exit(1);
        }
        language = args[3].to_lowercase();
        consumed_until = 4;
    }

    // Auto-detect language if not specified
    if language.is_empty() {
        language = detect_language_from_extension(&filepath)
            .unwrap_or_else(|| "lumen".to_string());
    }

    // Remaining arguments are program arguments
    if args.len() > consumed_until {
        program_args = args[consumed_until..].to_vec();
    }

    (filepath, language, program_args)
}

fn detect_language_from_extension(filepath: &str) -> Option<String> {
    let path = Path::new(filepath);
    let extension = path.extension()?.to_str()?;

    let language = match extension {
        "lm" => "lumen",
        "rs" => "rust_core",
        "py" => "python_core",
        _ => return None,
    };

    Some(language.to_string())
}

/// Generic embedded file lookup: queries the embedded virtual filesystem by path
/// The kernel performs a simple path-based lookup with no semantic knowledge
fn get_embedded_file(path: &str) -> Option<&'static str> {
    embedded_files::EMBEDDED_FILES
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, contents)| *contents)
}

/// Process include directives in source code
/// Recursively expands `include "path"` directives by inlining embedded file contents
fn process_includes(source: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut processed_files = std::collections::HashSet::new();

    fn process_recursive(
        source: &str,
        processed_files: &mut std::collections::HashSet<String>,
        result: &mut String,
    ) -> Result<(), String> {
        for line in source.lines() {
            let trimmed = line.trim();

            // Check if line is an include directive
            if trimmed.starts_with("include ") {
                // Extract the file path from: include "path"
                let rest = trimmed.strip_prefix("include ").unwrap().trim();

                if !rest.starts_with('"') || !rest.ends_with('"') {
                    return Err(format!("Invalid include syntax: {}", line));
                }

                let path = &rest[1..rest.len()-1];

                // Prevent circular includes
                if processed_files.contains(path) {
                    continue; // Skip already processed files
                }
                processed_files.insert(path.to_string());

                // Retrieve from embedded virtual filesystem
                let file_contents = get_embedded_file(path)
                    .ok_or_else(|| format!("File not found in embedded filesystem: {}", path))?;

                // Recursively process the included file
                process_recursive(file_contents, processed_files, result)?;
                result.push('\n');
            } else {
                // Regular line - keep it
                result.push_str(line);
                result.push('\n');
            }
        }
        Ok(())
    }

    process_recursive(source, &mut processed_files, &mut result)?;
    Ok(result)
}
