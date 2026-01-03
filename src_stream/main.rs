// lumen-lang main entry point
// Supports both stream and microcode execution models
// Use --kernel to select: stream (default) or microcode

use std::env;
use std::fs;
use std::path::Path;
use std::process;

mod schema;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse arguments: [binary] [--kernel stream|microcode] <file>
    let (kernel_type, filepath) = parse_args(&args);

    // Read source file
    let source = match fs::read_to_string(&filepath) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Failed to read {}: {}", filepath, e);
            process::exit(1);
        }
    };

    // Route to appropriate kernel
    match kernel_type.as_str() {
        "stream" => execute_stream_kernel(&source),
        "microcode" => execute_microcode_kernel(&source),
        _ => {
            eprintln!("Error: Unknown kernel '{}'", kernel_type);
            process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> (String, String) {
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    // Check for --kernel flag
    let mut kernel = "stream".to_string();
    let mut file_idx = 1;

    if args.len() >= 3 && args[1] == "--kernel" {
        kernel = args[2].to_lowercase();
        file_idx = 3;
    }

    if file_idx >= args.len() {
        print_usage(&args[0]);
        process::exit(1);
    }

    (kernel, args[file_idx].clone())
}

fn print_usage(binary: &str) {
    eprintln!("Usage: {} [--kernel stream|microcode] <file>", binary);
    eprintln!();
    eprintln!("Kernels:");
    eprintln!("  stream    - Procedural stream model (default)");
    eprintln!("  microcode - Declarative data-driven model");
    eprintln!();
    eprintln!("File detection:");
    eprintln!("  .lm  - Lumen");
    eprintln!("  .rs  - Mini-Rust");
    eprintln!("  .py, .mpy - Mini-Python");
}

fn execute_stream_kernel(source: &str) {
    // Stream kernel execution is in src_stream/
    println!("Error: Stream kernel not yet available in new structure");
    println!("Stream kernel is being refactored to src_stream/");
    process::exit(1);
}

fn execute_microcode_kernel(source: &str) {
    // Microcode kernel execution
    println!("Error: Microcode kernel routing not yet implemented");
    process::exit(1);
}
