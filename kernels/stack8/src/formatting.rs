// A specification governs a single field. Its marks are read before the
// value is written, so a mark without a meaning never loses its voice.

use num_traits::{Signed, ToPrimitive};
use crate::lang::Lang;
use crate::value::{Value, Wording};

type Result<T> = std::result::Result<T, String>;

pub struct Writer<'a> {
    pub lang: &'a Lang,
    pub words: Wording<'a>,
}

impl Writer<'_> {
    pub fn fault(&self, key: &str, pieces: &[&str]) -> String {
        let words = match key {
            "ext.text.format.zero.string" => &self.lang.fmt_text_format_zero_string,
            "ext.text.format.zero.integer" => &self.lang.fmt_text_format_zero_integer,
            "ext.op.rem.format.infinity" => &self.lang.fmt_op_rem_format_infinity,
            "ext.op.rem.format.nan" => &self.lang.fmt_op_rem_format_nan,
            "ext.text.format.invalid" => &self.lang.fmt_text_format_invalid,
            "ext.text.format.unknown" => &self.lang.fmt_text_format_unknown,
            "ext.text.format.kinds" => &self.lang.fmt_text_format_kinds,
            "ext.text.format.unready" => &self.lang.fmt_text_format_unready,
            "ext.text.format.precision.integer" => &self.lang.fmt_text_format_precision_integer,
            "ext.text.format.precision.missing" => &self.lang.fmt_text_format_precision_missing,
            "ext.text.format.sign.string" => &self.lang.fmt_text_format_sign_string,
            "ext.text.format.alternate.string" => &self.lang.fmt_text_format_alternate_string,
            "ext.text.format.align.string" => &self.lang.fmt_text_format_align_string,
            "ext.text.format.sign.character" => &self.lang.fmt_text_format_sign_character,
            "ext.text.format.alternate.character" => &self.lang.fmt_text_format_alternate_character,
            "ext.text.format.character" => &self.lang.fmt_text_format_character,
            "ext.text.format.spec.type" => &self.lang.fmt_text_format_spec_type,
            "ext.text.format.numbered.auto" => &self.lang.fmt_text_format_numbered_auto,
            "ext.text.format.numbered.manual" => &self.lang.fmt_text_format_numbered_manual,
            "ext.text.format.index" => &self.lang.fmt_text_format_index,
            "ext.text.format.key" => &self.lang.fmt_text_format_key,
            "ext.text.format.brace.open" => &self.lang.fmt_text_format_brace_open,
            "ext.text.format.brace.close" => &self.lang.fmt_text_format_brace_close,
            "ext.text.format.conversion" => &self.lang.fmt_text_format_conversion,
            "ext.text.format.recursion" => &self.lang.fmt_text_format_recursion,
            "ext.op.rem.format.few" => &self.lang.fmt_op_rem_format_few,
            "ext.op.rem.format.many" => &self.lang.fmt_op_rem_format_many,
            "ext.op.rem.format.mapping" => &self.lang.fmt_op_rem_format_mapping,
            "ext.op.rem.format.number" => &self.lang.fmt_op_rem_format_number,
            "ext.op.rem.format.integer" => &self.lang.fmt_op_rem_format_integer,
            "ext.op.rem.format.real" => &self.lang.fmt_op_rem_format_real,
            "ext.op.rem.format.character" => &self.lang.fmt_op_rem_format_character,
            "ext.op.rem.format.star" => &self.lang.fmt_op_rem_format_star,
            "ext.op.rem.format.incomplete" => &self.lang.fmt_op_rem_format_incomplete,
            "ext.op.rem.format.code" => &self.lang.fmt_op_rem_format_code,
            _ => &self.lang.fmt_text_format_invalid,
        };
        let mut out = String::new();
        for (i, word) in words.iter().enumerate() {
            out.push_str(word);
            if let Some(piece) = pieces.get(i) { out.push_str(piece); }
        }
        out
    }

    pub fn kind<'a>(&'a self, value: &Value) -> &'a str {
        let at = match value {
            Value::Small(_) | Value::Huge(_) => 0, Value::Real(_) => 1,
            Value::Text(_) => 2, Value::Flag(_) => 3, Value::Array(_) => 4,
            Value::Map(_) => 5, Value::Null => 6, _ => 7,
        };
        self.lang.fmt_text_format_kinds.get(at).map_or("", String::as_str)
    }

    pub fn representation(&self, value: &Value, ascii: bool) -> Result<String> {
        if matches!(value, Value::Collection(..) | Value::View(_)) { return self.representation(&value.contents(), ascii); }
        match value {
            Value::Text(_) => {
                let quoted = value.string_field(&self.words, "", if ascii { "a" } else { "r" }).ok_or_else(|| self.fault("ext.text.format.unready", &[]))?;
                let mut out = String::new();
                for c in quoted.chars() {
                    if !c.is_ascii() && c.is_whitespace() {
                        let n = c as u32;
                        if n <= 255 { out.push_str(&format!("\\x{n:02x}")); }
                        else if n <= 65535 { out.push_str(&format!("\\u{n:04x}")); }
                        else { out.push_str(&format!("\\U{n:08x}")); }
                    } else {
                        // A preceding letter keeps combining marks ordinary.
                        let probe = format!("a{c}");
                        if !c.is_ascii() && probe.escape_debug().skip(1).take(3).collect::<String>() == "\\u{" {
                            return Err(self.fault("ext.text.format.unready", &[]));
                        }
                        out.push(c);
                    }
                }
                Ok(out)
            }
            Value::Tuple(items) => {
                let parts = items.iter().map(|v| self.representation(v, ascii)).collect::<Result<Vec<_>>>()?;
                Ok(format!("({}{})", parts.join(", "), if items.len() == 1 { "," } else { "" }))
            }
            Value::Array(items) => {
                let parts = items.iter().map(|v| self.representation(v, ascii)).collect::<Result<Vec<_>>>()?;
                Ok(format!("[{}]", parts.join(", ")))
            }
            Value::Map(pairs) => {
                let mut parts = Vec::new();
                for (k, v) in pairs.iter() { parts.push(format!("{}: {}", self.representation(k, ascii)?, self.representation(v, ascii)?)); }
                Ok(format!("{{{}}}", parts.join(", ")))
            }
            Value::Real(r) => {
                if r.outside() { return Ok(if r.no_number() { "nan" } else if r.p.is_negative() { "-inf" } else { "inf" }.to_string()); }
                let mut text = value.display(&self.words);
                if !r.outside() && !text.contains(['.', 'e', 'E']) { text.push_str(".0"); }
                Ok(text)
            }
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Null | Value::Ellipsis => Ok(value.display(&self.words)),
            Value::Bond(cell) => self.representation(&cell.borrow(), ascii),
            _ => Err(self.lang.format_unsupported.clone().unwrap_or_default()),
        }
    }

    pub fn field(&self, value: &Value, spec: &str, conversion: &str) -> Result<String> {
        if matches!(value, Value::Bond(_) | Value::Collection(..) | Value::View(_)) { return self.field(&value.contents(), spec, conversion); }
        if !conversion.is_empty() {
            let text = match conversion {
                "r" | "a" => self.representation(value, conversion == "a")?,
                "s" => match value { Value::Text(s) => s.to_string(), _ => self.representation(value, false)? },
                _ => return Err(self.fault("ext.text.format.conversion", &[conversion])),
            };
            return self.field(&Value::text(&text), spec, "");
        }
        if spec.is_empty() { return self.representation_plain(value); }
        let mut rule = self.parse(spec)?;
        let kind = rule.code;
        if kind == 'n' { return Err(self.fault("ext.text.format.unready", &[])); }
        if let Value::Text(text) = value {
            if !matches!(kind, '\0' | 's') { return Err(self.unknown(value, kind)); }
            if rule.sign != '\0' { return Err(self.fault("ext.text.format.sign.string", &[])); }
            if rule.unsigned_zero { return Err(self.fault("ext.text.format.zero.string", &[])); }
            if rule.alternate { return Err(self.fault("ext.text.format.alternate.string", &[])); }
            if rule.align == '=' { return Err(self.fault("ext.text.format.align.string", &[])); }
            if rule.group != '\0' || rule.fraction_group != '\0' { return Err(self.fault("ext.text.format.invalid", &[])); }
            let shown: String = text.chars().take(rule.precision.unwrap_or(usize::MAX)).collect();
            return Ok(rule.pad("", &shown, '<'));
        }
        if rule.zero && rule.align == '\0' { rule.align = '='; }
        let whole = matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_));
        let real = matches!(value, Value::Real(_));
        if !whole && !real { return Err(self.fault("ext.text.format.unready", &[])); }
        if whole && matches!(kind, '\0' | 'd' | 'b' | 'o' | 'x' | 'X' | 'c') {
            if rule.unsigned_zero { return Err(self.fault("ext.text.format.zero.integer", &[])); }
            if rule.precision.is_some() { return Err(self.fault("ext.text.format.precision.integer", &[])); }
            let number = value.as_big()?;
            if kind == 'c' {
                if rule.sign != '\0' { return Err(self.fault("ext.text.format.sign.character", &[])); }
                if rule.alternate { return Err(self.fault("ext.text.format.alternate.character", &[])); }
                if rule.group != '\0' { return Err(self.fault("ext.text.format.invalid", &[])); }
                return Ok(rule.pad("", &self.character(value)?, '>'));
            }
            let base = match kind { 'b' => 2, 'o' => 8, 'x' | 'X' => 16, _ => 10 };
            if rule.fraction_group != '\0' || rule.group == ',' && base != 10 { return Err(self.fault("ext.text.format.invalid", &[])); }
            let mut figures = number.abs().to_str_radix(base);
            if kind == 'X' { figures.make_ascii_uppercase(); }
            let mut head = rule.sign_for(number.is_negative());
            if rule.alternate { head.push_str(match kind { 'b' => "0b", 'o' => "0o", 'x' => "0x", 'X' => "0X", _ => "" }); }
            rule.group_digits(&mut figures, head.len(), if base == 10 { 3 } else { 4 });
            return Ok(rule.pad(&head, &figures, '>'));
        }
        if !matches!(kind, '\0' | 'f' | 'F' | 'e' | 'E' | 'g' | 'G' | '%') { return Err(self.unknown(value, kind)); }
        let mut number = match value {
            Value::Real(r) => {
                let n = crate::value::as_binary(&r.p, &r.q);
                if r.below && n == 0.0 { -0.0 } else { n }
            }
            _ => value.as_big()?.to_f64().filter(|n| n.is_finite()).ok_or_else(|| self.fault("ext.text.format.unready", &[]))?,
        };
        if kind == '%' { number *= 100.0; }
        let magnitude = number.abs();
        let mut body = if magnitude.is_nan() { "nan".to_string() }
            else if magnitude.is_infinite() { "inf".to_string() }
            else { decimal(magnitude, &rule) };
        if kind.is_ascii_uppercase() { body.make_ascii_uppercase(); }
        let suppress = rule.unsigned_zero && body.parse::<f64>().ok() == Some(0.0);
        let head = rule.sign_for(number.is_sign_negative() && !number.is_nan() && !suppress);
        if kind == '%' { body.push('%'); }
        if magnitude.is_finite() { rule.group_digits(&mut body, head.len(), 3); }
        Ok(rule.pad(&head, &body, '>'))
    }

    fn representation_plain(&self, value: &Value) -> Result<String> {
        match value { Value::Text(s) => Ok(s.to_string()), Value::Real(_) => Ok(value.display(&self.words)), _ => self.representation(value, false) }
    }

    fn unknown(&self, value: &Value, code: char) -> String {
        self.fault("ext.text.format.unknown", &[&code.to_string(), self.kind(value)])
    }

    fn parse(&self, spec: &str) -> Result<Rule> {
        let letters: Vec<char> = spec.chars().collect();
        let mut at = 0;
        let mut rule = Rule::default();
        let fill_given = letters.get(1).map_or(false, |c| "<>=^".contains(*c));
        if letters.get(1).map_or(false, |c| "<>=^".contains(*c)) {
            rule.fill = letters[0]; rule.align = letters[1]; at = 2;
        } else if letters.first().map_or(false, |c| "<>=^".contains(*c)) { rule.align = letters[0]; at = 1; }
        if letters.get(at).map_or(false, |c| "+- ".contains(*c)) { rule.sign = letters[at]; at += 1; }
        if letters.get(at) == Some(&'z') { rule.unsigned_zero = true; at += 1; }
        if letters.get(at) == Some(&'#') { rule.alternate = true; at += 1; }
        if letters.get(at) == Some(&'0') {
            if !fill_given { rule.fill = '0'; }
            rule.zero = true;
            at += 1;
        }
        rule.width = self.count(&letters, &mut at)?.unwrap_or(0);
        if letters.get(at).map_or(false, |c| matches!(c, ',' | '_')) { rule.group = letters[at]; at += 1; }
        if letters.get(at) == Some(&'.') {
            at += 1;
            rule.precision = self.count(&letters, &mut at)?;
            if letters.get(at).map_or(false, |c| matches!(c, ',' | '_')) { rule.fraction_group = letters[at]; at += 1; }
            else if rule.precision.is_none() { return Err(self.fault("ext.text.format.precision.missing", &[])); }
        }
        if let Some(&c) = letters.get(at) { rule.code = c; at += 1; }
        if at != letters.len() { return Err(self.fault("ext.text.format.invalid", &[])); }
        Ok(rule)
    }

    fn count(&self, chars: &[char], at: &mut usize) -> Result<Option<usize>> {
        let begin = *at;
        let mut total = 0usize;
        while let Some(d) = chars.get(*at).and_then(|c| c.to_digit(10)) {
            total = total.checked_mul(10).and_then(|v| v.checked_add(d as usize))
                .filter(|n| *n <= 100000).ok_or_else(|| self.fault("ext.text.format.unready", &[]))?;
            *at += 1;
        }
        Ok((*at != begin).then_some(total))
    }

    pub fn template(&self, text: &str, args: &[(Option<String>, Value)]) -> Result<String> {
        self.fill_template(text, args, &mut 0, &mut false, 0)
    }

    fn fill_template(&self, text: &str, args: &[(Option<String>, Value)], next: &mut usize, manual: &mut bool, depth: usize) -> Result<String> {
        if depth > 2 { return Err(self.fault("ext.text.format.recursion", &[])); }
        let chars: Vec<char> = text.chars().collect();
        let mut at = 0;
        let mut out = String::new();
        while let Some(&c) = chars.get(at) {
            at += 1;
            if !matches!(c, '{' | '}') { out.push(c); continue; }
            if chars.get(at) == Some(&c) { out.push(c); at += 1; continue; }
            if c == '}' { return Err(self.fault("ext.text.format.brace.close", &[])); }
            if depth == 2 { return Err(self.fault("ext.text.format.recursion", &[])); }
            let from = at;
            let mut brackets = false;
            while let Some(&c) = chars.get(at) {
                if c == '[' { brackets = true; }
                if c == ']' { brackets = false; }
                if !brackets && matches!(c, '!' | ':' | '}') { break; }
                at += 1;
            }
            if at == chars.len() { return Err(self.fault("ext.text.format.brace.open", &[])); }
            let name: String = chars[from..at].iter().collect();
            let value = self.lookup(&name, args, next, manual)?;
            let mut conversion = String::new();
            if chars.get(at) == Some(&'!') {
                at += 1;
                conversion.push(*chars.get(at).ok_or_else(|| self.fault("ext.text.format.brace.open", &[]))?);
                at += 1;
            }
            let mut spec = String::new();
            if chars.get(at) == Some(&':') {
                at += 1;
                let from = at;
                let mut nested = 0usize;
                while let Some(&c) = chars.get(at) {
                    if c == '}' && nested == 0 { break; }
                    if c == '{' { nested += 1; }
                    if c == '}' { nested -= 1; }
                    at += 1;
                }
                spec = self.fill_template(&chars[from..at].iter().collect::<String>(), args, next, manual, depth + 1)?;
            }
            if chars.get(at) != Some(&'}') { return Err(self.fault("ext.text.format.brace.open", &[])); }
            at += 1;
            out.push_str(&self.field(&value, &spec, &conversion)?);
        }
        Ok(out)
    }

    fn lookup(&self, name: &str, args: &[(Option<String>, Value)], next: &mut usize, manual: &mut bool) -> Result<Value> {
        let end = name.find(['.', '[']).unwrap_or(name.len());
        let first = &name[..end];
        let position = if first.is_empty() {
            if *manual { return Err(self.fault("ext.text.format.numbered.auto", &[])); }
            let n = *next; *next += 1; Some(n)
        } else if first.chars().all(|c| c.is_ascii_digit()) {
            if *next > 0 { return Err(self.fault("ext.text.format.numbered.manual", &[])); }
            *manual = true;
            Some(first.parse().map_err(|_| self.fault("ext.text.format.index", &[first]))?)
        } else { None };
        let mut value = if let Some(n) = position {
            args.iter().filter(|(key, _)| key.is_none()).nth(n).map(|(_, v)| v.clone())
                .ok_or_else(|| self.fault("ext.text.format.index", &[&n.to_string()]))?
        } else {
            args.iter().find(|(key, _)| key.as_deref() == Some(first)).map(|(_, v)| v.clone())
                .ok_or_else(|| self.fault("ext.text.format.key", &[first]))?
        };
        let mut rest = &name[end..];
        while !rest.is_empty() {
            value = value.contents();
            if let Some(tail) = rest.strip_prefix('.') {
                let end = tail.find(['.', '[']).unwrap_or(tail.len());
                let member = &tail[..end];
                value = match &value {
                    Value::Object(o) => o.fields.borrow().iter().find(|(k, _)| k == member).map(|(_, v)| v.clone()),
                    _ => None,
                }.ok_or_else(|| self.fault("ext.text.format.unready", &[]))?;
                rest = &tail[end..];
            } else if let Some(tail) = rest.strip_prefix('[') {
                let end = tail.find(']').ok_or_else(|| self.fault("ext.text.format.invalid", &[]))?;
                let key = &tail[..end];
                let index = key.parse::<usize>().ok();
                let found = match &value {
                    Value::Map(pairs) => pairs.iter().find(|(k, _)| match (k, index) {
                        (Value::Text(s), None) => s.as_ref() == key,
                        (k, Some(n)) => k.equals(&Value::of_big(n.into())), _ => false,
                    }).map(|(_, v)| v.clone()),
                    Value::Array(a) | Value::Tuple(a) => index.and_then(|n| a.get(n).cloned()),
                    Value::Text(s) => index.and_then(|n| s.chars().nth(n)).map(|c| Value::text(&c.to_string())),
                    _ => None,
                };
                value = found.ok_or_else(|| self.fault("ext.text.format.key", &[key]))?;
                rest = &tail[end + 1..];
            } else { return Err(self.fault("ext.text.format.invalid", &[])); }
        }
        Ok(value)
    }

    pub fn percent(&self, text: &str, argument: &Value) -> Result<String> {
        let settled = argument.contents();
        let argument = &settled;
        let args: Vec<&Value> = match argument { Value::Tuple(a) => a.iter().collect(), one => vec![one] };
        let mut used = 0;
        let mut at = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        let mut mapped = false;
        while let Some(&c) = chars.get(at) {
            at += 1;
            if c != '%' { out.push(c); continue; }
            if chars.get(at) == Some(&'%') { out.push('%'); at += 1; continue; }
            let mut keyed = None;
            if chars.get(at) == Some(&'(') {
                at += 1;
                let begin = at;
                let mut nesting = 1;
                while let Some(&c) = chars.get(at) {
                    if c == ')' { nesting -= 1; }
                    if nesting == 0 { break; }
                    if c == '(' { nesting += 1; }
                    at += 1;
                }
                if chars.get(at).is_none() { return Err(self.fault("ext.op.rem.format.incomplete", &[])); }
                let key: String = chars[begin..at].iter().collect(); at += 1;
                let Value::Map(pairs) = argument else { return Err(self.fault("ext.op.rem.format.mapping", &[])); };
                keyed = Some(pairs.iter().find(|(k, _)| matches!(k, Value::Text(s) if s.as_ref() == key)).map(|(_, v)| v)
                    .ok_or_else(|| self.fault("ext.text.format.key", &[&key]))?);
                mapped = true;
                used = args.len();
            }
            let mut rule = Rule::default();
            while let Some(&flag) = chars.get(at).filter(|c| "-+ #0".contains(**c)) {
                match flag {
                    '-' => rule.align = '<', '+' => rule.sign = '+',
                    ' ' if rule.sign != '+' => rule.sign = ' ', '#' => rule.alternate = true,
                    '0' => rule.fill = '0', _ => {}
                }
                at += 1;
            }
            if rule.align == '<' { rule.fill = ' '; }
            else if rule.fill == '0' { rule.align = '='; }
            rule.width = if chars.get(at) == Some(&'*') {
                if keyed.is_some() { return Err(self.fault("ext.text.format.unready", &[])); }
                at += 1;
                let n = self.star(&args, &mut used)?;
                if n < 0 { rule.align = '<'; rule.fill = ' '; }
                n.unsigned_abs() as usize
            } else { self.count(&chars, &mut at)?.unwrap_or(0) };
            if chars.get(at) == Some(&'.') {
                at += 1;
                rule.precision = Some(if chars.get(at) == Some(&'*') {
                    if keyed.is_some() { return Err(self.fault("ext.text.format.unready", &[])); }
                    at += 1; self.star(&args, &mut used)?.max(0) as usize
                } else { self.count(&chars, &mut at)?.unwrap_or(0) });
            }
            if chars.get(at).map_or(false, |c| "hlL".contains(*c)) { at += 1; }
            let code = *chars.get(at).ok_or_else(|| self.fault("ext.op.rem.format.incomplete", &[]))?;
            at += 1;
            let value = match keyed { Some(v) => v, None => {
                let v = args.get(used).copied().ok_or_else(|| self.fault("ext.op.rem.format.few", &[]))?;
                used += 1; v
            }};
            rule.code = code;
            if !"srad iuoxXeEfFgGc".replace(' ', "").contains(code) {
                return Err(self.fault("ext.op.rem.format.code", &[&code.to_string(), &format!("{:x}", code as u32), &(at - 1).to_string()]));
            }
            let result = if matches!(code, 's' | 'r' | 'a') {
                let shown = if code == 's' && matches!(value, Value::Text(_)) { self.representation_plain(value)? } else { self.representation(value, code == 'a')? };
                let shown: String = shown.chars().take(rule.precision.unwrap_or(usize::MAX)).collect();
                rule.fill = ' '; if rule.align == '=' { rule.align = '>'; }
                rule.pad("", &shown, '>')
            } else if code == 'c' {
                let shown = match value {
                    Value::Text(s) if s.chars().count() == 1 => s.to_string(),
                    Value::Small(_) | Value::Huge(_) | Value::Flag(_) => self.character(value)?,
                    _ => return Err(self.fault("ext.op.rem.format.character", &[])),
                };
                rule.fill = ' '; if rule.align == '=' { rule.align = '>'; }
                rule.pad("", &shown, '>')
            } else if matches!(code, 'd' | 'i' | 'u' | 'o' | 'x' | 'X') {
                let integral = matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_));
                let decimal = matches!(code, 'd' | 'i' | 'u');
                if decimal {
                    if let Value::Real(r) = value {
                        if r.outside() { return Err(self.fault(if r.no_number() { "ext.op.rem.format.nan" } else { "ext.op.rem.format.infinity" }, &[])); }
                    }
                }
                if !integral && !(decimal && matches!(value, Value::Real(r) if !r.outside())) {
                    let key = if decimal { "ext.op.rem.format.number" } else { "ext.op.rem.format.integer" };
                    return Err(self.fault(key, &[&code.to_string(), self.kind(value)]));
                }
                let n = value.as_big()?;
                let mut digits = n.abs().to_str_radix(if decimal { 10 } else if code == 'o' { 8 } else { 16 });
                if code == 'X' { digits.make_ascii_uppercase(); }
                if let Some(p) = rule.precision { digits = "0".repeat(p.saturating_sub(digits.len())) + &digits; }
                let mut head = rule.sign_for(n.is_negative());
                if rule.alternate { head.push_str(match code { 'o' => "0o", 'x' => "0x", 'X' => "0X", _ => "" }); }
                rule.pad(&head, &digits, '>')
            } else {
                let n = match value {
                    Value::Real(r) => { let n = crate::value::as_binary(&r.p, &r.q); if r.below && n == 0.0 { -0.0 } else { n } },
                    Value::Small(_) | Value::Huge(_) | Value::Flag(_) => value.as_big()?.to_f64().filter(|n| n.is_finite()).ok_or_else(|| self.fault("ext.text.format.unready", &[]))?,
                    _ => return Err(self.fault("ext.op.rem.format.real", &[self.kind(value)])),
                };
                let mut body = if n.is_nan() { "nan".into() } else if n.is_infinite() { "inf".into() } else { decimal(n.abs(), &rule) };
                if code.is_ascii_uppercase() { body.make_ascii_uppercase(); }
                rule.pad(&rule.sign_for(n.is_sign_negative() && !n.is_nan()), &body, '>')
            };
            out.push_str(&result);
        }
        if !mapped && used != args.len() && !matches!(argument, Value::Map(_)) { return Err(self.fault("ext.op.rem.format.many", &[])); }
        Ok(out)
    }

    fn character(&self, value: &Value) -> Result<String> {
        let n = value.as_big()?.to_u32().filter(|n| *n <= 0x10ffff)
            .ok_or_else(|| self.fault("ext.text.format.character", &[]))?;
        char::from_u32(n).map(|c| c.to_string()).ok_or_else(|| self.fault("ext.text.format.unready", &[]))
    }

    fn star(&self, args: &[&Value], used: &mut usize) -> Result<i64> {
        let v = args.get(*used).ok_or_else(|| self.fault("ext.op.rem.format.few", &[]))?;
        *used += 1;
        if !matches!(v, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { return Err(self.fault("ext.op.rem.format.star", &[])); }
        v.as_big()?.to_i64().filter(|n| n.unsigned_abs() <= 100000).ok_or_else(|| self.fault("ext.text.format.unready", &[]))
    }
}

