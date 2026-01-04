// Microcode Kernel Main Entry Point
// Handles language detection and routing for the microcode kernel
// Usage: microcode <file> [--lang <language>]

use std::env;
use std::fs;
use std::path::Path;
use std::process;

mod kernel;
mod languages;
mod runtime;
mod schema;

pub use kernel::Microcode;

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

    // Route to appropriate language
    match language.as_str() {
        "lumen" => run_lumen_microcode(&source),
        "mini-rust" => run_mini_rust_microcode(&source),
        "mini-python" => run_mini_python_microcode(&source),
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            process::exit(1);
        }
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
        "rs" => "mini-rust",
        "py" | "mpy" => "mini-python",
        _ => return None,
    };

    Some(language.to_string())
}

fn run_lumen_microcode(source: &str) {
    use crate::Microcode;
    use crate::languages::lumen_schema;

    let schema = lumen_schema::get_schema();

    if let Err(e) = Microcode::execute(source, &schema) {
        eprintln!("MicrocodeError: {e}");
        process::exit(1);
    }
}

fn run_mini_rust_microcode(source: &str) {
    use crate::Microcode;
    use crate::languages::mini_rust_schema;

    let schema = mini_rust_schema::get_schema();

    if let Err(e) = Microcode::execute(source, &schema) {
        eprintln!("MicrocodeError: {e}");
        process::exit(1);
    }
}

fn run_mini_python_microcode(source: &str) {
    use crate::Microcode;
    use crate::languages::mini_python_schema;

    let schema = mini_python_schema::get_schema();

    if let Err(e) = Microcode::execute(source, &schema) {
        eprintln!("MicrocodeError: {e}");
        process::exit(1);
    }
}
