use crate::data::Value;

fn sequence(value: &Value) -> Option<Vec<Value>> {
    if let Value::Vector(values) | Value::Tuple(values) = value.settled() { Some(values.to_vec()) } else { None }
}
fn ascii_character(value: &Value) -> Option<u8> {
    if let Value::Text(word) = value.settled() {
        if word.is_ascii() && word.len() == 1 { return word.bytes().next(); }
    }
    None
}
fn matches_atom(tree: &Value, code: u8, mode: &[Value]) -> Option<bool> {
    let cells = sequence(tree)?;
    let tag = cells.first()?.bare();
    let fold = mode.first()?.is_true();
    if tag == "dot" { return Some(mode.get(2)?.is_true() || code != 10); }
    if tag == "lit" {
        let literal = ascii_character(cells.get(1)?)?;
        return Some(if fold { literal.to_ascii_lowercase() == code.to_ascii_lowercase() } else { code == literal });
    }
    if tag == "range" {
        let low = ascii_character(cells.get(1)?)?;
        let high = ascii_character(cells.get(2)?)?;
        let mut candidates = vec![code];
        if fold { candidates.extend([code.to_ascii_lowercase(), code.to_ascii_uppercase()]); }
        return Some(candidates.into_iter().any(|candidate| (low..=high).contains(&candidate)));
    }
    if tag == "class" {
        let entries = sequence(cells.get(2)?)?;
        let mut yes = false;
        for entry in entries { if matches_atom(&entry, code, mode)? { yes = true; } }
        return Some(if cells.get(1)?.is_true() { !yes } else { yes });
    }
    if tag != "kind" { return None; }
    let category = ascii_character(cells.get(1)?)?;
    let positive = match category {
        b'd' | b'D' => (b'0'..=b'9').contains(&code),
        b'w' | b'W' => code == b'_' || code.is_ascii_digit() || code.is_ascii_alphabetic(),
        b's' | b'S' => matches!(code, 9..=13 | 32) || matches!(code, 28..=31) && !mode.get(3)?.is_true(),
        _ => return None,
    };
    Some(if matches!(category, b'D' | b'W' | b'S') { !positive } else { positive })
}
fn capture_offsets(mapping: &Value, number: i64) -> Option<(usize, usize)> {
    let Value::Dict(records) = mapping.settled() else { return None };
    for (key, record) in records.iter() {
        if !matches!(key.settled(), Value::Small(found) if found == number) { continue; }
        let items = sequence(record)?;
        if items.len() != 2 { return None; }
        if let (Value::Small(first), Value::Small(last)) = (&items[0], &items[1]) {
            return usize::try_from(*first).ok().zip(usize::try_from(*last).ok());
        }
    }
    None
}
pub(crate) fn apply(values: &[Value]) -> Result<Value, String> {
    let task = values.first().map(Value::bare).unwrap_or_default();
    if task == "ascii" && values.len() == 4 {
        let Some(settings) = sequence(&values[2]) else { return Ok(Value::Nil) };
        let excluded = values[3].is_true();
        let mut output = Vec::new();
        for number in 0..=127u8 {
            let Some(yes) = matches_atom(&values[1], number, &settings) else { return Ok(Value::Nil) };
            let newline_allowed = !excluded || number != 10 || settings.get(2).is_some_and(Value::is_true);
            if yes != excluded && newline_allowed { output.push(number); }
        }
        return Ok(Value::text(std::str::from_utf8(&output).unwrap()));
    }
    if task == "end" && values.len() == 4 {
        if let (Value::Small(n), Value::Small(total)) = (values[2].settled(), values[3].settled()) {
            if !(0..=total).contains(&n) { return Err("IndexError: no such group".to_owned()); }
            return Ok(Value::Small(capture_offsets(&values[1], n).map(|pair| pair.1 as i64).unwrap_or(-1)));
        }
    }
    if task == "groups" && values.len() == 5 {
        if let (Value::Text(source), Value::Small(total)) = (values[1].settled(), values[3].settled()) {
            let mut result = Vec::with_capacity(total.max(0) as usize);
            for group in 1..=total {
                if let Some((from, until)) = capture_offsets(&values[2], group) {
                    let fragment: String = source.chars().enumerate().filter_map(|(index, ch)| (index >= from && index < until).then_some(ch)).collect();
                    result.push(Value::text(&fragment));
                } else { result.push(values[4].clone()); }
            }
            return Ok(Value::tuple(result));
        }
    }
    Ok(Value::Nil)
}
