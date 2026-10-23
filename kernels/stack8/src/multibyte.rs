// CJK mapping lookups; data provenance and layout are in langs/lib_python/data/multibyte/README.md.
use crate::value::Value;
use num_traits::ToPrimitive;
use std::{cell::RefCell, rc::Rc};
fn word(data: &[u8], at: usize) -> u32 { u32::from_le_bytes(data[at..at+4].try_into().unwrap()) }
fn wide(data: &[u8], at: usize) -> u64 { u64::from_le_bytes(data[at..at+8].try_into().unwrap()) }
// Build-owned compressed tables are expanded once, retaining their original layout.
fn expanded_mapping(packed: &'static [u8], cached: &'static std::sync::OnceLock<Option<Vec<u8>>>) -> Option<&'static [u8]> {
    if !packed.starts_with(b"\xfd7zXZ\0") { return Some(packed); }
    cached.get_or_init(|| {
        let (bytes, used) = lumen_lzma::decode_limited(packed, 1, 64 * 1024 * 1024, 64 * 1024 * 1024).ok()?;
        (used == packed.len()).then_some(bytes)
    }).as_deref()
}
static JP2_MAPPING: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
static GB18030_MAPPING: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
static GBK_MAPPING: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
fn mappings(name: &str) -> Option<&'static [u8]> {
    match name {
        "big5" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/big5.bin")),
        "big5hkscs" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/big5hkscs.bin")),
        "cp932" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/cp932.bin")),
        "cp949" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/cp949.bin")),
        "cp950" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/cp950.bin")),
        "euc_jp" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_jp.bin")),
        "euc_jis_2004" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_jis_2004.bin")),
        "euc_jisx0213" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_jisx0213.bin")),
        "euc_kr" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_kr.bin")),
        "gb2312" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/gb2312.bin")),
        "gbk" => expanded_mapping(include_bytes!("../../../langs/lib_python/data/multibyte/gbk.bin"), &GBK_MAPPING),
        "gb18030" => expanded_mapping(include_bytes!("../../../langs/lib_python/data/multibyte/gb18030.bin"), &GB18030_MAPPING),
        "johab" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/johab.bin")),
        "shift_jis" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis.bin")),
        "shift_jis_2004" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis_2004.bin")),
        "shift_jisx0213" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jisx0213.bin")),
        "iso2022_jp" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp.bin")),
        "iso2022_jp_1" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_1.bin")),
        "iso2022_jp_2" => expanded_mapping(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_2.bin"), &JP2_MAPPING),
        "iso2022_jp_2004" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_2004.bin")),
        "iso2022_jp_3" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_3.bin")),
        "iso2022_jp_ext" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_ext.bin")),
        "iso2022_kr" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_kr.bin")),
        "hz" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/hz.bin")),
        _ => None,
    }
}
fn locate(data: &[u8], start: usize, count: usize, key: (u32, u64), decoding: bool) -> Option<usize> {
    let mut lo = 0; let mut hi = count;
    while lo < hi {
        let mid = (lo + hi) / 2;
        let at = start + mid * 20;
        let found = if decoding { (word(data, at), wide(data, at+4)) } else { (word(data, at), u64::from(word(data, at+4))) };
        match found.cmp(&key) { std::cmp::Ordering::Less => lo = mid+1, std::cmp::Ordering::Greater => hi = mid, std::cmp::Ordering::Equal => return Some(at) }
    }
    None
}
fn scalar(value: &Value) -> Result<u32, String> { value.as_big()?.to_u32().ok_or("ValueError: invalid Unicode point".into()) }
fn output(bytes: Vec<u8>) -> Value { Value::Bytes(Rc::new(RefCell::new(bytes)), false, Rc::from("b")) }
/// The bytes of one character, or of a character and the one after it
/// where the table holds the pair; GB18030's arithmetic ranges stand in
/// for the rest of its single characters.
fn encode_point(data: &[u8], name: &str, encoding_count: usize, ranges: usize, first: u32, second: u32) -> Option<Vec<u8>> {
    if let Some(at) = locate(data, 8, encoding_count, (first, second.into()), false) {
        let n = word(data, at+8) as usize;
        return Some(wide(data, at+12).to_be_bytes()[8-n..].to_vec());
    }
    if second == 0 && name == "gb18030" {
        for row in 0..word(data, ranges) as usize {
            let at = ranges + 4 + row * 16;
            if (word(data, at)..=word(data, at+4)).contains(&first) {
                let pointer = word(data, at+8) + first - word(data, at);
                return Some(vec![(pointer/12600+129) as u8, (pointer/1260%10+48) as u8, (pointer/10%126+129) as u8, (pointer%10+48) as u8]);
            }
        }
    }
    None
}
/// Whether a character begins any pair the table knows.
fn pair_lead(data: &[u8], encoding_count: usize, point: u32) -> bool {
    let mut lo = 0; let mut hi = encoding_count;
    while lo < hi {
        let mid = (lo+hi)/2; let at = 8+mid*20;
        if (word(data,at), word(data,at+4)) < (point,1) { lo=mid+1; } else { hi=mid; }
    }
    lo < encoding_count && word(data,8+lo*20) == point && word(data,12+lo*20) != 0
}
/// The character or characters a whole unit of bytes stands for.
fn decode_unit(data: &[u8], name: &str, decoding_start: usize, decoding_count: usize, ranges: usize, raw: &[u8]) -> Option<Vec<u32>> {
    let length = raw.len();
    if length > 8 { return None; }
    let mut code = 0u64; for byte in raw.iter() { code = (code << 8) | u64::from(*byte); }
    if let Some(at) = locate(data, decoding_start, decoding_count, (length as u32, code), true) {
        let mut points = vec![word(data, at+12)]; let second = word(data, at+16);
        if second != 0 { points.push(second); }
        return Some(points);
    }
    if name == "gb18030" && length == 4 && (129..=254).contains(&raw[0]) && (48..=57).contains(&raw[1]) && (129..=254).contains(&raw[2]) && (48..=57).contains(&raw[3]) {
        let pointer = ((u32::from(raw[0])-129)*10+u32::from(raw[1])-48)*1260+(u32::from(raw[2])-129)*10+u32::from(raw[3])-48;
        for row in 0..word(data, ranges) as usize {
            let at = ranges + 4 + row * 16;
            if (word(data, at+8)..=word(data, at+12)).contains(&pointer) { return Some(vec![word(data, at)+pointer-word(data, at+8)]); }
        }
    }
    None
}
/// Whether some longer unit begins with these bytes.
fn longer_exists(data: &[u8], name: &str, decoding_start: usize, decoding_count: usize, raw: &[u8]) -> bool {
    let length = raw.len();
    if length > 8 { return false; }
    let mut code = 0u64; for byte in raw.iter() { code = (code << 8) | u64::from(*byte); }
    for width in length+1..=8 {
        let shift = (width-length)*8;
        if shift >= 64 { continue; }
        let low = code << shift; let high = low | if shift == 0 {0} else {(1u64 << shift)-1};
        let mut l = 0; let mut r = decoding_count;
        while l < r {
            let m = (l+r)/2; let at = decoding_start + m*20;
            if (word(data,at),wide(data,at+4)) < (width as u32, low) { l=m+1; } else { r=m; }
        }
        if l < decoding_count {
            let at = decoding_start+l*20;
            if word(data,at) == width as u32 && wide(data,at+4) <= high { return true; }
        }
    }
    if name == "gb18030" && length < 4 {
        let valid = !raw.is_empty() && (129..=254).contains(&raw[0]) && (length < 2 || (48..=57).contains(&raw[1])) && (length < 3 || (129..=254).contains(&raw[2]));
        if valid { return true; }
    }
    false
}
// Decode consecutive fixed-width units under an already selected character set.
fn decode_designated(data: &[u8], name: &str, input: &[u8], prefix: &[u8], width: usize, start: usize) -> (Vec<u32>, usize) {
    let base = 8 + word(data, 0) as usize * 20;
    let count = word(data, 4) as usize;
    let mut assembled = [0u8; 8];
    assembled[..prefix.len()].copy_from_slice(prefix);
    let mut output = Vec::new();
    let mut consumed = start;
    for part in input.get(start..).unwrap_or_default().chunks_exact(width) {
        if part[0] <= 31 { break; }
        let length = prefix.len() + width;
        assembled[prefix.len()..length].copy_from_slice(part);
        match decode_unit(data, name, base, count, base + count * 20, &assembled[..length]) {
            Some(characters) => output.extend(characters),
            None => break,
        }
        consumed += width;
    }
    (output, consumed)
}
// Convert an ordinary ISO feed transactionally; complex escape/error cases fall back.
fn iso_feed(data: &[u8], name: &str, input: &[u8], mut state: [u8; 8], final_input: bool) -> Option<(Vec<u32>, usize, [u8; 8])> {
    let mut text = Vec::new();
    let mut cursor = 0;
    let rows = word(data, 4) as usize;
    let start = 8 + word(data, 0) as usize * 20;
    while let Some(&lead) = input.get(cursor) {
        match lead {
            27 => {
                let escapes: &[(&[u8], usize, u8)] = &[(b"\x1b(B",0,66),(b"\x1b(J",0,74),(b"\x1b(I",0,73),
                    (b"\x1b$@",0,192),(b"\x1b$B",0,194),(b"\x1b$(D",0,196),
                    (b"\x1b$(O",0,207),(b"\x1b$(P",0,208),(b"\x1b$(Q",0,209)];
                if let Some((wire, field, value)) = escapes.iter().find(|(wire, _, _)| input[cursor..].starts_with(wire)) {
                    state[*field] = *value; cursor += wire.len(); continue;
                }
                if name == "iso2022_kr" && input[cursor..].starts_with(b"\x1b$)C") {
                    state[1] = 195; cursor += 4; continue;
                }
                return None;
            }
            14 | 15 if name == "iso2022_kr" => { state[4] = (state[4] & !1) | u8::from(lead == 14); cursor += 1; continue; }
            0..=31 => { if lead == 10 { state[4] &= !1; } text.push(lead.into()); cursor += 1; continue; }
            _ => {}
        }
        let korean = name == "iso2022_kr" && state[4] & 1 == 1;
        let designation = state[usize::from(korean)];
        let width = 1 + usize::from(designation >= 128);
        let prefix = match (designation, korean) {
            (66, _) => Vec::new(),
            (_, true) => vec![27,36,41,67,14],
            (192..=194, false) => vec![27,36,designation & 127],
            (128..=255, false) => vec![27,36,40,designation & 127],
            (_, false) => vec![27,40,designation],
        };
        let (segment, next) = decode_designated(data, name, input, &prefix, width, cursor);
        if next != cursor { text.extend(segment); cursor = next; continue; }
        let mut unfinished = prefix;
        unfinished.extend_from_slice(&input[cursor..]);
        if !final_input && input.len() - cursor < width && longer_exists(data, name, start, rows, &unfinished) { break; }
        return None;
    }
    Some((text, cursor, state))
}
pub fn call(args: &[Value]) -> Result<Value, String> {
    if args.len() != 4 { return Err("TypeError: multibyte mapping expects four arguments".into()); }
    let Some(Value::Text(name)) = args.get(1).map(Value::contents) else { return Err("TypeError: codec name must be str".into()); };
    let opcode = scalar(&args[0])?;
    if (4..=6).contains(&opcode) {
        let database: &[u8] = match name.as_ref() {
            "ucd16" => include_bytes!("../../../langs/lib_python/data/unicode/ucd16.bin"),
            "ucd32" => include_bytes!("../../../langs/lib_python/data/unicode/ucd32.bin"),
            _ => return Err("LookupError: unknown Unicode database".into()),
        };
        return Ok(unicode_lookup(database,opcode,scalar(&args[2])?,scalar(&args[3])?));
    }
    let data = mappings(&name).ok_or("LookupError: unknown multibyte codec")?;
    let encoding_count = word(data, 0) as usize;
    let decoding_count = word(data, 4) as usize;
    let decoding_start = 8 + encoding_count * 20;
    let ranges = decoding_start + decoding_count * 20;
    let op = scalar(&args[0])?;
    if op == 3 { return Ok(Value::Flag(pair_lead(data, encoding_count, scalar(&args[2])?))); }
    if op == 0 {
        let first = scalar(&args[2])?; let second = scalar(&args[3])?;
        return Ok(encode_point(data, &name, encoding_count, ranges, first, second).map_or(Value::Null, output));
    }
    if op == 10 {
        let codes = args[2].contents().text_codes().ok_or("TypeError: mapping input must be str")?;
        let Value::Tuple(spec) = args[3].contents() else { return Err("TypeError: encoding designation requires two items".into()); };
        if spec.len() != 2 { return Err("TypeError: encoding designation requires two items".into()); }
        let Value::Bytes(prefix, ..) = spec[0].contents() else { return Err("TypeError: designation prefix must be bytes".into()); };
        let prefix = prefix.borrow();
        if prefix.is_empty() { return Err("ValueError: empty designation".into()); }
        let packed = scalar(&spec[1])?;
        let mut pos = (packed >> 1) as usize;
        let mut output_bytes = Vec::new();
        while pos < codes.len() {
            let point = codes[pos];
            if pos + 1 == codes.len() && packed & 1 == 0 && pair_lead(data, encoding_count, point) { break; }
            let mut length = 1;
            let paired = codes.get(pos + 1).filter(|&&next| next != 0)
                .and_then(|&next| encode_point(data, &name, encoding_count, ranges, point, next));
            let encoded = match paired {
                Some(wire) => { length = 2; Some(wire) }
                None => encode_point(data, &name, encoding_count, ranges, point, 0),
            };
            match encoded {
                Some(wire) if wire.starts_with(&prefix) => output_bytes.extend_from_slice(&wire[prefix.len()..]),
                _ => break,
            }
            pos += length;
        }
        return Ok(Value::tuple(vec![output(output_bytes), Value::Small(pos as i64)]));
    }
    if op == 7 {
        // Whole text written by a stateless codec: the row of bytes, where
        // writing stopped, and why: 0 the end, 1 a character the codec has
        // no bytes for, 2 a last character that may yet begin a pair.
        let codes = args[2].contents().text_codes().ok_or("TypeError: mapping input must be str")?;
        let packed = scalar(&args[3])?;
        let (last_input, mut pos) = (packed & 1 == 1, (packed >> 1) as usize);
        let mut row = Vec::new();
        let mut stop = 0;
        while pos < codes.len() {
            let first = codes[pos];
            if pos + 1 == codes.len() && !last_input && pair_lead(data, encoding_count, first) { stop = 2; break; }
            // Zero denotes a single-point lookup, not a consumable companion.
            if pos + 1 < codes.len() && codes[pos+1] != 0 {
                if let Some(wire) = encode_point(data, &name, encoding_count, ranges, first, codes[pos+1]) { row.extend(wire); pos += 2; continue; }
            }
            match encode_point(data, &name, encoding_count, ranges, first, 0) {
                Some(wire) => { row.extend(wire); pos += 1; }
                None => { stop = 1; break; }
            }
        }
        return Ok(Value::tuple(vec![output(row), Value::Small(pos as i64), Value::Small(stop)]));
    }
    let Value::Bytes(raw, ..) = args[2].contents() else { return Err("TypeError: mapping input must be bytes".into()); };
    let raw = raw.borrow();
    if op == 11 {
        let Value::Tuple(spec) = args[3].contents() else { return Err("TypeError: expected ISO state and completion flag".into()); };
        if spec.len() != 2 { return Err("TypeError: expected ISO state and completion flag".into()); }
        let Value::Array(fields) = spec[0].contents() else { return Err("TypeError: ISO state must be a list".into()); };
        if fields.len() != 8 { return Err("ValueError: ISO state requires eight bytes".into()); }
        let mut state = [0u8; 8];
        for (index, field) in fields.iter().enumerate() { state[index] = u8::try_from(scalar(field)?).map_err(|_| "ValueError: ISO state byte out of range")?; }
        return Ok(match iso_feed(data, &name, &raw, state, scalar(&spec[1])? != 0) {
            Some((text, end, shift)) => Value::tuple(vec![Value::from_codes(text), Value::Small(end as i64), Value::array(shift.iter().map(|&b| Value::Small(i64::from(b))).collect())]),
            None => Value::Null,
        });
    }
    if op == 9 {
        let Value::Tuple(spec) = args[3].contents() else { return Err("TypeError: expected a designation triple".into()); };
        if spec.len() != 3 { return Err("TypeError: expected a designation triple".into()); }
        let Value::Bytes(prefix, ..) = spec[0].contents() else { return Err("TypeError: designation prefix must be bytes".into()); };
        let prefix = prefix.borrow();
        let width = scalar(&spec[1])? as usize;
        if width == 0 || width > 2 || prefix.len() > 8 - width { return Err("ValueError: invalid designation width".into()); }
        let (points, end) = decode_designated(data, &name, &raw, &prefix, width, scalar(&spec[2])? as usize);
        return Ok(Value::tuple(vec![Value::from_codes(points), Value::Small(end as i64)]));
    }
    if op == 8 {
        // Whole bytes read by a stateless codec: the text, where reading
        // stopped, and why: 0 the end, 1 an illegal unit, 2 an unfinished
        // unit with more bytes to come, 3 an unfinished unit at the end.
        let packed = scalar(&args[3])?;
        let (last_input, mut pos) = (packed & 1 == 1, (packed >> 1) as usize);
        let fixed_width = matches!(name.as_ref(), "cp932" | "shift_jis" | "shift_jis_2004" | "shift_jisx0213");
        let mut text = Vec::new();
        let mut stop = 0;
        while pos < raw.len() {
            let mut width = 1;
            while width <= 8 && pos + width <= raw.len() {
                let unit = &raw[pos..pos+width];
                if decode_unit(data, &name, decoding_start, decoding_count, ranges, unit).is_some() { break; }
                if !longer_exists(data, &name, decoding_start, decoding_count, unit) { break; }
                width += 1;
            }
            if width == 1 && raw[pos] >= 128 && !fixed_width && decode_unit(data, &name, decoding_start, decoding_count, ranges, &raw[pos..pos+1]).is_none() {
                width = if name.starts_with("euc_j") && raw[pos] == 143 { 3 } else { 2 };
            }
            if pos + width > raw.len() { stop = if last_input { 3 } else { 2 }; break; }
            match decode_unit(data, &name, decoding_start, decoding_count, ranges, &raw[pos..pos+width]) {
                Some(points) => { text.extend(points); pos += width; }
                None => { stop = 1; break; }
            }
        }
        return Ok(Value::tuple(vec![Value::from_codes(text), Value::Small(pos as i64), Value::Small(stop)]));
    }
    if op == 1 { return Ok(decode_unit(data, &name, decoding_start, decoding_count, ranges, &raw).map_or(Value::Null, Value::from_codes)); }
    if op != 2 { return Err("ValueError: invalid multibyte mapping operation".into()); }
    Ok(Value::Flag(longer_exists(data, &name, decoding_start, decoding_count, &raw)))
}
const CATEGORIES: &[&str] = &["Cn", "Lu", "Ll", "Lt", "Lm", "Lo", "Mn", "Mc", "Me", "Nd", "Nl", "No", "Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po", "Sm", "Sc", "Sk", "So", "Zs", "Zl", "Zp", "Cc", "Cf", "Cs", "Co"];
const BIDI: &[&str] = &["", "L", "R", "AL", "EN", "ES", "ET", "AN", "CS", "NSM", "BN", "B", "S", "WS", "ON", "LRE", "LRO", "RLE", "RLO", "PDF", "LRI", "RLI", "FSI", "PDI"];

