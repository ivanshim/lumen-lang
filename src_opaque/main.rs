// Opaque Kernel: Opaque Analysis Architecture Interpreter
//
// Completely native implementation of opaque kernel - no external dependencies
//
// This kernel demonstrates the opaque analysis pattern where the kernel
// maintains zero semantic knowledge of language constructs. All semantic
// information is provided by language modules via opaque (Box<dyn Any>) handlers.
//
// Full Implementation (✓ Complete):
// - Structure processing: Native opaque implementation
// - Expression parsing: Generic parser with operator precedence
// - Statement parsing: Generic statement parser with block handling
// - Evaluation: Generic evaluator with runtime type checking
// - Lumen language support: Full native implementation

mod kernel;
mod languages;

use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse arguments: [binary] <file> [--lang <language>]
    let (filepath, language) = parse_args(&args);

    // Read source file
    let source = match fs::read_to_string(&filepath) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Failed to read {}: {}", filepath, e);
            process::exit(1);
        }
    };

    // Route to appropriate language - native opaque implementation
    let result = match language.as_str() {
        "lumen" => languages::lumen::run(&source),
        "rust_core" => languages::rust_core::run(&source),
        "python_core" => languages::python_core::run(&source),
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn parse_args(args: &[String]) -> (String, String) {
    if args.len() < 2 {
        eprintln!("Usage: {} <file> [--lang <language>]", args.get(0).unwrap_or(&"lumen-lang".to_string()));
        process::exit(1);
    }

    let filepath = args[1].clone();
    let mut language = String::new();

    // Parse --lang flag
    if args.len() > 2 && args[2] == "--lang" {
        if args.len() < 4 {
            eprintln!("Error: --lang requires an argument");
            process::exit(1);
        }
        language = args[3].to_lowercase();
    }

    // Auto-detect language if not specified
    if language.is_empty() {
        language = detect_language_from_extension(&filepath)
            .unwrap_or_else(|| "lumen".to_string());
    }

    (filepath, language)
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

