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
    use std::collections::BTreeMap;
    use std::sync::OnceLock;
    static INDEX: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    let index = INDEX.get_or_init(|| {
        let mut index = BTreeMap::new();
        let mut first = None;
        for record in include_str!("../../../unicode-data/UnicodeData.txt").lines() {
            let columns: Vec<_> = record.split(';').collect();
            let Ok(point) = u32::from_str_radix(columns[0], 16) else { continue };
            let title = columns[1];
            if title.ends_with(", First>") {
                first = Some((point, title.to_string()));
                continue;
            }
            if title.ends_with(", Last>") {
                if let Some((start, description)) = first.take() {
                    let prefix = match description.as_str() {
                        s if s.starts_with("<CJK Ideograph") => "CJK UNIFIED IDEOGRAPH",
                        s if s.starts_with("<Tangut Ideograph") => "TANGUT IDEOGRAPH",
                        _ => continue,
                    };
                    for value in start..=point {
                        if let Some(symbol) = char::from_u32(value) {
                            index.insert(format!("{prefix}-{value:04X}"), symbol.to_string());
                        }
                    }
                }
            } else if !title.starts_with('<') {
                if let Some(symbol) = char::from_u32(point) { index.insert(title.to_string(), symbol.to_string()); }
            }
        }
        let initials = ["G", "GG", "N", "D", "DD", "R", "M", "B", "BB", "S", "SS", "", "J", "JJ", "C", "K", "T", "P", "H"];
        let medials = ["A", "AE", "YA", "YAE", "EO", "E", "YEO", "YE", "O", "WA", "WAE", "OE", "YO", "U", "WEO", "WE", "WI", "YU", "EU", "YI", "I"];
        let finals = ["", "G", "GG", "GS", "N", "NJ", "NH", "D", "L", "LG", "LM", "LB", "LS", "LT", "LP", "LH", "M", "B", "BS", "S", "SS", "NG", "J", "C", "K", "T", "P", "H"];
        for number in 0..11172usize {
            let syllable = char::from_u32(0xAC00 + number as u32).expect("Hangul range");
            let title = format!("HANGUL SYLLABLE {}{}{}", initials[number / 588], medials[number % 588 / 28], finals[number % 28]);
            index.insert(title, syllable.to_string());
        }
        for record in include_str!("../../../unicode-data/NameAliases.txt").lines().filter(|r| !r.starts_with('#')) {
            let cells: Vec<_> = record.split(';').collect();
            if cells.len() < 2 { continue; }
            if let Ok(number) = u32::from_str_radix(cells[0], 16) {
                if let Some(symbol) = char::from_u32(number) { index.insert(cells[1].to_string(), symbol.to_string()); }
            }
        }
        index
    });
    index.get(&word.to_ascii_uppercase()).cloned()
}

