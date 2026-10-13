// Supply UCD properties and normalization from reproducible release tables.
use crate::data::{Value, worth_of_binary};
use std::collections::BTreeMap;
use std::sync::OnceLock;

struct Tables {
    boundaries: Vec<u32>,
    properties: Vec<Vec<&'static str>>,
    mappings: BTreeMap<u32, &'static str>,
}
// Load compact runs for the modern or historical database.
fn tables(historic: bool) -> &'static Tables {
    static MODERN: OnceLock<Tables> = OnceLock::new();
    static LEGACY: OnceLock<Tables> = OnceLock::new();
    let cache = if historic { &LEGACY } else { &MODERN };
    cache.get_or_init(|| {
        let runs = if historic { include_str!("../../../unicode-data/generated/properties-3.2.tsv") }
            else { include_str!("../../../unicode-data/generated/properties.tsv") };
        let spelling = if historic { include_str!("../../../unicode-data/generated/decomposition-3.2.tsv") }
            else { include_str!("../../../unicode-data/generated/decomposition.tsv") };
        let mut boundaries = Vec::new();
        let mut properties = Vec::new();
        for record in runs.lines() {
            let columns: Vec<_> = record.split('\t').collect();
            boundaries.push(u32::from_str_radix(columns[1], 16).expect("UCD boundary"));
            properties.push(columns[2..].to_vec());
        }
        let mappings = spelling.lines().map(|record| {
            let (number, content) = record.split_once('\t').expect("UCD mapping");
            (u32::from_str_radix(number, 16).expect("UCD number"), content)
        }).collect();
        Tables { boundaries, properties, mappings }
    })
}
// Find the range containing a Unicode number without walking preceding ranges.
fn record(number: u32, historic: bool) -> &'static [&'static str] {
    let selected = tables(historic);
    let position = selected.boundaries.partition_point(|&end| end < number);
    &selected.properties[position]
}
// Convert a Unicode database field to its Python scalar value.
pub fn property(number: u32, index: usize, historic: bool) -> Result<Value, String> {
    let row = record(number, historic);
    let Some(&content) = row.get(index) else { return Err(String::from("ValueError: unknown Unicode property")); };
    if matches!(index, 0 | 1 | 4) { return Ok(Value::text(content)); }
    if content.is_empty() { return Ok(Value::Nil); }
    if index == 7 {
        let mut fraction = content.split('/');
        let numerator: f64 = fraction.next().unwrap().parse().unwrap();
        let denominator: f64 = fraction.next().unwrap_or("1").parse().unwrap();
        return Ok(worth_of_binary(numerator / denominator, 15));
    }
    Ok(Value::Small(content.parse().expect("UCD integral property")))
}
// Spell Hangul jamo algorithmically, otherwise return the recorded mapping.
pub fn decomposition(number: u32, historic: bool) -> String {
    if record(number, historic)[0] == "Cn" { return String::new(); }
    match number.checked_sub(0xAC00) {
        Some(s) if s < 19 * 21 * 28 => {
            let mut parts = vec![0x1100 + s / (21 * 28), 0x1161 + s / 28 % 21];
            if s % 28 > 0 { parts.push(0x11A7 + s % 28); }
            parts.iter().map(|code| format!("{code:04X}")).collect::<Vec<_>>().join(" ")
        }
        _ => tables(historic).mappings.get(&number).unwrap_or(&"").to_string(),
    }
}
// Keep canonical names indexed by character number.
fn canonical() -> &'static BTreeMap<u32, &'static str> {
    static CANONICAL: OnceLock<BTreeMap<u32, &'static str>> = OnceLock::new();
    CANONICAL.get_or_init(|| {
        include_str!("../../../unicode-data/generated/names.tsv").lines().map(|entry| {
            let pair: Vec<_> = entry.split('\t').collect();
            (u32::from_str_radix(pair[0], 16).unwrap(), pair[1])
        }).collect()
    })
}
// Reject names of points absent from Unicode 3.2 when the old view is selected.
pub fn name(number: u32, historic: bool) -> Option<&'static str> {
    if historic && record(number, historic)[0] == "Cn" { None }
    else { canonical().get(&number).copied() }
}
// Build separate reverse indexes so historical lookups cannot see aliases.
pub fn lookup(word: &str, historic: bool) -> Option<Vec<u32>> {
    static TODAY: OnceLock<BTreeMap<String, Vec<u32>>> = OnceLock::new();
    static YESTERDAY: OnceLock<BTreeMap<String, Vec<u32>>> = OnceLock::new();
    let slot = if historic { &YESTERDAY } else { &TODAY };
    let dictionary = slot.get_or_init(|| {
        let mut dictionary = BTreeMap::new();
        for (&number, &label) in canonical() {
            dictionary.insert(label.to_string(), vec![number]);
        }
        for line in include_str!("../../../unicode-data/generated/lookup-only.tsv").lines() {
            let (label, code) = line.split_once('\t').unwrap();
            dictionary.insert(label.to_string(), vec![u32::from_str_radix(code, 16).unwrap()]);
        }
        if !historic {
            for entry in include_str!("../../../unicode-data/generated/aliases.tsv").lines() {
                let (label, sequence) = entry.split_once('\t').unwrap();
                let characters = sequence.split_whitespace().map(|n| u32::from_str_radix(n, 16).unwrap()).collect();
                dictionary.insert(label.to_string(), characters);
            }
        }
        dictionary
    });
    dictionary.get(&word.to_ascii_uppercase()).cloned()
}
// Apply historical normalization corrections before recursive expansion.
fn spread(number: u32, historic: bool, compatible: bool, output: &mut Vec<u32>) {
    static CHANGES: OnceLock<BTreeMap<u32, u32>> = OnceLock::new();
    let code = if historic {
        let changes = CHANGES.get_or_init(|| {
            include_str!("../../../unicode-data/generated/normalization-3.2.tsv").lines().map(|entry| {
                let mut words = entry.split('\t').map(|s| u32::from_str_radix(s, 16).unwrap());
                (words.next().unwrap(), words.next().unwrap())
            }).collect()
        });
        changes.get(&number).copied().unwrap_or(number)
    } else { number };
    if (0xAC00..0xD7A4).contains(&code) {
        let offset = code - 0xAC00;
        output.push(0x1100 + offset / 588);
        output.push(0x1161 + offset / 28 % 21);
        if offset % 28 > 0 { output.push(0x11A7 + offset % 28); }
    } else {
        let mapping = tables(historic).mappings.get(&code);
        match mapping {
            Some(written) if compatible || !written.starts_with('<') => {
                for item in written.split_whitespace() {
                    if item.starts_with('<') { continue; }
                    spread(u32::from_str_radix(item, 16).unwrap(), historic, compatible, output);
                }
            }
            _ => output.push(code),
        }
    }
}
// Read canonical composition pairs after testing Hangul's two joining rules.
fn composed(left: u32, right: u32) -> Option<u32> {
    let l = left.wrapping_sub(0x1100);
    let v = right.wrapping_sub(0x1161);
    if l < 19 && v < 21 { return Some(0xAC00 + (l * 21 + v) * 28); }
    let s = left.wrapping_sub(0xAC00);
    let t = right.wrapping_sub(0x11A7);
    if s < 11172 && s % 28 == 0 && (1..28).contains(&t) { return Some(left + t); }
    static COMPOSITIONS: OnceLock<BTreeMap<(u32, u32), u32>> = OnceLock::new();
    let compositions = COMPOSITIONS.get_or_init(|| {
        let mut entries = BTreeMap::new();
        for line in include_str!("../../../unicode-data/generated/composition.tsv").lines() {
            let trio: Vec<u32> = line.split('\t').map(|s| u32::from_str_radix(s, 16).unwrap()).collect();
            entries.insert((trio[0], trio[1]), trio[2]);
        }
        entries
    });
    compositions.get(&(left, right)).copied()
}
// Implement all four forms with stable mark sorting and blocked composition.
pub fn normalize(form: &str, numbers: &[u32], historic: bool) -> Result<Vec<u32>, String> {
    if numbers.len() == 0 { return Ok(numbers.to_vec()); }
    let (compatible, combine) = match form {
        "NFC" => (false, true), "NFD" => (false, false),
        "NFKC" => (true, true), "NFKD" => (true, false),
        _ => return Err(String::from("ValueError: invalid normalization form")),
    };
    if numbers.iter().all(|&n| n <= 127) { return Ok(numbers.to_vec()); }
    let mut flattened = Vec::new();
    for &number in numbers { spread(number, historic, compatible, &mut flattened); }
    let rank = |number| record(number, false)[2].parse::<u8>().expect("combining class");
    let mut cursor = 0;
    while cursor < flattened.len() {
        if rank(flattened[cursor]) == 0 { cursor += 1; }
        let beginning = cursor;
        while cursor < flattened.len() && rank(flattened[cursor]) != 0 { cursor += 1; }
        flattened[beginning..cursor].sort_by_key(|&number| rank(number));
    }
    if !combine { return Ok(flattened); }
    let mut answer: Vec<u32> = Vec::new();
    let mut anchor = None;
    let mut last_rank = 0;
    for number in flattened {
        let current_rank = rank(number);
        let replacement = anchor.and_then(|place: usize| {
            if last_rank < current_rank || last_rank == 0 { composed(answer[place], number) } else { None }
        });
        match replacement {
            Some(new_number) => answer[anchor.unwrap()] = new_number,
            None => {
                if current_rank == 0 { anchor = Some(answer.len()); }
                answer.push(number);
                last_rank = current_rank;
            }
        }
    }
    Ok(answer)
}