fn unicode_lookup(database: &[u8], operation: u32, point: u32, other: u32) -> Value {
    let properties = word(database,0) as usize;
    let decompositions = word(database,4) as usize;
    let compositions = word(database,8) as usize;
    let dec_start = 12+properties*12;
    let comp_start = dec_start+decompositions*16;
    let pool = comp_start+compositions*12;
    if operation == 4 {
        let mut lo = 0; let mut hi = properties;
        while lo < hi {
            let mid = (lo+hi)/2; let at = 12+mid*12;
            if point < word(database,at) { hi=mid; }
            else if point > word(database,at+4) { lo=mid+1; }
            else {
                let categories = CATEGORIES; let bidi = BIDI;
                return Value::tuple(vec![Value::text(categories[database[at+8] as usize]),Value::text(bidi[database[at+9] as usize]),Value::Small(database[at+10].into()),Value::Small(database[at+11].into())]);
            }
        }
    } else if operation == 5 {
        let mut left = 0; let mut right = decompositions;
        while left < right {
            let mid = (left+right)/2; let at = dec_start+mid*16;
            match word(database,at).cmp(&point) {
                std::cmp::Ordering::Less => left=mid+1,
                std::cmp::Ordering::Greater => right=mid,
                std::cmp::Ordering::Equal => {
                    let first = word(database,at+8) as usize; let count = word(database,at+12) as usize;
                    let values = (first..first+count).map(|index| Value::Small(word(database,pool+index*4).into())).collect();
                    return Value::tuple(vec![Value::Flag(word(database,at+4)!=0),Value::array(values)]);
                }
            }
        }
    } else {
        let pair = u64::from(point)*0x110000+u64::from(other);
        let mut lower = 0; let mut upper = compositions;
        while lower < upper {
            let mid = (lower+upper)/2; let at = comp_start+mid*12;
            match wide(database,at).cmp(&pair) {
                std::cmp::Ordering::Less => lower=mid+1,
                std::cmp::Ordering::Greater => upper=mid,
                std::cmp::Ordering::Equal => return Value::Small(word(database,at+8).into()),
            }
        }
    }
    Value::Null
}

