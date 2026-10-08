//! Python binary encodings, following CPython v3.14.8 Modules/binascii.c.
use super::{Engine, Res, Value};

const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const HEX: &[u8] = b"0123456789abcdef";

fn digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode64(input: &[u8], strict: bool) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let (mut position, mut left, mut pads) = (0usize, 0u8, 0usize);
    for (index, byte) in input.iter().copied().enumerate() {
        if byte == b'=' {
            pads += 1;
            if position >= 2 && position + pads <= 4 { continue; }
            if !strict { continue; }
            if position == 1 { break; }
            return Err(if position == 0 && index == 0 { "Leading padding not allowed" } else { "Excess padding not allowed" }.into());
        }
        let Some(number) = BASE64.iter().position(|c| *c == byte).map(|n| n as u8) else {
            if strict { return Err("Only base64 data is allowed".into()); }
            continue;
        };
        if strict && pads != 0 {
            return Err(if position + pads == 4 { "Excess data after padding" } else { "Discontinuous padding not allowed" }.into());
        }
        pads = 0;
        match position {
            0 => left = number,
            1 => { out.push((left << 2) | (number >> 4)); left = number & 15; }
            2 => { out.push((left << 4) | (number >> 2)); left = number & 3; }
            _ => { out.push((left << 6) | number); left = 0; }
        }
        position = (position + 1) % 4;
    }
    if position == 1 { return Err(format!("Invalid base64-encoded string: number of data characters ({}) cannot be 1 more than a multiple of 4", out.len() / 3 * 4 + 1)); }
    if position != 0 && position + pads < 4 { return Err("Incorrect padding".into()); }
    Ok(out)
}

fn encode64(input: &[u8], newline: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len().div_ceil(3) * 4 + usize::from(newline));
    for triple in input.chunks(3) {
        let a = triple[0];
        let b = triple.get(1).copied().unwrap_or(0);
        let c = triple.get(2).copied().unwrap_or(0);
        out.extend_from_slice(&[BASE64[(a >> 2) as usize], BASE64[(((a & 3) << 4) | (b >> 4)) as usize],
            if triple.len() > 1 { BASE64[(((b & 15) << 2) | (c >> 6)) as usize] } else { b'=' },
            if triple.len() > 2 { BASE64[(c & 63) as usize] } else { b'=' }]);
    }
    if newline { out.push(b'\n'); }
    out
}

fn encodeuu(input: &[u8], backtick: bool) -> Result<Vec<u8>, String> {
    if input.len() > 45 { return Err("At most 45 bytes at once".into()); }
    let spelling = |n: u8| if n == 0 && backtick { b'`' } else { n + 32 };
    let mut out = vec![spelling(input.len() as u8)];
    for chunk in input.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16) | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8) | u32::from(chunk.get(2).copied().unwrap_or(0));
        for shift in [18, 12, 6, 0] { out.push(spelling(((bits >> shift) & 63) as u8)); }
    }
    out.push(b'\n');
    Ok(out)
}

fn decodeuu(input: &[u8]) -> Result<Vec<u8>, String> {
    let Some(first) = input.first() else { return Err("Missing length byte".into()); };
    let wanted = first.wrapping_sub(32) as usize & 63;
    let mut out = Vec::with_capacity(wanted);
    let (mut at, mut bits, mut buffer) = (1usize, 0usize, 0u32);
    while out.len() < wanted {
        let number = match input.get(at).copied() {
            None | Some(b'\r' | b'\n') => 0,
            Some(c @ 32..=96) => (c - 32) & 63,
            _ => return Err("Illegal char".into()),
        };
        at += 1;
        buffer = (buffer << 6) | u32::from(number);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if input.get(at..).unwrap_or_default().iter().any(|c| !matches!(c, b' ' | b'`' | b'\r' | b'\n')) { return Err("Trailing garbage".into()); }
    Ok(out)
}

fn decodeqp(input: &[u8], header: bool) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let byte = input[index];
        index += 1;
        if byte != b'=' { output.push(if header && byte == b'_' { b' ' } else { byte }); continue; }
        let Some(next) = input.get(index).copied() else { break; };
        match next {
            b'\n' => index += 1,
            b'\r' => { while index < input.len() && input[index] != b'\n' { index += 1; } index += usize::from(index < input.len()); }
            b'=' => { output.push(b'='); index += 1; }
            _ => {
                if let (Some(a), Some(b)) = (digit(next), input.get(index + 1).copied().and_then(digit)) {
                    output.push((a << 4) | b); index += 2;
                } else { output.push(b'='); }
            }
        }
    }
    output
}

fn encodeqp(input: &[u8], tabs: bool, text: bool, header: bool) -> Vec<u8> {
    let crlf = input.iter().position(|c| *c == b'\n').is_some_and(|i| i > 0 && input[i-1] == b'\r');
    let newline: &[u8] = if crlf { b"\r\n" } else { b"\n" };
    let upper = b"0123456789ABCDEF";
    let mut out = Vec::new();
    let (mut at, mut column) = (0, 0);
    while at < input.len() {
        let c = input[at];
        let next = input.get(at + 1).copied();
        let quote = c > 126 || c == b'=' || (header && c == b'_')
            || (c == b'.' && column == 0 && matches!(next, None | Some(b'\n' | b'\r' | 0)))
            || (!text && matches!(c, b'\r' | b'\n'))
            || (matches!(c, b'\t' | b' ') && next.is_none())
            || (c < 33 && !matches!(c, b'\r' | b'\n') && (tabs || !matches!(c, b'\t' | b' ')));
        if quote {
            if column + 3 >= 76 { out.push(b'='); out.extend_from_slice(newline); column = 0; }
            out.extend_from_slice(&[b'=', upper[(c >> 4) as usize], upper[(c & 15) as usize]]);
            column += 3;
            at += 1;
        } else if text && (c == b'\n' || (c == b'\r' && next == Some(b'\n'))) {
            column = 0;
            if out.last().is_some_and(|last| matches!(last, b' ' | b'\t')) {
                let tail = out.pop().unwrap();
                out.extend_from_slice(&[b'=', upper[(tail >> 4) as usize], upper[(tail & 15) as usize]]);
            }
            out.extend_from_slice(newline);
            at += if c == b'\r' { 2 } else { 1 };
        } else {
            if next.is_some() && next != Some(b'\n') && column + 1 >= 76 { out.push(b'='); out.extend_from_slice(newline); column = 0; }
            column += 1;
            out.push(if header && c == b' ' { b'_' } else { c });
            at += 1;
        }
    }
    out
}

