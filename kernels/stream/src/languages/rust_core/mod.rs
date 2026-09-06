pub mod values;
mod numeric;
pub mod structure;
pub mod expressions;
pub mod statements;
pub mod registry;
pub mod prelude;
pub mod src_rust_core;

pub use src_rust_core::register_all;

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
