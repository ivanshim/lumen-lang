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
fn span_state(parameters: &[Value]) -> Option<Value> {
    let Value::Text(input) = parameters.get(3)?.settled() else { return None };
    if !input.is_ascii() { return None; }
    let Value::Small(number) = parameters.get(4)?.settled() else { return None };
    let begin = usize::try_from(number).ok()?.min(input.len());
    if let Some(parts) = sequence(parameters.get(1)?) {
        let (Value::Small(minimum), Value::Small(maximum), Value::Text(alphabet)) = (parts.get(1)?.settled(), parts.get(2)?.settled(), parts.get(4)?.settled()) else { return None };
        let available = if maximum < 0 { input.len() - begin } else { (input.len() - begin).min(maximum as usize) };
        let wanted = if parts.get(3)?.is_true() { available } else { available.min(minimum.max(0) as usize) };
        let taken = input.as_bytes()[begin..begin + wanted].iter().take_while(|byte| alphabet.as_bytes().contains(byte)).count();
        if taken < minimum.max(0) as usize { return Some(Value::Flag(false)); }
        return Some(Value::Vector(crate::tuples::Sequence::plain(vec![Value::Small((begin + taken) as i64), Value::Dict(std::rc::Rc::new(Vec::new().into()))])));
    }
    let plan = sequence(parameters.get(2)?)?;
    let (Value::Small(a), Value::Small(b), Value::Text(accepted), Value::Text(passed)) = (plan.first()?.settled(), plan.get(1)?.settled(), plan.get(3)?.settled(), plan.get(4)?.settled()) else { return None };
    let skipped = input.as_bytes()[begin..].iter().take_while(|byte| passed.as_bytes().contains(byte)).count();
    let boundary = begin + skipped;
    let Some(byte) = input.as_bytes().get(boundary) else { return Some(Value::Flag(false)) };
    if !accepted.as_bytes().contains(byte) { return Some(Value::Flag(false)); }
    let pair = |lo, hi| Value::Vector(crate::tuples::Sequence::plain(vec![Value::Small(lo as i64), Value::Small(hi as i64)]));
    let entries = vec![(Value::Small(a), pair(begin, boundary)), (Value::Small(b), pair(boundary, boundary + 1))];
    Some(Value::Vector(crate::tuples::Sequence::plain(vec![Value::Small((boundary + 1) as i64), Value::Dict(std::rc::Rc::new(entries.into()))])))
}

pub(crate) fn apply(values: &[Value]) -> Result<Value, String> {
    let task = values.first().map(Value::bare).unwrap_or_default();
    if task == "match" && values.len() == 5 { return Ok(span_state(values).unwrap_or(Value::Nil)); }
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
