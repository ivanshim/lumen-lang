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
    if name == "gbk" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/gbk.bin")); }
    if name == "gb18030" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/gb18030.bin")); }
    if name == "johab" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/johab.bin")); }
    if name == "shift_jis" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis.bin")); }
    if name == "shift_jis_2004" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jis_2004.bin")); }
    if name == "shift_jisx0213" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/shift_jisx0213.bin")); }
    if name == "iso2022_jp" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp.bin")); }
    if name == "iso2022_jp_1" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_1.bin")); }
    if name == "iso2022_jp_2" { return Some(include_bytes!("../../../langs/lib_python/data/multibyte/iso2022_jp_2.bin")); }
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
    let table = table_for(&name).ok_or_else(|| String::from("LookupError: unknown multibyte codec"))?;
    let encoders = uint(table,0) as usize; let decoders = uint(table,4) as usize;
    let decoder_base = 8+20*encoders; let range_base = decoder_base+20*decoders;
    let action = integer(action)?;
    match action {
        3 => {
            let point = integer(input)?;
            let row = lower_bound(table,8,encoders,(point,1),false);
            Ok(Value::Flag(row != encoders && key(table,8+row*20,false).0 == point && uint(table,12+row*20) != 0))
        }
        0 => {
            let point = integer(input)?; let next = integer(following)?;
            let wanted = (point,u64::from(next)); let row = lower_bound(table,8,encoders,wanted,false);
            if row < encoders && key(table,8+row*20,false) == wanted {
                let index = 8+row*20; let packed = ulong(table,index+12).to_be_bytes();
                return Ok(bytes(&packed[8-uint(table,index+8) as usize..]));
            }
            if name.as_ref() == "gb18030" && next == 0 {
                let mut offset = range_base+4;
                for _ in 0..uint(table,range_base) {
                    let start = uint(table,offset); let end = uint(table,offset+4);
                    if point >= start && point <= end {
                        let mut number = uint(table,offset+8)+point-start;
                        let fourth = (number%10+48) as u8; number /= 10;
                        let third = (number%126+129) as u8; number /= 126;
                        let second = (number%10+48) as u8; number /= 10;
                        return Ok(bytes(&[(number+129) as u8,second,third,fourth]));
                    }
                    offset += 16;
                }
            }
            Ok(Value::Nil)
        }
        1 | 2 => {
            let Value::Octets {cell, ..} = input.settled() else { return Err(String::from("TypeError: mapping input must be bytes")); };
            let raw = cell.borrow();
            if raw.len() > 8 { return Ok(Value::Nil); }
            let packed = raw.iter().fold(0u64,|number,byte| number*256+u64::from(*byte));
            let width = raw.len() as u32;
            if action == 1 {
                let wanted = (width,packed); let row = lower_bound(table,decoder_base,decoders,wanted,true);
                if row != decoders && key(table,decoder_base+20*row,true) == wanted {
                    let at = decoder_base+20*row; let mut text = vec![uint(table,at+12)];
                    if uint(table,at+16) > 0 { text.push(uint(table,at+16)); }
                    return Ok(Value::characters(text));
                }
                if name.as_ref() == "gb18030" && width == 4 {
                    let good = raw.iter().enumerate().all(|(i,b)| if i%2==0 { (129..=254).contains(b) } else { (48..=57).contains(b) });
                    if good {
                        let number = (((u32::from(raw[0])-129)*10+u32::from(raw[1])-48)*126+u32::from(raw[2])-129)*10+u32::from(raw[3])-48;
                        let mut row = range_base+4;
                        while row < table.len() {
                            if number >= uint(table,row+8) && number <= uint(table,row+12) { return Ok(Value::characters(vec![number-uint(table,row+8)+uint(table,row)])); }
                            row += 16;
                        }
                    }
                }
                return Ok(Value::Nil);
            }
            let mut total = width+1;
            while total <= 8 {
                let bits = 8*(total-width);
                if bits < 64 {
                    let start = packed << bits; let end = start | ((1u64<<bits)-1);
                    let row = lower_bound(table,decoder_base,decoders,(total,start),true);
                    if row < decoders {
                        let found = key(table,decoder_base+row*20,true);
                        if found.0 == total && found.1 <= end { return Ok(Value::Flag(true)); }
                    }
                }
                total += 1;
            }
            let gb_prefix = name.as_ref() == "gb18030" && width > 0 && width < 4 && raw.iter().enumerate().all(|(i,b)| if i%2==0 { (129..=254).contains(b) } else { (48..=57).contains(b) });
            Ok(Value::Flag(gb_prefix))
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
