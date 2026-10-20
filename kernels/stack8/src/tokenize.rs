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

// A complete ordinary literal needs only delimiter and escape tracking.
// Refills and malformed or multiline strings retain the port's diagnostics.
fn quoted_end(bytes: &[u8], opening: usize) -> Option<usize> {
    let quote = *bytes.get(opening)?;
    if bytes.get(opening + 1..opening + 3) == Some(&[quote, quote][..]) { return None; }
    let mut cursor = opening + 1;
    loop {
        match *bytes.get(cursor)? {
            c if c == quote => return Some(cursor + 1),
            b'\n' | b'\r' => return None,
            b'\\' => {
                if matches!(bytes.get(cursor + 1), None | Some(b'\n' | b'\r')) { return None; }
                cursor += 2;
            }
            _ => cursor += 1,
        }
    }
}

// Accept complete uncomplicated numbers only; ambiguous suffixes, separators
// and invalid forms keep the adapter's full validation and error reporting.
fn numeric_end(bytes: &[u8], start: usize) -> Option<usize> {
    let first = *bytes.get(start)?;
    let mut at = start;
    let base = if first == b'0' { match bytes.get(start + 1) {
        Some(b'x' | b'X') => 16, Some(b'o' | b'O') => 8,
        Some(b'b' | b'B') => 2, _ => 10,
    } } else { 10 };
    if base != 10 {
        at += 2;
        let digits = at;
        while bytes.get(at).is_some_and(|byte| match base {
            16 => byte.is_ascii_hexdigit(), 8 => matches!(byte, b'0'..=b'7'),
            _ => matches!(byte, b'0' | b'1'),
        }) { at += 1; }
        if at == digits { return None; }
    } else {
        while bytes.get(at).is_some_and(u8::is_ascii_digit) { at += 1; }
        let integer_end = at;
        let mut floating = false;
        if bytes.get(at) == Some(&b'.') {
            floating = true;
            at += 1;
            let decimal = at;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) { at += 1; }
            if first == b'.' && at == decimal { return None; }
        }
        if matches!(bytes.get(at), Some(b'e' | b'E')) {
            floating = true;
            at += 1;
            if matches!(bytes.get(at), Some(b'+' | b'-')) { at += 1; }
            let exponent = at;
            while bytes.get(at).is_some_and(u8::is_ascii_digit) { at += 1; }
            if at == exponent { return None; }
        }
        let imaginary = matches!(bytes.get(at), Some(b'j' | b'J'));
        if imaginary { at += 1; }
        if !floating && !imaginary && first == b'0' && integer_end > start + 1 { return None; }
    }
    bytes.get(at).filter(|byte| byte.is_ascii() && !byte.is_ascii_alphanumeric() && !matches!(**byte, b'_' | b'.')).map(|_| at)
}

// Finish simple single-line interpolation pieces without another Python
// scanner/conversion pass. Escapes, doubled braces and multiline forms defer.
fn interpolation(iterator: &Instance, state: &Instance, mode: &Instance,
    depth: i64, cur: i64, inp: i64, base: i64, lineno: i64) -> Option<Value> {
    if depth <= 0 || cur >= inp || number(mode, "quote_size") != Some(1)
        || number(mode, "first_line") != Some(lineno) { return None; }
    let flavour = match number(mode, "string_kind")? { 0 => 0, 1 => 1, _ => return None };
    let Value::Text(quote) = field(mode, "quote") else { return None; };
    if quote.len() != 1 || !matches!(quote.as_bytes()[0], b'\'' | b'"') { return None; }
    let mark = quote.as_bytes()[0];
    let Value::Text(text) = field(state, "buf") else { return None; };
    let position = usize::try_from(cur - base).ok()?;
    let begin = crate::strings::character_byte(&text, position)?;
    let bytes = text.as_bytes();
    let ending = *bytes.get(begin)? == mark;
    let mut after = begin;
    let mut expression = None;
    if ending { after += 1; }
    else {
        while let Some(&byte) = bytes.get(after) {
            if byte == mark { break; }
            if byte == b'{' {
                if after == begin || bytes.get(after + 1) == Some(&b'{') { return None; }
                let next_depth = number(mode, "curly_bracket_expr_start_depth")?.checked_add(1)?;
                if next_depth >= number(state, "_maxexpr")? { return None; }
                expression = Some(next_depth);
                break;
            }
            if matches!(byte, b'}' | b'\\' | b'\n' | b'\r') { return None; }
            after += 1;
        }
        if after == begin || bytes.get(after).is_none() { return None; }
    }
    let finish = position + text[begin..after].chars().count();
    let Value::Tuple(types) = field(state, "_token_types") else { return None; };
    if types.len() != 11 { return None; }
    let token = types[if ending { 9 + flavour } else { 7 + flavour }].clone();
    let line = if number(iterator, "last_lineno") == Some(lineno) {
        let cached = field(iterator, "last_line");
        if matches!(cached, Value::Null) { return None; }
        cached
    } else if matches!(field(state, "implicit_newline"), Value::Flag(true)) {
        Value::text(text.strip_suffix('\n')?)
    } else { Value::Text(text.clone()) };
    put(state, "start", Value::Small(cur));
    put(state, "cur", Value::Small(base + finish as i64));
    put(state, "first_lineno", Value::Small(lineno));
    if ending { put(state, "tok_mode_stack_index", Value::Small(depth - 1)); }
    else { put(state, "multi_line_start", Value::Small(base)); }
    if let Some(next_depth) = expression {
        put(mode, "last_expr_start", Value::Small(base + finish as i64 + 1));
        put(mode, "curly_bracket_expr_start_depth", Value::Small(next_depth));
        put(mode, "kind", Value::Small(0));
        put(mode, "in_format_spec", Value::Flag(false));
    }
    put(iterator, "last_line", line.clone());
    put(iterator, "last_lineno", Value::Small(lineno));
    Some(Value::tuple(vec![token, Value::text(&text[begin..after]),
        Value::tuple(vec![Value::Small(lineno), Value::Small(position as i64)]),
        Value::tuple(vec![Value::Small(lineno), Value::Small(finish as i64)]), line]))
}