struct Rule {
    fill: char, align: char, sign: char, alternate: bool, zero: bool, unsigned_zero: bool,
    width: usize, group: char, fraction_group: char, precision: Option<usize>, code: char,
}

impl Default for Rule {
    fn default() -> Self {
        Self { fill: ' ', align: '\0', sign: '\0', alternate: false, zero: false, unsigned_zero: false, width: 0, group: '\0', fraction_group: '\0', precision: None, code: '\0' }
    }
}

impl Rule {
    fn sign_for(&self, negative: bool) -> String {
        if negative { "-".into() } else if matches!(self.sign, '+' | ' ') { self.sign.to_string() } else { String::new() }
    }

    fn pad(&self, head: &str, body: &str, default: char) -> String {
        let extra = self.width.saturating_sub(head.chars().count() + body.chars().count());
        let align = if self.align == '\0' { default } else { self.align };
        let (before, middle) = match align { '<' => (0, 0), '^' => (extra / 2, 0), '=' => (0, extra), _ => (extra, 0) };
        format!("{}{}{}{}{}", self.fill.to_string().repeat(before), head, self.fill.to_string().repeat(middle), body, self.fill.to_string().repeat(extra - before - middle))
    }

    fn group_digits(&mut self, body: &mut String, head: usize, span: usize) {
        let point = if matches!(self.code, 'b' | 'o' | 'x' | 'X' | 'd') { body.len() }
            else { body.find(['.', 'e', 'E', '%']).unwrap_or(body.len()) };
        let mut tail = body[point..].to_string();
        if self.fraction_group != '\0' && tail.starts_with('.') {
            let end = tail[1..].find(['e', 'E', '%']).map_or(tail.len(), |n| n + 1);
            let mut grouped = String::from('.');
            for (i, c) in tail[1..end].chars().enumerate() {
                if i > 0 && i % 3 == 0 { grouped.push(self.fraction_group); }
                grouped.push(c);
            }
            tail = grouped + &tail[end..];
        }
        let mut digits = body[..point].to_string();
        if self.group != '\0' {
            if self.align == '=' && self.fill == '0' {
                let mut wanted = digits.len();
                while head + wanted + wanted.saturating_sub(1) / span + tail.len() < self.width { wanted += 1; }
                digits = "0".repeat(wanted - digits.len()) + &digits;
            }
            let length = digits.len();
            let mut grouped = String::new();
            for (i, c) in digits.chars().enumerate() {
                if i > 0 && (length - i) % span == 0 { grouped.push(self.group); }
                grouped.push(c);
            }
            digits = grouped;
        }
        *body = digits + &tail;
    }
}

