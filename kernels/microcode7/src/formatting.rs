// Text is laid out only after its description has been read whole.
// The description belongs to the field, never to ordinary real output.

use num_traits::{ToPrimitive, Signed};
use crate::data::{Names, Value};
use crate::table::Table;

pub struct Layout<'a> {
    pub table: &'a Table,
    pub names: Names<'a>,
}

type Answer = Result<String, String>;

impl Layout<'_> {
    pub fn complain(&self, label: &str, inserts: &[&str]) -> String {
        self.table.strings(label).iter().enumerate().map(|(n, word)| {
            let mut fragment = word.clone();
            fragment.push_str(inserts.get(n).copied().unwrap_or(""));
            fragment
        }).collect()
    }

    fn refused(&self) -> String { self.complain("ext.text.format.unready", &[]) }
    fn invalid(&self) -> String { self.complain("ext.text.format.invalid", &[]) }

    pub fn typename(&self, item: &Value) -> &str {
        let position = match item {
            Value::Text(_) => 2, Value::Small(_) | Value::Huge(_) => 0,
            Value::Frac(_) => 1, Value::Flag(_) => 3,
            Value::Nil => 6, Value::Dict(_) => 5, Value::Vector(_) => 4, _ => 7,
        };
        self.table.strings("ext.text.format.kinds").get(position).map_or("", String::as_str)
    }

    pub fn quote(&self, item: &Value, escaped: bool) -> Answer {
        Ok(match item {
            Value::Shared(cell) => return self.quote(&cell.borrow(), escaped),
            Value::Text(_) => {
                let original = item.in_field(self.names, "", if escaped { "a" } else { "r" });
                original.chars().map(|letter| {
                    if letter.is_ascii() { return Ok(letter.to_string()); }
                    if letter.is_whitespace() {
                        let ordinal = u32::from(letter);
                        return Ok(match ordinal {
                            0..=0xff => format!("\\x{:02x}", ordinal),
                            0x100..=0xffff => format!("\\u{:04x}", ordinal),
                            _ => format!("\\U{:08x}", ordinal),
                        });
                    }
                    let with_base = ['a', letter].iter().collect::<String>();
                    if with_base.escape_debug().skip(1).next() == Some('\\') { return Err(self.refused()); }
                    Ok(letter.to_string())
                }).collect::<Result<String, String>>()?
            }
            Value::Dict(entries) => {
                let rendered = entries.iter().map(|(a, b)| {
                    Ok(format!("{}: {}", self.quote(a, escaped)?, self.quote(b, escaped)?))
                }).collect::<Result<Vec<_>, String>>()?;
                format!("{{{}}}", rendered.join(", "))
            }
            Value::Vector(entries) => {
                let mut rendered = Vec::with_capacity(entries.len());
                for entry in entries.iter() { rendered.push(self.quote(entry, escaped)?); }
                format!("[{}]", rendered.join(", "))
            }
            Value::Frac(r) => {
                if r.past_numbers() {
                    return Ok(match (r.answers_none(), r.above.is_negative()) {
                        (true, _) => "nan", (false, true) => "-inf", _ => "inf",
                    }.to_owned());
                }
                let said = item.render(self.names);
                if r.past_numbers() || said.contains(['.', 'e', 'E']) { said } else { said + ".0" }
            }
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Nil | Value::Ellipsis => item.render(self.names),
            _ => return Err(self.table.single("ext.op.rem.format.unsupported").unwrap_or_default().to_string()),
        })
    }

    fn plain(&self, item: &Value) -> Answer {
        if let Value::Text(s) = item { Ok(s.to_string()) } else { self.quote(item, false) }
    }

    fn binary(&self, value: &Value) -> Result<f64, String> {
        match value {
            Value::Frac(r) => {
                let n = crate::data::nearest_binary(&r.above, &r.beneath);
                Ok(if n == 0.0 && r.under { -0.0 } else { n })
            }
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) => value.as_big()?.to_f64().filter(|n| n.is_finite()).ok_or_else(|| self.refused()),
            _ => Err(self.complain("ext.op.rem.format.real", &[self.typename(value)])),
        }
    }

    fn read_count(&self, input: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<Option<usize>, String> {
        let mut digits = String::new();
        while input.peek().map_or(false, |c| c.is_ascii_digit()) { digits.push(input.next().unwrap()); }
        if digits.is_empty() { return Ok(None); }
        digits.parse::<usize>().ok().filter(|n| *n <= 100000).map(Some).ok_or_else(|| self.refused())
    }

    fn description(&self, pattern: &str) -> Result<Presentation, String> {
        let mut marks = pattern.chars().peekable();
        let mut shape = Presentation::new();
        let first = marks.peek().copied();
        let second = marks.clone().nth(1);
        if second.map_or(false, |c| matches!(c, '<' | '>' | '=' | '^')) {
            shape.padding = marks.next().unwrap();
            shape.justify = marks.next();
        } else if first.map_or(false, |c| matches!(c, '<' | '>' | '=' | '^')) { shape.justify = marks.next(); }
        if marks.peek().map_or(false, |c| matches!(c, '+' | '-' | ' ')) { shape.polarity = marks.next(); }
        if marks.peek() == Some(&'z') { marks.next(); shape.no_minus_zero = true; }
        if marks.peek() == Some(&'#') { marks.next(); shape.alternative = true; }
        if marks.peek() == Some(&'0') {
            marks.next(); shape.zero = true;
            if !second.map_or(false, |c| matches!(c, '<' | '>' | '=' | '^')) { shape.padding = '0'; }
        }
        shape.extent = self.read_count(&mut marks)?.unwrap_or_default();
        if marks.peek().map_or(false, |c| matches!(c, ',' | '_')) { shape.separator = marks.next(); }
        if marks.peek() == Some(&'.') {
            marks.next();
            shape.digits = self.read_count(&mut marks)?;
            if marks.peek().map_or(false, |c| matches!(c, ',' | '_')) { shape.fraction_separator = marks.next(); }
            else if shape.digits.is_none() { return Err(self.complain("ext.text.format.precision.missing", &[])); }
        }
        shape.letter = marks.next();
        if marks.next().is_some() { return Err(self.invalid()); }
        Ok(shape)
    }

    pub fn present(&self, item: &Value, pattern: &str, convert: &str) -> Answer {
        if let Value::Shared(held) = item { return self.present(&held.borrow(), pattern, convert); }
        if !convert.is_empty() {
            let rendered = match convert {
                "a" => self.quote(item, true)?, "r" => self.quote(item, false)?, "s" => self.plain(item)?,
                _ => return Err(self.complain("ext.text.format.conversion", &[convert])),
            };
            return self.present(&Value::text(&rendered), pattern, "");
        }
        if pattern.is_empty() {
            if matches!(item, Value::Frac(_)) { return Ok(item.render(self.names)); }
            return self.plain(item);
        }
        let mut shape = self.description(pattern)?;
        let unknown = || self.complain("ext.text.format.unknown", &[&shape.letter.unwrap_or('\0').to_string(), self.typename(item)]);
        if shape.letter == Some('n') { return Err(self.refused()); }
        if let Value::Text(text) = item {
            if shape.letter.is_some() && shape.letter != Some('s') { return Err(unknown()); }
            let bad = if shape.polarity.is_some() { Some("sign") }
                else if shape.no_minus_zero { Some("zero") }
                else if shape.alternative { Some("alternate") }
                else if shape.justify == Some('=') { Some("align") } else { None };
            if let Some(why) = bad { return Err(self.complain(&format!("ext.text.format.{why}.string"), &[])); }
            if shape.separator.is_some() || shape.fraction_separator.is_some() { return Err(self.invalid()); }
            let kept: String = text.chars().take(shape.digits.unwrap_or(usize::MAX)).collect();
            return Ok(shape.padded(String::new(), kept, '<'));
        }
        let integer = matches!(item, Value::Small(_) | Value::Huge(_) | Value::Flag(_));
        let real = matches!(item, Value::Frac(n) if n.places.is_some());
        if !integer && !real { return Err(self.refused()); }
        if integer && shape.letter.map_or(true, |c| "dboxXc".contains(c)) {
            if shape.no_minus_zero { return Err(self.complain("ext.text.format.zero.integer", &[])); }
            if shape.digits.is_some() { return Err(self.complain("ext.text.format.precision.integer", &[])); }
            if shape.letter == Some('c') {
                if shape.polarity.is_some() { return Err(self.complain("ext.text.format.sign.character", &[])); }
                if shape.alternative { return Err(self.complain("ext.text.format.alternate.character", &[])); }
                if shape.separator.is_some() { return Err(self.invalid()); }
                let text = self.character(item)?;
                return Ok(shape.padded(String::new(), text, '>'));
            }
            let radix = match shape.letter { Some('b') => 2, Some('o') => 8, Some('x' | 'X') => 16, _ => 10 };
            if shape.fraction_separator.is_some() || radix != 10 && shape.separator == Some(',') { return Err(self.invalid()); }
            let n = item.as_big()?;
            let mut digits = n.abs().to_str_radix(radix);
            if shape.letter == Some('X') { digits = digits.to_ascii_uppercase(); }
            let mut front = shape.front(n.is_negative());
            if shape.alternative {
                front.push_str(match shape.letter { Some('b') => "0b", Some('o') => "0o", Some('x') => "0x", Some('X') => "0X", _ => "" });
            }
            if shape.zero && shape.justify.is_none() { shape.justify = Some('='); }
            digits = shape.grouped(digits, front.len(), if radix == 10 { 3 } else { 4 });
            return Ok(shape.padded(front, digits, '>'));
        }
        if shape.letter.map_or(false, |c| !"eEfFgG%".contains(c)) { return Err(unknown()); }
        let mut number = self.binary(item)?;
        if integer && !number.is_finite() { return Err(self.refused()); }
        if shape.letter == Some('%') { number *= 100.0; }
        if shape.zero && shape.justify.is_none() { shape.justify = Some('='); }
        let body = shape.real_digits(number.abs());
        let nought = shape.no_minus_zero && body.trim_end_matches('%').parse::<f64>().ok() == Some(0.0);
        let prefix = shape.front(number.is_sign_negative() && !number.is_nan() && !nought);
        let body = if number.is_finite() { shape.grouped(body, prefix.len(), 3) } else { body };
        Ok(shape.padded(prefix, body, '>'))
    }

    fn character(&self, item: &Value) -> Answer {
        match item.as_big()?.to_u32() {
            Some(n @ 0..=0x10ffff) => char::from_u32(n).map(String::from).ok_or_else(|| self.refused()),
            _ => Err(self.complain("ext.text.format.character", &[])),
        }
    }

    pub fn interpolate(&self, pattern: &str, positions: &[Value], names: &[(String, Value)]) -> Answer {
        self.weave(pattern, positions, names, &mut 0, 2)
    }

    fn weave(&self, mut rest: &str, positions: &[Value], names: &[(String, Value)], numbering: &mut i64, allowance: i32) -> Answer {
        if allowance < 0 { return Err(self.complain("ext.text.format.recursion", &[])); }
        let mut finished = String::new();
        while let Some(start) = rest.find(['{', '}']) {
            finished.push_str(&rest[..start]);
            rest = &rest[start..];
            if rest.starts_with("{{") || rest.starts_with("}}") {
                finished.push(rest.chars().next().unwrap()); rest = &rest[2..]; continue;
            }
            if rest.starts_with('}') { return Err(self.complain("ext.text.format.brace.close", &[])); }
            if allowance == 0 { return Err(self.complain("ext.text.format.recursion", &[])); }
            rest = &rest[1..];
            let mut inside_key = false;
            let split = rest.char_indices().find_map(|(i, c)| {
                if c == '[' { inside_key = true; }
                if c == ']' { inside_key = false; }
                (!inside_key && matches!(c, ':' | '!' | '}')).then_some(i)
            }).ok_or_else(|| self.complain("ext.text.format.brace.open", &[]))?;
            let selector = &rest[..split];
            let value = self.select(selector, positions, names, numbering)?;
            rest = &rest[split..];
            let conversion = if rest.starts_with('!') {
                let c = rest[1..].chars().next().ok_or_else(|| self.invalid())?;
                rest = &rest[1 + c.len_utf8()..]; c.to_string()
            } else { String::new() };
            let specification = if rest.starts_with(':') {
                rest = &rest[1..];
                let mut balance = 0;
                let end = rest.char_indices().find_map(|(i, ch)| {
                    match ch {
                        '{' => balance += 1,
                        '}' if balance == 0 => return Some(i),
                        '}' => balance -= 1,
                        _ => {}
                    }
                    None
                }).ok_or_else(|| self.complain("ext.text.format.brace.open", &[]))?;
                let spec = self.weave(&rest[..end], positions, names, numbering, allowance - 1)?;
                rest = &rest[end..]; spec
            } else { String::new() };
            if !rest.starts_with('}') { return Err(self.complain("ext.text.format.brace.open", &[])); }
            finished.push_str(&self.present(&value, &specification, &conversion)?);
            rest = &rest[1..];
        }
        finished.push_str(rest);
        Ok(finished)
    }

    fn select(&self, field: &str, positions: &[Value], names: &[(String, Value)], numbering: &mut i64) -> Result<Value, String> {
        let first_end = field.find(['[', '.']).unwrap_or(field.len());
        let first = &field[..first_end];
        let index = if first.is_empty() {
            if *numbering < 0 { return Err(self.complain("ext.text.format.numbered.auto", &[])); }
            let slot = *numbering as usize;
            *numbering += 1;
            Some(slot)
        } else if first.bytes().all(|b| b.is_ascii_digit()) {
            if *numbering > 0 { return Err(self.complain("ext.text.format.numbered.manual", &[])); }
            *numbering = -1;
            Some(first.parse::<usize>().map_err(|_| self.complain("ext.text.format.index", &[first]))?)
        } else { None };
        let mut selected = match index {
            Some(n) => positions.get(n).cloned().ok_or_else(|| self.complain("ext.text.format.index", &[&n.to_string()]))?,
            None => names.iter().find(|(k, _)| k == first).map(|(_, v)| v.clone())
                .ok_or_else(|| self.complain("ext.text.format.key", &[first]))?,
        };
        let mut following = &field[first_end..];
        while !following.is_empty() {
            let bracket = following.starts_with('[');
            let tail = following.get(1..).ok_or_else(|| self.invalid())?;
            let end = if bracket { tail.find(']').ok_or_else(|| self.invalid())? }
                else if following.starts_with('.') { tail.find(['[', '.']).unwrap_or(tail.len()) }
                else { return Err(self.invalid()); };
            let asked = &tail[..end];
            let found = if bracket {
                let numeric = asked.parse::<usize>().ok();
                match &selected {
                    Value::Vector(list) => numeric.and_then(|n| list.get(n)).cloned(),
                    Value::Text(chars) => numeric.and_then(|n| chars.chars().nth(n)).map(|c| Value::text(&c.to_string())),
                    Value::Dict(pairs) => {
                        let key = numeric.map_or_else(|| Value::text(asked), |n| Value::from_big(n.into()));
                        pairs.iter().find(|(k, _)| k.equals(&key)).map(|(_, v)| v.clone())
                    }
                    _ => None,
                }
            } else {
                match &selected {
                    Value::Thing(thing) => thing.holds.borrow().iter().find(|(key, _)| key == asked).map(|(_, value)| value.clone()),
                    _ => None,
                }
            };
            selected = found.ok_or_else(|| if bracket { self.complain("ext.text.format.key", &[asked]) } else { self.refused() })?;
            following = &tail[end + usize::from(bracket)..];
        }
        Ok(selected)
    }

    pub fn remainder(&self, pattern: &str, supplied: &Value) -> Answer {
        let positional = match supplied { Value::Vector(items) => items.as_slice(), _ => std::slice::from_ref(supplied) };
        let mut used = 0usize;
        let mut named_seen = false;
        let mut input = pattern.chars().peekable();
        let mut output = String::new();
        while let Some(mark) = input.next() {
            if mark != '%' { output.push(mark); continue; }
            if input.peek() == Some(&'%') { input.next(); output.push('%'); continue; }
            let named = if input.peek() == Some(&'(') {
                input.next();
                let mut key = String::new();
                let mut balance = 1usize;
                loop {
                    let c = input.next().ok_or_else(|| self.complain("ext.op.rem.format.incomplete", &[]))?;
                    if c == '(' { balance += 1; }
                    if c == ')' { balance -= 1; }
                    if balance == 0 { break; }
                    key.push(c);
                }
                let Value::Dict(pairs) = supplied else { return Err(self.complain("ext.op.rem.format.mapping", &[])); };
                named_seen = true;
                used = positional.len();
                Some(pairs.iter().find(|(k, _)| matches!(k, Value::Text(s) if s.as_ref() == key)).map(|(_, v)| v)
                    .ok_or_else(|| self.complain("ext.text.format.key", &[&key]))?)
            } else { None };
            let mut shape = Presentation::new();
            loop {
                match input.peek() {
                    Some('-') => shape.justify = Some('<'),
                    Some('+') => shape.polarity = Some('+'),
                    Some(' ') => if shape.polarity != Some('+') { shape.polarity = Some(' '); },
                    Some('#') => shape.alternative = true,
                    Some('0') => shape.zero = true,
                    _ => break,
                }
                input.next();
            }
            if input.peek() == Some(&'*') {
                if named.is_some() { return Err(self.refused()); }
                input.next();
                let width = self.dynamic(positional, &mut used)?;
                shape.extent = width.unsigned_abs() as usize;
                if width < 0 { shape.justify = Some('<'); }
            } else { shape.extent = self.read_count(&mut input)?.unwrap_or(0); }
            if input.peek() == Some(&'.') {
                input.next();
                shape.digits = Some(if input.peek() == Some(&'*') {
                    if named.is_some() { return Err(self.refused()); }
                    input.next(); self.dynamic(positional, &mut used)?.max(0) as usize
                } else { self.read_count(&mut input)?.unwrap_or(0) });
            }
            if input.peek().map_or(false, |c| matches!(c, 'h' | 'l' | 'L')) { input.next(); }
            let conversion = input.next().ok_or_else(|| self.complain("ext.op.rem.format.incomplete", &[]))?;
            shape.letter = Some(conversion);
            let item = match named {
                Some(item) => item,
                None => {
                    used += 1;
                    positional.get(used - 1).ok_or_else(|| self.complain("ext.op.rem.format.few", &[]))?
                }
            };
            if shape.zero && shape.justify != Some('<') {
                shape.padding = '0'; shape.justify = Some('=');
            }
            let (head, body) = match conversion {
                'a' | 'r' | 's' => {
                    let text = if conversion == 's' { self.plain(item)? } else { self.quote(item, conversion == 'a')? };
                    shape.padding = ' ';
                    if shape.justify == Some('=') { shape.justify = Some('>'); }
                    (String::new(), text.chars().take(shape.digits.unwrap_or(usize::MAX)).collect())
                }
                'c' => {
                    shape.padding = ' ';
                    if shape.justify == Some('=') { shape.justify = Some('>'); }
                    let c = match item {
                        Value::Text(text) if text.chars().count() == 1 => text.to_string(),
                        Value::Huge(_) | Value::Small(_) | Value::Flag(_) => self.character(item)?,
                        _ => return Err(self.complain("ext.op.rem.format.character", &[])),
                    };
                    (String::new(), c)
                }
                'd' | 'i' | 'u' | 'o' | 'x' | 'X' => {
                    let accepts_real = matches!(conversion, 'd' | 'i' | 'u');
                    if let Value::Frac(r) = item {
                        if accepts_real && r.past_numbers() {
                            let fault = match r.answers_none() { true => "ext.op.rem.format.nan", false => "ext.op.rem.format.infinity" };
                            return Err(self.complain(fault, &[]));
                        }
                    }
                    if !matches!(item, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) &&
                        !(accepts_real && matches!(item, Value::Frac(r) if r.places.is_some() && !r.past_numbers())) {
                        return Err(self.complain(if accepts_real { "ext.op.rem.format.number" } else { "ext.op.rem.format.integer" }, &[&conversion.to_string(), self.typename(item)]));
                    }
                    let number = item.as_big()?;
                    let radix = match conversion { 'x' | 'X' => 16, 'o' => 8, _ => 10 };
                    let mut digits = number.abs().to_str_radix(radix);
                    if conversion == 'X' { digits = digits.to_uppercase(); }
                    digits = "0".repeat(shape.digits.unwrap_or(0).saturating_sub(digits.len())) + &digits;
                    let mut lead = shape.front(number.is_negative());
                    if shape.alternative { lead += match conversion { 'x' => "0x", 'X' => "0X", 'o' => "0o", _ => "" }; }
                    (lead, digits)
                }
                'e' | 'E' | 'f' | 'F' | 'g' | 'G' => {
                    let number = self.binary(item)?;
                    (shape.front(number.is_sign_negative() && !number.is_nan()), shape.real_digits(number.abs()))
                }
                _ => {
                    let index = pattern.chars().count() - input.count() - 1;
                    return Err(self.complain("ext.op.rem.format.code", &[&conversion.to_string(), &format!("{:x}", u32::from(conversion)), &index.to_string()]));
                }
            };
            output.push_str(&shape.padded(head, body, '>'));
        }
        if !named_seen && used < positional.len() && !matches!(supplied, Value::Dict(_)) {
            return Err(self.complain("ext.op.rem.format.many", &[]));
        }
        Ok(output)
    }

    fn dynamic(&self, supplied: &[Value], used: &mut usize) -> Result<i64, String> {
        let value = supplied.get(*used).ok_or_else(|| self.complain("ext.op.rem.format.few", &[]))?;
        *used += 1;
        match value {
            Value::Flag(_) | Value::Small(_) | Value::Huge(_) => value.as_big()?.to_i64()
                .filter(|n| (-100000..=100000).contains(n)).ok_or_else(|| self.refused()),
            _ => Err(self.complain("ext.op.rem.format.star", &[])),
        }
    }
}

