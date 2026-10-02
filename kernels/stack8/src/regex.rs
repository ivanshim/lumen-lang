use crate::value::Value;

fn row(value: &Value) -> Option<Vec<Value>> {
    match value.contents() { Value::Array(items) | Value::Tuple(items) => Some(items.to_vec()), _ => None }
}
fn letter(value: &Value) -> Option<u8> {
    let Value::Text(text) = value.contents() else { return None };
    (text.len() == 1 && text.is_ascii()).then(|| text.as_bytes()[0])
}
fn accepts(node: &Value, character: u8, flags: &[Value]) -> Option<bool> {
    let parts = row(node)?;
    let Value::Text(kind) = parts.first()?.contents() else { return None };
    let insensitive = flags.first()?.is_true();
    Some(match kind.as_ref() {
        "lit" => { let wanted = letter(parts.get(1)?)?; if insensitive { wanted.eq_ignore_ascii_case(&character) } else { wanted == character } },
        "range" => {
            let first = letter(parts.get(1)?)?; let last = letter(parts.get(2)?)?;
            let contained = |c| first <= c && c <= last;
            contained(character) || insensitive && (contained(character.to_ascii_lowercase()) || contained(character.to_ascii_uppercase()))
        },
        "dot" => flags.get(2)?.is_true() || character != b'\n',
        "kind" => {
            let mark = letter(parts.get(1)?)?;
            let yes = match mark.to_ascii_lowercase() {
                b'd' => character.is_ascii_digit(),
                b'w' => character.is_ascii_alphanumeric() || character == b'_',
                b's' => [9, 10, 11, 12, 13, 32].contains(&character) || !flags.get(3)?.is_true() && (28..=31).contains(&character),
                _ => return None,
            };
            yes != mark.is_ascii_uppercase()
        },
        "class" => {
            let mut found = false;
            for member in row(parts.get(2)?)? { found |= accepts(&member, character, flags)?; }
            found != parts.get(1)?.is_true()
        },
        _ => return None,
    })
}
fn bound(captures: &Value, index: i64) -> Option<(usize, usize)> {
    let Value::Map(entries) = captures.contents() else { return None };
    let (_, span) = entries.iter().find(|(key, _)| matches!(key.contents(), Value::Small(n) if n == index))?;
    let parts = row(span)?;
    let [Value::Small(start), Value::Small(end)] = parts.as_slice() else { return None };
    Some((usize::try_from(*start).ok()?, usize::try_from(*end).ok()?))
}
pub(crate) fn shortcut(arguments: &[Value]) -> Result<Value, String> {
    let Some(Value::Text(operation)) = arguments.first().map(Value::contents) else { return Ok(Value::Null) };
    match operation.as_ref() {
        "ascii" if arguments.len() == 4 => {
            let Some(flags) = row(&arguments[2]) else { return Ok(Value::Null) };
            let inverse = arguments[3].is_true(); let mut characters = String::new();
            for code in 0..128u8 {
                let Some(matched) = accepts(&arguments[1], code, &flags) else { return Ok(Value::Null) };
                if matched != inverse && (!inverse || flags.get(2).is_some_and(Value::is_true) || code != b'\n') { characters.push(code as char); }
            }
            Ok(Value::text(&characters))
        },
        "end" if arguments.len() == 4 => {
            let (Value::Small(group), Value::Small(count)) = (arguments[2].contents(), arguments[3].contents()) else { return Ok(Value::Null) };
            if group < 0 || group > count { return Err("IndexError: no such group".into()); }
            Ok(Value::Small(bound(&arguments[1], group).map_or(-1, |span| span.1 as i64)))
        },
        "groups" if arguments.len() == 5 => {
            let (Value::Text(text), Value::Small(count)) = (arguments[1].contents(), arguments[3].contents()) else { return Ok(Value::Null) };
            let mut groups = Vec::new();
            for number in 1..=count {
                groups.push(match bound(&arguments[2], number) {
                    Some((start, stop)) => Value::text(&text.chars().skip(start).take(stop.saturating_sub(start)).collect::<String>()),
                    None => arguments[4].clone(),
                });
            }
            Ok(Value::tuple(groups))
        },
        _ => Ok(Value::Null),
    }
}
