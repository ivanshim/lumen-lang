// CJK mapping lookups; data provenance and layout are in langs/lib_python/data/multibyte/README.md.
use crate::value::Value;
use num_traits::ToPrimitive;
use std::{cell::RefCell, rc::Rc};
fn word(data: &[u8], at: usize) -> u32 { u32::from_le_bytes(data[at..at+4].try_into().unwrap()) }
fn wide(data: &[u8], at: usize) -> u64 { u64::from_le_bytes(data[at..at+8].try_into().unwrap()) }
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
        "gbk" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/gbk.bin")),
        "gb18030" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/gb18030.bin")),
        "johab" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/johab.bin")),
        "shift_jis" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis.bin")),
        "shift_jis_2004" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis_2004.bin")),
        "shift_jisx0213" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jisx0213.bin")),
        "iso2022_jp" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp.bin")),
        "iso2022_jp_1" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_1.bin")),
        "iso2022_jp_2" => Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_2.bin")),
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
    if op == 3 {
        let point = scalar(&args[2])?;
        let mut lo = 0; let mut hi = encoding_count;
        while lo < hi {
            let mid = (lo+hi)/2; let at = 8+mid*20;
            if (word(data,at), word(data,at+4)) < (point,1) { lo=mid+1; } else { hi=mid; }
        }
        return Ok(Value::Flag(lo < encoding_count && word(data,8+lo*20) == point && word(data,12+lo*20) != 0));
    }
    if op == 0 {
        let first = scalar(&args[2])?; let second = scalar(&args[3])?;
        if let Some(at) = locate(data, 8, encoding_count, (first, second.into()), false) {
            let n = word(data, at+8) as usize;
            return Ok(output(wide(data, at+12).to_be_bytes()[8-n..].to_vec()));
        }
        if second == 0 && name.as_ref() == "gb18030" {
            for row in 0..word(data, ranges) as usize {
                let at = ranges + 4 + row * 16;
                if (word(data, at)..=word(data, at+4)).contains(&first) {
                    let pointer = word(data, at+8) + first - word(data, at);
                    return Ok(output(vec![(pointer/12600+129) as u8, (pointer/1260%10+48) as u8, (pointer/10%126+129) as u8, (pointer%10+48) as u8]));
                }
            }
        }
        return Ok(Value::Null);
    }
    let Value::Bytes(raw, ..) = args[2].contents() else { return Err("TypeError: mapping input must be bytes".into()); };
    let raw = raw.borrow(); let length = raw.len();
    if length > 8 { return Ok(Value::Null); }
    let mut code = 0u64; for byte in raw.iter() { code = (code << 8) | u64::from(*byte); }
    if op == 1 {
        if let Some(at) = locate(data, decoding_start, decoding_count, (length as u32, code), true) {
            let mut points = vec![word(data, at+12)]; let second = word(data, at+16);
            if second != 0 { points.push(second); }
            return Ok(Value::from_codes(points));
        }
        if name.as_ref() == "gb18030" && length == 4 && (129..=254).contains(&raw[0]) && (48..=57).contains(&raw[1]) && (129..=254).contains(&raw[2]) && (48..=57).contains(&raw[3]) {
            let pointer = ((u32::from(raw[0])-129)*10+u32::from(raw[1])-48)*1260+(u32::from(raw[2])-129)*10+u32::from(raw[3])-48;
            for row in 0..word(data, ranges) as usize {
                let at = ranges + 4 + row * 16;
                if (word(data, at+8)..=word(data, at+12)).contains(&pointer) { return Ok(Value::from_codes(vec![word(data, at)+pointer-word(data, at+8)])); }
            }
        }
        return Ok(Value::Null);
    }
    if op != 2 { return Err("ValueError: invalid multibyte mapping operation".into()); }
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
            if word(data,at) == width as u32 && wide(data,at+4) <= high { return Ok(Value::Flag(true)); }
        }
    }
    if name.as_ref() == "gb18030" && length < 4 {
        let valid = !raw.is_empty() && (129..=254).contains(&raw[0]) && (length < 2 || (48..=57).contains(&raw[1])) && (length < 3 || (129..=254).contains(&raw[2]));
        if valid { return Ok(Value::Flag(true)); }
    }
    Ok(Value::Flag(false))
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
