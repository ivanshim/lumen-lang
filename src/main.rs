// src/main.rs
// Language-agnostic interpreter kernel
// Supports multiple language implementations

mod kernel;

#[path = "../src_lumen/mod.rs"]
mod src_lumen;

#[path = "../src_mini_rust/mod.rs"]
mod src_mini_rust;

#[path = "../src_mini_php/mod.rs"]
mod src_mini_php;

#[path = "../src_mini_sh/mod.rs"]
mod src_mini_sh;

#[path = "../src_mini_c/mod.rs"]
mod src_mini_c;

#[path = "../src_mini_apple_pascal/mod.rs"]
mod src_mini_apple_pascal;

#[path = "../src_mini_apple_basic/mod.rs"]
mod src_mini_apple_basic;

use std::env;
use std::fs;
use std::path::Path;

use crate::kernel::lexer::lex;
use crate::kernel::parser::Parser;
use crate::kernel::registry::Registry;
use crate::kernel::eval;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: lumen-lang [--lang <language>] <file>");
        eprintln!("\nSupported languages:");
        eprintln!("  lumen         (Python-like with indentation)        [.lm]");
        eprintln!("  mini-rust     (Rust-like with curly braces)         [.rs]");
        eprintln!("  mini-php      (PHP-like with $ variables)           [.php]");
        eprintln!("  mini-sh       (Shell-like syntax)                   [.sh, .ms]");
        eprintln!("  mini-c        (C-like syntax)                       [.c, .mc]");
        eprintln!("  mini-pascal   (Pascal-like with BEGIN/END)          [.p, .mp]");
        eprintln!("  mini-basic    (BASIC-like syntax)                   [.mb, .basic]");
        eprintln!("\nExamples:");
        eprintln!("  lumen-lang --lang mini-rust program.rs     (explicit language)");
        eprintln!("  lumen-lang program.lm                      (auto-detect via extension)");
        std::process::exit(1);
    }

    let (language, filepath) = if args.len() >= 3 && args[1] == "--lang" {
        (args[2].to_lowercase(), args[3].clone())
    } else {
        let file = &args[1];
        (detect_language_from_extension(file).unwrap_or_else(|| "lumen".to_string()), file.clone())
    };

    let source = match fs::read_to_string(&filepath) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Failed to read {}: {}", filepath, e);
            std::process::exit(1);
        }
    };

    match language.as_str() {
        "lumen" => run_lumen(&source),
        "mini-rust" => run_mini_rust(&source),
        "mini-php" => run_mini_php(&source),
        "mini-sh" => run_mini_sh(&source),
        "mini-c" => run_mini_c(&source),
        "mini-pascal" | "mini-apple-pascal" => run_mini_pascal(&source),
        "mini-basic" | "mini-apple-basic" => run_mini_basic(&source),
        _ => {
            eprintln!("Error: Unknown language '{}'", language);
            std::process::exit(1);
        }
    }
}

/// Detect language from file extension
fn detect_language_from_extension(filepath: &str) -> Option<String> {
    let path = Path::new(filepath);
    let extension = path.extension()?.to_str()?;

    let language = match extension {
        "lm" => "lumen",
        "rs" => "mini-rust",
        "php" => "mini-php",
        "sh" | "ms" => "mini-sh",
        "c" | "mc" => "mini-c",
        "p" | "mp" => "mini-pascal",
        "basic" | "mb" => "mini-basic",
        _ => return None,
    };

    Some(language.to_string())
}

fn run_lumen(source: &str) {
    use crate::src_lumen;
    use crate::src_lumen::structure::structural;

    let mut registry = Registry::new();
    src_lumen::dispatcher::register_all(&mut registry);

    let raw_tokens = match lex(source, &registry.tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("LexError: {e}");
            std::process::exit(1);
        }
    };

    let processed_tokens = match structural::process_indentation(source, raw_tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("IndentationError: {e}");
            std::process::exit(1);
        }
    };

    let mut parser = match Parser::new_with_tokens(&registry, processed_tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}

fn run_mini_rust(source: &str) {
    use crate::src_mini_rust;
    use crate::src_mini_rust::structure::structural;

    let mut registry = Registry::new();
    src_mini_rust::register_all(&mut registry);

    let mut parser = match Parser::new(&registry, source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}

fn run_mini_php(source: &str) {
    use crate::src_mini_php;
    use crate::src_mini_php::structure::structural;

    let mut registry = Registry::new();
    src_mini_php::register_all(&mut registry);

    let mut parser = match Parser::new(&registry, source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}

fn run_mini_sh(source: &str) {
    use crate::src_mini_sh;
    use crate::src_mini_sh::structure::structural;

    let mut registry = Registry::new();
    src_mini_sh::register_all(&mut registry);

    let mut parser = match Parser::new(&registry, source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}

fn run_mini_c(source: &str) {
    use crate::src_mini_c;
    use crate::src_mini_c::structure::structural;

    let mut registry = Registry::new();
    src_mini_c::register_all(&mut registry);

    let mut parser = match Parser::new(&registry, source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}

fn run_mini_pascal(source: &str) {
    use crate::src_mini_apple_pascal;
    use crate::src_mini_apple_pascal::structure::structural;

    let mut registry = Registry::new();
    src_mini_apple_pascal::register_all(&mut registry);

    let mut parser = match Parser::new(&registry, source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}

fn run_mini_basic(source: &str) {
    use crate::src_mini_apple_basic;
    use crate::src_mini_apple_basic::structure::structural;

    let mut registry = Registry::new();
    src_mini_apple_basic::register_all(&mut registry);

    let mut parser = match Parser::new(&registry, source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match structural::parse_program(&mut parser) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}