fn decimal(number: f64, rule: &Rule) -> String {
    let places = rule.precision.unwrap_or(6);
    let code = rule.code.to_ascii_lowercase();
    let mut body = match code {
        'f' | '%' => format!("{number:.places$}"),
        'e' => format!("{number:.places$e}"),
        '\0' if rule.precision.is_none() => {
            if number != 0.0 && !(0.0001..1e16).contains(&number) { format!("{number:e}") }
            else { number.to_string() }
        }
        _ => {
            let significant = places.max(1);
            let exponential = format!("{:.*e}", significant - 1, number);
            let (mantissa, exponent) = exponential.split_once('e').unwrap();
            let exponent: i32 = exponent.parse().unwrap();
            let threshold = significant as i32 - i32::from(code == '\0');
            if exponent < -4 || exponent >= threshold {
                let mantissa = if rule.alternate { mantissa } else { mantissa.trim_end_matches('0').trim_end_matches('.') };
                // A mantissa without a point has no trailing fractional noughts.
                let mantissa = if significant == 1 { exponential.split_once('e').unwrap().0 } else { mantissa };
                format!("{mantissa}e{exponent}")
            } else {
                let digits = (significant as i32 - 1 - exponent).max(0) as usize;
                let fixed = format!("{number:.digits$}");
                if !rule.alternate && fixed.contains('.') { fixed.trim_end_matches('0').trim_end_matches('.').to_string() } else { fixed }
            }
        }
    };
    if rule.alternate || code == '\0' && !body.contains('e') {
        let at = body.find('e').unwrap_or(body.len());
        if !body[..at].contains('.') {
            body.insert(at, '.');
            if code == '\0' && at == body.len() - 1 { body.push('0'); }
        }
    }
    if let Some((before, after)) = body.split_once('e') {
        let power: i32 = after.parse().unwrap();
        body = format!("{before}e{}{:02}", if power < 0 { '-' } else { '+' }, power.unsigned_abs());
    }
    body
}

