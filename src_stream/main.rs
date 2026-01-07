// Stream Kernel Main Entry Point
// Handles language detection and routing for the stream kernel
// Usage: stream <file> [--lang <language>]

use std::env;
use std::fs;
use std::path::Path;
use std::process;

mod kernel;
mod languages;

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
        "lumen" => run_lumen_stream(&source),
        "rust" => run_rust_stream(&source),
        "python" => run_python_stream(&source),
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
        "rs" => "rust",
        "py" => "python",
        _ => return None,
    };

    Some(language.to_string())
}

fn run_lumen_stream(source: &str) {
    use crate::kernel::lexer::lex;
    use crate::kernel::parser::Parser;
    use crate::languages::lumen::registry::Registry;
    use crate::kernel::eval;
    use crate::languages::lumen::structure::structural;

    let mut registry = Registry::new();
    crate::languages::lumen::dispatcher::register_all(&mut registry);

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

    let mut parser = match Parser::new_with_tokens(processed_tokens, &registry.tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser, &registry) {
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

fn run_rust_stream(source: &str) {
    use crate::kernel::lexer::lex;
    use crate::kernel::parser::Parser;
    use crate::languages::rust::registry::Registry;
    use crate::kernel::eval;
    use crate::languages::rust::structure::structural;

    let mut registry = Registry::new();
    crate::languages::rust::register_all(&mut registry);

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

    let mut parser = match Parser::new_with_tokens(processed_tokens, &registry.tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser, &registry) {
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

fn run_python_stream(source: &str) {
    use crate::kernel::lexer::lex;
    use crate::kernel::parser::Parser;
    use crate::languages::python::registry::Registry;
    use crate::kernel::eval;
    use crate::languages::python::structure::structural;

    let mut registry = Registry::new();
    crate::languages::python::register_all(&mut registry);

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

    let mut parser = match Parser::new_with_tokens(processed_tokens, &registry.tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser, &registry) {
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
