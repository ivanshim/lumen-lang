mod ast;
mod eval;
mod lexer;
mod parser;
mod registry;
mod runtime;

mod expr;

use std::env;
use std::fs;

use crate::parser::Parser;
use crate::registry::Registry;

// --- feature imports ---
use crate::expr::literals::NumberLiteralPrefix;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: lumen <file.lm>");
        std::process::exit(1);
    }

    let source = fs::read_to_string(&args[1])
        .expect("Failed to read source file");

    // --- registry setup ---
    let mut registry = Registry::new();

    // expression features
    registry.register_prefix(Box::new(NumberLiteralPrefix));

    // --- parsing ---
    let mut parser = match Parser::new(&registry, &source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    // --- evaluation ---
    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}
