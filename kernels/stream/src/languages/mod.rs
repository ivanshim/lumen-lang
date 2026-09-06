// Language implementations hosted by the stream kernel.

pub mod lumen;

// Identifier character classes. The language sets one flag from its
// definition and routes every identifier check through `word_start` and
// `word_char`, which come here. Underscore always belongs to a word.

/// Whether `c` may begin an identifier.
pub fn word_start(unicode: bool, c: char) -> bool {
    c == '_' || if unicode { c.is_alphabetic() } else { c.is_ascii_alphabetic() }
}

/// Whether `c` may continue an identifier.
pub fn word_char(unicode: bool, c: char) -> bool {
    c == '_' || if unicode { c.is_alphanumeric() } else { c.is_ascii_alphanumeric() }
}
