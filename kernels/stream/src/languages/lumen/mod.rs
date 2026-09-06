// Lumen on the stream kernel: handlers give each construct its meaning;
// the spelling of every construct comes from configs/lumen.json through
// `definition`.

pub mod definition;
pub mod registry;
pub mod prelude;
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

/// Whether `c` may begin an identifier in Lumen.
pub fn word_start(c: char) -> bool {
    crate::languages::word_start(definition::def().identifier_unicode, c)
}

/// Whether `c` may continue an identifier in Lumen.
pub fn word_char(c: char) -> bool {
    crate::languages::word_char(definition::def().identifier_unicode, c)
}
