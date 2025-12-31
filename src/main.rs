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
use crate::registry::{Precedence, Registry};
use crate::lexer::Token;

// ---- expr handlers ----
use crate::expr::literals::{NumberLiteralPrefix, BoolLiteralPrefix};
use crate::expr::variable::VariablePrefix;
use crate::expr::grouping::GroupingPrefix;
use crate::expr::arithmetic::{UnaryMinusPrefix, ArithmeticInfix};
use crate::expr::comparison::ComparisonInfix;
use crate::expr::logic::{LogicInfix, NotPrefix};

// ---- stmt handlers ----
use crate::stmt::print::PrintStmtHandler;
use crate::stmt::assignment::AssignStmtHandler;
use crate::stmt::if_else::IfStmtHandler;
use crate::stmt::while_loop::WhileStmtHandler;
use crate::stmt::break_stmt::BreakStmtHandler;
use crate::stmt::continue_stmt::ContinueStmtHandler;

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

    // ---- expression prefixes ----
    registry.register_prefix(Box::new(NumberLiteralPrefix));
    registry.register_prefix(Box::new(BoolLiteralPrefix));
    registry.register_prefix(Box::new(VariablePrefix));
    registry.register_prefix(Box::new(UnaryMinusPrefix));
    registry.register_prefix(Box::new(GroupingPrefix));
    registry.register_prefix(Box::new(NotPrefix));

    // ---- arithmetic infix ----
    registry.register_infix(Box::new(
        ArithmeticInfix::new(Token::Plus, Precedence::Term),
    ));
    registry.register_infix(Box::new(
        ArithmeticInfix::new(Token::Minus, Precedence::Term),
    ));
    registry.register_infix(Box::new(
        ArithmeticInfix::new(Token::Star, Precedence::Factor),
    ));
    registry.register_infix(Box::new(
        ArithmeticInfix::new(Token::Slash, Precedence::Factor),
    ));

    // ---- comparison infix ----
    registry.register_infix(Box::new(ComparisonInfix::new(Token::EqEq)));
    registry.register_infix(Box::new(ComparisonInfix::new(Token::NotEq)));
    registry.register_infix(Box::new(ComparisonInfix::new(Token::Lt)));
    registry.register_infix(Box::new(ComparisonInfix::new(Token::Gt)));
    registry.register_infix(Box::new(ComparisonInfix::new(Token::LtEq)));
    registry.register_infix(Box::new(ComparisonInfix::new(Token::GtEq)));

    // ---- logical infix ----
    registry.register_infix(Box::new(LogicInfix::new(Token::And)));
    registry.register_infix(Box::new(LogicInfix::new(Token::Or)));

    // ---- statements ----
    registry.register_stmt(Box::new(PrintStmtHandler));
    registry.register_stmt(Box::new(AssignStmtHandler));
    registry.register_stmt(Box::new(IfStmtHandler));
    registry.register_stmt(Box::new(WhileStmtHandler));
    registry.register_stmt(Box::new(BreakStmtHandler));
    registry.register_stmt(Box::new(ContinueStmtHandler));

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
