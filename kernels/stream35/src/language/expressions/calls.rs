// Call argument lists: `( expr, expr, ... )` with the brackets and separator
// the definition spells them with. Shared by function calls, the pipe
// operator and extern.

use crate::language::prelude::*;
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;

/// Whether the current token opens a call.
pub fn at_call_open(parser: &Parser) -> bool {
    def().is("syntax.call.open", &parser.peek().lexeme)
}

/// An argument label (Swift's `fib(n: 10)`): a word followed by the label
/// token. Labels name arguments for the reader; the kernel passes arguments
/// by position, so the label is dropped.
fn skip_label(parser: &mut Parser) {
    let d = def();
    if d.list("syntax.call.label").is_empty() {
        return;
    }
    let is_word_token = |t: &crate::kernel::lexer::Token| t.lexeme.chars().count() == 1 && t.lexeme.chars().all(word_char);
    let mut n = 0;
    while parser.peek_n(n).map_or(false, is_word_token) {
        n += 1;
    }
    if n > 0 && parser.peek_n(n).map_or(false, |t| d.is("syntax.call.label", &t.lexeme)) {
        for _ in 0..=n {
            parser.advance();
        }
        parser.skip_tokens();
    }
}

/// Consume `( args )`, the opening bracket included, and return the arguments.
pub fn parse_arguments(parser: &mut Parser, registry: &Registry, context: &str) -> LumenResult<Vec<Box<dyn ExprNode>>> {
    let d = def();
    if !d.is("syntax.call.open", &parser.advance().lexeme) {
        return Err(format!("Expected '{}' {}", d.first("syntax.call.open"), context));
    }
    parser.skip_tokens();

    let mut args = Vec::new();
    while !d.is("syntax.call.close", &parser.peek().lexeme) {
        skip_label(parser);
        args.push(parser.parse_expr(registry)?);
        parser.skip_tokens();
        if d.is("syntax.call.separator", &parser.peek().lexeme) {
            parser.advance();
            parser.skip_tokens();
        } else if !d.is("syntax.call.close", &parser.peek().lexeme) {
            return Err(format!(
                "Expected '{}' or '{}' after argument {}",
                d.first("syntax.call.separator"),
                d.first("syntax.call.close"),
                context
            ));
        }
    }
    parser.advance(); // closing bracket
    Ok(args)
}