struct Presentation {
    padding: char,
    justify: Option<char>,
    polarity: Option<char>,
    alternative: bool,
    zero: bool,
    no_minus_zero: bool,
    extent: usize,
    digits: Option<usize>,
    separator: Option<char>,
    fraction_separator: Option<char>,
    letter: Option<char>,
}

impl Presentation {
    fn new() -> Self {
        Self { padding: ' ', justify: None, polarity: None, alternative: false, zero: false, no_minus_zero: false,
            extent: 0, digits: None, separator: None, fraction_separator: None, letter: None }
    }

    fn front(&self, below: bool) -> String {
        match (below, self.polarity) {
            (true, _) => String::from("-"),
            (false, Some(c @ ('+' | ' '))) => c.to_string(),
            _ => String::new(),
        }
    }

    fn padded(&self, mut prefix: String, content: String, usual: char) -> String {
        let count = self.extent.saturating_sub(prefix.chars().count() + content.chars().count());
        let direction = self.justify.unwrap_or(usual);
        let make = |n| self.padding.to_string().repeat(n);
        if direction == '=' { prefix.push_str(&make(count)); }
        prefix.push_str(&content);
        match direction {
            '<' => prefix + &make(count),
            '^' => make(count / 2) + &prefix + &make(count - count / 2),
            '=' => prefix,
            _ => make(count) + &prefix,
        }
    }

