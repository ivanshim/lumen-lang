// The Unicode 16.0 Character Database supplies these ranges and changes.
// Source: https://www.unicode.org/Public/16.0.0/ucd/ ; its terms stand at
// https://www.unicode.org/license.txt .

pub fn property(c: char, mask: u16) -> bool {
    lumen_unicode::bits(c) & mask != 0
}

pub fn altered(c: char, which: usize) -> String {
    match lumen_unicode::case_of(c, which) {
        Some(text) => text.to_owned(),
        None => String::from(c),
    }
}

/// Resolve a Unicode 16.0 name from the UCD, including aliases.
/// UAX #44 defines the Hangul, CJK and Tangut algorithmic names.
/// Source: https://www.unicode.org/Public/16.0.0/ucd/ ; license:
/// https://www.unicode.org/license.txt .
pub fn name_value(word: &str) -> Option<String> {
    lumen_unicode::name_lookup(word)
}

/// One of the four normalization forms, named by a code: 0 NFC,
/// 1 NFD, 2 NFKC, 3 NFKD. Every point is expanded through the
/// database's decomposition column (compatibility mappings included
/// only for the K forms), the marks are sorted by combining class,
/// and the C forms put the pieces back by canonical composition;
/// Hangul syllables go by arithmetic instead of a listed
/// decomposition. NFKC is the form Python gives every identifier.
pub fn collated(text: &str, shape: u8) -> String {
    lumen_unicode::normalized(text, shape)
}

/// Spell the public mapping for a character, with Hangul decomposition.
pub fn decomposition(point: u32) -> String {
    crate::ucd::decomposition(point, false)
}
