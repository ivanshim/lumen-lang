// Native character tables for the released multibyte codec wrappers.
use crate::data::Value;
use num_traits::ToPrimitive;
use std::{cell::RefCell, rc::Rc};
fn uint(table: &[u8], offset: usize) -> u32 {
    let part = &table[offset..offset+4];
    u32::from_le_bytes([part[0], part[1], part[2], part[3]])
}
fn ulong(table: &[u8], offset: usize) -> u64 {
    let mut part = [0u8; 8]; part.copy_from_slice(&table[offset..offset+8]); u64::from_le_bytes(part)
}
// Keep independent table ownership while the byte codec supplies lossless expansion.
fn unpack_table(encoded: &'static [u8], cell: &'static std::sync::OnceLock<Option<Vec<u8>>>) -> Option<&'static [u8]> {
    if !encoded.starts_with(b"\xfd7zXZ\0") { return Some(encoded); }
    let content = cell.get_or_init(|| match lumen_lzma::decode_limited(encoded, 1, 64 * 1024 * 1024, 64 * 1024 * 1024) {
        Ok((content, length)) if length == encoded.len() => Some(content),
        _ => None,
    });
    content.as_deref()
}
static JAPANESE_TWO: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
static CHINESE_FOUR: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
static CHINESE_TWO: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
fn table_for(name: &str) -> Option<&'static [u8]> {
    if name == "big5" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/big5.bin")); }
    if name == "big5hkscs" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/big5hkscs.bin")); }
    if name == "cp932" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/cp932.bin")); }
    if name == "cp949" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/cp949.bin")); }
    if name == "cp950" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/cp950.bin")); }
    if name == "euc_jp" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_jp.bin")); }
    if name == "euc_jis_2004" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_jis_2004.bin")); }
    if name == "euc_jisx0213" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_jisx0213.bin")); }
    if name == "euc_kr" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/euc_kr.bin")); }
    if name == "gb2312" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/gb2312.bin")); }
    if name == "gbk" { return unpack_table(include_bytes!("../../../langs/lib_python/data/multibyte/gbk.bin"), &CHINESE_TWO); }
    if name == "gb18030" { return unpack_table(include_bytes!("../../../langs/lib_python/data/multibyte/gb18030.bin"), &CHINESE_FOUR); }
    if name == "johab" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/johab.bin")); }
    if name == "shift_jis" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis.bin")); }
    if name == "shift_jis_2004" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis_2004.bin")); }
    if name == "shift_jisx0213" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jisx0213.bin")); }
    if name == "iso2022_jp" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp.bin")); }
    if name == "iso2022_jp_1" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_1.bin")); }
    if name == "iso2022_jp_2" { return unpack_table(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_2.bin"), &JAPANESE_TWO); }
    if name == "iso2022_jp_2004" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_2004.bin")); }
    if name == "iso2022_jp_3" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_3.bin")); }
    if name == "iso2022_jp_ext" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_ext.bin")); }
    if name == "iso2022_kr" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_kr.bin")); }
    if name == "hz" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/hz.bin")); }
    None
}
fn key(table: &[u8], row: usize, decode: bool) -> (u32, u64) {
    (uint(table,row), if decode { ulong(table,row+4) } else { uint(table,row+4).into() })
}
fn lower_bound(table: &[u8], base: usize, size: usize, needle: (u32,u64), decode: bool) -> usize {
    let mut first = 0; let mut remaining = size;
    while remaining != 0 {
        let half = remaining / 2; let next = first + half;
        if key(table, base + next*20, decode) < needle { first = next+1; remaining -= half+1; }
        else { remaining = half; }
    }
    first
}
fn integer(item: &Value) -> Result<u32, String> {
    item.as_big()?.to_u32().ok_or_else(|| String::from("ValueError: invalid Unicode point"))
}
fn bytes(raw: &[u8]) -> Value {
    Value::Octets {cell: Rc::new(RefCell::new(raw.into())), changeable: false, lead: "b".into()}
}
/// A table opened by codec name: its base offsets and record counts.
struct Table { data: &'static [u8], name: String, encoders: usize, decoders: usize, decoder_base: usize, range_base: usize }
impl Table {
    fn open(name: &str) -> Result<Table, String> {
        let data = table_for(name).ok_or_else(|| String::from("LookupError: unknown multibyte codec"))?;
        let encoders = uint(data,0) as usize; let decoders = uint(data,4) as usize;
        let decoder_base = 8+20*encoders;
        Ok(Table { data, name: name.to_owned(), encoders, decoders, decoder_base, range_base: decoder_base+20*decoders })
    }
    /// Does any pair in the table open with this character?
    fn opens_pair(&self, point: u32) -> bool {
        let row = lower_bound(self.data,8,self.encoders,(point,1),false);
        row != self.encoders && key(self.data,8+row*20,false).0 == point && uint(self.data,12+row*20) != 0
    }
    /// The bytes for a character, or for the pair it makes with the next.
    fn bytes_for(&self, point: u32, next: u32) -> Option<Vec<u8>> {
        let table = self.data;
        let wanted = (point,u64::from(next)); let row = lower_bound(table,8,self.encoders,wanted,false);
        if row < self.encoders && key(table,8+row*20,false) == wanted {
            let index = 8+row*20; let packed = ulong(table,index+12).to_be_bytes();
            return Some(packed[8-uint(table,index+8) as usize..].to_vec());
        }
        if self.name == "gb18030" && next == 0 {
            let mut offset = self.range_base+4;
            for _ in 0..uint(table,self.range_base) {
                let start = uint(table,offset); let end = uint(table,offset+4);
                if point >= start && point <= end {
                    let mut number = uint(table,offset+8)+point-start;
                    let fourth = (number%10+48) as u8; number /= 10;
                    let third = (number%126+129) as u8; number /= 126;
                    let second = (number%10+48) as u8; number /= 10;
                    return Some(vec![(number+129) as u8,second,third,fourth]);
                }
                offset += 16;
            }
        }
        None
    }
    /// The character(s) one unit of bytes stands for.
    fn text_for(&self, raw: &[u8]) -> Option<Vec<u32>> {
        let table = self.data;
        if raw.len() > 8 { return None; }
        let packed = raw.iter().fold(0u64,|number,byte| number*256+u64::from(*byte));
        let width = raw.len() as u32;
        let wanted = (width,packed); let row = lower_bound(table,self.decoder_base,self.decoders,wanted,true);
        if row != self.decoders && key(table,self.decoder_base+20*row,true) == wanted {
            let at = self.decoder_base+20*row; let mut text = vec![uint(table,at+12)];
            if uint(table,at+16) > 0 { text.push(uint(table,at+16)); }
            return Some(text);
        }
        if self.name == "gb18030" && width == 4 {
            let good = raw.iter().enumerate().all(|(i,b)| if i%2==0 { (129..=254).contains(b) } else { (48..=57).contains(b) });
            if good {
                let number = (((u32::from(raw[0])-129)*10+u32::from(raw[1])-48)*126+u32::from(raw[2])-129)*10+u32::from(raw[3])-48;
                let mut row = self.range_base+4;
                while row < table.len() {
                    if number >= uint(table,row+8) && number <= uint(table,row+12) { return Some(vec![number-uint(table,row+8)+uint(table,row)]); }
                    row += 16;
                }
            }
        }
        None
    }
    /// Could a longer unit begin with these bytes?
    fn continues(&self, raw: &[u8]) -> bool {
        let table = self.data;
        if raw.len() > 8 { return false; }
        let packed = raw.iter().fold(0u64,|number,byte| number*256+u64::from(*byte));
        let width = raw.len() as u32;
        let mut total = width+1;
        while total <= 8 {
            let bits = 8*(total-width);
            if bits < 64 {
                let start = packed << bits; let end = start | ((1u64<<bits)-1);
                let row = lower_bound(table,self.decoder_base,self.decoders,(total,start),true);
                if row < self.decoders {
                    let found = key(table,self.decoder_base+row*20,true);
                    if found.0 == total && found.1 <= end { return true; }
                }
            }
            total += 1;
        }
        self.name == "gb18030" && width > 0 && width < 4 && raw.iter().enumerate().all(|(i,b)| if i%2==0 { (129..=254).contains(b) } else { (48..=57).contains(b) })
    }
    /// A stateless codec writing text from `from`: the bytes, the place it
    /// stopped, and a reason (0 done, 1 unwritable character, 2 a final
    /// character that may begin a pair once more text arrives).
    fn write_run(&self, codes: &[u32], from: usize, complete: bool) -> (Vec<u8>, usize, i64) {
        let mut wire = Vec::new();
        let mut at = from;
        let reason = loop {
            let Some(&point) = codes.get(at) else { break 0 };
            let trailing = at + 1 == codes.len();
            if trailing && !complete && self.opens_pair(point) { break 2 }
            // A missing companion is stored as zero; leave a following NUL for its own step.
            let paired = if trailing || codes[at + 1] == 0 { None } else { self.bytes_for(point, codes[at + 1]) };
            if let Some(part) = paired { wire.extend(part); at += 2; continue; }
            match self.bytes_for(point, 0) {
                Some(part) => { wire.extend(part); at += 1; }
                None => break 1,
            }
        };
        (wire, at, reason)
    }
    // Trial a valid ISO feed on a private shift-state copy. Unsupported escapes
    // and errors leave the Python rollback/error-handler path in charge.
    fn read_iso(&self, raw: &[u8], mut shift: [u8; 8], complete: bool) -> Option<(Vec<u32>, usize, [u8; 8])> {
        let mut out = Vec::new();
        let mut at = 0;
        while at < raw.len() {
            let byte = raw[at];
            if byte == 27 {
                let rest = &raw[at..];
                let (length, group, mark) = if rest.starts_with(b"\x1b(") && rest.len() >= 3 && b"BJI".contains(&rest[2]) {
                    (3, 0, rest[2])
                } else if rest.starts_with(b"\x1b$") && rest.len() >= 3 && b"@B".contains(&rest[2]) {
                    (3, 0, rest[2] | 128)
                } else if rest.starts_with(b"\x1b$(") && rest.len() >= 4 && b"DOPQ".contains(&rest[3]) {
                    (4, 0, rest[3] | 128)
                } else if rest.starts_with(b"\x1b$)C") && self.name == "iso2022_kr" {
                    (4, 1, 195)
                } else { return None; };
                shift[group] = mark; at += length; continue;
            }
            if self.name == "iso2022_kr" && (byte == 14 || byte == 15) {
                if byte == 14 { shift[4] |= 1; } else { shift[4] &= !1; }
                at += 1; continue;
            }
            if byte < 32 {
                if byte == 10 { shift[4] &= !1; }
                out.push(u32::from(byte)); at += 1; continue;
            }
            let group = usize::from(self.name == "iso2022_kr" && shift[4] & 1 != 0);
            let mark = shift[group];
            let width = if mark & 128 != 0 { 2 } else { 1 };
            let prefix = if mark == 66 { Vec::new() }
                else if group == 1 { b"\x1b$)C\x0e".to_vec() }
                else if mark & 128 != 0 {
                    let mut p = if (192..=194).contains(&mark) { b"\x1b$".to_vec() } else { b"\x1b$(".to_vec() };
                    p.push(mark & 127); p
                } else { vec![27, 40, mark] };
            let (points, end) = self.read_designated(raw, &prefix, width, at);
            if end > at { out.extend(points); at = end; continue; }
            let mut tail = prefix;
            tail.extend_from_slice(&raw[at..]);
            if !complete && raw.len() - at < width && self.continues(&tail) { break; }
            return None;
        }
        Some((out, at, shift))
    }
    // Keep the current designation while consecutive encoded units use it.
    fn write_designated(&self, codes: &[u32], prefix: &[u8], from: usize, complete: bool) -> (Vec<u8>, usize) {
        let mut out = Vec::new();
        let mut at = from;
        while let Some(&point) = codes.get(at) {
            let trailing = at + 1 == codes.len();
            if trailing && !complete && self.opens_pair(point) { break; }
            let pair = if trailing || codes[at + 1] == 0 { None } else { self.bytes_for(point, codes[at + 1]) };
            let (unit, count) = match pair {
                Some(unit) => (Some(unit), 2),
                None => (self.bytes_for(point, 0), 1),
            };
            let Some(unit) = unit else { break; };
            let Some(tail) = unit.strip_prefix(prefix) else { break; };
            out.extend_from_slice(tail);
            at += count;
        }
        (out, at)
    }
    // Read mapped units in one designation, leaving controls and errors to the adapter.
    fn read_designated(&self, raw: &[u8], prefix: &[u8], width: usize, from: usize) -> (Vec<u32>, usize) {
        let mut unit = prefix.to_vec();
        unit.resize(prefix.len() + width, 0);
        let mut points = Vec::new();
        let mut at = from;
        while let Some(part) = at.checked_add(width).and_then(|end| raw.get(at..end)) {
            if part[0] < 32 { break; }
            unit[prefix.len()..].copy_from_slice(part);
            let Some(decoded) = self.text_for(&unit) else { break; };
            points.extend(decoded);
            at += width;
        }
        (points, at)
    }
    /// A stateless codec reading bytes from `from`: the text, the place it
    /// stopped, and a reason (0 done, 1 illegal unit, 2 unfinished unit
    /// with more to come, 3 unfinished unit at the very end).
    fn read_run(&self, raw: &[u8], from: usize, complete: bool) -> (Vec<u32>, usize, i64) {
        let shift_jis = ["cp932", "shift_jis", "shift_jis_2004", "shift_jisx0213"].contains(&self.name.as_str());
        let mut text = Vec::new();
        let mut at = from;
        let reason = loop {
            if at >= raw.len() { break 0 }
            let mut width = 1;
            loop {
                if width > 8 || at + width > raw.len() { break }
                let unit = &raw[at..at + width];
                if self.text_for(unit).is_some() || !self.continues(unit) { break }
                width += 1;
            }
            let lone_high = width == 1 && raw[at] >= 128 && !shift_jis && self.text_for(&raw[at..at + 1]).is_none();
            if lone_high { width = if self.name.starts_with("euc_j") && raw[at] == 143 { 3 } else { 2 }; }
            if at + width > raw.len() { break if complete { 3 } else { 2 } }
            let Some(points) = self.text_for(&raw[at..at + width]) else { break 1 };
            text.extend(points);
            at += width;
        };
        (text, at, reason)
    }
}
pub fn invoke(parameters: &[Value]) -> Result<Value, String> {
    let [action, named, input, following] = parameters else { return Err(String::from("TypeError: multibyte mapping expects four arguments")); };
    let name = match named.settled() { Value::Text(s) => s, _ => return Err(String::from("TypeError: codec name must be str")) };
    let operation = integer(action)?;
    if operation >= 4 && operation <= 6 {
        let content: &[u8] = if name.as_ref() == "ucd16" {include_bytes!("../../../langs/lib_python/data/unicode/ucd16.bin")}
            else if name.as_ref() == "ucd32" {include_bytes!("../../../langs/lib_python/data/unicode/ucd32.bin")}
            else {return Err(String::from("LookupError: unknown Unicode database"));};
        return Ok(ucd_record(content,operation,integer(input)?,integer(following)?));
    }
    let table = Table::open(&name)?;
    let action = integer(action)?;
    match action {
        3 => Ok(Value::Flag(table.opens_pair(integer(input)?))),
        0 => Ok(table.bytes_for(integer(input)?, integer(following)?).map_or(Value::Nil, |part| bytes(&part))),
        7 => {
            let Some(codes) = input.settled().character_numbers() else { return Err(String::from("TypeError: mapping input must be str")); };
            let marker = integer(following)?;
            let (wire, at, reason) = table.write_run(&codes, (marker / 2) as usize, marker % 2 == 1);
            Ok(Value::tuple(vec![bytes(&wire), Value::Small(at as i64), Value::Small(reason)]))
        }
        11 => {
            let Value::Octets { cell, .. } = input.settled() else { return Err("TypeError: mapping input must be bytes".into()); };
            let Value::Tuple(options) = following.settled() else { return Err("TypeError: ISO feed needs state and final flag".into()); };
            let [state, final_flag] = options.as_slice() else { return Err("TypeError: ISO feed needs state and final flag".into()); };
            let Value::Vector(values) = state.settled() else { return Err("TypeError: ISO state must be a list".into()); };
            if values.len() != 8 { return Err("ValueError: invalid ISO state size".into()); }
            let mut shift = [0u8; 8];
            for (slot, value) in shift.iter_mut().zip(values.iter()) { *slot = u8::try_from(integer(value)?).map_err(|_| "ValueError: invalid ISO state byte")?; }
            let decoded = table.read_iso(&cell.borrow(), shift, integer(final_flag)? != 0);
            Ok(match decoded {
                Some((text, end, state)) => Value::tuple(vec![Value::characters(text), Value::Small(end as i64), Value::Vector(crate::tuples::Sequence::plain(state.into_iter().map(|v| Value::Small(i64::from(v))).collect()))]),
                None => Value::Nil,
            })
        }
        10 => {
            let codes = input.settled().character_numbers().ok_or("TypeError: mapping input must be str")?;
            let Value::Tuple(options) = following.settled() else { return Err("TypeError: encoding designation requires a pair".into()); };
            let [prefix, marker] = options.as_slice() else { return Err("TypeError: encoding designation requires a pair".into()); };
            let Value::Octets { cell: prefix, .. } = prefix.settled() else { return Err("TypeError: designation prefix must be bytes".into()); };
            let prefix = prefix.borrow();
            if prefix.is_empty() { return Err("ValueError: empty designation".into()); }
            let marker = integer(marker)?;
            let (wire, at) = table.write_designated(&codes, &prefix, (marker / 2) as usize, marker % 2 == 1);
            Ok(Value::tuple(vec![bytes(&wire), Value::Small(at as i64)]))
        }
        9 => {
            let Value::Octets { cell, .. } = input.settled() else { return Err("TypeError: mapping input must be bytes".into()); };
            let Value::Tuple(options) = following.settled() else { return Err("TypeError: designation requires prefix, width and offset".into()); };
            let [prefix, width, from] = options.as_slice() else { return Err("TypeError: designation requires prefix, width and offset".into()); };
            let Value::Octets { cell: prefix, .. } = prefix.settled() else { return Err("TypeError: designation prefix must be bytes".into()); };
            let width = integer(width)? as usize;
            let prefix = prefix.borrow();
            if !(1..=2).contains(&width) || prefix.len() + width > 8 { return Err("ValueError: invalid designation width".into()); }
            let (text, at) = table.read_designated(&cell.borrow(), &prefix, width, integer(from)? as usize);
            Ok(Value::tuple(vec![Value::characters(text), Value::Small(at as i64)]))
        }
        1 | 2 | 8 => {
            let Value::Octets {cell, ..} = input.settled() else { return Err(String::from("TypeError: mapping input must be bytes")); };
            let raw = cell.borrow();
            if action == 8 {
                let marker = integer(following)?;
                let (text, at, reason) = table.read_run(&raw, (marker / 2) as usize, marker % 2 == 1);
                return Ok(Value::tuple(vec![Value::characters(text), Value::Small(at as i64), Value::Small(reason)]));
            }
            if action == 1 { return Ok(table.text_for(&raw).map_or(Value::Nil, Value::characters)); }
            Ok(Value::Flag(table.continues(&raw)))
        }
        _ => Err(String::from("ValueError: invalid multibyte mapping operation")),
    }
}
const CATEGORIES: &[&str] = &["Cn", "Lu", "Ll", "Lt", "Lm", "Lo", "Mn", "Mc", "Me", "Nd", "Nl", "No", "Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po", "Sm", "Sc", "Sk", "So", "Zs", "Zl", "Zp", "Cc", "Cf", "Cs", "Co"];
const BIDI: &[&str] = &["", "L", "R", "AL", "EN", "ES", "ET", "AN", "CS", "NSM", "BN", "B", "S", "WS", "ON", "LRE", "LRO", "RLE", "RLO", "PDF", "LRI", "RLI", "FSI", "PDI"];

fn ucd_record(data: &[u8], action: u32, character: u32, companion: u32) -> Value {
    let sizes = [uint(data,0) as usize,uint(data,4) as usize,uint(data,8) as usize];
    let decomposition_base = 12+sizes[0]*12;
    let composition_base = decomposition_base+sizes[1]*16;
    if action == 4 {
        let mut first = 0; let mut count = sizes[0];
        while count > 0 {
            let half = count/2; let row = first+half; let at = 12+12*row;
            if uint(data,at+4) < character { first=row+1; count-=half+1; }
            else { count=half; }
        }
        if first < sizes[0] {
            let index = 12+first*12;
            return Value::tuple(vec![Value::text(CATEGORIES[data[index+8] as usize]),Value::text(BIDI[data[index+9] as usize]),Value::Small(i64::from(data[index+10])),Value::Small(i64::from(data[index+11]))]);
        }
    }
    let (base, width, size, searched) = if action == 5 {(decomposition_base,16,sizes[1],u64::from(character))}
        else {(composition_base,12,sizes[2],u64::from(character)*0x110000+u64::from(companion))};
    let mut next = 0; let mut remaining = size;
    while remaining != 0 {
        let half = remaining/2; let mid = next+half; let address = base+width*mid;
        let found = if action == 5 {u64::from(uint(data,address))} else {ulong(data,address)};
        if found < searched { next=mid+1; remaining-=half+1; } else { remaining=half; }
    }
    if next != size {
        let address = base+width*next;
        let found = if action == 5 {u64::from(uint(data,address))} else {ulong(data,address)};
        if found == searched {
            if action != 5 { return Value::Small(i64::from(uint(data,address+8))); }
            let pool = composition_base+sizes[2]*12;
            let start = uint(data,address+8); let end = start+uint(data,address+12);
            let mut result = Vec::new();
            for index in start..end {result.push(Value::Small(i64::from(uint(data,pool+index as usize*4))));}
            return Value::tuple(vec![Value::Flag(uint(data,address+4)>0),Value::Vector(crate::tuples::Sequence::plain(result))]);
        }
    }
    Value::Nil
}

#[cfg(test)]
mod designated_tests {
    use super::*;

    // Preserve complete units and stop before every scalar fallback boundary.
    #[test]
    fn designated_runs_leave_controls_invalid_and_partial_units() {
        let table = Table::open("iso2022_jp").unwrap();
        let prefix = b"\x1b$B";
        let one = table.text_for(b"\x1b$B!!").unwrap();
        let initial = [66,66,66,0,0,0,0,0];
        let (text, end, state) = table.read_iso(b"\x1b$B!!\n\x1b(Bx", initial, true).unwrap();
        assert_eq!(text, vec![one[0], 10, 120]);
        assert_eq!(end, 10);
        assert_eq!(state, initial);
        assert!(table.read_iso(b"\x1b$B\xff\xff", initial, true).is_none());
        let (text, end, state) = table.read_iso(b"\x1b$B!", initial, false).unwrap();
        assert!(text.is_empty()); assert_eq!(end, 3); assert_eq!(state[0], 194);
        assert!(table.read_iso(b"\x1b$B!", initial, true).is_none());
        let points = [one[0], one[0], 0, one[0]];
        assert_eq!(table.write_designated(&points, prefix, 0, true), (b"!!!!".to_vec(), 2));
        assert_eq!(table.write_designated(&points, prefix, 2, true), (Vec::new(), 2));
        for (raw, end, count) in [(&b"!!!!\x1b(B"[..], 4, 2), (&b"!!\0!!"[..], 2, 1), (&b"!!!"[..], 2, 1), (&b"\xff\xff"[..], 0, 0)] {
            assert_eq!(table.read_designated(raw, prefix, 2, 0), (one.repeat(count), end));
        }
        assert!(invoke(&[Value::Small(9), Value::text("iso2022_jp"), bytes(b"!!"), Value::tuple(vec![bytes(prefix), Value::Small(0), Value::Small(0)])]).is_err());
    }
}
