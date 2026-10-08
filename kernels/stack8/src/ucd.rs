// Read generated Unicode 16.0 properties and CPython's 3.2 compatibility view.
use std::{collections::HashMap, sync::OnceLock};
use crate::value::{Value, real_of};

struct Range { end: u32, fields: [&'static str; 8] }
struct Database {
    ranges: Vec<Range>,
    decomp: HashMap<u32, &'static str>,
}
// Parse the generated, trusted range and decomposition tables once per version.
fn database(old: bool) -> &'static Database {
    static CURRENT: OnceLock<Database> = OnceLock::new();
    static OLD: OnceLock<Database> = OnceLock::new();
    let (slot, properties, decompositions) = if old {
        (&OLD, include_str!("../../../unicode-data/generated/properties-3.2.tsv"), include_str!("../../../unicode-data/generated/decomposition-3.2.tsv"))
    } else {
        (&CURRENT, include_str!("../../../unicode-data/generated/properties.tsv"), include_str!("../../../unicode-data/generated/decomposition.tsv"))
    };
    slot.get_or_init(|| Database {
        ranges: properties.lines().map(|line| {
            let cells: Vec<_> = line.split('\t').collect();
            Range { end: u32::from_str_radix(cells[1], 16).unwrap(), fields: cells[2..].try_into().unwrap() }
        }).collect(),
        decomp: decompositions.lines().map(|line| {
            let (code, value) = line.split_once('\t').unwrap();
            (u32::from_str_radix(code, 16).unwrap(), value)
        }).collect(),
    })
}
// Locate one property record, including unassigned characters and surrogates.
fn fields(point: u32, old: bool) -> &'static [&'static str; 8] {
    let db = database(old);
    &db.ranges[db.ranges.partition_point(|row| row.end < point)].fields
}
// Return a property in its public Python type, or None for an absent number.
pub fn property(point: u32, field: usize, old: bool) -> Result<Value, String> {
    let cells = fields(point, old);
    let text = *cells.get(field).ok_or("ValueError: unknown Unicode property")?;
    Ok(match field {
        0 | 1 | 4 => Value::text(text),
        2 | 3 => Value::Small(text.parse::<i64>().unwrap()),
        5 | 6 if !text.is_empty() => Value::Small(text.parse::<i64>().unwrap()),
        7 if !text.is_empty() => {
            let (a, b) = text.split_once('/').unwrap_or((text, "1"));
            real_of(a.parse::<f64>().unwrap() / b.parse::<f64>().unwrap(), 15)
        }
        _ => Value::Null,
    })
}
// Return the public decomposition spelling, with algorithmic Hangul mappings.
pub fn decomposition(point: u32, old: bool) -> String {
    if fields(point, old)[0] == "Cn" { return String::new(); }
    let syllable = point.wrapping_sub(0xAC00);
    if syllable < 11172 {
        let leading = 0x1100 + syllable / 588;
        let vowel = 0x1161 + syllable % 588 / 28;
        let tail = syllable % 28;
        return if tail == 0 { format!("{leading:04X} {vowel:04X}") }
            else { format!("{leading:04X} {vowel:04X} {:04X}", 0x11A7 + tail) };
    }
    database(old).decomp.get(&point).copied().unwrap_or("").to_owned()
}
// Index canonical names separately from aliases and named sequences.
fn names() -> &'static HashMap<u32, &'static str> {
    static NAMES: OnceLock<HashMap<u32, &'static str>> = OnceLock::new();
    NAMES.get_or_init(|| include_str!("../../../unicode-data/generated/names.tsv").lines().map(|line| {
        let (hex, name) = line.split_once('\t').unwrap();
        (u32::from_str_radix(hex, 16).unwrap(), name)
    }).collect())
}
// Return only canonical names, filtered by assignment in the selected version.
pub fn name(point: u32, old: bool) -> Option<&'static str> {
    if old && fields(point, true)[0] == "Cn" { return None; }
    names().get(&point).copied()
}
// Resolve ASCII-insensitive names; the old view also accepts newer canonical names.
pub fn lookup(label: &str, old: bool) -> Option<Vec<u32>> {
    static INDEX: OnceLock<HashMap<String, Vec<u32>>> = OnceLock::new();
    let folded = label.to_ascii_uppercase();
    if old {
        static HISTORIC: OnceLock<HashMap<&'static str, u32>> = OnceLock::new();
        let map = HISTORIC.get_or_init(|| {
            let mut map: HashMap<&str, u32> = names().iter().map(|(&cp, &word)| (word, cp)).collect();
            for line in include_str!("../../../unicode-data/generated/lookup-only.tsv").lines() {
                let (word, hex) = line.split_once('\t').unwrap();
                map.insert(word, u32::from_str_radix(hex, 16).unwrap());
            }
            map
        });
        return map.get(folded.as_str()).map(|&cp| vec![cp]);
    }
    let index = INDEX.get_or_init(|| {
        let mut map: HashMap<String, Vec<u32>> = names().iter().map(|(&cp, &word)| (word.to_owned(), vec![cp])).collect();
        for line in include_str!("../../../unicode-data/generated/aliases.tsv").lines().chain(include_str!("../../../unicode-data/generated/lookup-only.tsv").lines()) {
            let (word, codes) = line.split_once('\t').unwrap();
            map.insert(word.to_owned(), codes.split_whitespace().map(|hex| u32::from_str_radix(hex, 16).unwrap()).collect());
        }
        map
    });
    index.get(&folded).cloned()
}
// Recursively decompose one code point, honoring 3.2 normalization corrections.
fn expand(point: u32, old: bool, compatibility: bool, out: &mut Vec<u32>) {
    let mut code = point;
    if old {
        static CORRECTIONS: OnceLock<HashMap<u32, u32>> = OnceLock::new();
        let fixes = CORRECTIONS.get_or_init(|| include_str!("../../../unicode-data/generated/normalization-3.2.tsv").lines().map(|line| {
            let (from, to) = line.split_once('\t').unwrap();
            (u32::from_str_radix(from, 16).unwrap(), u32::from_str_radix(to, 16).unwrap())
        }).collect());
        code = fixes.get(&code).copied().unwrap_or(code);
    }
    let syllable = code.wrapping_sub(0xAC00);
    if syllable < 11172 {
        out.extend([0x1100 + syllable / 588, 0x1161 + syllable % 588 / 28]);
        if syllable % 28 != 0 { out.push(0x11A7 + syllable % 28); }
        return;
    }
    if let Some(mapping) = database(old).decomp.get(&code) {
        let tagged = mapping.starts_with('<');
        if compatibility || !tagged {
            for hex in mapping.split_whitespace().filter(|part| !part.starts_with('<')) {
                expand(u32::from_str_radix(hex, 16).unwrap(), old, compatibility, out);
            }
            return;
        }
    }
    out.push(code);
}
// Compose a canonical pair; Hangul uses arithmetic, other pairs use exclusions.
fn join(a: u32, b: u32) -> Option<u32> {
    if (0x1100..0x1113).contains(&a) && (0x1161..0x1176).contains(&b) {
        return Some(0xAC00 + ((a - 0x1100) * 21 + b - 0x1161) * 28);
    }
    if (0xAC00..0xD7A4).contains(&a) && (a - 0xAC00) % 28 == 0 && (0x11A8..0x11C3).contains(&b) {
        return Some(a + b - 0x11A7);
    }
    static PAIRS: OnceLock<HashMap<(u32, u32), u32>> = OnceLock::new();
    let pairs = PAIRS.get_or_init(|| include_str!("../../../unicode-data/generated/composition.tsv").lines().map(|line| {
        let codes: Vec<_> = line.split('\t').map(|hex| u32::from_str_radix(hex, 16).unwrap()).collect();
        ((codes[0], codes[1]), codes[2])
    }).collect());
    pairs.get(&(a, b)).copied()
}
// Normalize code points directly, preserving surrogate halves and stable ordering.
pub fn normalize(form: &str, text: &[u32], old: bool) -> Result<Vec<u32>, String> {
    if text.is_empty() { return Ok(Vec::new()); }
    if !matches!(form, "NFC" | "NFD" | "NFKC" | "NFKD") { return Err("ValueError: invalid normalization form".into()); }
    if text.iter().all(|&cp| cp < 128) { return Ok(text.to_vec()); }
    let compatibility = form.starts_with("NFK");
    let mut expanded = Vec::with_capacity(text.len());
    for &point in text { expand(point, old, compatibility, &mut expanded); }
    let class = |point: u32| fields(point, false)[2].parse::<u8>().unwrap();
    let mut start = 0;
    for at in 0..=expanded.len() {
        if at == expanded.len() || class(expanded[at]) == 0 {
            expanded[start..at].sort_by_key(|&cp| class(cp));
            start = at + 1;
        }
    }
    if form.ends_with('D') { return Ok(expanded); }
    let mut result = Vec::with_capacity(expanded.len());
    let mut starter: Option<usize> = None;
    let mut previous = 0;
    for point in expanded {
        let rank = class(point);
        if let Some(at) = starter {
            if previous == 0 || previous < rank {
                if let Some(composed) = join(result[at], point) {
                    result[at] = composed;
                    continue;
                }
            }
        }
        result.push(point);
        if rank == 0 { starter = Some(result.len() - 1); }
        previous = rank;
    }
    Ok(result)
}

