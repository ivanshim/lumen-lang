// Language implementations hosted by the stream kernel.

pub mod text;

pub mod lumen;
pub mod rust_core;
pub mod python_core;

/// Bytes that form words in all three languages (ASCII letters, digits, `_`).
pub fn ascii_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
