//! CPython v3.14.8 binascii operations for the Python primitive table.
use super::{Machine, Value};

type Conversion = Result<Vec<u8>, String>;

fn hex_value(c: u8) -> Option<u8> {
    let lower = c.to_ascii_lowercase();
    if c.is_ascii_digit() { Some(c - 48) }
    else if (b'a'..=b'f').contains(&lower) { Some(lower - 87) }
    else { None }
}
fn sextet(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62), b'/' => Some(63), _ => None,
    }
}

struct Base64Read {
    result: Vec<u8>,
    stage: usize,
    padding: usize,
    carry: u8,
}
impl Base64Read {
    fn read(source: &[u8], checked: bool) -> Conversion {
        let mut state = Self { result: Vec::new(), stage: 0, padding: 0, carry: 0 };
        for cursor in 0..source.len() {
            let character = source[cursor];
            if character == b'=' {
                state.padding += 1;
                if state.stage >= 2 && state.stage + state.padding < 5 { continue; }
                if checked {
                    if state.stage == 1 { break; }
                    let refusal = if cursor == 0 && state.stage == 0 { "Leading padding not allowed" } else { "Excess padding not allowed" };
                    return Err(refusal.to_owned());
                }
                continue;
            }
            let Some(decoded) = sextet(character) else {
                if checked { return Err(String::from("Only base64 data is allowed")); }
                continue;
            };
            if checked && state.padding > 0 {
                let refusal = if state.stage + state.padding != 4 { "Discontinuous padding not allowed" } else { "Excess data after padding" };
                return Err(refusal.to_owned());
            }
            state.padding = 0;
            if state.stage > 0 {
                let shift = 6 - 2 * state.stage;
                state.result.push((state.carry << (2 * state.stage)) | (decoded >> shift));
            }
            state.carry = decoded & ((1 << (6 - 2 * state.stage)) - 1);
            state.stage = (state.stage + 1) & 3;
        }
        match state.stage {
            1 => Err(format!("Invalid base64-encoded string: number of data characters ({}) cannot be 1 more than a multiple of 4", 1 + 4 * (state.result.len() / 3))),
            2 | 3 if state.stage + state.padding < 4 => Err(String::from("Incorrect padding")),
            _ => Ok(state.result),
        }
    }
}

fn base64_write(source: &[u8], line_end: bool) -> Vec<u8> {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();
    let mut position = 0;
    while position < source.len() {
        let count = (source.len() - position).min(3);
        let mut word = 0u32;
        for offset in 0..3 { word = (word << 8) | u32::from(source.get(position + offset).copied().unwrap_or(0)); }
        for slot in 0..4 {
            result.push(if slot > count { b'=' } else { alphabet[((word >> (18 - slot * 6)) & 63) as usize] });
        }
        position += count;
    }
    if line_end { result.extend(b"\n"); }
    result
}

fn uu_write(source: &[u8], ticks: bool) -> Conversion {
    if source.len() >= 46 { return Err(String::from("At most 45 bytes at once")); }
    let mut result = Vec::new();
    let character = |value: usize| if ticks && value == 0 { 96 } else { 32 + value as u8 };
    result.push(character(source.len()));
    let mut shift_register = 0usize;
    let mut available = 0;
    for byte in source.iter().copied().chain(std::iter::repeat_n(0, (3 - source.len() % 3) % 3)) {
        shift_register = (shift_register << 8) | usize::from(byte);
        available += 8;
        while available >= 6 {
            available -= 6;
            result.push(character((shift_register >> available) & 63));
        }
        shift_register &= (1 << available) - 1;
    }
    result.push(10);
    Ok(result)
}

fn uu_read(source: &[u8]) -> Conversion {
    if source.is_empty() { return Err(String::from("Missing length byte")); }
    let size = usize::from(source[0].wrapping_sub(b' ') & 63);
    let groups = (size * 8).div_ceil(6);
    let mut numbers = Vec::with_capacity(groups);
    for offset in 1..=groups {
        let byte = source.get(offset).copied();
        let six = match byte {
            Some(b'\n' | b'\r') | None => 0,
            Some(c) if (32..=96).contains(&c) => c.wrapping_sub(32) & 63,
            _ => return Err(String::from("Illegal char")),
        };
        numbers.push(six);
    }
    if source.iter().skip(groups + 1).any(|c| ![32, 96, 10, 13].contains(c)) { return Err(String::from("Trailing garbage")); }
    let mut result = Vec::with_capacity(size);
    for offset in 0..size {
        let bit = offset * 8;
        let index = bit / 6;
        let shift = bit % 6;
        let pair = (u16::from(numbers[index]) << 6) | u16::from(numbers[index+1]);
        result.push((pair >> (4 - shift)) as u8);
    }
    Ok(result)
}

