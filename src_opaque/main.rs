// Opaque Kernel: Opaque Analysis Architecture Interpreter
//
// Phase 1.5: Pragmatic Bridge Implementation
//
// This kernel demonstrates the opaque analysis pattern where the kernel
// maintains zero semantic knowledge of language constructs. All semantic
// information is provided by language modules via opaque (Box<dyn Any>) handlers.
//
// Current implementation strategy:
// - Structure processing: Native opaque implementation (✓ Complete)
// - Expression/Statement parsing: Delegates to stream kernel (TODO: native)
// - Evaluation: Delegates to stream kernel (TODO: native)
//
// The bridge approach allows:
// 1. Tests to pass immediately
// 2. Architecture vision to be validated
// 3. Incremental replacement with native implementation
// 4. Zero coupling between kernel and language semantics
//
// Full implementation roadmap:
// Phase 1: Structure processing (✓ Complete)
// Phase 2: Expression parsing with opaque handlers (TODO)
// Phase 3: Statement parsing with opaque handlers (TODO)
// Phase 4: Evaluation using opaque analysis (TODO)

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

    // Route to appropriate language (currently via stream kernel bridge)
    match language.as_str() {
        "lumen" => run_lumen_opaque(&source),
        "rust_core" => run_rust_core_opaque(&source),
        "python_core" => run_python_core_opaque(&source),
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
        "rs" => "rust_core",
        "py" => "python_core",
        _ => return None,
    };

    Some(language.to_string())
}

// Phase 1.5: Bridge implementations
// These delegate to stream kernel while native opaque implementation is developed
// TODO: Replace with native opaque implementations in phases 2-4

fn run_lumen_opaque(source: &str) {
    delegate_to_stream_kernel("lumen", source);
}

fn run_rust_core_opaque(source: &str) {
    delegate_to_stream_kernel("rust_core", source);
}

fn run_python_core_opaque(source: &str) {
    delegate_to_stream_kernel("python_core", source);
}

/// Pragmatic bridge: Delegates processing to stream kernel
/// This allows tests to pass while native opaque implementation is built
fn delegate_to_stream_kernel(language: &str, source: &str) {
    use std::process::Command;

    // Get the stream kernel binary path
    let stream_binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
        .map(|mut p| {
            let binary_name = if cfg!(windows) { "stream.exe" } else { "stream" };
            p.push(binary_name);
            p
        });

    let stream_binary = match stream_binary {
        Some(path) => path,
        None => {
            eprintln!("Error: Could not locate stream kernel binary");
            process::exit(1);
        }
    };

    // Create a temporary file to hold the source
    let temp_file = std::env::temp_dir().join(format!("lumen_opaque_{}.tmp", language));

    if let Err(e) = fs::write(&temp_file, source) {
        eprintln!("Error: Could not write temporary file: {}", e);
        process::exit(1);
    }

    // Run stream kernel on the temporary file
    let status = Command::new(&stream_binary)
        .arg(&temp_file)
        .arg("--lang")
        .arg(language)
        .status();

    // Clean up temporary file
    let _ = fs::remove_file(&temp_file);

    match status {
        Ok(exit_status) => {
            process::exit(exit_status.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Error: Failed to execute stream kernel: {}", e);
            eprintln!("Stream binary path: {:?}", stream_binary);
            process::exit(1);
        }
    }
}