    fn grouped(&self, text: String, prefix: usize, chunk: usize) -> String {
        let stop = if self.letter.map_or(false, |c| matches!(c, 'b' | 'o' | 'x' | 'X' | 'd')) { text.len() }
            else { text.find(['e', 'E', '%']).unwrap_or(text.len()) };
        let (integer, fraction) = text[..stop].split_once('.').map_or((&text[..stop], None), |(a, b)| (a, Some(b)));
        let mut ending = String::new();
        if let Some(fraction) = fraction {
            ending.push('.');
            for (i, c) in fraction.chars().enumerate() {
                if i != 0 && i % 3 == 0 {
                    if let Some(mark) = self.fraction_separator { ending.push(mark); }
                }
                ending.push(c);
            }
        }
        ending += &text[stop..];
        let mut whole = integer.to_string();
        if let Some(separator) = self.separator {
            if self.justify == Some('=') && self.padding == '0' {
                let occupied = prefix + ending.len();
                let mut wanted = whole.len();
                while occupied + wanted + wanted.saturating_sub(1) / chunk < self.extent { wanted += 1; }
                whole = "0".repeat(wanted - whole.len()) + &whole;
            }
            let mut backwards = String::new();
            for (i, c) in whole.chars().rev().enumerate() {
                if i % chunk == 0 && i > 0 { backwards.push(separator); }
                backwards.push(c);
            }
            whole = backwards.chars().rev().collect();
        }
        whole + &ending
    }

