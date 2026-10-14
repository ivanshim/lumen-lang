use crate::data::{Thing, Value};
use std::rc::Rc;
use std::cell::RefCell;

// Fetch a scanner record entry through its native storage.
fn slot(record: &Thing, key: &str) -> Value {
    let row = record.holds.borrow();
    row.iter().find_map(|(name, held)| (name == key).then(|| held.settled())).unwrap_or(Value::Nil)
}

// Decode the scanner's integer counters, declining other shapes.
fn counter(record: &Thing, key: &str) -> Option<i64> {
    if let Value::Small(value) = slot(record, key) { Some(value) } else { None }
}

// Publish a recognized token's new position and cache state.
fn store(record: &Thing, key: &str, held: Value) {
    let mut row = record.holds.borrow_mut();
    for (name, value) in row.iter_mut() { if name == key { *value = held; break; } }
}

// Keep the cell behind a private scanner list for in-place bracket updates.
fn sequence(record: &Thing, key: &str) -> Option<Rc<RefCell<Value>>> {
    let row = record.holds.borrow();
    let held = row.iter().find(|(name, _)| name == key)?.1.clone();
    if let Value::Mutable(cell, _) | Value::Shared(cell) = held { Some(cell) } else { None }
}

// Locate a single-line literal's closing delimiter, stepping over escaped
// characters. Anything needing another input line stays with the adapter.
fn closing_quote(input: &[u8], first: usize) -> Option<usize> {
    let mark = input[first];
    if input.get(first + 1) == Some(&mark) && input.get(first + 2) == Some(&mark) { return None; }
    let mut at = first + 1;
    while let Some(&byte) = input.get(at) {
        if byte == mark { return Some(at + 1); }
        if byte == 10 || byte == 13 { break; }
        if byte == 92 {
            at += 1;
            match input.get(at) { None | Some(10 | 13) => break, _ => () }
        }
        at += 1;
    }
    None
}

