// Binary layouts and conversions follow Modules/_struct.c at CPython v3.14.8.
use crate::value::Value;
use num_bigint::{BigInt, Sign};
use num_traits::{ToPrimitive, Zero};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
struct Field { code: u8, at: usize, count: usize, width: usize }
#[derive(Clone)]
struct Format { fields: Vec<Field>, size: usize, little: bool }
fn complaint(message: &str) -> String { format!("ValueError: {message}") }
fn bytes(data: Vec<u8>) -> Value { Value::Bytes(Rc::new(RefCell::new(data)), false, Rc::from("b")) }
fn parse(text: &str) -> Result<Format, String> {
    if text.contains('\0') { return Err(complaint("embedded null character")); }
    let source = text.as_bytes();
    let prefix = source.first().copied().unwrap_or(b'@');
    let has_prefix = b"@=<>!".contains(&prefix);
    let native = !has_prefix || prefix == b'@';
    let mut cursor = usize::from(has_prefix);
    let mut result = Format { fields: Vec::new(), size: 0, little: !matches!(prefix, b'>' | b'!') };
    while cursor < source.len() {
        if source[cursor].is_ascii_whitespace() { cursor += 1; continue; }
        let mut repeat = 0usize;
        let began = cursor;
        while cursor < source.len() && source[cursor].is_ascii_digit() {
            repeat = repeat.checked_mul(10).and_then(|n| n.checked_add((source[cursor]-b'0') as usize)).filter(|n| *n <= isize::MAX as usize).ok_or_else(|| complaint("total struct size too long"))?;
            cursor += 1;
        }
        if cursor == source.len() { return Err(complaint("repeat count given without format specifier")); }
        if began == cursor { repeat = 1; }
        let code = source[cursor]; cursor += 1;
        let width = match code {
            b'x' | b'c' | b'b' | b'B' | b'?' | b's' | b'p' => 1,
            b'h' | b'H' | b'e' => 2,
            b'i' | b'I' | b'f' => 4,
            b'l' | b'L' => if native { 8 } else { 4 },
            b'q' | b'Q' | b'd' | b'F' => 8,
            b'D' => 16,
            b'n' | b'N' | b'P' if native => 8,
            _ => return Err(complaint("bad char in struct format")),
        };
        let align = if !native { 1 } else if code == b'F' { 4 } else if code == b'D' { 8 } else { width };
        let at = result.size.checked_add((align-result.size%align)%align).ok_or_else(|| complaint("total struct size too long"))?;
        result.size = repeat.checked_mul(width).and_then(|n| at.checked_add(n)).filter(|n| *n <= isize::MAX as usize).ok_or_else(|| complaint("total struct size too long"))?;
        result.fields.push(Field {code, at, count: repeat, width});
    }
    Ok(result)
}
thread_local! {
    static FORMATS: RefCell<std::collections::HashMap<String, Format>> = RefCell::new(std::collections::HashMap::new());
}
fn compile(text: &str) -> Result<Format, String> {
    if let Some(held) = FORMATS.with(|cache| cache.borrow().get(text).cloned()) { return Ok(held); }
    let format = parse(text)?;
    FORMATS.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 100 { cache.clear(); }
        cache.insert(text.to_string(), format.clone());
    });
    Ok(format)
}
fn floating(value: &Value) -> Result<f64, String> {
    Ok(match value {
        Value::Real(r) => { let x=crate::value::as_binary(&r.p, &r.q); if r.below && r.p.is_zero() { -x.abs() } else { x } },
        Value::Small(n) => *n as f64,
        Value::Huge(n) => n.to_f64().ok_or_else(|| complaint("required argument is not a float"))?,
        _ => return Err(complaint("required argument is not a float")),
    })
}
fn half(number: f64) -> Result<u16,String> {
    let sign = if number.is_sign_negative() { 0x8000 } else { 0 };
    let x=number.abs();
    if x.is_nan() { return Ok(sign | 0x7e00); }
    if x.is_infinite() { return Ok(sign | 0x7c00); }
    if x >= 65520.0 { return Err("OverflowError: float too large to pack with e format".into()); }
    if x < 2f64.powi(-14) { return Ok(sign | (x * 2f64.powi(24)).round_ties_even() as u16); }
    let exponent = ((x.to_bits() >> 52) & 2047) as i32 - 1023;
    let mantissa = (x * 2f64.powi(10-exponent)).round_ties_even() as u16;
    Ok(sign | (((exponent + 14) as u16) << 10) + mantissa)
}
fn pack_float(number: f64, code: u8) -> Result<Vec<u8>,String> {
    match code {
        b'e' => Ok(half(number)?.to_le_bytes().to_vec()),
        b'f' | b'F' => { let f=number as f32; if number.is_finite() && f.is_infinite() { return Err("OverflowError: float too large to pack with f format".into()); } Ok(f.to_le_bytes().to_vec()) },
        _ => Ok(number.to_le_bytes().to_vec()),
    }
}
fn unpack_float(data: &[u8]) -> f64 {
    match data.len() {
        2 => { let bits=u16::from_le_bytes(data.try_into().unwrap()); let sign=if bits&0x8000!=0 {-1.0} else {1.0}; let exponent=(bits>>10)&31; let fraction=bits&1023; sign * match exponent { 0 => (fraction as f64)*2f64.powi(-24), 31 => if fraction==0 {f64::INFINITY} else {f64::NAN}, _ => (1024+fraction) as f64 * 2f64.powi(exponent as i32-25) } },
        4 => f32::from_le_bytes(data.try_into().unwrap()) as f64,
        _ => f64::from_le_bytes(data.try_into().unwrap()),
    }
}
pub fn apply(args: &[Value]) -> Result<Value,String> {
    let Value::Small(action)=args[0] else { return Err(complaint("invalid struct operation")); };
    let Value::Text(text)=&args[1] else { return Err("TypeError: Struct() argument 1 must be a str or bytes object".into()); };
    if action==4 { FORMATS.with(|cache| cache.borrow_mut().clear()); return Ok(Value::Null); }
    let format=compile(text)?;
    if action==0 { return Ok(Value::tuple(vec![Value::Small(format.size as i64), Value::tuple(format.fields.iter().map(|f| Value::tuple(vec![Value::text(&(f.code as char).to_string()),Value::Small(f.count as i64)])).collect())])); }
    let payload=args.get(2).ok_or_else(|| complaint("missing struct data"))?.contents();
    if action==6 {
        let Value::Bytes(cell,..)=payload else {return Err("TypeError: a bytes-like object is required".into());};
        return Ok(Value::Flag(crate::engine::ByteExport::active(&cell)));
    }
    if matches!(action,7..=9) {
        let width=format.size;
        if width==0 {return Err(complaint("zero array item size"));}
        let (source, selection)=if action==9 {
            let Value::Tuple(parts)=payload else {return Err(complaint("invalid array slice"));};
            let bounds:Vec<i64>=parts.iter().skip(1).map(|v| match v {Value::Small(n)=>Ok(*n),Value::Huge(n)=>Ok(n.to_i64().unwrap_or(if n.sign()==Sign::Minus {i64::MIN} else {i64::MAX})),_=>Err(complaint("invalid slice index"))}).collect::<Result<_,_>>()?;
            (parts[0].contents(),Some(bounds))
        } else {(payload,None)};
        let Value::Bytes(cell,..)=source else {return Err("TypeError: a bytes-like object is required".into());};
        let source=cell.borrow();
        if source.len()%width!=0 {return Err(complaint("bytes length not a multiple of item size"));}
        let mut out=Vec::new();
        if let Some(bounds)=selection {
            if bounds.len()!=3 || bounds[2]==0 {return Err(complaint("invalid array slice"));}
            let mut cursor=bounds[0];
            while if bounds[2]>0 {cursor<bounds[1]} else {cursor>bounds[1]} {
                let first=usize::try_from(cursor).ok().and_then(|n|n.checked_mul(width)).ok_or_else(||complaint("invalid array slice"))?;
                if first+width>source.len() {return Err(complaint("invalid array slice"));}
                out.extend_from_slice(&source[first..first+width]);
                cursor=cursor.saturating_add(bounds[2]);
            }
        } else if action==7 {
            for chunk in source.chunks_exact(width).rev() {out.extend_from_slice(chunk);}
        } else {
            out=source.clone();
            for chunk in out.chunks_exact_mut(width) {chunk.reverse();}
        }
        return Ok(bytes(out));
    }
    if action==3 {
        let Value::Bytes(storage,..)=payload else {return Err("TypeError: a bytes-like object is required".into());};
        let row=storage.borrow();
        return Ok(Value::tuple(vec![Value::Small(if row.is_empty() {0} else {row.as_ptr() as usize as i64}),Value::Small(row.len() as i64)]));
    }
    if action==1 {
        let Value::Tuple(values)=payload else { return Err(complaint("missing pack arguments")); };
        let expected: usize=format.fields.iter().map(|f| if f.code==b'x' {0} else if matches!(f.code,b's'|b'p') {1} else {f.count}).sum();
        if expected!=values.len() {return Err(complaint(&format!("pack expected {expected} items for packing (got {})",values.len())));}
        let mut output=Vec::new(); output.try_reserve_exact(format.size).map_err(|_| "MemoryError: ".to_string())?; output.resize(format.size,0);
        let mut next=0;
        for field in &format.fields {
            if field.code==b'x' {continue;}
            if matches!(field.code,b's'|b'p') {
                let Value::Bytes(data,..)=values[next].contents() else {return Err(complaint(&format!("argument for '{}' must be a bytes object",field.code as char)));}; next+=1;
                let data=data.borrow(); let pascal=usize::from(field.code==b'p');
                if field.count>0 { let take=data.len().min(field.count-pascal); output[field.at+pascal..field.at+pascal+take].copy_from_slice(&data[..take]); if pascal==1 {output[field.at]=take.min(255) as u8;} }
                continue;
            }
            for j in 0..field.count {
                let value=values[next].contents(); next+=1;
                let mut encoded=match field.code {
                    b'c' => { let Value::Bytes(data,false,_)=value else {return Err(complaint("char format requires a bytes object of length 1"));}; if data.borrow().len()!=1 {return Err(complaint("char format requires a bytes object of length 1"));} let out=data.borrow().clone(); out },
                    b'?' => vec![u8::from(value.is_true())],
                    b'e'|b'f'|b'd' => pack_float(floating(&value)?,field.code)?,
                    b'F'|b'D' => { let Value::Tuple(pair)=value else {return Err(complaint("required argument is not a complex"));}; let mut out=pack_float(floating(&pair[0])?,field.code)?; out.extend(pack_float(floating(&pair[1])?,field.code)?); out },
                    _ => {
                        let integer=match value {Value::Small(n)=>BigInt::from(n),Value::Huge(n)=>(*n).clone(),Value::Flag(b)=>BigInt::from(u8::from(b)),_=>return Err(complaint("required argument is not an integer"))};
                        let signed=b"bhilqn".contains(&field.code);
                        let bits=field.width*8; let limit=BigInt::from(1u8) << (bits-usize::from(signed)); let low=if signed {-&limit} else {BigInt::zero()}; let high=&limit-1;
                        if field.code!=b'P' && (integer<low || integer>high) {return Err(complaint(&format!("'{}' format requires {} <= number <= {}",field.code as char,low,high)));}
                        if field.code==b'P' && (integer < -(BigInt::from(1u8) << (bits-1)) || integer >= (BigInt::from(1u8)<<bits)) {return Err(complaint("int too large to convert"));}
                        let mut raw=integer.to_signed_bytes_le(); raw.resize(field.width,if integer.sign()==Sign::Minus {255} else {0}); raw.truncate(field.width); raw
                    }
                };
                if !format.little {if matches!(field.code,b'F'|b'D') {let half=encoded.len()/2; encoded[..half].reverse();encoded[half..].reverse();} else {encoded.reverse();}}
                let start=field.at+j*field.width;output[start..start+field.width].copy_from_slice(&encoded);
            }
        }
        return Ok(bytes(output));
    }
    let Value::Bytes(data,..)=payload else {return Err("TypeError: a bytes-like object is required".into());};let data=data.borrow();
    if data.len()!=format.size {return Err(complaint(&format!("unpack requires a buffer of {} bytes",format.size)));}
    let mut values=Vec::new();
    for field in &format.fields {
        if field.code==b'x' {continue;}
        if matches!(field.code,b's'|b'p') {let start=field.at+usize::from(field.code==b'p' && field.count>0);let count=if field.code==b'p' {if field.count==0 {0} else {(data[field.at] as usize).min(field.count-1)}} else {field.count};values.push(bytes(data[start..start+count].to_vec()));continue;}
        for j in 0..field.count {
            let start=field.at+j*field.width;let mut raw=data[start..start+field.width].to_vec();
            if !format.little {if matches!(field.code,b'F'|b'D') {let half=raw.len()/2;raw[..half].reverse();raw[half..].reverse();} else {raw.reverse();}}
            values.push(match field.code {
                b'c' => bytes(raw),b'?' => Value::Flag(raw[0]!=0),
                b'e'|b'f'|b'd' => crate::value::real_of(unpack_float(&raw),17),
                b'F'|b'D' => Value::tuple(vec![crate::value::real_of(unpack_float(&raw[..raw.len()/2]),17),crate::value::real_of(unpack_float(&raw[raw.len()/2..]),17)]),
                _ => Value::of_big(if b"bhilqn".contains(&field.code) {BigInt::from_signed_bytes_le(&raw)} else {BigInt::from_bytes_le(Sign::Plus,&raw)}),
            });
        }
    }
    Ok(Value::tuple(values))
}
