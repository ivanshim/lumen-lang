pub mod structure;
pub mod lexer;

pub use structure::rust_structure;

use crate::kernel::parser::{parse_program, Parser};
use crate::kernel::evaluator::Evaluator;
use lexer::lex_rust;
use crate::kernel::lexer::Token;

/// Run Rust program through opaque kernel
pub fn run(source: &str) -> Result<(), String> {
    // Step 1: Lex the source code with Rust keywords
    let tokens = lex_rust(source);

    // Step 2: For Rust, we don't process indentation - braces are explicit
    // Just pass tokens through but handle semicolons

    // Step 3: Create parser
    let parser = Parser::new(tokens);

    // Step 4: Parse the program
    let program = parse_program(parser)?;

    // Step 5: Create and run evaluator
    let mut evaluator = Evaluator::new();
    evaluator.eval_program(&program)?;

    Ok(())
}
