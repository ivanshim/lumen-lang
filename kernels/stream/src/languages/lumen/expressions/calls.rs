// Call argument lists: `( expr, expr, ... )` with the brackets and separator
// the definition spells them with. Shared by function calls, the pipe
// operator and extern.

use crate::languages::lumen::prelude::*;
use crate::kernel::ast::ExprNode;
use crate::kernel::parser::Parser;

/// Whether the current token opens a call.
pub fn at_call_open(parser: &Parser) -> bool {
    def().is("syntax.call.open", &parser.peek().lexeme)
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
