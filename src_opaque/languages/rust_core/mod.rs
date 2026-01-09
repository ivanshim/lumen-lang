pub mod structure;
pub mod lexer;
pub mod handlers;

pub use structure::rust_structure;

use crate::kernel::parser::{parse_program, Parser};
use crate::kernel::evaluator::Evaluator;
use crate::kernel::handlers::HandlerRegistry;
use lexer::lex_rust;
use handlers::create_handlers;

/// Run Rust program through opaque kernel
pub fn run(source: &str) -> Result<(), String> {
    // Step 1: Lex the source code with Rust keywords
    let tokens = lex_rust(source);

    // Step 2: For Rust, we don't process indentation - braces are explicit
    // Just pass tokens through but handle semicolons

    // Step 3: Create parser
    let parser = Parser::new(tokens);

    // Step 4: Create handlers for Rust statement types
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