// Return a buffered token, leaving ambiguous syntax and input reads to the port.
pub fn atom(values: &[Value]) -> Result<Value, String> {
    if values.len() != 1 { return Err("TypeError: tokenizer scan needs an iterator".to_owned()); }
    let Value::Thing(owner) = values[0].settled() else { return Ok(Value::Nil); };
    let Value::Thing(scanner) = slot(&owner, "tok") else { return Ok(Value::Nil); };
    if !matches!(slot(&owner, "done"), Value::Flag(false)) { return Ok(Value::Nil); }
    if counter(&scanner, "pendin") != Some(0)
        || counter(&scanner, "done") != Some(10) { return Ok(Value::Nil); }
    let (Some(cursor), Some(limit), Some(line_base), Some(row_number), Some(nesting)) =
        (counter(&scanner, "cur"), counter(&scanner, "inp"), counter(&scanner, "line_start"), counter(&scanner, "lineno"), counter(&scanner, "tok_mode_stack_index"))
        else { return Ok(Value::Nil); };
    if line_base < 0 || cursor < line_base || nesting < 0 { return Ok(Value::Nil); }
    let Value::Vector(stack) = slot(&scanner, "tok_mode_stack") else { return Ok(Value::Nil); };
    let Some(Value::Thing(active)) = stack.get(nesting as usize).map(Value::settled) else { return Ok(Value::Nil); };
    if counter(&active, "kind") != Some(0) { return Ok(Value::Nil); }
    let beginning = match slot(&scanner, "atbol") { Value::Flag(yes) => yes, _ => return Ok(Value::Nil) };
    if beginning && cursor == limit { return Ok(Value::Flag(false)); }
    if cursor >= limit { return Ok(Value::Nil); }
    let Value::Text(source) = slot(&scanner, "buf") else { return Ok(Value::Nil); };
    let offset = (cursor - line_base) as usize;
    let Some(origin) = crate::text::string_boundary(&source, offset) else { return Ok(Value::Nil); };
    let input = source.as_bytes();
    let mut at = origin;
    while matches!(input.get(at), Some(b' ' | b'\t' | 12)) { at += 1; }
    let Some(&head) = input.get(at) else { return Ok(Value::Nil); };
    if beginning {
        if cursor != line_base || !input[origin..at].iter().all(|byte| *byte == 32)
            || matches!(head, 13 | 92 | 9 | 12) { return Ok(Value::Nil); }
        let trivia = head == 35 || head == 10;
        if trivia && !matches!(slot(&scanner, "tok_extra_tokens"), Value::Flag(true)) { return Ok(Value::Nil); }
        match counter(&scanner, "level") {
            Some(0) if !trivia => {
                let Some(index) = counter(&scanner, "indent").filter(|n| *n >= 0) else { return Ok(Value::Nil); };
                let Value::Vector(primary) = slot(&scanner, "indstack") else { return Ok(Value::Nil); };
                let Value::Vector(secondary) = slot(&scanner, "altindstack") else { return Ok(Value::Nil); };
                let width = (at - origin) as i64;
                for entries in [&primary, &secondary] {
                    if !matches!(entries.get(index as usize).map(Value::settled), Some(Value::Small(n)) if n == width) { return Ok(Value::Nil); }
                }
            }
            Some(n) if n >= 0 => (),
            _ => return Ok(Value::Nil),
        }
        store(&scanner, "cur", Value::Small(line_base + (at - origin) as i64));
        store(&scanner, "atbol", Value::Flag(false));
    }
    let mut after = at + 1;
    let tag = match head {
        b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
            while let Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_') = input.get(after) { after += 1; }
            if matches!(input.get(after), Some(34 | 39)) {
                let letters = source[at..after].to_ascii_lowercase();
                if !["r", "u", "b", "rb", "br"].contains(&letters.as_str()) { return Ok(Value::Nil); }
                after = match closing_quote(input, after) { Some(boundary) => boundary, None => return Ok(Value::Nil) };
                3
            } else {
                if input.get(after).is_none_or(|byte| *byte >= 128) { return Ok(Value::Nil); }
                0
            }
        }
        b'0'..=b'9' => {
            while matches!(input.get(after), Some(b'0'..=b'9')) { after += 1; }
            if head == 48 && after - at > 1 { return Ok(Value::Nil); }
            let Some(&following) = input.get(after) else { return Ok(Value::Nil); };
            if following >= 128 || following.is_ascii_alphanumeric() || following == 95 || following == 46 { return Ok(Value::Nil); }
            1
        }
        34 | 39 => {
            after = match closing_quote(input, at) { Some(boundary) => boundary, None => return Ok(Value::Nil) };
            3
        }
        10 => {
            if nesting != 0 { return Ok(Value::Nil); }
            let Some(level) = counter(&scanner, "level") else { return Ok(Value::Nil); };
            if level < 0 { return Ok(Value::Nil); }
            if beginning || level != 0 || matches!(slot(&scanner, "comment_newline"), Value::Flag(true)) {
                if !matches!(slot(&scanner, "tok_extra_tokens"), Value::Flag(true)) { return Ok(Value::Nil); }
                7
            } else { 5 }
        }
        35 => {
            if nesting != 0 || !matches!(slot(&scanner, "tok_extra_tokens"), Value::Flag(true)) { return Ok(Value::Nil); }
            while !matches!(input.get(after), None | Some(10 | 13)) { after += 1; }
            6
        }
        46 => {
            if matches!(input.get(after), Some(b'0'..=b'9')) { return Ok(Value::Nil); }
            if input.get(at..at + 3) == Some(&[46, 46, 46][..]) { after += 2; }
            2
        }
        40 | 41 | 91 | 93 | 123 | 125 => {
            if nesting != 0 { return Ok(Value::Nil); }
            4
        }
        33 | 37 | 38 | 42 | 43 | 44 | 45 | 47 | 58 | 59 | 60 | 61 | 62 | 64 | 94 | 124 | 126 => {
            if head == 58 && nesting > 0 { return Ok(Value::Nil); }
            let joined = match (head, input.get(after).copied()) {
                (33 | 37 | 38 | 42 | 43 | 45 | 47 | 58 | 60 | 61 | 62 | 64 | 94 | 124, Some(61)) => true,
                (42, Some(42)) | (47, Some(47)) | (60, Some(60)) | (62, Some(62)) | (45, Some(62)) => true,
                _ => false,
            };
            if joined {
                let second = input[after];
                after += 1;
                if head == second && matches!(head, 42 | 47 | 60 | 62) && input.get(after) == Some(&61) { after += 1; }
            }
            2
        }
        _ => return Ok(Value::Nil),
    };
    let column = offset + (at - origin);
    let stop = column + source[at..after].chars().count();
    let nesting_change = if tag == 4 {
        let Some(used) = counter(&scanner, "level") else { return Ok(Value::Nil); };
        let Some(capacity) = counter(&scanner, "_maxlevel") else { return Ok(Value::Nil); };
        let (Some(delimiters), Some(rows)) = (sequence(&scanner, "parenstack"), sequence(&scanner, "parenlinenostack")) else { return Ok(Value::Nil); };
        let add = matches!(head, 40 | 91 | 123);
        {
            let d = delimiters.borrow(); let r = rows.borrow();
            let (Value::Vector(left), Value::Vector(right)) = (&*d, &*r) else { return Ok(Value::Nil); };
            if used < 0 || left.len() != used as usize || left.len() != right.len() { return Ok(Value::Nil); }
            if add && used >= capacity { return Ok(Value::Nil); }
            if !add {
                let wanted = if head == 41 { "(" } else if head == 93 { "[" } else { "{" };
                if !matches!(left.last().map(Value::settled), Some(Value::Text(word)) if word.as_ref() == wanted) { return Ok(Value::Nil); }
            }
        }
        Some((add, used, delimiters, rows))
    } else { None };
    let Value::Tuple(kinds) = slot(&scanner, "_token_types") else { return Ok(Value::Nil); };
    if kinds.len() != 7 { return Ok(Value::Nil); }
    let token_type = match tag {
        2 | 4 => {
            let Value::Dict(lookup) = slot(&scanner, "_operator_types") else { return Ok(Value::Nil); };
            let Some(position) = lookup.namespace_position(&source[at..after]).ok().flatten() else { return Ok(Value::Nil); };
            if matches!(slot(&scanner, "tok_extra_tokens"), Value::Flag(true)) { kinds[3].clone() } else { lookup[position].1.clone() }
        }
        3 => kinds[2].clone(),
        5 => kinds[4].clone(),
        6 => kinds[5].clone(),
        7 => kinds[6].clone(),
        other => kinds[other as usize].clone(),
    };
    let text_line = match counter(&owner, "last_lineno") {
        Some(previous) if previous == row_number => {
            let old = slot(&owner, "last_line");
            if matches!(old, Value::Nil) { return Ok(Value::Nil); }
            old
        }
        _ if matches!(slot(&scanner, "implicit_newline"), Value::Flag(true)) => {
            let Some(original) = source.strip_suffix('\n') else { return Ok(Value::Nil); };
            Value::text(original)
        }
        _ => Value::Text(source.clone()),
    };
    if let Some((add, used, delimiters, rows)) = nesting_change {
        if let Value::Vector(parts) = &mut *delimiters.borrow_mut() {
            let entries = Rc::make_mut(parts);
            if add { entries.push(Value::text(&source[at..after])); } else { entries.pop(); }
        }
        if let Value::Vector(parts) = &mut *rows.borrow_mut() {
            let entries = Rc::make_mut(parts);
            if add { entries.push(Value::Small(row_number)); } else { entries.pop(); }
        }
        store(&scanner, "level", Value::Small(if add { used + 1 } else { used - 1 }));
    }
    store(&scanner, "cur", Value::Small(line_base + stop as i64));
    store(&scanner, "start", Value::Small(line_base + column as i64));
    if tag == 3 {
        store(&scanner, "multi_line_start", Value::Small(line_base));
        store(&scanner, "first_lineno", Value::Small(row_number));
    }
    if tag == 2 && source[at..after].eq("=") {
        match (counter(&active, "curly_bracket_depth"), counter(&active, "curly_bracket_expr_start_depth")) {
            (Some(current), Some(beginning)) if current - beginning == 1 => store(&active, "in_debug", Value::Flag(true)),
            _ => (),
        }
    }
    match tag {
        5 => { store(&scanner, "cont_line", Value::Flag(false)); store(&scanner, "atbol", Value::Flag(true)); }
        6 => store(&scanner, "comment_newline", Value::Flag(beginning)),
        7 => {
            store(&scanner, "comment_newline", Value::Flag(false));
            store(&scanner, "atbol", Value::Flag(true));
        }
        _ => (),
    }
    let extra_tokens = matches!(slot(&scanner, "tok_extra_tokens"), Value::Flag(true));
    let synthetic = matches!(slot(&scanner, "implicit_newline"), Value::Flag(true));
    let lexeme = if (tag == 5 && (!extra_tokens || synthetic)) || (tag == 7 && synthetic) { Value::text("") } else { Value::text(&source[at..after]) };
    let endpoint = if tag == 5 && !extra_tokens { column } else { stop };
    store(&owner, "last_lineno", Value::Small(row_number));
    store(&owner, "last_line", text_line.clone());
    let begin_pair = Value::tuple(vec![Value::Small(row_number), Value::Small(column as i64)]);
    let end_pair = Value::tuple(vec![Value::Small(row_number), Value::Small(endpoint as i64)]);
    Ok(Value::tuple(vec![token_type, lexeme, begin_pair, end_pair, text_line]))
}
