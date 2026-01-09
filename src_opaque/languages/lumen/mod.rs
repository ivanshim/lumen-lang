pub mod structure;
pub mod expressions;
pub mod lexer;
pub mod structure_processor;
pub mod handlers;

pub use structure::lumen_structure;

use crate::kernel::parser::{parse_program, Parser};
use crate::kernel::evaluator::Evaluator;
use crate::kernel::handlers::HandlerRegistry;
use lexer::lex_lumen;
use structure_processor::process_indentation;
use handlers::create_handlers;

/// Run Lumen program through opaque kernel (native, no delegation)
pub fn run(source: &str) -> Result<(), String> {
    // Step 1: Lex the source code with Lumen keywords
    let tokens = lex_lumen(source);

    // Step 2: Process indentation to add block markers
    let processed_tokens = process_indentation(source, tokens)?;

    // Step 3: Create parser
    let parser = Parser::new(processed_tokens);

    // Step 4: Create handlers for Lumen statement types
    let handlers_vec = create_handlers();
    let mut registry = HandlerRegistry::new();
    for handler in handlers_vec {
        registry.register_boxed(handler);
    }

    // Step 5: Set the registry as current (for recursive handler access)
    registry.set_as_current();

    // Step 6: Parse the program with language-specific handlers
    let program = parse_program(parser, &registry)?;

    // Step 7: Clear the current registry
    HandlerRegistry::clear_current();

    // Step 8: Create and run evaluator
    let mut evaluator = Evaluator::new();
    evaluator.eval_program(&program)?;

    Ok(())
}

