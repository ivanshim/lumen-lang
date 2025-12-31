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

pub fn register(_reg: &mut Registry) {
    // Note: These tokens are recognized directly by the lexer
    // and don't need registry entries. This function exists
    // for consistency with the modular architecture.
    //
    // The lexer hardcodes recognition of:
    // - '(' and ')' as LPAREN/RPAREN
    // - Indentation as INDENT/DEDENT
    // - Line breaks as NEWLINE
    // - End of input as EOF
}
