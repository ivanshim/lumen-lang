// src/main.rs
// Language-agnostic interpreter framework
// Currently running Lumen language

mod framework;

#[path = "../src_lumen/mod.rs"]
mod src_lumen;

use std::env;
use std::fs;

use crate::framework::lexer::lex;
use crate::framework::parser::Parser;
use crate::framework::registry::Registry;
use crate::framework::eval;
use crate::src_lumen::dispatcher;
use crate::src_lumen::structure::structural;

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
    // Tokenize (Framework Lexer - pure tokenization)
    // --------------------
    let raw_tokens = match lex(&source, &registry.tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("LexError: {e}");
            std::process::exit(1);
        }
    };

    // --------------------
    // Post-process Tokens (Language-Specific)
    // --------------------
    // For Lumen: add INDENT/DEDENT/NEWLINE/EOF tokens based on indentation
    let processed_tokens = match structural::process_indentation(&source, raw_tokens) {
        Ok(toks) => toks,
        Err(e) => {
            eprintln!("IndentationError: {e}");
            std::process::exit(1);
        }
    };

    // --------------------
    // Parse
    // --------------------
    // Create parser with Lumen's structural token configuration
    let mut parser = match Parser::new_with_tokens(&registry, processed_tokens, structural::tokens()) {
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