/// One of the four normalization forms, named by a code: 0 NFC,
/// 1 NFD, 2 NFKC, 3 NFKD. Every point is expanded through the
/// database's decomposition column (compatibility mappings included
/// only for the K forms), the marks are sorted by combining class,
/// and the C forms put the pieces back by canonical composition;
/// Hangul syllables go by arithmetic instead of a listed
/// decomposition. NFKC is the form Python gives every identifier.
pub fn collated(text: &str, form: u8) -> String {
    if text.is_ascii() { return text.to_owned(); }
    let wholesale = form >= 2;
    let recompose = form % 2 == 0;
    use std::collections::BTreeMap;
    use std::sync::OnceLock;
    struct Glyphs {
        spread: BTreeMap<u32, (bool, Vec<u32>)>,
        rank: BTreeMap<u32, u8>,
        merge: BTreeMap<(u32, u32), u32>,
    }
    static GLYPHS: OnceLock<Glyphs> = OnceLock::new();
    let glyphs = GLYPHS.get_or_init(|| {
        let mut spread: BTreeMap<u32, (bool, Vec<u32>)> = BTreeMap::new();
        let mut rank: BTreeMap<u32, u8> = BTreeMap::new();
        for record in include_str!("../../../unicode-data/UnicodeData.txt").lines() {
            let columns: Vec<&str> = record.split(';').collect();
            if columns.len() < 6 { continue; }
            let Ok(point) = u32::from_str_radix(columns[0], 16) else { continue };
            let level: u8 = columns[3].parse().unwrap_or(0);
            if level != 0 { rank.insert(point, level); }
            let field = columns[5];
            if field.is_empty() { continue; }
            let compat = field.starts_with('<');
            let body = if compat { field.split_once('>').map_or("", |(_, rest)| rest) } else { field };
            let pieces = body.split(' ').filter_map(|part| u32::from_str_radix(part, 16).ok()).collect();
            spread.insert(point, (compat, pieces));
        }
        let barred: [u32; 81] = [0x0958, 0x0959, 0x095A, 0x095B, 0x095C, 0x095D, 0x095E, 0x095F, 0x09DC, 0x09DD, 0x09DF, 0x0A33, 0x0A36, 0x0A59, 0x0A5A, 0x0A5B, 0x0A5E, 0x0B5C, 0x0B5D, 0x0F43, 0x0F4D, 0x0F52, 0x0F57, 0x0F5C, 0x0F69, 0x0F76, 0x0F78, 0x0F93, 0x0F9D, 0x0FA2, 0x0FA7, 0x0FAC, 0x0FB9, 0x2ADC, 0xFB1D, 0xFB1F, 0xFB2A, 0xFB2B, 0xFB2C, 0xFB2D, 0xFB2E, 0xFB2F, 0xFB30, 0xFB31, 0xFB32, 0xFB33, 0xFB34, 0xFB35, 0xFB36, 0xFB38, 0xFB39, 0xFB3A, 0xFB3B, 0xFB3C, 0xFB3E, 0xFB40, 0xFB41, 0xFB43, 0xFB44, 0xFB46, 0xFB47, 0xFB48, 0xFB49, 0xFB4A, 0xFB4B, 0xFB4C, 0xFB4D, 0xFB4E, 0x1D15E, 0x1D15F, 0x1D160, 0x1D161, 0x1D162, 0x1D163, 0x1D164, 0x1D1BB, 0x1D1BC, 0x1D1BD, 0x1D1BE, 0x1D1BF, 0x1D1C0];
        let mut merge: BTreeMap<(u32, u32), u32> = BTreeMap::new();
        for (point, (compat, pair)) in &spread {
            if *compat || pair.len() != 2 { continue; }
            if barred.contains(point) { continue; }
            if rank.get(&pair[0]).copied().unwrap_or(0) > 0 { continue; }
            merge.insert((pair[0], pair[1]), *point);
        }
        Glyphs { spread, rank, merge }
    });
    fn expand(point: u32, glyphs: &Glyphs, wholesale: bool, bag: &mut Vec<u32>) {
        let syllable = point.wrapping_sub(0xAC00);
        if syllable < 19 * 21 * 28 {
            let chunk = 21 * 28;
            bag.push(0x1100 + syllable / chunk);
            bag.push(0x1161 + (syllable % chunk) / 28);
            let tail = syllable % 28;
            if tail != 0 { bag.push(0x11A7 + tail); }
            return;
        }
        match glyphs.spread.get(&point) {
            Some((compat, pieces)) => {
                if *compat && !wholesale { bag.push(point); }
                else { for piece in pieces { expand(*piece, glyphs, wholesale, bag); } }
            }
            None => bag.push(point),
        }
    }
    let mut bag: Vec<u32> = Vec::new();
    for ch in text.chars() { expand(ch as u32, glyphs, wholesale, &mut bag); }
    let mut placed = 1;
    while placed < bag.len() {
        let level = *glyphs.rank.get(&bag[placed]).unwrap_or(&0);
        if level > 0 {
            let mut back = placed;
            while back > 0 {
                let above = *glyphs.rank.get(&bag[back - 1]).unwrap_or(&0);
                if above == 0 || above <= level { break; }
                bag.swap(back - 1, back);
                back -= 1;
            }
        }
        placed += 1;
    }
    if !recompose {
        return bag.iter().map(|point| char::from_u32(*point).unwrap_or('\u{FFFD}')).collect();
    }
    let span = 19 * 21 * 28;
    let mut joined: Vec<u32> = Vec::new();
    let mut root: Option<usize> = None;
    let mut previous: u8 = 0;
    for point in bag {
        let level = *glyphs.rank.get(&point).unwrap_or(&0);
        if let Some(at) = root {
            if previous == 0 || previous < level {
                let a = joined[at];
                let b = point;
                let made = if (0x1100..0x1100 + 19).contains(&a) && (0x1161..0x1161 + 21).contains(&b) {
                    Some(0xAC00 + ((a - 0x1100) * 21 + (b - 0x1161)) * 28)
                } else if (0xAC00..0xAC00 + span).contains(&a) && (a - 0xAC00) % 28 == 0 && b > 0x11A7 && b < 0x11A7 + 28 {
                    Some(a + (b - 0x11A7))
                } else {
                    glyphs.merge.get(&(a, b)).copied()
                };
                if let Some(made) = made {
                    joined[at] = made;
                    continue;
                }
            }
        }
        joined.push(point);
        if level == 0 { root = Some(joined.len() - 1); }
        previous = level;
    }
    joined.iter().map(|point| char::from_u32(*point).unwrap_or('\u{FFFD}')).collect()
}

/// Spell the public mapping for a character, with Hangul decomposition.
pub fn decomposition(point: u32) -> String {
    crate::ucd::decomposition(point, false)
}
