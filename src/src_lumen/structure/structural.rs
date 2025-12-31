// src/syntax/structural.rs
//
// Core structural tokens that define the syntax system.
// These are not optional "features" but fundamental syntax elements.

use crate::framework::registry::Registry;

// --------------------
// Token definitions
// --------------------

// Grouping
pub const LPAREN: &str = "LPAREN";
pub const RPAREN: &str = "RPAREN";

// Layout (Python-style indentation)
pub const NEWLINE: &str = "NEWLINE";
pub const INDENT: &str = "INDENT";
pub const DEDENT: &str = "DEDENT";

// End of file
pub const EOF: &str = "EOF";

// --------------------
// Registration
// --------------------

pub fn register(reg: &mut Registry) {
    // Register structural tokens with the registry
    // The lexer and parser will retrieve these dynamically
    reg.tokens.set_lparen(LPAREN);
    reg.tokens.set_rparen(RPAREN);
    reg.tokens.set_newline(NEWLINE);
    reg.tokens.set_indent(INDENT);
    reg.tokens.set_dedent(DEDENT);
    reg.tokens.set_eof(EOF);
}
