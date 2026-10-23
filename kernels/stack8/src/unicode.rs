// Character properties and full case mappings from the Unicode 16.0
// Character Database: https://www.unicode.org/Public/16.0.0/ucd/ .
// Terms of use: https://www.unicode.org/license.txt .

pub fn bits(letter: char) -> u16 {
    lumen_unicode::bits(letter)
}

pub fn change(letter: char, kind: usize) -> String {
    lumen_unicode::case_of(letter, kind).map_or_else(|| letter.to_string(), str::to_string)
}

/// Unicode 16.0 character names, aliases, and the
/// algorithmic Hangul/CJK/Tangut names described by UAX #44.
/// Data: https://www.unicode.org/Public/16.0.0/ucd/ (Unicode License).
pub fn named_text(name: &str) -> Option<String> {
    lumen_unicode::name_lookup(name)
}

/// One of the four normalization forms, named by a code: 0 NFC,
/// 1 NFD, 2 NFKC, 3 NFKD. Each point is broken into its decomposition
/// (compatibility mappings included only for the K forms), the marks
/// are put in canonical order, and the C forms join the pieces back
/// by canonical composition. NFKC is the form Python gives every
/// identifier. The tables are built once from the same Unicode
/// Character Database file the names above read.
pub fn normalized(text: &str, form: u8) -> String {
    lumen_unicode::normalized(text, form)
}

/// Return the Unicode database mapping, including algorithmic Hangul jamo.
pub fn decomposition(point: u32) -> String {
    crate::ucd::decomposition(point, false)
}
