// src/main.rs
// Language-agnostic interpreter framework
// Currently running Lumen language

mod framework;
mod src_lumen;

use std::env;
use std::fs;

use crate::framework::parser::Parser;
use crate::framework::registry::Registry;
use crate::framework::eval;
use crate::src_lumen::dispatcher;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: lumen <file.lm>");
        std::process::exit(1);
    }

    let source = fs::read_to_string(&args[1])
        .expect("Failed to read source file");

    // ========================================================================
    // LANGUAGE FEATURE REGISTRATION
    // All language features are registered by the language dispatcher
    // To change languages, change which dispatcher is called
    // ========================================================================
    let mut registry = Registry::new();

    // Register all Lumen language features
    dispatcher::register_all(&mut registry);

    // --------------------
    // Parse
    // --------------------
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

    // --------------------
    // Execute
    // --------------------
    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}
