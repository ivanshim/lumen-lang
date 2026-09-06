// src/src_lumen/mod.rs
// Lumen language module
// Complete language definition for Lumen

pub mod registry;
pub mod prelude;
pub mod patterns;
pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod structure;
pub mod extern_system;
pub mod memo;

// The dispatcher module
pub mod dispatcher {
    include!("src_lumen.rs");
}

/// Whether identifiers may use letters and digits beyond ASCII.
pub const IDENTIFIER_UNICODE: bool = true;

/// Whether `c` may begin an identifier in this language.
pub fn word_start(c: char) -> bool {
    crate::languages::word_start(IDENTIFIER_UNICODE, c)
}

/// Whether `c` may continue an identifier in this language.
pub fn word_char(c: char) -> bool {
    crate::languages::word_char(IDENTIFIER_UNICODE, c)
}
