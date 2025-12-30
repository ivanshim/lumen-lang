// src/main.rs
//
// CLI entry point + feature wiring.
// No language logic lives here — only registration.

mod ast;
mod eval;
mod lexer;
mod parser;
mod registry;
mod runtime;

// expression modules
mod expr {
    pub mod literals;
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
}

use std::env;
use std::fs;

use crate::parser::Parser;
use crate::registry::{Precedence, Registry};
use crate::lexer::Token;

// ─────────────────────────────
// Expression features
// ─────────────────────────────

use crate::expr::literals::NumberLiteral;
use crate::expr::grouping::Grouping;
use crate::expr::arithmetic::{UnaryMinus, Arithmetic};
use crate::expr::comparison::Comparison;
use crate::expr::logic::{Logic, Not};

// ─────────────────────────────
// Statement features
// ─────────────────────────────

use crate::stmt::print::PrintStmt;
use crate::stmt::assignment::AssignStmt;
use crate::stmt::if_else::IfStmt;
use crate::stmt::while_loop::WhileStmt;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: lumen <file.lm>");
        std::process::exit(1);
    }

    let source = match fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read file: {e}");
            std::process::exit(1);
        }
    };

    // ─────────────────────────────
    // Registry wiring
    // ─────────────────────────────

    let mut registry = Registry::new();

    // ----- expression prefixes -----
    registry.register_prefix(Box::new(NumberLiteral));
    registry.register_prefix(Box::new(Grouping));
    registry.register_prefix(Box::new(UnaryMinus));
    registry.register_prefix(Box::new(Not));

    // ----- arithmetic infix -----
    registry.register_infix(Box::new(Arithmetic::new(
        Token::Plus,
        Precedence::Term,
    )));
    registry.register_infix(Box::new(Arithmetic::new(
        Token::Minus,
        Precedence::Term,
    )));
    registry.register_infix(Box::new(Arithmetic::new(
        Token::Star,
        Precedence::Factor,
    )));
    registry.register_infix(Box::new(Arithmetic::new(
        Token::Slash,
        Precedence::Factor,
    )));

    // ----- comparison infix -----
    registry.register_infix(Box::new(Comparison::new(Token::EqEq)));
    registry.register_infix(Box::new(Comparison::new(Token::NotEq)));
    registry.register_infix(Box::new(Comparison::new(Token::Lt)));
    registry.register_infix(Box::new(Comparison::new(Token::Gt)));
    registry.register_infix(Box::new(Comparison::new(Token::LtEq)));
    registry.register_infix(Box::new(Comparison::new(Token::GtEq)));

    // ----- logical infix -----
    registry.register_infix(Box::new(Logic::new(Token::And)));
    registry.register_infix(Box::new(Logic::new(Token::Or)));

    // ----- statements -----
    registry.register_stmt(Box::new(PrintStmt));
    registry.register_stmt(Box::new(AssignStmt));
    registry.register_stmt(Box::new(IfStmt));
    registry.register_stmt(Box::new(WhileStmt));

    // ─────────────────────────────
    // Parse
    // ─────────────────────────────

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

    // ─────────────────────────────
    // Execute
    // ─────────────────────────────

    if let Err(e) = eval::eval(&program) {
        eprintln!("RuntimeError: {e}");
        std::process::exit(1);
    }
}
