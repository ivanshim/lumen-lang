// src/main.rs

mod ast;
mod eval;
mod lexer;
mod parser;
mod registry;
mod runtime;

use std::env;
use std::fs;

use crate::parser::Parser;
use crate::registry::Registry;

// ============================================================================
// AI DIRECTIVE: MODULAR LANGUAGE FEATURE SYSTEM
// ============================================================================
// To disable a feature:
//   1. Comment out its module declaration below
//   2. Comment out its register() call in main()
// Both must be synchronized for the system to compile.
// ============================================================================

// Expression modules
mod expr {
    pub mod literals;      // Number and boolean literals (true, false)
    pub mod variable;      // Variable references (x, y, foo)
    pub mod grouping;      // Parenthesized expressions (...)
    pub mod arithmetic;    // Arithmetic operators (+, -, *, /, %)
    pub mod comparison;    // Comparison operators (==, !=, <, >, <=, >=)
    pub mod logic;         // Logical operators (and, or, not)
}

// Statement modules
mod stmt {
    pub mod print;         // print() statement
    pub mod assignment;    // Assignment (x = expr)
    pub mod if_else;       // if/else statements
    pub mod while_loop;    // while loops
    pub mod break_stmt;    // break statement
    pub mod continue_stmt; // continue statement
}

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
    // Keep synchronized with module declarations at the top of this file
    // ========================================================================
    let mut registry = Registry::new();

    // Expression features
    expr::literals::register(&mut registry);      // Number and boolean literals
    expr::variable::register(&mut registry);      // Variable references
    expr::grouping::register(&mut registry);      // Parenthesized expressions
    expr::arithmetic::register(&mut registry);    // Arithmetic operators
    expr::comparison::register(&mut registry);    // Comparison operators
    expr::logic::register(&mut registry);         // Logical operators

    // Statement features
    stmt::print::register(&mut registry);         // print() statement
    stmt::assignment::register(&mut registry);    // Assignment
    stmt::if_else::register(&mut registry);       // if/else statements
    stmt::while_loop::register(&mut registry);    // while loops
    stmt::break_stmt::register(&mut registry);    // break statement
    stmt::continue_stmt::register(&mut registry); // continue statement

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
