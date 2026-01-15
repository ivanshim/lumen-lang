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
            // Prepend Lumen standard library
            // The library provides user-facing I/O functions (print, write) built on top of emit() primitive
            // Load in order: write.lm first (defines write), then print.lm (depends on write),
            // then math functions (factorial, e_integer, pi_machin), then constants (1024-digit, full-precision, defaults)
            let stdlib_write = include_str!("../lib_lumen/write.lm");
            let stdlib_print = include_str!("../lib_lumen/print.lm");
            let stdlib_factorial = include_str!("../lib_lumen/factorial.lm");
            let stdlib_round = include_str!("../lib_lumen/round.lm");
            let stdlib_e_integer = include_str!("../lib_lumen/e_integer.lm");
            let stdlib_pi_machin = include_str!("../lib_lumen/pi_machin.lm");
            let stdlib_constants_1024 = include_str!("../lib_lumen/constants_1024.lm");
            let stdlib_constants = include_str!("../lib_lumen/constants.lm");
            let stdlib_constants_default = include_str!("../lib_lumen/constants_default.lm");
            let stdlib_primes = include_str!("../lib_lumen/primes.lm");
            let full_source = format!("{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
                stdlib_write, stdlib_print, stdlib_factorial, stdlib_round, stdlib_e_integer,
                stdlib_pi_machin, stdlib_constants_1024, stdlib_constants, stdlib_constants_default,
                stdlib_primes, source);
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
