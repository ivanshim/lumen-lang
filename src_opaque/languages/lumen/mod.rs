pub mod structure;
pub mod expressions;
pub mod lexer;
pub mod structure_processor;

pub use structure::lumen_structure;

use crate::kernel::parser::{parse_program, Parser};
use crate::kernel::evaluator::Evaluator;
use lexer::lex_lumen;
use structure_processor::process_indentation;

/// Run Lumen program through opaque kernel (native, no delegation)
pub fn run(source: &str) -> Result<(), String> {
    // Step 1: Lex the source code with Lumen keywords
    let tokens = lex_lumen(source);

    // Step 2: Process indentation to add block markers
    let processed_tokens = process_indentation(source, tokens)?;

    // Step 3: Create parser
    let parser = Parser::new(processed_tokens);

    // Step 4: Parse the program
    let program = parse_program(parser)?;

    // Step 5: Create and run evaluator
    let mut evaluator = Evaluator::new();
    evaluator.eval_program(&program)?;

    Ok(())
}

