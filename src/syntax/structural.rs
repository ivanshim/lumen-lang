// src/syntax/structural.rs
//
// Core structural tokens that define the syntax system.
// These are not optional "features" but fundamental syntax elements.

use crate::registry::Registry;

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
    reg.tokens.add_structural("lparen", LPAREN);
    reg.tokens.add_structural("rparen", RPAREN);
    reg.tokens.add_structural("newline", NEWLINE);
    reg.tokens.add_structural("indent", INDENT);
    reg.tokens.add_structural("dedent", DEDENT);
    reg.tokens.add_structural("eof", EOF);
}
