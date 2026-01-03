// lumen-lang main entry point
// Routes between stream and microcode kernels
// Usage: lumen-lang [--kernel stream|microcode] [--lang <language>] <file>

mod schema;

// Load stream kernel track
#[path = "../src_stream/mod.rs"]
mod src_stream;

// Load microcode kernel track
#[path = "../src_microcode/mod.rs"]
mod src_microcode;

use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse arguments: [binary] [--kernel stream|microcode] [--lang <language>] <file>
    let (kernel_type, language, filepath) = parse_args(&args);

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
        "stream" => execute_stream_kernel(&source, &language),
        "microcode" => execute_microcode_kernel(&source, &language),
        _ => {
            eprintln!("Error: Unknown kernel '{}'", kernel_type);
            print_usage(&args[0]);
            process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> (String, String, String) {
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let mut kernel = "stream".to_string();
    let mut language = String::new();
    let mut file_idx = 1;

    // Parse --kernel flag
    if args.len() > 1 && args[1] == "--kernel" {
        if args.len() < 3 {
            eprintln!("Error: --kernel requires an argument");
            print_usage(&args[0]);
            process::exit(1);
        }
        kernel = args[2].to_lowercase();
        file_idx = 3;
    }

    // Parse --lang flag
    if file_idx + 1 < args.len() && args[file_idx] == "--lang" {
        language = args[file_idx + 1].to_lowercase();
        file_idx += 2;
    }

    if file_idx >= args.len() {
        print_usage(&args[0]);
        process::exit(1);
    }

    let filepath = args[file_idx].clone();

    // Auto-detect language if not specified
    if language.is_empty() {
        language = detect_language_from_extension(&filepath)
            .unwrap_or_else(|| "lumen".to_string());
    }

    (kernel, language, filepath)
}

fn print_usage(binary: &str) {
    eprintln!("Usage: {} [--kernel stream|microcode] [--lang <language>] <file>", binary);
    eprintln!();
    eprintln!("Kernels:");
    eprintln!("  stream    - Procedural stream model (default)");
    eprintln!("  microcode - Declarative data-driven model");
    eprintln!();
    eprintln!("Languages:");
    eprintln!("  lumen         (Python-like with indentation)  [.lm]");
    eprintln!("  mini-rust     (Rust-like with curly braces)   [.rs]");
    eprintln!("  mini-python   (Python-like with indentation)  [.py, .mpy]");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {} --kernel stream program.lm", binary);
    eprintln!("  {} --kernel microcode program.lm", binary);
    eprintln!("  {} --lang mini-python program.py", binary);
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

fn execute_stream_kernel(source: &str, language: &str) {
    match language {
        "lumen" => run_lumen_stream(source),
        "mini-rust" => run_mini_rust_stream(source),
        "mini-python" => run_mini_python_stream(source),
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            process::exit(1);
        }
    }
}

fn execute_microcode_kernel(source: &str, language: &str) {
    match language {
        "lumen" => run_lumen_microcode(source),
        "mini-rust" => {
            eprintln!("Error: Mini-Rust not yet supported in microcode kernel");
            process::exit(1);
        }
        "mini-python" => {
            eprintln!("Error: Mini-Python not yet supported in microcode kernel");
            process::exit(1);
        }
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            process::exit(1);
        }
    }
}

fn run_lumen_stream(source: &str) {
    use crate::src_stream::kernel::lexer::lex;
    use crate::src_stream::kernel::parser::Parser;
    use crate::src_stream::kernel::registry::Registry;
    use crate::src_stream::kernel::eval;
    use crate::src_stream::languages::lumen::structure::structural;

    let mut registry = Registry::new();
    crate::src_stream::languages::lumen::dispatcher::register_all(&mut registry);

    let raw_tokens = match lex(source, &registry.tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("LexError: {e}");
            process::exit(1);
        }
    };

    let processed_tokens = match structural::process_indentation(source, raw_tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("IndentationError: {e}");
            process::exit(1);
        }
    };

    let mut parser = match Parser::new_with_tokens(&registry, processed_tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        process::exit(1);
    }
}

fn run_mini_rust_stream(source: &str) {
    use crate::src_stream::kernel::lexer::lex;
    use crate::src_stream::kernel::parser::Parser;
    use crate::src_stream::kernel::registry::Registry;
    use crate::src_stream::kernel::eval;
    use crate::src_stream::languages::mini_rust::structure::structural;

    let mut registry = Registry::new();
    crate::src_stream::languages::mini_rust::register_all(&mut registry);

    let raw_tokens = match lex(source, &registry.tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("LexError: {e}");
            process::exit(1);
        }
    };

    let processed_tokens = match structural::process_tokens(raw_tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("TokenError: {e}");
            process::exit(1);
        }
    };

    let mut parser = match Parser::new_with_tokens(&registry, processed_tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        process::exit(1);
    }
}

fn run_mini_python_stream(source: &str) {
    use crate::src_stream::kernel::lexer::lex;
    use crate::src_stream::kernel::parser::Parser;
    use crate::src_stream::kernel::registry::Registry;
    use crate::src_stream::kernel::eval;
    use crate::src_stream::languages::mini_python::structure::structural;

    let mut registry = Registry::new();
    crate::src_stream::languages::mini_python::register_all(&mut registry);

    let raw_tokens = match lex(source, &registry.tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("LexError: {e}");
            process::exit(1);
        }
    };

    let processed_tokens = match structural::process_indentation(source, raw_tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("IndentationError: {e}");
            process::exit(1);
        }
    };

    let mut parser = match Parser::new_with_tokens(&registry, processed_tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        process::exit(1);
    }
}

fn run_lumen_microcode(source: &str) {
    use crate::src_microcode::Microcode;
    use crate::src_microcode::languages::lumen_schema;

    let schema = lumen_schema::get_schema();

    if let Err(e) = Microcode::execute(source, &schema) {
        eprintln!("MicrocodeError: {e}");
        process::exit(1);
    }
}
