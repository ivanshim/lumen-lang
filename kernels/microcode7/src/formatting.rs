// Text is laid out only after its description has been read whole.
// The description belongs to the field, never to ordinary real output.

use num_traits::{ToPrimitive, Signed};
use crate::data::{Among, Names, Value};
use crate::table::Table;

pub struct Layout<'a> {
    pub table: &'a Table,
    pub names: Names<'a>,
}

/// A field whose value the layout has no marks of its own for is put to
/// whoever asked for the layout. The answer is the field as it stands,
/// and nothing at all where the layout is to lay the value out itself.
pub trait Elsewhere {
    fn field_member(&mut self, _item: &Value, _key: &Value, _bracket: bool) -> Result<Option<Value>, String> { Ok(None) }
    fn field_laid(&mut self, item: &Value, pattern: &str, convert: &str) -> Result<Option<String>, String>;
    fn value_worded(&mut self, item: &Value, quoted: bool) -> Result<Option<String>, String>;
    fn value_numbered(&mut self, _item: &Value, _code: char) -> Result<NumberAnswer, String> { Ok(NumberAnswer::Missing(String::new())) }
}

type Answer = Result<String, String>;

/// What a numeric `%`-mark asks an object to stand for. `Whole` carries
/// the number it answered with; `Missing` and `BadMethod` carry the
/// name a complaint should give the value, and mark a method that was
/// absent or answered with the wrong kind.
pub enum NumberAnswer {
    Whole(Value),
    Missing(String),
    BadMethod(String),
}

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

    /// A name under which nothing was supplied. Where the table knows
    /// the exceptions of the language, the name travels whole and the
    /// fault raised keeps it as its own value, quoted once over; without
    /// them the words of the label stand in its place.
    fn name_absent(&self, key: &str) -> String {
        match self.table.has_any("ext.builtin.exceptions") {
            true => format!("\0absent-text={key}"),
            false => self.complain("ext.text.format.key", &[key]),
        }
    }

    pub fn typename(&self, item: &Value) -> &str {
        let position = match item {
            Value::Text(_) => 2, Value::Small(_) | Value::Huge(_) => 0,
            Value::Frac(_) => 1, Value::Flag(_) => 3,
            Value::Nil => 6, Value::Dict(_) => 5, Value::Vector(_) => 4,
            Value::Complex(_) | Value::Imaginary { .. } => 8,
            Value::Tuple(_) | Value::Row(_) => 9, Value::Set(_) => 10, Value::Progression(_) => 11,
            Value::Octets { changeable: false, .. } => 12,
            Value::Octets { changeable: true, .. } => 13,
            Value::Blueprint(_) | Value::OctetKind { .. } => 14,
            _ => 7,
        };
        self.table.strings("ext.text.format.kinds").get(position).map_or("", String::as_str)
    }

    pub fn quote(&self, item: &Value, escaped: bool) -> Answer {
        Ok(match item {
            Value::Shared(cell) | Value::Mutable(cell, _) => return self.quote(&cell.borrow(), escaped),
            // A text is quoted by the hand that quotes it everywhere
            // else, so that a field and the representation builtin
            // agree on a letter that cannot be shown as itself.
            Value::Text(word) if !escaped => crate::text::quotation(word),
            Value::Text(_) => item.in_field(self.names, "", "a").ok_or_else(|| self.refused())?,
            Value::Octets { .. } => item.render(self.names),
            // A field lays a collection out by walking its members
            // here, so it leaves the same note on them that the other
            // writers leave, and a collection reached from inside
            // itself is laid out as the marks it would have stood
            // between rather than walked round without end.
            Value::Dict(entries) => {
                let among = Among::members(item);
                if let Some(marks) = among.instead { return Ok(marks.to_string()); }
                let rendered = entries.iter().map(|(a, b)| {
                    Ok(format!("{}: {}", self.quote(a, escaped)?, self.quote(b, escaped)?))
                }).collect::<Result<Vec<_>, String>>()?;
                format!("{{{}}}", rendered.join(", "))
            }
            Value::Tuple(row) | Value::Row(row) | Value::Arguments(row) => {
                let among = Among::members(item);
                if let Some(marks) = among.instead { return Ok(marks.to_string()); }
                let pieces = row.iter().map(|x| self.quote(x, escaped)).collect::<Result<Vec<_>, _>>()?;
                format!("({}{})", pieces.join(", "), if row.len() == 1 { "," } else { "" })
            }
            Value::Vector(entries) => {
                let among = Among::members(item);
                if let Some(marks) = among.instead { return Ok(marks.to_string()); }
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
            // A value the layout has no marks of its own for is laid
            // out as the quoting builtin writes it, so that a field
            // says of it what a plain quoting says rather than refusing
            // it outside the road a raised value travels.
            other => {
                let said = other.quoted(self.names.brief_reals);
                if escaped { crate::text::ascii_escaped(&said) } else { said }
            }
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

    /// A run of decimal digits read as a width or a precision. CPython
    /// parses these into a machine word first: a run wide enough to
    /// overflow it (the format mini-language alone; `%`-style
    /// formatting parses further before this same complaint is worded)
    /// is "too many decimal digits" whatever the run means, and a run
    /// that fits but still names a width or a precision past what the
    /// field could ever use is worded by which of the two it was.
    fn read_count(&self, input: &mut std::iter::Peekable<std::str::Chars<'_>>, big: &str, strict: bool) -> Result<Option<usize>, String> {
        let mut digits = String::new();
        while input.peek().map_or(false, |c| c.is_ascii_digit()) { digits.push(input.next().unwrap()); }
        if digits.is_empty() { return Ok(None); }
        let total = digits.chars().fold(0u128, |acc, c| acc.saturating_mul(10).saturating_add(u128::from(c.to_digit(10).unwrap())));
        if strict && total >= (1u128 << 63) { return Err(self.complain("ext.text.format.digits", &[])); }
        if total > i32::MAX as u128 { return Err(self.complain(big, &[])); }
        Ok(Some(total as usize))
    }

    fn group_error(&self, left: char, right: char) -> String {
        let detail = if left == right { format!("'{left}' with '{right}'") }
            else { "both ',' and '_'".to_owned() };
        self.complain("ext.text.format.group.conflict", &[&detail])
    }

    fn description(&self, pattern: &str, item: &Value) -> Result<Presentation, String> {
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
        shape.extent = self.read_count(&mut marks, "ext.text.format.width.big", true)?.unwrap_or_default();
        if marks.peek().map_or(false, |c| matches!(c, ',' | '_')) { shape.separator = marks.next(); }
        if matches!(marks.peek(), Some(',' | '_')) {
            return Err(self.group_error(shape.separator.unwrap(), *marks.peek().unwrap()));
        }
        if marks.peek() == Some(&'.') {
            marks.next();
            shape.digits = self.read_count(&mut marks, "ext.text.format.precision.big", true)?;
            if marks.peek().map_or(false, |c| matches!(c, ',' | '_')) { shape.fraction_separator = marks.next(); }
            else if shape.digits.is_none() { return Err(self.complain("ext.text.format.precision.missing", &[])); }
        }
        if matches!(marks.peek(), Some(',' | '_')) && shape.fraction_separator.is_some() {
            return Err(self.group_error(shape.separator.unwrap_or(shape.fraction_separator.unwrap()), *marks.peek().unwrap()));
        }
        shape.letter = marks.next();
        if marks.next().is_some() { return Err(self.complain("ext.text.format.invalid.detail", &[pattern, self.typename(item)])); }
        Ok(shape)
    }

    pub fn present(&self, item: &Value, pattern: &str, convert: &str) -> Answer {
        if let Value::Shared(held) | Value::Mutable(held, _) = item { return self.present(&held.borrow(), pattern, convert); }
        if !convert.is_empty() {
            let rendered = match convert {
                "a" => self.quote(item, true)?, "r" => self.quote(item, false)?, "s" => self.plain(item)?,
                _ => return Err(self.complain("ext.text.format.conversion", &[convert])),
            };
            return self.present(&Value::text(&rendered), pattern, "");
        }
        if pattern.is_empty() && !matches!(item, Value::Frac(_)) {
            return self.plain(item);
        }
        let mut shape = self.description(pattern, item)?;
        // A separator belongs to the presentation letter, even if the
        // receiver cannot use that letter. Check it before flags and type.
        if let Some(separator) = shape.separator {
            let letter = shape.letter.unwrap_or(if matches!(item, Value::Text(_)) { 's' } else { '\0' });
            let permitted = if separator == ',' {
                matches!(letter, '\0' | 'd' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%')
            } else {
                matches!(letter, '\0' | 'd' | 'b' | 'o' | 'x' | 'X' | 'e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%')
            };
            if !permitted {
                let mark = format!("'{separator}'");
                let code = format!("'{letter}'");
                return Err(self.complain("ext.text.format.group.type", &[&mark, &code]));
            }
        }
        if shape.letter == Some('n') {
            if let Some(separator) = shape.fraction_separator {
                return Err(self.complain("ext.text.format.group.type", &[&format!("'{separator}'"), "'n'"]));
            }
        }
        // The locale presentation writes the figures the plain ones
        // write, this run keeping a single locale: a whole number
        // stays with the tens and everything else takes the general
        // form. Text has no such presentation and says so. The
        // grouping belongs to the locale, so none may stand beside it.
        if shape.letter == Some('n') {
            if shape.separator.is_some() || shape.fraction_separator.is_some() { return Err(self.invalid()); }
            if !matches!(item, Value::Text(_)) {
                shape.letter = Some(match item { Value::Small(_) | Value::Huge(_) | Value::Flag(_) => 'd', _ => 'g' });
            }
        }
        let unknown = || self.complain("ext.text.format.unknown", &[&shape.letter.unwrap_or('\0').to_string(), self.typename(item)]);
        if let Value::Text(text) = item {
            if shape.letter.is_some() && shape.letter != Some('s') { return Err(unknown()); }
            if shape.polarity == Some(' ') { return Err(String::from("ValueError: Space not allowed in string format specifier")); }
            let bad = if shape.polarity.is_some() { Some("sign") }
                else if shape.no_minus_zero { Some("zero") }
                else if shape.alternative { Some("alternate") }
                else if shape.justify == Some('=') { Some("align") } else { None };
            if let Some(why) = bad { return Err(self.complain(&format!("ext.text.format.{why}.string"), &[])); }
            let kept: String = text.chars().take(shape.digits.unwrap_or(usize::MAX)).collect();
            return Ok(shape.padded(String::new(), kept, '<'));
        }
        // A complex worth is laid out as two real parts with the
        // second always under a sign, and the pair stands in brackets
        // where no presentation was named. Nought padding and the '='
        // justification carry no meaning across two numbers.
        if let Some((re, im)) = complex_parts(item) {
            if shape.letter.map_or(false, |c| !"eEfFgG".contains(c)) { return Err(unknown()); }
            if shape.padding == '0' { return Err(self.complain("ext.text.format.complex.zero", &[])); }
            if shape.justify == Some('=') { return Err(self.complain("ext.text.format.complex.align", &[])); }
            // Nothing named is the writing the representation gives:
            // the figures that read back again, in brackets, with a
            // real part that is a plain nought left out of them. A
            // precision named beside it asks the general form instead.
            let bare = shape.letter.is_none() && re == 0.0 && !re.is_sign_negative();
            let brackets = shape.letter.is_none() && !bare;
            let mut form = shape;
            if form.letter.is_none() && form.digits.is_some() { form.letter = Some('g'); }
            let mut laid = String::new();
            if !bare { laid.push_str(&side(re, &form, form.polarity)); }
            laid.push_str(&side(im, &form, if bare { form.polarity } else { Some('+') }));
            laid.push('j');
            if brackets { laid = format!("({laid})"); }
            let room = Presentation { padding: form.padding, justify: form.justify, extent: form.extent, ..Presentation::new() };
            return Ok(room.padded(String::new(), laid, '>'));
        }
        let integer = matches!(item, Value::Small(_) | Value::Huge(_) | Value::Flag(_));
        let real = matches!(item, Value::Frac(n) if n.places.is_some());
        // A kind with no figures to write takes no specification of
        // any sort, which is told against the name the protocol would
        // have asked under. A number this layout has no figures for is
        // another matter, and says what it said.
        if !integer && !real {
            if matches!(item, Value::Frac(_)) { return Err(self.refused()); }
            return Err(self.complain("ext.stmt.class.format.amiss", &[self.typename(item)]));
        }
        if integer && shape.letter.map_or(true, |c| "dboxXc".contains(c)) {
            if shape.digits.is_some() { return Err(self.complain("ext.text.format.precision.integer", &[])); }
            if shape.no_minus_zero { return Err(self.complain("ext.text.format.zero.integer", &[])); }
            if shape.letter == Some('c') {
                if shape.polarity.is_some() { return Err(self.complain("ext.text.format.sign.character", &[])); }
                if shape.alternative { return Err(self.complain("ext.text.format.alternate.character", &[])); }
                let text = self.character(item, false)?;
                return Ok(shape.padded(String::new(), text, '>'));
            }
            let radix = match shape.letter { Some('b') => 2, Some('o') => 8, Some('x' | 'X') => 16, _ => 10 };
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

    fn character(&self, item: &Value, of_bytes: bool) -> Answer {
        let most = if of_bytes { 0xff } else { 0x10ffff };
        match item.as_big()?.to_u32() {
            Some(n) if n <= most => char::from_u32(n).map(String::from).ok_or_else(|| self.refused()),
            _ => Err(self.complain("ext.text.format.character", &[])),
        }
    }

    pub fn interpolate(&self, pattern: &str, positions: &[Value], names: &[(String, Value)], asked: &mut dyn Elsewhere) -> Answer {
        self.weave(pattern, positions, names, &mut 0, 2, asked)
    }

    fn weave(&self, mut rest: &str, positions: &[Value], names: &[(String, Value)], numbering: &mut i64, allowance: i32, asked: &mut dyn Elsewhere) -> Answer {
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
            // Nothing after the opening brace at all: the brace stands
            // alone, which the language words apart from a field that
            // was begun and never closed.
            if rest.is_empty() { return Err(self.complain("ext.text.format.brace.single", &[])); }
            let mut inside_key = false;
            let split = rest.char_indices().find_map(|(i, c)| {
                if c == '[' { inside_key = true; }
                if c == ']' { inside_key = false; }
                (!inside_key && matches!(c, ':' | '!' | '}')).then_some(i)
            }).ok_or_else(|| self.complain("ext.text.format.brace.open", &[]))?;
            let mut key_brackets = false;
            for letter in rest[..split].chars() {
                if letter == '[' { key_brackets = true; }
                if letter == ']' { key_brackets = false; }
                if letter == '{' && !key_brackets { return Err(self.invalid()); }
            }
            let mut level = 0;
            let mut bracketed = false;
            let mut closes = false;
            for ch in rest.chars() {
                match ch {
                    '[' => bracketed = true,
                    ']' => bracketed = false,
                    '{' if !bracketed => level += 1,
                    '}' if !bracketed => { if level == 0 { closes = true; break; } level -= 1; }
                    _ => (),
                }
            }
            if !closes { return Err(self.complain("ext.text.format.brace.open", &[])); }
            if rest[split..].starts_with('!') && !rest[split + 1..].starts_with(['s','r','a']) {
                return Err(self.complain("ext.text.format.conversion", &[]));
            }
            let selector = &rest[..split];
            let value = self.select(selector, positions, names, numbering, asked)?;
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
                let spec = self.weave(&rest[..end], positions, names, numbering, allowance - 1, asked)?;
                rest = &rest[end..]; spec
            } else { String::new() };
            if !rest.starts_with('}') { return Err(self.complain("ext.text.format.brace.open", &[])); }
            let laid = match asked.field_laid(&value, &specification, &conversion)? {
                Some(ready) => ready,
                None => self.present(&value, &specification, &conversion)?,
            };
            finished.push_str(&laid);
            rest = &rest[1..];
        }
        finished.push_str(rest);
        Ok(finished)
    }

    fn select(&self, field: &str, positions: &[Value], names: &[(String, Value)], numbering: &mut i64, access: &mut dyn Elsewhere) -> Result<Value, String> {
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
            // A run of digits too wide for a machine word overflows the
            // same way a width or a precision that wide would, whether
            // it ever names a field the arguments hold or not.
            let wide: u128 = first.parse().unwrap_or(u128::MAX);
            if wide >= (1u128 << 63) { return Err(self.complain("ext.text.format.digits", &[])); }
            Some(first.parse::<usize>().map_err(|_| self.complain("ext.text.format.index", &[first]))?)
        } else { None };
        let mut selected = match index {
            Some(n) => positions.get(n).cloned().ok_or_else(|| self.complain("ext.text.format.index", &[&n.to_string()]))?,
            None => names.iter().find(|(k, _)| k == first).map(|(_, v)| v.clone())
                .ok_or_else(|| self.name_absent(first))?,
        };
        let mut following = &field[first_end..];
        while !following.is_empty() {
            selected = selected.settled();
            let bracket = following.starts_with('[');
            let tail = following.get(1..).ok_or_else(|| self.invalid())?;
            let end = if bracket { tail.find(']').ok_or_else(|| self.invalid())? }
                else if following.starts_with('.') { tail.find(['[', '.']).unwrap_or(tail.len()) }
                else { return Err(self.invalid()); };
            let asked = &tail[..end];
            if !bracket && asked.is_empty() { return Err(self.invalid()); }
            if bracket && !asked.is_empty() && asked.bytes().all(|digit| digit.is_ascii_digit()) && asked.parse::<i64>().is_err() { return Err(self.complain("ext.text.format.digits", &[])); }
            let key = if bracket { asked.parse::<i64>().map(Value::Small).unwrap_or_else(|_| Value::text(asked)) } else { Value::text(asked) };
            let found = access.field_member(&selected, &key, bracket)?;
            selected = found.ok_or_else(|| if bracket { self.name_absent(asked) } else { self.refused() })?;
            following = &tail[end + usize::from(bracket)..];
        }
        Ok(selected)
    }

    /// Marks filled one at a time. A pattern read off a row of bytes
    /// says so: it knows the mark that shows a row of bytes, it seeks a
    /// keyed mark's name among keys that are rows of bytes, and a
    /// character mark holds one byte alone.
    pub fn remainder(&self, pattern: &str, supplied: &Value, asked: &mut dyn Elsewhere, of_bytes: bool) -> Answer {
        let mut stored = supplied.settled();
        if let Value::Thing(object) = &stored {
            let contents = object.holds.borrow().iter().find_map(|(key, item)| {
                if key != "\0underlying" { return None; }
                match item.settled() { row @ Value::Tuple(_) => Some(row), _ => None }
            });
            if let Some(row) = contents { stored = row; }
        }
        let supplied = &stored;
        let positional = match supplied { Value::Tuple(items) | Value::Row(items) | Value::Arguments(items) => items.as_slice(), _ => std::slice::from_ref(supplied) };
        let mut used = 0usize;
        let mut named_seen = false;
        let mut input = pattern.chars().peekable();
        let mut output = String::new();
        while let Some(mark) = input.next() {
            if mark != '%' { output.push(mark); continue; }
            if input.peek() == Some(&'%') { input.next(); output.push('%'); continue; }
            let mark_position = pattern.chars().count() - input.clone().count() - 1;
            let mut named_key = None;
            let named = if input.peek() == Some(&'(') {
                input.next();
                let mut key = String::new();
                let mut balance = 1usize;
                loop {
                    let c = input.next().ok_or_else(|| self.complain("ext.op.rem.format.key.incomplete", &[&mark_position.to_string()]))?;
                    if c == '(' { balance += 1; }
                    if c == ')' { balance -= 1; }
                    if balance == 0 { break; }
                    key.push(c);
                }
                named_key = Some(key.clone());
                let Value::Dict(pairs) = supplied else { return Err(self.complain("ext.op.rem.format.mapping", &[self.typename(supplied)])); };
                named_seen = true;
                used = positional.len();
                let spelt = |k: &Value| match k {
                    Value::Text(s) => !of_bytes && s.as_ref() == key,
                    Value::Octets { cell, .. } => of_bytes && cell.borrow().iter().copied().map(char::from).eq(key.chars()),
                    _ => false,
                };
                Some(pairs.iter().find(|(k, _)| spelt(k)).map(|(_, v)| v)
                    .ok_or_else(|| self.name_absent(&key))?)
            } else { None };
            if named_seen && named.is_none() { return Err(self.complain("ext.op.rem.format.mapping.key", &[&mark_position.to_string()])); }
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
                if named.is_some() { return Err(self.complain("ext.op.rem.format.mapping.star", &[&mark_position.to_string()])); }
                input.next();
                let width = self.dynamic(positional, &mut used, "width")?;
                shape.extent = width.unsigned_abs() as usize;
                if width < 0 { shape.justify = Some('<'); }
            } else { shape.extent = self.read_count(&mut input, "ext.text.format.width.big", false)
                .map_err(|_| self.complain("ext.op.rem.format.width.big", &[&mark_position.to_string()]))?.unwrap_or(0); }
            if input.peek() == Some(&'.') {
                input.next();
                shape.digits = Some(if input.peek() == Some(&'*') {
                    if named.is_some() { return Err(self.complain("ext.op.rem.format.mapping.star", &[&mark_position.to_string()])); }
                    input.next();
                    let precision = self.dynamic(positional, &mut used, "precision")?;
                    if precision < i32::MIN as i64 || precision > i32::MAX as i64 {
                        return Err(self.complain("ext.op.rem.format.star.big", &[&used.to_string(), "precision"]));
                    }
                    precision.max(0) as usize
                } else { self.read_count(&mut input, "ext.text.format.precision.big", false)
                    .map_err(|_| self.complain("ext.op.rem.format.precision.big", &[&mark_position.to_string()]))?.unwrap_or(0) });
            }
            if input.peek().map_or(false, |c| matches!(c, 'h' | 'l' | 'L')) { input.next(); }
            let conversion = input.next().ok_or_else(|| self.complain("ext.op.rem.format.incomplete", &[&mark_position.to_string()]))?;
            shape.letter = Some(conversion);
            let item = match named {
                Some(item) => item,
                None => {
                    used += 1;
                    positional.get(used - 1).ok_or_else(|| self.complain("ext.op.rem.format.few", &[&positional.len().to_string()]))?
                }
            };
            let location = match named_key {
                Some(key) if of_bytes => format!(" b'{key}'"),
                Some(key) => format!(" '{key}'"),
                None if matches!(supplied, Value::Tuple(_) | Value::Row(_) | Value::Arguments(_)) => format!(" {used}"),
                None => String::new(),
            };
            if shape.zero && shape.justify != Some('<') {
                shape.padding = '0'; shape.justify = Some('=');
            }
            let shows = matches!(conversion, 'a' | 'r' | 's') || of_bytes && conversion == 'b';
            let (head, body) = match conversion {
                'a' | 'r' | 's' | 'b' if shows => {
                    let text = match asked.value_worded(item, !matches!(conversion, 's' | 'b'))? {
                        Some(ready) => ready,
                        None if of_bytes && matches!(conversion, 'b' | 's') => {
                            return Err(self.complain("ext.op.rem.format.byte", &[&location, self.typename(item)]));
                        }
                        None if conversion == 's' => self.plain(item)?,
                        None => self.quote(item, conversion == 'a')?,
                    };
                    shape.padding = ' ';
                    if shape.justify == Some('=') { shape.justify = Some('>'); }
                    (String::new(), text.chars().take(shape.digits.unwrap_or(usize::MAX)).collect())
                }
                'c' => {
                    shape.padding = ' ';
                    if shape.justify == Some('=') { shape.justify = Some('>'); }
                    let c = match item {
                        Value::Octets { cell, .. } if of_bytes && cell.borrow().len() == 1 => char::from(cell.borrow()[0]).to_string(),
                        Value::Text(text) if !of_bytes && text.chars().count() == 1 => text.to_string(),
                        Value::Huge(_) | Value::Small(_) | Value::Flag(_) => self.character(item, of_bytes)
                            .map_err(|_| self.complain("ext.op.rem.format.character.range", &[&location, if of_bytes { "256" } else { "0x110000" }]))?,
                        _ => match asked.value_numbered(item, conversion)? {
                            NumberAnswer::Whole(whole) => self.character(&whole, of_bytes)
                                .map_err(|_| self.complain("ext.op.rem.format.character.range", &[&location, if of_bytes { "256" } else { "0x110000" }]))?,
                            NumberAnswer::Missing(name) | NumberAnswer::BadMethod(name) => {
                                let expected = if of_bytes { "an integer in range(256) or a single byte" }
                                    else { "an integer or a unicode character" };
                                let given = if !name.is_empty() {
                                    name
                                } else {
                                    match item {
                                        Value::Text(text) if !of_bytes => format!("a string of length {}", text.chars().count()),
                                        Value::Octets { cell, changeable, .. } if of_bytes => format!("a {} object of length {}", if *changeable { "bytearray" } else { "bytes" }, cell.borrow().len()),
                                        _ => item.kind_word(),
                                    }
                                };
                                return Err(self.complain("ext.op.rem.format.character", &[&location, expected, &given]));
                            },
                        },
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
                    let mut held: Option<Value> = None;
                    let mut accepted = matches!(item, Value::Small(_) | Value::Huge(_) | Value::Flag(_))
                        || (accepts_real && matches!(item, Value::Frac(r) if r.places.is_some() && !r.past_numbers()));
                    if !accepted {
                        let key = if accepts_real { "ext.op.rem.format.number" } else { "ext.op.rem.format.integer" };
                        match asked.value_numbered(item, conversion)? {
                            NumberAnswer::Whole(whole) => {
                                accepted = matches!(whole, Value::Small(_) | Value::Huge(_) | Value::Flag(_))
                                    || (accepts_real && matches!(&whole, Value::Frac(r) if r.places.is_some() && !r.past_numbers()));
                                held = Some(whole);
                            }
                            NumberAnswer::Missing(name) | NumberAnswer::BadMethod(name) => {
                                let named = if name.is_empty() { item.kind_word() } else { name };
                                return Err(self.complain(key, &[&location, &conversion.to_string(), &named]));
                            }
                        }
                    }
                    if accepts_real {
                        if let Some(Value::Frac(r)) = &held {
                            if r.past_numbers() {
                                let fault = match r.answers_none() { true => "ext.op.rem.format.nan", false => "ext.op.rem.format.infinity" };
                                return Err(self.complain(fault, &[]));
                            }
                        }
                    }
                    if !accepted {
                        let key = if accepts_real { "ext.op.rem.format.number" } else { "ext.op.rem.format.integer" };
                        let named = item.kind_word();
                        return Err(self.complain(key, &[&location, &conversion.to_string(), &named]));
                    }
                    let number = match &held { Some(whole) => whole.as_big()?, None => item.as_big()? };
                    let radix = match conversion { 'x' | 'X' => 16, 'o' => 8, _ => 10 };
                    let mut digits = number.abs().to_str_radix(radix);
                    if conversion == 'X' { digits = digits.to_uppercase(); }
                    digits = "0".repeat(shape.digits.unwrap_or(0).saturating_sub(digits.len())) + &digits;
                    let mut lead = shape.front(number.is_negative());
                    if shape.alternative { lead += match conversion { 'x' => "0x", 'X' => "0X", 'o' => "0o", _ => "" }; }
                    (lead, digits)
                }
                'e' | 'E' | 'f' | 'F' | 'g' | 'G' => {
                    let mut held: Option<Value> = None;
                    let ready = matches!(item, Value::Frac(_) | Value::Small(_) | Value::Huge(_) | Value::Flag(_));
                    if !ready {
                        match asked.value_numbered(item, conversion)? {
                            NumberAnswer::Whole(whole) if matches!(&whole, Value::Frac(_) | Value::Small(_) | Value::Huge(_) | Value::Flag(_)) => held = Some(whole),
                            NumberAnswer::Missing(name) | NumberAnswer::BadMethod(name) => {
                                let named = if name.is_empty() { item.kind_word() } else { name };
                                return Err(self.complain("ext.op.rem.format.real", &[&location, &conversion.to_string(), &named]));
                            },
                            _ => unreachable!(),
                        }
                    }
                    let chosen = held.as_ref().unwrap_or(item);
                    let number = self.binary(chosen).map_err(|_| self.complain("ext.op.rem.format.real", &[&location, &conversion.to_string(), &item.kind_word()]))?;
                    (shape.front(number.is_sign_negative() && !number.is_nan()), shape.real_digits(number.abs()))
                }
                _ => {
                    return Err(self.unsupported(conversion, mark_position, pattern.chars().count() - input.clone().count() - 1, of_bytes));
                }
            };
            output.push_str(&shape.padded(head, body, '>'));
        }
        if !named_seen && used < positional.len() && !matches!(supplied, Value::Dict(_)) {
            return Err(self.complain("ext.op.rem.format.many", &[if of_bytes { "bytes" } else { "string" }, &used.to_string(), &positional.len().to_string()]));
        }
        Ok(output)
    }

    fn unsupported(&self, letter: char, start: usize, at: usize, is_bytes: bool) -> String {
        if letter.is_ascii_alphanumeric() {
            return self.complain("ext.op.rem.format.code", &[&letter.to_string(), &start.to_string()]);
        }
        let described = if letter == '\'' { "\"'\"".to_owned() }
            else if is_bytes && !letter.is_ascii_graphic() && letter != ' ' { format!("with code 0x{:02x}", u32::from(letter)) }
            else if !is_bytes && (letter.is_control() || letter == '\u{7f}' || letter == '\u{80}') { format!("U+{:04X}", u32::from(letter)) }
            else if !letter.is_ascii() && !is_bytes { format!("'{letter}' (U+{:04X})", u32::from(letter)) }
            else { format!("'{letter}'") };
        self.complain("ext.op.rem.format.unexpected", &[&start.to_string(), &described, &at.to_string()])
    }

    fn dynamic(&self, supplied: &[Value], used: &mut usize, measure: &str) -> Result<i64, String> {
        if *used + 1 >= supplied.len() { return Err(self.complain("ext.op.rem.format.few", &[&supplied.len().to_string()])); }
        let value = &supplied[*used];
        *used += 1;
        match value {
            Value::Flag(_) | Value::Small(_) | Value::Huge(_) => value.as_big()?.to_i64()
                .filter(|n| measure != "width" || (-1_000_000..=1_000_000).contains(n))
                .ok_or_else(|| self.complain("ext.op.rem.format.star.big", &[&used.to_string(), measure])),
            _ => Err(self.complain("ext.op.rem.format.star", &[&used.to_string(), self.typename(value)])),
        }
    }
}

#[derive(Clone, Copy)]
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
        else if mode == 'f' || mode == '%' { long_real_digits(x, precision, false) }
        else if mode == 'e' { long_real_digits(x, precision, true) }
        else if self.letter.is_none() && self.digits.is_none() {
            if x > 0.0 && (x < 0.0001 || x >= 1e16) { format!("{:e}", x) } else { format!("{}", x) }
        } else {
            let significant = precision.max(1);
            let sci = long_real_digits(x, significant - 1, true);
            let (coefficient, exponent) = sci.split_once('e').unwrap();
            let power = exponent.parse::<i32>().unwrap();
            let cutoff = significant as i32 - if self.letter.is_none() { 1 } else { 0 };
            if power >= cutoff || power < -4 {
                let keep = if self.alternative || !coefficient.contains('.') { coefficient }
                    else { coefficient.trim_end_matches('0').trim_end_matches('.') };
                keep.to_owned() + "e" + exponent
            } else {
                let places = (significant as i32 - power - 1).max(0) as usize;
                let fixed = long_real_digits(x, places, false);
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

fn long_real_digits(value: f64, wanted: usize, exponent: bool) -> String {
    let bounded = wanted.min(1074);
    let text = if exponent { format!("{:.*e}", bounded, value) } else { format!("{:.*}", bounded, value) };
    if wanted == bounded || !value.is_finite() { return text; }
    // No binary64 value has further nonzero fractional digits here.
    let end = text.find('e').unwrap_or(text.len());
    let mut answer = text[..end].to_owned();
    answer.extend(std::iter::repeat('0').take(wanted - bounded));
    answer.push_str(&text[end..]);
    answer
}

/// The two real parts of a worth that keeps them, and nothing at all
/// for a worth that does not.
fn complex_parts(item: &Value) -> Option<(f64, f64)> {
    match item {
        Value::Complex(pair) => Some((pair.0, pair.1)),
        Value::Imaginary { coefficient, .. } => Some((0.0, *coefficient)),
        _ => None,
    }
}

/// One part of a complex worth: the figures the presentation asks for,
/// under the sign this part is to stand with. The pair is padded once
/// over, after both parts are laid out, so the extent belongs to the
/// whole of it and not to either number.
fn side(number: f64, form: &Presentation, polarity: Option<char>) -> String {
    let size = number.abs();
    let mut body = if form.letter.is_none() && form.digits.is_none() {
        if size.is_nan() { "nan".to_owned() } else if size.is_infinite() { "inf".to_owned() } else { crate::data::brief_decimal(size) }
    } else { form.real_digits(size) };
    if form.alternative && form.letter.is_none() && size.is_finite() && !body.contains(['.', 'e', 'E']) {
        body.push('.');
    }
    // A part that rounds to nought loses its minus where the
    // presentation asks for no signed nought.
    let nought = form.no_minus_zero && body.parse::<f64>().ok() == Some(0.0);
    let front = match (number.is_sign_negative() && !number.is_nan() && !nought, polarity) {
        (true, _) => String::from("-"),
        (false, Some(c @ ('+' | ' '))) => c.to_string(),
        _ => String::new(),
    };
    let body = match size.is_finite() {
        true => Presentation { letter: form.letter, separator: form.separator, fraction_separator: form.fraction_separator, ..Presentation::new() }
            .grouped(body, front.len(), 3),
        false => body,
    };
    front + &body
}

pub fn is_complaint(table: &Table, message: &str) -> bool {
    let labels = "ext.text.format.complex.zero ext.text.format.complex.align ext.text.format.zero.integer ext.text.format.zero.string ext.op.rem.format.nan ext.op.rem.format.infinity ext.text.format.invalid ext.text.format.invalid.detail ext.text.format.group.conflict ext.text.format.group.type ext.text.format.unknown ext.text.format.unready ext.text.format.digits ext.text.format.width.big ext.text.format.precision.big ext.text.format.precision.integer ext.text.format.precision.missing ext.text.format.sign.string ext.text.format.alternate.string ext.text.format.align.string ext.text.format.sign.character ext.text.format.alternate.character ext.text.format.character ext.text.format.spec.type ext.text.format.numbered.auto ext.text.format.numbered.manual ext.text.format.index ext.text.format.key ext.text.format.brace.open ext.text.format.brace.single ext.text.format.brace.close ext.text.format.conversion ext.text.format.recursion ext.op.rem.format.few ext.op.rem.format.many ext.op.rem.format.mapping ext.op.rem.format.mapping.key ext.op.rem.format.mapping.star ext.op.rem.format.width.big ext.op.rem.format.precision.big ext.op.rem.format.number ext.op.rem.format.integer ext.op.rem.format.real ext.op.rem.format.character ext.op.rem.format.character.range ext.op.rem.format.star ext.op.rem.format.star.big ext.op.rem.format.incomplete ext.op.rem.format.key.incomplete ext.op.rem.format.unexpected ext.op.rem.format.code";
    labels.split_whitespace().filter_map(|label| table.single(label))
        .any(|opening| !opening.is_empty() && message.starts_with(opening))
}
