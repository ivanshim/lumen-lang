use crate::languages::rust::prelude::*;
// Identifier handling for mini-rust
// Identifiers are handled as variables in variable.rs

use crate::kernel::registry::LumenResult;
use crate::languages::rust::registry::Registry;

pub fn register(_reg: &mut Registry) {
    // Identifiers are registered as a basic token type by the lexer
    // They're handled as variables in variable.rs
}
