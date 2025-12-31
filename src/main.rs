// src/main.rs

mod ast;
mod eval;
mod lexer;
mod parser;
mod registry;
mod runtime;

// expression modules
mod expr {
    pub mod literals;
    pub mod variable;
    pub mod grouping;
    pub mod arithmetic;
    pub mod comparison;
    pub mod logic;
}

// statement modules
mod stmt {
    pub mod print;
    pub mod assignment;
    pub mod if_else;
    pub mod while_loop;
    pub mod break_stmt;
    pub mod continue_stmt;
}

use std::env;
use std::fs;

use crate::parser::Parser;
use crate::registry::Registry;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: lumen <file.lm>");
        std::process::exit(1);
    }

    let source = fs::read_to_string(&args[1])
        .expect("Failed to read source file");

    // --------------------
    // Registry wiring
    // --------------------
    let mut registry = Registry::new();

    // Register all language features
    // Comment out any line to disable that feature
    expr::literals::register(&mut registry);
    expr::variable::register(&mut registry);
    expr::grouping::register(&mut registry);
    expr::arithmetic::register(&mut registry);
    expr::comparison::register(&mut registry);
    expr::logic::register(&mut registry);
    stmt::print::register(&mut registry);
    stmt::assignment::register(&mut registry);
    stmt::if_else::register(&mut registry);
    stmt::while_loop::register(&mut registry);
    stmt::break_stmt::register(&mut registry);
    stmt::continue_stmt::register(&mut registry);

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
