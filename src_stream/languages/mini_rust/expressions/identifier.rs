// Identifier handling for mini-rust
// Identifiers are handled as variables in variable.rs

use crate::src_stream::src_stream::kernel::registry::Registry;

pub fn register(_reg: &mut Registry) {
    // Identifiers are registered as a basic token type by the lexer
    // They're handled as variables in variable.rs
}
