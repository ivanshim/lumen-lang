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
    use std::collections::HashMap;
    use std::sync::OnceLock;
    static NAMES: OnceLock<HashMap<String, String>> = OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let mut names = HashMap::new();
        let mut range: Option<(u32, String)> = None;
        for line in include_str!("../../../unicode-data/UnicodeData.txt").lines() {
            let mut fields = line.split(';');
            let Some(code) = fields.next().and_then(|s| u32::from_str_radix(s, 16).ok()) else { continue };
            let label = fields.next().unwrap_or("");
            if let Some(kind) = label.strip_prefix('<').and_then(|s| s.strip_suffix(", First>")) {
                range = Some((code, kind.to_owned()));
            } else if label.ends_with(", Last>") {
                if let Some((first, kind)) = range.take() {
                    let family = if kind.starts_with("CJK Ideograph") { Some("CJK UNIFIED IDEOGRAPH") }
                        else if kind.starts_with("Tangut Ideograph") { Some("TANGUT IDEOGRAPH") }
                        else { None };
                    if let Some(family) = family {
                        for number in first..=code {
                            if let Some(ch) = char::from_u32(number) {
                                names.insert(format!("{family}-{number:04X}"), ch.to_string());
                            }
                        }
                    }
                }
            } else if !label.starts_with('<') {
                if let Some(ch) = char::from_u32(code) { names.insert(label.to_owned(), ch.to_string()); }
            }
        }
        let leads = ["G", "GG", "N", "D", "DD", "R", "M", "B", "BB", "S", "SS", "", "J", "JJ", "C", "K", "T", "P", "H"];
        let vowels = ["A", "AE", "YA", "YAE", "EO", "E", "YEO", "YE", "O", "WA", "WAE", "OE", "YO", "U", "WEO", "WE", "WI", "YU", "EU", "YI", "I"];
        let tails = ["", "G", "GG", "GS", "N", "NJ", "NH", "D", "L", "LG", "LM", "LB", "LS", "LT", "LP", "LH", "M", "B", "BS", "S", "SS", "NG", "J", "C", "K", "T", "P", "H"];
        for offset in 0..11172u32 {
            let lead = leads[(offset / 588) as usize];
            let vowel = vowels[((offset % 588) / 28) as usize];
            let tail = tails[(offset % 28) as usize];
            let syllable = char::from_u32(0xAC00 + offset).expect("Hangul syllable");
            names.insert(format!("HANGUL SYLLABLE {lead}{vowel}{tail}"), syllable.to_string());
        }
        for row in include_str!("../../../unicode-data/NameAliases.txt").lines() {
            if row.starts_with('#') || row.is_empty() { continue; }
            let mut parts = row.split(';');
            if let (Some(hex), Some(alias)) = (parts.next(), parts.next()) {
                if let Ok(code) = u32::from_str_radix(hex, 16) {
                    if let Some(ch) = char::from_u32(code) { names.insert(alias.to_owned(), ch.to_string()); }
                }
            }
        }
        names
    });
    names.get(&name.to_ascii_uppercase()).cloned()
}