// Name the native query family without introducing language-definition labels.
pub const METHODS: [&str; 13] = ["category", "bidirectional", "combining", "mirrored", "east_asian_width", "decimal", "digit", "numeric", "decomposition", "name", "lookup", "normalize", "is_normalized"];
// Check receivers and defaults at the public native interface.
pub fn invoke(query: u8, historic: bool, supplied: &[Value]) -> Result<Value, String> {
    let name_of_method = METHODS[usize::from(query)];
    let default_allowed = (5..=7).contains(&query) || query == 9;
    let required = usize::from(query >= 11) + 1;
    if !(required..=required + usize::from(default_allowed)).contains(&supplied.len()) {
        let count = supplied.len();
        let explanation = match query {
            11 | 12 => format!("{name_of_method} expected 2 arguments, got {count}"),
            5..=7 | 9 if count == 0 => format!("{name_of_method} expected at least 1 argument, got 0"),
            5..=7 | 9 => format!("{name_of_method} expected at most 2 arguments, got {count}"),
            _ => format!("{}.{}() takes exactly one argument ({count} given)", if historic { "UCD" } else { "unicodedata" }, name_of_method),
        };
        return Err(format!("TypeError: {explanation}"));
    }
    let kind = |value: &Value| match value.settled() { Value::Nil => String::from("None"), other => other.kind_word() };
    let receiver = match supplied[0].settled() {
        Value::Octets { cell, changeable: false, .. } if query == 10 => Value::text(&String::from_utf8_lossy(&cell.borrow())),
        _ => supplied[0].type_text(),
    };
    if query >= 10 {
        let label = match receiver {
            Value::Text(word) => word,
            _ if query == 10 => {
                let typename = supplied[0].settled().kind_word();
                let words = if typename == "memoryview" || matches!(supplied[0].settled(), Value::Octets { .. } | Value::Export(_)) {
                    format!("lookup() argument must be read-only bytes-like object, not {typename}")
                } else { format!("a bytes-like object is required, not '{typename}'") };
                return Err(format!("TypeError: {words}"));
            }
            _ => return Err(format!("TypeError: {name_of_method}() argument 1 must be str, not {}", kind(&supplied[0]))),
        };
        if query == 10 {
            let numbers = lookup(&label, historic).ok_or_else(|| {
                if label.len() > 256 { String::from("KeyError: name too long") }
                else { format!("KeyError: undefined character name '{}'", label.split('\0').next().unwrap_or("")) }
            })?;
            return Ok(Value::characters(numbers));
        }
        let source = supplied[1].type_text().character_numbers().ok_or_else(|| format!("TypeError: {name_of_method}() argument 2 must be str, not {}", kind(&supplied[1])))?;
        let result = normalize(&label, &source, historic)?;
        return if query == 12 { Ok(Value::Flag(source == result)) } else { Ok(Value::characters(result)) };
    }
    let argument = if default_allowed { "argument 1" } else { "argument" };
    let numbers = receiver.character_numbers().ok_or_else(|| format!("TypeError: {name_of_method}() {argument} must be a unicode character, not {}", kind(&supplied[0])))?;
    if numbers.len() != 1 { return Err(format!("TypeError: {name_of_method}(): {argument} must be a unicode character, not a string of length {}", numbers.len())); }
    let code = numbers[0];
    let answer = if query == 8 { Value::text(&decomposition(code, historic)) }
        else if query == 9 { name(code, historic).map(Value::text).unwrap_or(Value::Nil) }
        else { property(code, usize::from(query), historic)? };
    if default_allowed && matches!(answer, Value::Nil) {
        return match supplied.get(1) {
            Some(default) => Ok(default.clone()),
            None => Err(String::from(match query {
                5 => "ValueError: not a decimal", 6 => "ValueError: not a digit",
                7 => "ValueError: not a numeric character", _ => "ValueError: no such name",
            })),
        };
    }
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::{normalize, decomposition, lookup, name};
    // Exercise the UCD boundary rules and both historical and modern forms.
    #[test]
    fn unicode_forms() {
        for (form, input, historic, expected) in [
            ("NFD", vec![0x61, 0x300, 0x327], false, vec![0x61, 0x327, 0x300]),
            ("NFC", vec![0x61, 0x300, 0x327], false, vec![0xE0, 0x327]),
            ("NFC", vec![0x1100, 0x300, 0x1161], false, vec![0x1100, 0x300, 0x1161]),
            ("NFC", vec![0xD800, 65, 0x30A, 0xDFFF], false, vec![0xD800, 0xC5, 0xDFFF]),
            ("NFKC", vec![0x1CCD6], true, vec![0x1CCD6]),
            ("NFKC", vec![0x1CCD6], false, vec![65]),
            ("NFD", vec![0x2F9BF], true, vec![0x4D57]),
        ] {
            assert_eq!(normalize(form, &input, historic).unwrap(), expected);
        }
        assert_eq!(decomposition(0xAC00, true), "1100 1161");
        assert_eq!(lookup("KEYCAP NUMBER SIGN", false), Some(vec![35, 65039, 8419]));
        assert_eq!(lookup("LF", true), None);
        assert_eq!(name(0x221, true), None);
    }
}