fn qp_read(source: &[u8], mail_header: bool) -> Vec<u8> {
    let mut tail = source;
    let mut result = Vec::new();
    while let Some((&head, rest)) = tail.split_first() {
        tail = rest;
        if head == b'=' {
            match tail.first() {
                None => break,
                Some(b'\n') => tail = &tail[1..],
                Some(b'\r') => tail = tail.iter().position(|c| *c == b'\n').map_or(&[], |n| &tail[n+1..]),
                Some(b'=') => { result.push(61); tail = &tail[1..]; }
                Some(c) => {
                    let parsed = hex_value(*c).zip(tail.get(1).copied().and_then(hex_value));
                    if let Some((high, low)) = parsed { result.push(high * 16 + low); tail = &tail[2..]; }
                    else { result.push(61); }
                }
            }
        } else { result.push(if mail_header && head == 95 { 32 } else { head }); }
    }
    result
}

struct QpWrite {
    bytes: Vec<u8>,
    line: usize,
    crlf: bool,
}
impl QpWrite {
    fn newline(&mut self) {
        if self.crlf { self.bytes.push(13); }
        self.bytes.push(10);
        self.line = 0;
    }
    fn escape(&mut self, byte: u8) {
        let digits = b"0123456789ABCDEF";
        self.bytes.push(61);
        self.bytes.push(digits[usize::from(byte / 16)]);
        self.bytes.push(digits[usize::from(byte % 16)]);
    }
    fn write(source: &[u8], quote_space: bool, keep_lines: bool, mail_header: bool) -> Vec<u8> {
        let crlf = source.iter().position(|c| *c == 10).is_some_and(|p| p != 0 && source[p-1] == 13);
        let mut output = Self { bytes: Vec::new(), line: 0, crlf };
        let mut cursor = 0;
        while let Some(&byte) = source.get(cursor) {
            let after = source.get(cursor + 1).copied();
            let whitespace = byte == 32 || byte == 9;
            let endline = byte == 10 || byte == 13;
            let dot = byte == 46 && output.line == 0 && after.is_none_or(|c| [0, 10, 13].contains(&c));
            let control = byte < 33 && !endline && (quote_space || !whitespace);
            let escaped = byte >= 127 || byte == 61 || (mail_header && byte == 95) || dot || (!keep_lines && endline) || (whitespace && after.is_none()) || control;
            if escaped {
                if output.line >= 73 { output.bytes.push(61); output.newline(); }
                output.escape(byte);
                output.line += 3;
                cursor += 1;
                continue;
            }
            if keep_lines && (byte == 10 || (byte == 13 && after == Some(10))) {
                if output.bytes.last().is_some_and(|c| [9, 32].contains(c)) {
                    let last = output.bytes.pop().unwrap();
                    output.escape(last);
                }
                output.newline();
                cursor += if byte == 10 { 1 } else { 2 };
                continue;
            }
            if output.line >= 75 && after.is_some() && after != Some(10) { output.bytes.push(61); output.newline(); }
            output.bytes.push(if mail_header && byte == 32 { 95 } else { byte });
            output.line += 1;
            cursor += 1;
        }
        output.bytes
    }
}

fn checksum(source: &[u8], initial: u32, hqx: bool) -> u32 {
    let mut state = if hqx { initial } else { initial ^ u32::MAX };
    for octet in source {
        if hqx {
            state ^= u32::from(*octet) * 256;
            for _bit in 0..8 {
                let polynomial = if state & 32768 == 0 { 0 } else { 4129 };
                state = ((state * 2) ^ polynomial) & 65535;
            }
        } else {
            state ^= u32::from(*octet);
            for _bit in 0..8 {
                let polynomial = 0xedb88320u32.wrapping_mul(state & 1);
                state = (state / 2) ^ polynomial;
            }
        }
    }
    if hqx { state } else { state ^ u32::MAX }
}

