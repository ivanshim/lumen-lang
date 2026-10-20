// Supply UCD properties and normalization from reproducible release tables.
use crate::data::{Value, worth_of_binary};
pub use lumen_unicode::decomposition;
use lumen_unicode::{fields as record, name, lookup, normalize};

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
