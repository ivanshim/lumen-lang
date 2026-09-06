// The language module of the stream kernel. Handlers give each construct
// its meaning in code; which language is being hosted, and how every
// construct is spelled, comes from a definition in langs/ through
// `definition`.

pub mod definition;
pub mod registry;
pub mod prelude;
pub mod values;
mod numeric;
pub mod expressions;
pub mod statements;
pub mod postfix;
pub mod structure;
pub mod extern_system;
pub mod memo;
pub mod dispatcher;

/// Whether `c` may begin an identifier.
pub fn word_start(c: char) -> bool {
    let unicode = definition::def().identifier_unicode;
    c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
}

/// Whether `c` may continue an identifier.
pub fn word_char(c: char) -> bool {
    let unicode = definition::def().identifier_unicode;
    c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
}
