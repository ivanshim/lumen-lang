// Read generated Unicode 16.0 properties and CPython's 3.2 compatibility view.
use crate::value::{Value, real_of};

pub use lumen_unicode::decomposition;
use lumen_unicode::{fields, name, lookup, normalize};

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