impl Engine<'_> {
    pub(super) fn binary_buffer(&mut self, argument: &Value) -> Res<Vec<u8>> {
        let held = argument.contents();
        if let Value::Bytes(row, ..) = &held { return Ok(row.borrow().clone()); }
        if let Value::Object(object) = &held {
            for (module_name, class_name, method_name) in [("builtins", "memoryview", "__bytes__"), ("array", "array", "tobytes")] {
                let Some(Value::Object(module)) = self.modules.get(module_name) else { continue; };
                let class = module.fields.borrow().iter().find(|(name, _)| name == class_name).map(|(_, value)| value.contents());
                let Some(Value::Class(class)) = class else { continue; };
                let actual = object.class_now();
                if !std::rc::Rc::ptr_eq(&actual, &class) && !actual.lineage.borrow().iter().any(|base| std::rc::Rc::ptr_eq(base, &class)) { continue; }
                if class_name == "memoryview" {
                    match self.class_get(held.clone(), "c_contiguous", false) {
                        Ok(value) if !self.truth(&value) => return Err("BufferError: memoryview: underlying buffer is not C-contiguous".into()),
                        Ok(_) => {},
                        Err(fault) => { self.carried = Some(fault); return Err(String::new()); }
                    }
                }
                let method = self.class_value(&actual, method_name).ok_or_else(|| "TypeError: buffer has no byte export".to_string())?;
                match self.class_apply(method, vec![held.clone()]) {
                    Ok(Value::Bytes(row, ..)) => return Ok(row.borrow().clone()),
                    Ok(_) => return Err("TypeError: buffer export is not bytes".into()),
                    Err(fault) => { self.carried = Some(fault); return Err(String::new()); }
                }
            }
        }
        Err(format!("TypeError: a bytes-like object is required, not '{}'", held.core_kind()))
    }

    pub(super) fn binary_ascii(&self, values: &[Value]) -> Res<Value> {
        let Value::Text(operation) = values[0].contents() else { return Err("TypeError: binary operation must be a string".into()); };
        let Value::Bytes(bytes, ..) = values[1].contents() else { return Err("TypeError: binary operation requires bytes".into()); };
        let input = bytes.borrow();
        let flag = |i: usize| values.get(i).is_some_and(Value::is_true);
        let number = |i: usize| match values.get(i).map(Value::contents) { Some(Value::Small(n)) => n, _ => 0 };
        if operation.as_ref() == "crc32" || operation.as_ref() == "hqx" {
            let mut crc = number(2) as u32;
            if operation.as_ref() == "crc32" {
                crc = !crc;
                for byte in input.iter() {
                    crc ^= u32::from(*byte);
                    for _ in 0..8 { crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 }; }
                }
                crc = !crc;
            } else {
                for byte in input.iter() {
                    crc ^= u32::from(*byte) << 8;
                    for _ in 0..8 { crc = ((crc << 1) ^ if crc & 0x8000 != 0 { 0x1021 } else { 0 }) & 65535; }
                }
            }
            return Ok(Value::Small(i64::from(crc)));
        }
        let answer = match operation.as_ref() {
            "decode64" => decode64(&input, flag(2)),
            "encode64" => Ok(encode64(&input, flag(2))),
            "decodeuu" => decodeuu(&input),
            "encodeuu" => encodeuu(&input, flag(2)),
            "decodeqp" => Ok(decodeqp(&input, flag(2))),
            "encodeqp" => Ok(encodeqp(&input, flag(2), flag(3), flag(4))),
            "unhex" => {
                if input.len() % 2 != 0 { Err("Odd-length string".into()) }
                else { input.chunks_exact(2).map(|pair| digit(pair[0]).zip(digit(pair[1])).map(|(a, b)| (a << 4) | b).ok_or_else(|| "Non-hexadecimal digit found".to_string())).collect() }
            }
            "hex" => {
                let separator = match values.get(2).map(Value::contents) { Some(Value::Bytes(row, ..)) => row.borrow().first().copied(), _ => None };
                let group = number(3);
                let width = group.unsigned_abs() as usize;
                let mut encoded = Vec::new();
                for (i, byte) in input.iter().copied().enumerate() {
                    if i > 0 && width != 0 && (if group > 0 { input.len() - i } else { i }) % width == 0 {
                        if let Some(sep) = separator { encoded.push(sep); }
                    }
                    encoded.extend_from_slice(&[HEX[(byte >> 4) as usize], HEX[(byte & 15) as usize]]);
                }
                Ok(encoded)
            }
            _ => return Err("ValueError: unknown binary operation".into()),
        };
        Ok(match answer { Ok(row) => self.byte_make(row, false), Err(message) => Value::text(&message) })
    }
}