// Bind direct native callables so whole-range callers avoid Python wrapper overhead.
pub const METHODS: [&str; 13] = ["category", "bidirectional", "combining", "mirrored", "east_asian_width", "decimal", "digit", "numeric", "decomposition", "name", "lookup", "normalize", "is_normalized"];
// Validate public arguments and preserve explicit defaults in the native call.
pub fn call(field: u8, old: bool, args: &[Value]) -> Result<Value, String> {
    let word = METHODS[field as usize];
    let optional = matches!(field, 5..=7 | 9);
    let minimum = if field >= 11 { 2 } else { 1 };
    if args.len() < minimum || args.len() > minimum + usize::from(optional) {
        let message = if field >= 11 { format!("{word} expected 2 arguments, got {}", args.len()) }
            else if optional { format!("{word} expected {} {} argument{}, got {}", if args.is_empty() { "at least" } else { "at most" }, if args.is_empty() { 1 } else { 2 }, if args.is_empty() { "" } else { "s" }, args.len()) }
            else { format!("{}.{}() takes exactly one argument ({} given)", if old { "UCD" } else { "unicodedata" }, word, args.len()) };
        return Err(format!("TypeError: {message}"));
    }
    let kind = |value: &Value| match value.contents() { Value::Null => "None".to_owned(), other => other.core_kind() };
    if field >= 10 {
        let first = if field == 10 {
            match args[0].contents() {
                Value::Bytes(bytes, false, _) => Value::text(&String::from_utf8_lossy(&bytes.borrow())),
                _ => args[0].type_text(),
            }
        } else { args[0].type_text() };
        let Value::Text(first) = first else {
            if field == 10 {
                let source_kind = args[0].contents().core_kind();
                return Err(if source_kind == "memoryview" || matches!(args[0].contents(), Value::Bytes(..) | Value::Export(_)) {
                    format!("TypeError: lookup() argument must be read-only bytes-like object, not {source_kind}")
                } else { format!("TypeError: a bytes-like object is required, not '{source_kind}'") });
            }
            return Err(format!("TypeError: {word}() argument 1 must be str, not {}", kind(&args[0])));
        };
        if field == 10 {
            return lookup(&first, old).map(Value::from_codes).ok_or_else(|| {
                if first.len() > 256 { "KeyError: name too long".to_owned() }
                else { format!("KeyError: undefined character name '{}'", first.split('\0').next().unwrap_or("")) }
            });
        }
        let codes = args[1].type_text().text_codes().ok_or_else(|| format!("TypeError: {word}() argument 2 must be str, not {}", kind(&args[1])))?;
        let normalized = normalize(&first, &codes, old)?;
        return Ok(if field == 12 { Value::Flag(normalized == codes) } else { Value::from_codes(normalized) });
    }
    let source = args[0].type_text();
    let argument = if optional { "argument 1" } else { "argument" };
    let codes = source.text_codes().ok_or_else(|| format!("TypeError: {word}() {argument} must be a unicode character, not {}", kind(&args[0])))?;
    let [point] = codes.as_slice() else { return Err(format!("TypeError: {word}(): {argument} must be a unicode character, not a string of length {}", codes.len())); };
    let result = match field {
        8 => Value::text(&decomposition(*point, old)),
        9 => name(*point, old).map_or(Value::Null, Value::text),
        _ => property(*point, field as usize, old)?,
    };
    if optional && matches!(result, Value::Null) {
        if let Some(default) = args.get(1) { return Ok(default.clone()); }
        let reason = match field { 5 => "not a decimal", 6 => "not a digit", 7 => "not a numeric character", _ => "no such name" };
        return Err(format!("ValueError: {reason}"));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    // Check stable ordering, blocking, old corrections, and surrogate preservation.
    #[test]
    fn normalization_and_versions() {
        assert!(normalize("invalid", &[], false).unwrap().is_empty());
        assert_eq!(normalize("NFC", &[0x61, 0x300, 0x327], false).unwrap(), [0xE0, 0x327]);
        assert_eq!(normalize("NFD", &[0x61, 0x300, 0x327], false).unwrap(), [0x61, 0x327, 0x300]);
        assert_eq!(normalize("NFC", &[0x1100, 0x300, 0x1161], false).unwrap(), [0x1100, 0x300, 0x1161]);
        assert_eq!(normalize("NFC", &[0xD800, 65, 0x30A, 0xDFFF], false).unwrap(), [0xD800, 0xC5, 0xDFFF]);
        assert_eq!(normalize("NFKC", &[0x1CCD6], false).unwrap(), [65]);
        assert_eq!(normalize("NFKC", &[0x1CCD6], true).unwrap(), [0x1CCD6]);
        assert_eq!(normalize("NFD", &[0x2F9BF], true).unwrap(), [0x4D57]);
        assert_eq!(decomposition(0xD7A3, false), "1112 1175 11C2");
        assert_eq!(lookup("KEYCAP NUMBER SIGN", false).unwrap(), [35, 65039, 8419]);
        assert!(lookup("LF", true).is_none());
        assert!(name(0x221, true).is_none());
    }
}