/// One of the four normalization forms, named by a code: 0 NFC,
/// 1 NFD, 2 NFKC, 3 NFKD. Each point is broken into its decomposition
/// (compatibility mappings included only for the K forms), the marks
/// are put in canonical order, and the C forms join the pieces back
/// by canonical composition. NFKC is the form Python gives every
/// identifier. The tables are built once from the same Unicode
/// Character Database file the names above read.
pub fn normalized(text: &str, form: u8) -> String {
    if text.is_ascii() { return text.to_owned(); }
    let wholesale = form >= 2;
    let recompose = form % 2 == 0;
    use std::collections::HashMap;
    use std::sync::OnceLock;
    struct Norm {
        parts: HashMap<u32, (bool, Vec<u32>)>,
        marks: HashMap<u32, u8>,
        joins: HashMap<(u32, u32), u32>,
    }
    static NORMS: OnceLock<Norm> = OnceLock::new();
    let norm = NORMS.get_or_init(|| {
        let mut parts: HashMap<u32, (bool, Vec<u32>)> = HashMap::new();
        let mut marks: HashMap<u32, u8> = HashMap::new();
        for row in include_str!("../../../unicode-data/UnicodeData.txt").lines() {
            let cells: Vec<&str> = row.split(';').collect();
            if cells.len() < 6 { continue; }
            let Ok(point) = u32::from_str_radix(cells[0], 16) else { continue };
            if let Ok(class) = cells[3].parse::<u8>() { if class != 0 { marks.insert(point, class); } }
            if cells[5].is_empty() { continue; }
            let (compat, written) = match cells[5].split_once('>') {
                Some((tag, rest)) if tag.starts_with('<') => (true, rest),
                _ => (false, cells[5]),
            };
            let list = written.split_whitespace().filter_map(|word| u32::from_str_radix(word, 16).ok()).collect();
            parts.insert(point, (compat, list));
        }
        let refused: [u32; 81] = [0x0958, 0x0959, 0x095A, 0x095B, 0x095C, 0x095D, 0x095E, 0x095F, 0x09DC, 0x09DD, 0x09DF, 0x0A33, 0x0A36, 0x0A59, 0x0A5A, 0x0A5B, 0x0A5E, 0x0B5C, 0x0B5D, 0x0F43, 0x0F4D, 0x0F52, 0x0F57, 0x0F5C, 0x0F69, 0x0F76, 0x0F78, 0x0F93, 0x0F9D, 0x0FA2, 0x0FA7, 0x0FAC, 0x0FB9, 0x2ADC, 0xFB1D, 0xFB1F, 0xFB2A, 0xFB2B, 0xFB2C, 0xFB2D, 0xFB2E, 0xFB2F, 0xFB30, 0xFB31, 0xFB32, 0xFB33, 0xFB34, 0xFB35, 0xFB36, 0xFB38, 0xFB39, 0xFB3A, 0xFB3B, 0xFB3C, 0xFB3E, 0xFB40, 0xFB41, 0xFB43, 0xFB44, 0xFB46, 0xFB47, 0xFB48, 0xFB49, 0xFB4A, 0xFB4B, 0xFB4C, 0xFB4D, 0xFB4E, 0x1D15E, 0x1D15F, 0x1D160, 0x1D161, 0x1D162, 0x1D163, 0x1D164, 0x1D1BB, 0x1D1BC, 0x1D1BD, 0x1D1BE, 0x1D1BF, 0x1D1C0];
        let mut joins: HashMap<(u32, u32), u32> = HashMap::new();
        for (&point, (compat, list)) in &parts {
            if *compat || list.len() != 2 || refused.contains(&point) { continue; }
            if marks.get(&list[0]).copied().unwrap_or(0) != 0 { continue; }
            joins.insert((list[0], list[1]), point);
        }
        Norm { parts, marks, joins }
    });
    const S_BASE: u32 = 0xAC00;
    const L_BASE: u32 = 0x1100;
    const V_BASE: u32 = 0x1161;
    const T_BASE: u32 = 0x11A7;
    const L_COUNT: u32 = 19;
    const V_COUNT: u32 = 21;
    const T_COUNT: u32 = 28;
    const S_COUNT: u32 = L_COUNT * V_COUNT * T_COUNT;
    // A Hangul syllable is an arithmetic bundle of jamo, not a listed
    // decomposition, so it comes apart and rejoins by the same rule.
    fn unfold(point: u32, norm: &Norm, wholesale: bool, out: &mut Vec<u32>) {
        const S_BASE: u32 = 0xAC00;
        const L_BASE: u32 = 0x1100;
        const V_BASE: u32 = 0x1161;
        const T_BASE: u32 = 0x11A7;
        let step = 21 * 28;
        let span = 19 * step;
        if (S_BASE..S_BASE + span).contains(&point) {
            let index = point - S_BASE;
            out.push(L_BASE + index / step);
            out.push(V_BASE + index % step / 28);
            let tail = T_BASE + index % 28;
            if tail != T_BASE { out.push(tail); }
        } else if let Some((compat, list)) = norm.parts.get(&point) {
            if *compat && !wholesale { out.push(point); }
            else { for &piece in list { unfold(piece, norm, wholesale, out); } }
        } else {
            out.push(point);
        }
    }
    let mut points: Vec<u32> = Vec::new();
    for c in text.chars() { unfold(u32::from(c), norm, wholesale, &mut points); }
    for at in 1..points.len() {
        let class = norm.marks.get(&points[at]).copied().unwrap_or(0);
        let mut to = at;
        while class != 0 && to > 0 {
            let before = norm.marks.get(&points[to - 1]).copied().unwrap_or(0);
            if before == 0 || before <= class { break; }
            points.swap(to, to - 1);
            to -= 1;
        }
    }
    if !recompose {
        return points.into_iter().map(|point| char::from_u32(point).unwrap_or('\u{FFFD}')).collect();
    }
    let mut out: Vec<u32> = Vec::with_capacity(points.len());
    let mut head: Option<usize> = None;
    let mut latest: u8 = 0;
    for point in points {
        let class = norm.marks.get(&point).copied().unwrap_or(0);
        if let Some(seat) = head {
            if latest == 0 || latest < class {
                let a = out[seat];
                let b = point;
                let joined = if (L_BASE..L_BASE + L_COUNT).contains(&a) && (V_BASE..V_BASE + V_COUNT).contains(&b) {
                    Some(S_BASE + ((a - L_BASE) * V_COUNT + (b - V_BASE)) * T_COUNT)
                } else if (S_BASE..S_BASE + S_COUNT).contains(&a) && (a - S_BASE) % T_COUNT == 0 && b > T_BASE && b < T_BASE + T_COUNT {
                    Some(a + (b - T_BASE))
                } else {
                    norm.joins.get(&(a, b)).copied()
                };
                if let Some(joined) = joined {
                    out[seat] = joined;
                    continue;
                }
            }
        }
        out.push(point);
        if class == 0 { head = Some(out.len() - 1); }
        latest = class;
    }
    out.into_iter().map(|point| char::from_u32(point).unwrap_or('\u{FFFD}')).collect()
}

/// Return the Unicode database mapping, including algorithmic Hangul jamo.
pub fn decomposition(point: u32) -> String {
    crate::ucd::decomposition(point, false)
}
