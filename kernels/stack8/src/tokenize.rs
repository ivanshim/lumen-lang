use crate::value::{Instance, Value};
use std::{cell::RefCell, rc::Rc};

// Read a private adapter state field without Python descriptor dispatch.
fn field(object: &Instance, name: &str) -> Value {
    object.fields.borrow().iter().find(|(key, _)| key == name).map_or(Value::Null, |(_, value)| value.contents())
}

// Read a small integer from the scanner's private state.
fn number(object: &Instance, name: &str) -> Option<i64> {
    match field(object, name) { Value::Small(n) => Some(n), _ => None }
}

// Replace an existing private state slot after a lexeme is fully recognized.
fn put(object: &Instance, name: &str, value: Value) {
    if let Some((_, slot)) = object.fields.borrow_mut().iter_mut().find(|(key, _)| key == name) { *slot = value; }
}

// Locate a private mutable list while preserving its holding cell.
fn list(object: &Instance, name: &str) -> Option<Rc<RefCell<Value>>> {
    match object.fields.borrow().iter().find(|(key, _)| key == name)?.1.clone() {
        Value::Collection(cell, _) | Value::Bond(cell) => Some(cell), _ => None,
    }
}

// Recognize buffered tokens; leave refills and diagnostic cases to the adapter.
pub fn scan(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 { return Err("TypeError: tokenizer scan needs an iterator".into()); }
    let Value::Object(iterator) = args[0].contents() else { return Ok(Value::Null); };
    let Value::Object(state) = field(&iterator, "tok") else { return Ok(Value::Null); };
    if !matches!(field(&iterator, "done"), Value::Flag(false))
        || number(&state, "pendin") != Some(0) || number(&state, "done") != Some(10) { return Ok(Value::Null); }
    let (Some(cur), Some(inp), Some(base), Some(lineno), Some(depth)) =
        (number(&state, "cur"), number(&state, "inp"), number(&state, "line_start"), number(&state, "lineno"), number(&state, "tok_mode_stack_index"))
        else { return Ok(Value::Null); };
    if base < 0 || cur < base || depth < 0 { return Ok(Value::Null); }
    let Value::Array(modes) = field(&state, "tok_mode_stack") else { return Ok(Value::Null); };
    let Some(Value::Object(mode)) = modes.get(depth as usize).map(Value::contents) else { return Ok(Value::Null); };
    if number(&mode, "kind") != Some(0) { return Ok(Value::Null); }
    let atbol = match field(&state, "atbol") { Value::Flag(value) => value, _ => return Ok(Value::Null) };
    // Request the adapter's usual next-character call so reader callbacks and faults stay unchanged.
    if atbol && cur == inp { return Ok(Value::Flag(false)); }
    if cur >= inp { return Ok(Value::Null); }
    let Value::Text(text) = field(&state, "buf") else { return Ok(Value::Null); };
    let position = (cur - base) as usize;
    let Some(byte) = crate::strings::character_byte(&text, position) else { return Ok(Value::Null); };
    let bytes = text.as_bytes();
    let mut start = byte;
    while bytes.get(start).is_some_and(|c| matches!(c, b' ' | b'\t' | 12)) { start += 1; }
    let Some(&first) = bytes.get(start) else { return Ok(Value::Null); };
    if atbol {
        if cur != base || bytes[byte..start].iter().any(|c| *c != b' ')
            || matches!(first, b'\r' | b'\\' | b'\t' | 12) { return Ok(Value::Null); }
        let Some(level) = number(&state, "level") else { return Ok(Value::Null); };
        if level < 0 { return Ok(Value::Null); }
        let blank = matches!(first, b'#' | b'\n');
        if blank && !matches!(field(&state, "tok_extra_tokens"), Value::Flag(true)) { return Ok(Value::Null); }
        if level == 0 && !blank {
            let Some(indent) = number(&state, "indent").filter(|n| *n >= 0) else { return Ok(Value::Null); };
            let (Value::Array(columns), Value::Array(alternates)) = (field(&state, "indstack"), field(&state, "altindstack")) else { return Ok(Value::Null); };
            let column = (start - byte) as i64;
            if !matches!(columns.get(indent as usize).map(Value::contents), Some(Value::Small(n)) if n == column)
                || !matches!(alternates.get(indent as usize).map(Value::contents), Some(Value::Small(n)) if n == column) { return Ok(Value::Null); }
        }
        put(&state, "atbol", Value::Flag(false));
        put(&state, "cur", Value::Small(base + (start - byte) as i64));
    }
    let mut end = start + 1;
    let kind;
    if first.is_ascii_alphabetic() || first == b'_' {
        while bytes.get(end).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') { end += 1; }
        if !bytes.get(end).is_some_and(|c| c.is_ascii() && !matches!(c, b'\'' | b'"')) { return Ok(Value::Null); }
        kind = 0;
    } else if first.is_ascii_digit() {
        while bytes.get(end).is_some_and(u8::is_ascii_digit) { end += 1; }
        if first == b'0' && end != start + 1 { return Ok(Value::Null); }
        if !bytes.get(end).is_some_and(|c| c.is_ascii() && !c.is_ascii_alphanumeric() && !matches!(c, b'_' | b'.')) { return Ok(Value::Null); }
        kind = 1;
    } else if matches!(first, b'\'' | b'"') {
        if bytes.get(end) == Some(&first) && bytes.get(end + 1) == Some(&first) { return Ok(Value::Null); }
        loop {
            match bytes.get(end) {
                Some(c) if *c == first => { end += 1; break; }
                None | Some(b'\\' | b'\n' | b'\r') => return Ok(Value::Null),
                _ => end += 1,
            }
        }
        kind = 3;
    } else if first == b'\n' {
        if depth != 0 { return Ok(Value::Null); }
        let Some(level) = number(&state, "level").filter(|n| *n >= 0) else { return Ok(Value::Null); };
        if atbol || level > 0 || matches!(field(&state, "comment_newline"), Value::Flag(true)) {
            if !matches!(field(&state, "tok_extra_tokens"), Value::Flag(true)) { return Ok(Value::Null); }
            kind = 7;
        } else { kind = 5; }
    } else if first == b'#' {
        if depth != 0 || !matches!(field(&state, "tok_extra_tokens"), Value::Flag(true)) { return Ok(Value::Null); }
        while bytes.get(end).is_some_and(|c| !matches!(c, b'\r' | b'\n')) { end += 1; }
        kind = 6;
    } else if first == b'.' {
        if bytes.get(end).is_some_and(u8::is_ascii_digit) { return Ok(Value::Null); }
        if bytes.get(start..start + 3) == Some(b"...") { end = start + 3; }
        kind = 2;
    } else if matches!(first, b'(' | b')' | b'[' | b']' | b'{' | b'}') {
        if depth != 0 { return Ok(Value::Null); }
        kind = 4;
    } else if b"%,;^~+-*/|&<>=@!:".contains(&first) {
        if first == b':' && depth != 0 { return Ok(Value::Null); }
        if let Some(pair) = bytes.get(start..start + 2) {
            if matches!(pair, b":=" | b"!=" | b"%=" | b"&=" | b"**" | b"*=" | b"+=" | b"-=" | b"->" | b"//" | b"/=" | b"<<" | b"<=" | b"==" | b">=" | b">>" | b"@=" | b"^=" | b"|=") {
                end += 1;
                if matches!(pair, b"**" | b"//" | b"<<" | b">>") && bytes.get(end) == Some(&b'=') { end += 1; }
            }
        }
        kind = 2;
    } else { return Ok(Value::Null); }
    let begin = position + start - byte;
    let finish = begin + text[start..end].chars().count();
    let brackets = if kind == 4 {
        let (Some(level), Some(maximum), Some(chars), Some(lines)) =
            (number(&state, "level"), number(&state, "_maxlevel"), list(&state, "parenstack"), list(&state, "parenlinenostack"))
            else { return Ok(Value::Null); };
        let opening = matches!(first, b'(' | b'[' | b'{');
        {
            let held_chars = chars.borrow();
            let held_lines = lines.borrow();
            let (Value::Array(a), Value::Array(b)) = (&*held_chars, &*held_lines) else { return Ok(Value::Null); };
            if level < 0 || a.len() != level as usize || b.len() != a.len() { return Ok(Value::Null); }
            if opening {
                if level >= maximum { return Ok(Value::Null); }
            } else {
                let expected = match first { b')' => "(", b']' => "[", _ => "{" };
                if a.last().is_none_or(|value| value.plain() != expected) { return Ok(Value::Null); }
            }
        }
        Some((chars, lines, level, opening))
    } else { None };
    let Value::Tuple(types) = field(&state, "_token_types") else { return Ok(Value::Null); };
    if types.len() != 7 { return Ok(Value::Null); }
    let token = if matches!(kind, 2 | 4) {
        let Value::Map(operators) = field(&state, "_operator_types") else { return Ok(Value::Null); };
        let Some(at) = operators.named_row(&text[start..end]).ok().flatten() else { return Ok(Value::Null); };
        if matches!(field(&state, "tok_extra_tokens"), Value::Flag(true)) { types[3].clone() } else { operators[at].1.clone() }
    } else { types[match kind { 3 => 2, 5 => 4, 6 => 5, 7 => 6, other => other as usize }].clone() };
    let line = if number(&iterator, "last_lineno") == Some(lineno) {
        let cached = field(&iterator, "last_line");
        if matches!(cached, Value::Null) { return Ok(Value::Null); }
        cached
    } else if matches!(field(&state, "implicit_newline"), Value::Flag(true)) {
        let Some(line) = text.strip_suffix('\n') else { return Ok(Value::Null); };
        Value::text(line)
    } else { Value::Text(text.clone()) };
    if let Some((chars, lines, level, opening)) = brackets {
        if let Value::Array(row) = &mut *chars.borrow_mut() {
            if opening { Rc::make_mut(row).push(Value::text(&text[start..end])); } else { Rc::make_mut(row).pop(); }
        }
        if let Value::Array(row) = &mut *lines.borrow_mut() {
            if opening { Rc::make_mut(row).push(Value::Small(lineno)); } else { Rc::make_mut(row).pop(); }
        }
        put(&state, "level", Value::Small(level + if opening { 1 } else { -1 }));
    }
    put(&state, "start", Value::Small(base + begin as i64));
    put(&state, "cur", Value::Small(base + finish as i64));
    if kind == 3 {
        put(&state, "first_lineno", Value::Small(lineno));
        put(&state, "multi_line_start", Value::Small(base));
    } else if kind == 2 && &text[start..end] == "=" {
        if let (Some(level), Some(first)) = (number(&mode, "curly_bracket_depth"), number(&mode, "curly_bracket_expr_start_depth")) {
            if level - first == 1 { put(&mode, "in_debug", Value::Flag(true)); }
        }
    }
    if kind == 5 {
        put(&state, "atbol", Value::Flag(true));
        put(&state, "cont_line", Value::Flag(false));
    } else if kind == 6 { put(&state, "comment_newline", Value::Flag(atbol)); }
    else if kind == 7 {
        put(&state, "atbol", Value::Flag(true));
        put(&state, "comment_newline", Value::Flag(false));
    }
    let extra = matches!(field(&state, "tok_extra_tokens"), Value::Flag(true));
    let implicit = matches!(field(&state, "implicit_newline"), Value::Flag(true));
    let spelling = if (kind == 5 && (!extra || implicit)) || (kind == 7 && implicit) { Value::text("") } else { Value::text(&text[start..end]) };
    let last_column = if kind == 5 && !extra { begin } else { finish };
    put(&iterator, "last_line", line.clone());
    put(&iterator, "last_lineno", Value::Small(lineno));
    Ok(Value::tuple(vec![token, spelling,
        Value::tuple(vec![Value::Small(lineno), Value::Small(begin as i64)]),
        Value::tuple(vec![Value::Small(lineno), Value::Small(last_column as i64)]), line]))
}
