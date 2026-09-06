// Language implementations hosted by the stream kernel.

pub mod lumen;
pub mod rust_core;
pub mod python_core;

// Identifier character classes. Each language sets one flag, IDENTIFIER_UNICODE,
// and routes every identifier check through its own `word_start` and
// `word_char`, which come here. Underscore always belongs to a word.

/// Whether `c` may begin an identifier.
pub fn word_start(unicode: bool, c: char) -> bool {
    c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
}

/// Whether `c` may continue an identifier.
pub fn word_char(unicode: bool, c: char) -> bool {
    c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
}
