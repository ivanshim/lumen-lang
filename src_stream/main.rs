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
        "lumen" => run_lumen_stream(&source, &program_args),
        "rust_core" => run_rust_core_stream(&source, &program_args),
        "python_core" => run_python_core_stream(&source, &program_args),
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            process::exit(1);
        }
    }
}

fn parse_args(args: &[String]) -> (String, String, Vec<String>) {
    if args.len() < 2 {
        eprintln!("Usage: {} <file> [--lang <language>] [program_args...]", args.get(0).unwrap_or(&"lumen-lang".to_string()));
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

fn run_lumen_stream(source: &str, program_args: &[String]) {
    use crate::kernel::lexer::lex;
    use crate::kernel::parser::Parser;
    use crate::languages::lumen::registry::Registry;
    use crate::kernel::eval;
    use crate::languages::lumen::structure::structural;

    let mut registry = Registry::new();
    crate::languages::lumen::dispatcher::register_all(&mut registry);

    // Prepend Lumen standard library
    // The library provides user-facing I/O functions (print, write) built on top of emit() primitive
    let stdlib_io = include_str!("../library/io.lm");
    let full_source = format!("{}\n{}", stdlib_io, source);

    let raw_tokens = match lex(&full_source, &registry.tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("LexError: {e}");
            process::exit(1);
        }
    };

    let processed_tokens = match structural::process_indentation(&full_source, raw_tokens) {
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

    // Initialize environment with system values (ARGS, etc.)
    let init_env = move |env: &mut crate::kernel::runtime::Env| {
        use crate::languages::lumen::values::LumenString;
        // Bind ARGS: system-provided semantic value containing all program arguments
        // ARGS is immutable and read-only (cannot be reassigned by user code)
        let args_str = if program_args.is_empty() {
            String::new()
        } else {
            program_args.join(" ")
        };
        env.define("ARGS".to_string(), Box::new(LumenString::new(args_str)));
        Ok(())
    };

    if let Err(e) = eval::eval(&program, init_env) {
        eprintln!("RuntimeError: {e}");
        process::exit(1);
    }
}

fn run_rust_core_stream(source: &str, program_args: &[String]) {
    use crate::kernel::lexer::lex;
    use crate::kernel::parser::Parser;
    use crate::languages::rust_core::registry::Registry;
    use crate::kernel::eval;
    use crate::languages::rust_core::structure::structural;

    let mut registry = Registry::new();
    crate::languages::rust_core::register_all(&mut registry);

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

    // Initialize environment with system values (ARGS, etc.)
    // Note: rust_core doesn't have a String value type, so ARGS is not currently supported
    let init_env = |_env: &mut crate::kernel::runtime::Env| {
        Ok(())
    };

    if let Err(e) = eval::eval(&program, init_env) {
        eprintln!("RuntimeError: {e}");
        process::exit(1);
    }
}

fn run_python_core_stream(source: &str, program_args: &[String]) {
    use crate::kernel::lexer::lex;
    use crate::kernel::parser::Parser;
    use crate::languages::python_core::registry::Registry;
    use crate::kernel::eval;
    use crate::languages::python_core::structure::structural;

    let mut registry = Registry::new();
    crate::languages::python_core::register_all(&mut registry);

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

    // Initialize environment with system values (ARGS, etc.)
    // Note: python_core doesn't have a String value type, so ARGS is not currently supported
    let init_env = |_env: &mut crate::kernel::runtime::Env| {
        Ok(())
    };

    if let Err(e) = eval::eval(&program, init_env) {
        eprintln!("RuntimeError: {e}");
        process::exit(1);
    }
}