#[cfg(test)]
mod designation_tests {
    use super::*;

    // Match scalar mappings without consuming controls, bad units or a trailing byte.
    #[test]
    fn designation_bulk_stops_at_scalar_boundaries() {
        let data = mappings("iso2022_jp").unwrap();
        let base = 8 + word(data, 0) as usize * 20;
        let rows = word(data, 4) as usize;
        let one = decode_unit(data, "iso2022_jp", base, rows, base + rows * 20, b"\x1b$B!!").unwrap();
        let prefix = b"\x1b$B";
        let initial = [66,66,66,0,0,0,0,0];
        assert_eq!(iso_feed(data, "iso2022_jp", b"\x1b$B!!\n\x1b(Bx", initial, true), Some((vec![one[0],10,120],10,initial)));
        assert!(iso_feed(data, "iso2022_jp", b"\x1b$B\xff\xff", initial, true).is_none());
        let mut shifted = initial; shifted[0] = 194;
        assert_eq!(iso_feed(data, "iso2022_jp", b"\x1b$B!", initial, false), Some((Vec::new(),3,shifted)));
        assert!(iso_feed(data, "iso2022_jp", b"\x1b$B!", initial, true).is_none());
        let encoded = call(&[Value::Small(10), Value::text("iso2022_jp"), Value::from_codes(vec![one[0], one[0], 0, one[0]]), Value::tuple(vec![output(prefix.to_vec()), Value::Small(1)])]).unwrap();
        let Value::Tuple(parts) = encoded.contents() else { panic!("expected encoded run"); };
        let Value::Bytes(wire, ..) = parts[0].contents() else { panic!("expected bytes"); };
        assert_eq!(&*wire.borrow(), b"!!!!");
        assert!(matches!(parts[1].contents(), Value::Small(2)));
        for (raw, end, count) in [(&b"!!!!\x1b(B"[..], 4, 2), (&b"!!\0!!"[..], 2, 1), (&b"!!!"[..], 2, 1), (&b"\xff\xff"[..], 0, 0)] {
            assert_eq!(decode_designated(data, "iso2022_jp", raw, prefix, 2, 0), (one.repeat(count), end));
        }
        assert!(call(&[Value::Small(9), Value::text("iso2022_jp"), output(b"!!".to_vec()), Value::tuple(vec![output(prefix.to_vec()), Value::Small(0), Value::Small(0)])]).is_err());
    }
}