// Commit one decoded callback result to the already-reset physical buffer.
// Keep non-native strings and callback-altered records on the Python path.
fn prepare_line(iterator: &Instance, supplied: &Value) -> Value {
    let Value::Object(state) = field(iterator, "tok") else { return Value::Flag(false); };
    let Value::Text(text) = supplied.contents() else { return Value::Flag(false); };
    if number(&state, "cur") != Some(0) || number(&state, "inp") != Some(0)
        || number(&state, "tok_mode_stack_index") != Some(0) || number(&state, "done") != Some(10) { return Value::Flag(false); }
    let Some(buffer) = list(&state, "bufc") else { return Value::Flag(false); };
    if !matches!(&*buffer.borrow(), Value::Array(row) if row.is_empty()) { return Value::Flag(false); }
    let Some(lineno) = number(&state, "lineno") else { return Value::Flag(false); };
    put(&state, "line_start", Value::Small(0));
    if text.is_empty() {
        put(&state, "done", Value::Small(11));
        return Value::Small(0);
    }
    let mut characters = text.chars().map(|character| {
        let mut encoded = [0u8; 4];
        Value::text(character.encode_utf8(&mut encoded))
    }).collect::<Vec<_>>();
    let implicit = !text.ends_with('\n');
    if implicit { characters.push(Value::text("\n")); }
    let length = characters.len();
    *buffer.borrow_mut() = Value::array(characters);
    let null = text.contains('\0');
    put(&state, "line_null", Value::Flag(null));
    put(&state, "inp", Value::Small(length as i64));
    put(&state, "implicit_newline", Value::Flag(implicit));
    let physical = if implicit { Value::text(&format!("{text}\n")) } else { Value::Text(text) };
    put(&state, "buf", physical);
    put(&state, "lineno", Value::Small(lineno + 1));
    Value::Small(if null { -1 } else { 1 })
}

// Recognize buffered tokens; leave refills and diagnostic cases to the adapter.
pub fn scan(args: &[Value]) -> Result<Value, String> {
    if !(1..=2).contains(&args.len()) { return Err("TypeError: tokenizer scan needs an iterator and optional decoded line".into()); }
    let Value::Object(iterator) = args[0].contents() else { return Ok(Value::Null); };
    if args.len() == 2 { return Ok(prepare_line(&iterator, &args[1])); }
    let Value::Object(state) = field(&iterator, "tok") else { return Ok(Value::Null); };
    if !matches!(field(&iterator, "done"), Value::Flag(false))
        || number(&state, "pendin") != Some(0) || number(&state, "done") != Some(10) { return Ok(Value::Null); }
    let (Some(cur), Some(inp), Some(base), Some(lineno), Some(depth)) =
        (number(&state, "cur"), number(&state, "inp"), number(&state, "line_start"), number(&state, "lineno"), number(&state, "tok_mode_stack_index"))
        else { return Ok(Value::Null); };
    if base < 0 || cur < base || depth < 0 { return Ok(Value::Null); }
    let Value::Array(modes) = field(&state, "tok_mode_stack") else { return Ok(Value::Null); };
    let Some(Value::Object(mode)) = modes.get(depth as usize).map(Value::contents) else { return Ok(Value::Null); };
    if number(&mode, "kind") == Some(1) {
        return Ok(interpolation(&iterator, &state, &mode, depth, cur, inp, base, lineno).unwrap_or(Value::Null));
    }
    if number(&mode, "kind") != Some(0) { return Ok(Value::Null); }
    let atbol = match field(&state, "atbol") { Value::Flag(value) => value, _ => return Ok(Value::Null) };
    // Reset before the callback, exactly where ordinary underflow does so.
    if atbol && cur == inp {
        if depth != 0 { return Ok(Value::Null); }
        put(&state, "start", Value::Null);
        put(&state, "bufc", Value::array(Vec::new()).held(true));
        put(&state, "line_null", Value::Flag(false));
        put(&state, "cur", Value::Small(0));
        put(&state, "inp", Value::Small(0));
        return Ok(Value::Flag(false));
    }
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
        if matches!(bytes.get(end), Some(b'\'' | b'"')) {
            let prefix = text[start..end].to_ascii_lowercase();
            if !matches!(prefix.as_str(), "b" | "r" | "u" | "br" | "rb") { return Ok(Value::Null); }
            let Some(stop) = quoted_end(bytes, end) else { return Ok(Value::Null); };
            end = stop;
            kind = 3;
        } else {
            if !bytes.get(end).is_some_and(u8::is_ascii) { return Ok(Value::Null); }
            kind = 0;
        }
    } else if first.is_ascii_digit() {
        let Some(stop) = numeric_end(bytes, start) else { return Ok(Value::Null); };
        end = stop;
        kind = 1;
    } else if matches!(first, b'\'' | b'"') {
        let Some(stop) = quoted_end(bytes, start) else { return Ok(Value::Null); };
        end = stop;
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
        if bytes.get(end).is_some_and(u8::is_ascii_digit) {
            let Some(stop) = numeric_end(bytes, start) else { return Ok(Value::Null); };
            end = stop;
            kind = 1;
        } else {
            if bytes.get(start..start + 3) == Some(b"...") { end = start + 3; }
            kind = 2;
        }
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
    if types.len() != 11 { return Ok(Value::Null); }
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
