pub mod structure;
pub mod expressions;
pub mod lexer;

pub use structure::lumen_structure;

use crate::kernel::parser::{parse_program, Parser};
use crate::kernel::evaluator::Evaluator;
use lexer::lex_lumen;

/// Run Lumen program through opaque kernel
pub fn run(source: &str) -> Result<(), String> {
    // Lex the source code with Lumen keywords
    let tokens = lex_lumen(source);

    // Create parser
    let parser = Parser::new(tokens);

    // Parse the program
    let program = parse_program(parser)?;

    // Create and run evaluator
    let mut evaluator = Evaluator::new();
    evaluator.eval_program(&program)?;

    Ok(())
}