pub fn names_fault(lang: &Lang, text: &str) -> bool {
    [
        &lang.fmt_text_format_zero_string,
        &lang.fmt_text_format_zero_integer,
        &lang.fmt_op_rem_format_infinity,
        &lang.fmt_op_rem_format_nan,
        &lang.fmt_text_format_invalid,
        &lang.fmt_text_format_unknown,
        &lang.fmt_text_format_unready,
        &lang.fmt_text_format_precision_integer,
        &lang.fmt_text_format_precision_missing,
        &lang.fmt_text_format_sign_string,
        &lang.fmt_text_format_alternate_string,
        &lang.fmt_text_format_align_string,
        &lang.fmt_text_format_sign_character,
        &lang.fmt_text_format_alternate_character,
        &lang.fmt_text_format_character,
        &lang.fmt_text_format_spec_type,
        &lang.fmt_text_format_numbered_auto,
        &lang.fmt_text_format_numbered_manual,
        &lang.fmt_text_format_index,
        &lang.fmt_text_format_key,
        &lang.fmt_text_format_brace_open,
        &lang.fmt_text_format_brace_close,
        &lang.fmt_text_format_conversion,
        &lang.fmt_text_format_recursion,
        &lang.fmt_op_rem_format_few,
        &lang.fmt_op_rem_format_many,
        &lang.fmt_op_rem_format_mapping,
        &lang.fmt_op_rem_format_number,
        &lang.fmt_op_rem_format_integer,
        &lang.fmt_op_rem_format_real,
        &lang.fmt_op_rem_format_character,
        &lang.fmt_op_rem_format_star,
        &lang.fmt_op_rem_format_incomplete,
        &lang.fmt_op_rem_format_code,
    ].iter().any(|parts| parts.first().map_or(false, |start| !start.is_empty() && text.starts_with(start)))
}