    fn real_digits(&self, x: f64) -> String {
        let mode = self.letter.unwrap_or('\0').to_ascii_lowercase();
        let precision = self.digits.unwrap_or(6);
        let mut raw = if x.is_nan() { "nan".to_owned() }
        else if x.is_infinite() { "inf".to_owned() }
        else if mode == 'f' || mode == '%' { format!("{:.*}", precision, x) }
        else if mode == 'e' { format!("{:.*e}", precision, x) }
        else if self.letter.is_none() && self.digits.is_none() {
            if x > 0.0 && (x < 0.0001 || x >= 1e16) { format!("{:e}", x) } else { format!("{}", x) }
        } else {
            let significant = precision.max(1);
            let sci = format!("{:.*e}", significant - 1, x);
            let (coefficient, exponent) = sci.split_once('e').unwrap();
            let power = exponent.parse::<i32>().unwrap();
            let cutoff = significant as i32 - if self.letter.is_none() { 1 } else { 0 };
            if power >= cutoff || power < -4 {
                let keep = if self.alternative || !coefficient.contains('.') { coefficient }
                    else { coefficient.trim_end_matches('0').trim_end_matches('.') };
                keep.to_owned() + "e" + exponent
            } else {
                let places = (significant as i32 - power - 1).max(0) as usize;
                let fixed = format!("{:.*}", places, x);
                if self.alternative || !fixed.contains('.') { fixed }
                else { fixed.trim_end_matches('0').trim_end_matches('.').to_owned() }
            }
        };
        if x.is_finite() {
            let split = raw.find('e').unwrap_or(raw.len());
            if (self.alternative || self.letter.is_none() && split == raw.len()) && !raw[..split].contains('.') {
                let suffix = if self.letter.is_none() && split == raw.len() { ".0" } else { "." };
                raw.insert_str(split, suffix);
            }
            if let Some((mantissa, exponent)) = raw.split_once('e') {
                let power = exponent.parse::<i32>().unwrap();
                raw = format!("{}e{:+03}", mantissa, power);
            }
        }
        if self.letter.map_or(false, |c| c.is_ascii_uppercase()) { raw = raw.to_uppercase(); }
        if mode == '%' { raw.push('%'); }
        raw
    }
}

pub fn is_complaint(table: &Table, message: &str) -> bool {
    let labels = "ext.text.format.zero.integer ext.text.format.zero.string ext.op.rem.format.nan ext.op.rem.format.infinity ext.text.format.invalid ext.text.format.unknown ext.text.format.unready ext.text.format.precision.integer ext.text.format.precision.missing ext.text.format.sign.string ext.text.format.alternate.string ext.text.format.align.string ext.text.format.sign.character ext.text.format.alternate.character ext.text.format.character ext.text.format.spec.type ext.text.format.numbered.auto ext.text.format.numbered.manual ext.text.format.index ext.text.format.key ext.text.format.brace.open ext.text.format.brace.close ext.text.format.conversion ext.text.format.recursion ext.op.rem.format.few ext.op.rem.format.many ext.op.rem.format.mapping ext.op.rem.format.number ext.op.rem.format.integer ext.op.rem.format.real ext.op.rem.format.character ext.op.rem.format.star ext.op.rem.format.incomplete ext.op.rem.format.code";
    labels.split_whitespace().filter_map(|label| table.single(label))
        .any(|opening| !opening.is_empty() && message.starts_with(opening))
}