impl Machine<'_> {
    pub(super) fn octet_buffer(&mut self, supplied: &Value) -> Result<Vec<u8>, String> {
        let value = supplied.settled();
        match &value {
            Value::Octets { cell, .. } => return Ok(cell.borrow().to_vec()),
            Value::Thing(instance) => {
                let candidates = [("array", "array", "tobytes"), ("builtins", "memoryview", "__bytes__")];
                for (owner, member, export) in candidates {
                    let definition = self.imported.get(owner).and_then(|module| {
                        let Value::Thing(namespace) = module else { return None; };
                        namespace.holds.borrow().iter().find(|entry| entry.0 == member).map(|entry| entry.1.settled())
                    });
                    let Some(Value::Blueprint(definition)) = definition else { continue; };
                    let blueprint = instance.blueprint();
                    let recognized = std::rc::Rc::ptr_eq(&blueprint, &definition) || blueprint.ancestry.borrow().iter().any(|base| std::rc::Rc::ptr_eq(base, &definition));
                    if !recognized { continue; }
                    if member == "memoryview" {
                        let contiguous = match self.read_class_member(value.clone(), "c_contiguous", false) {
                            Ok(flag) => flag.is_true(),
                            Err(escape) => { self.got_away = Some(escape); return Err(self.bad_answer()); }
                        };
                        if !contiguous { return Err(String::from("BufferError: memoryview: underlying buffer is not C-contiguous")); }
                    }
                    let method = self.inherited_entry(&blueprint, export).ok_or_else(|| String::from("TypeError: missing buffer export"))?;
                    let exported = match self.apply_held(method, vec![value.clone()]) {
                        Ok(answer) => answer,
                        Err(escape) => { self.got_away = Some(escape); return Err(self.bad_answer()); }
                    };
                    if let Value::Octets { cell, .. } = exported.settled() { return Ok(cell.borrow().to_vec()); }
                    return Err(String::from("TypeError: invalid buffer export"));
                }
            }
            _ => {},
        }
        Err(format!("TypeError: a bytes-like object is required, not '{}'", value.kind_word()))
    }

    pub(super) fn ascii_binary_work(&self, arguments: &[Value]) -> Result<Value, String> {
        let task = arguments[0].settled().bare();
        let Value::Octets { cell, .. } = arguments[1].settled() else { return Err(String::from("TypeError: expected binary data")); };
        let content = cell.borrow();
        let option = |n| arguments.get(n).is_some_and(Value::is_true);
        let integer = |n| match arguments.get(n).map(Value::settled) { Some(Value::Small(value)) => value, _ => 0 };
        if task == "hqx" || task == "crc32" { return Ok(Value::Small(i64::from(checksum(&content, integer(2) as u32, task == "hqx")))); }
        let converted: Conversion = match task.as_str() {
            "decode64" => Base64Read::read(&content, option(2)),
            "encode64" => Ok(base64_write(&content, option(2))),
            "decodeuu" => uu_read(&content),
            "encodeuu" => uu_write(&content, option(2)),
            "decodeqp" => Ok(qp_read(&content, option(2))),
            "encodeqp" => Ok(QpWrite::write(&content, option(2), option(3), option(4))),
            "hex" => {
                let separator = arguments.get(2).and_then(|arg| match arg.settled() { Value::Octets { cell, .. } => cell.borrow().first().copied(), _ => None });
                let stride = integer(3);
                let mut result = Vec::new();
                let alphabet = b"0123456789abcdef";
                let length = content.len();
                for offset in 0..length {
                    let distance = if stride < 0 { offset } else { length - offset };
                    if let Some(delimiter) = separator {
                        if offset != 0 && stride != 0 && distance % stride.unsigned_abs() as usize == 0 { result.push(delimiter); }
                    }
                    result.push(alphabet[usize::from(content[offset] / 16)]);
                    result.push(alphabet[usize::from(content[offset] % 16)]);
                }
                Ok(result)
            }
            "unhex" => {
                let mut result = Vec::new();
                if content.len() & 1 == 1 { Err(String::from("Odd-length string")) }
                else {
                    let mut failed = false;
                    for offset in (0..content.len()).step_by(2) {
                        if let Some((high, low)) = hex_value(content[offset]).zip(hex_value(content[offset+1])) { result.push(high * 16 + low); }
                        else { failed = true; break; }
                    }
                    if failed { Err(String::from("Non-hexadecimal digit found")) } else { Ok(result) }
                }
            }
            _ => return Err(String::from("ValueError: invalid binary conversion")),
        };
        match converted { Err(words) => Ok(Value::text(&words)), Ok(bytes) => Ok(self.octets(bytes, false)) }
    }
}
