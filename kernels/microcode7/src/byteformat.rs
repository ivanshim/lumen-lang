// CPython v3.14.8 Modules/_struct.c specifies these layouts and conversions.
use crate::data::{Value, worth_of_binary, nearest_binary};
use num_bigint::BigInt;
use num_traits::{Zero, ToPrimitive, Signed};
use std::rc::Rc;
use std::cell::RefCell;

fn invalid(why: impl std::fmt::Display) -> String { format!("ValueError: {why}") }
fn octets(row: &[u8]) -> Value {
    Value::Octets { cell: Rc::new(RefCell::new(row.to_vec())), changeable: false, lead: Rc::from("b") }
}
#[derive(Clone)]
struct Piece(u8, usize, usize, usize);
fn describe(spelling: &str) -> Result<(usize, bool, Vec<Piece>), String> {
    if spelling.as_bytes().contains(&0) { return Err(invalid("embedded null character")); }
    let mut chars = spelling.bytes().peekable();
    let first = chars.peek().copied().unwrap_or(b'@');
    let system = first == b'@' || !b"@=<>!".contains(&first);
    let reverse = first == b'>' || first == b'!';
    if b"@=<>!".contains(&first) { chars.next(); }
    let mut position = 0usize;
    let mut parts = Vec::new();
    while let Some(mut letter) = chars.next() {
        if letter.is_ascii_whitespace() { continue; }
        let mut repetitions = 1usize;
        if letter.is_ascii_digit() {
            repetitions = 0;
            loop {
                repetitions = repetitions.checked_mul(10).and_then(|v| v.checked_add(usize::from(letter-b'0'))).filter(|v| *v <= isize::MAX as usize).ok_or_else(|| invalid("total struct size too long"))?;
                letter = chars.next().ok_or_else(|| invalid("repeat count given without format specifier"))?;
                if !letter.is_ascii_digit() { break; }
            }
        }
        let size = match letter as char {
            's'|'p'|'x'|'c'|'b'|'B'|'?' => 1,
            'h'|'H'|'e' => 2,
            'i'|'I'|'f' => 4,
            'l'|'L' => if system {8} else {4},
            'n'|'N'|'P' if system => 8,
            'q'|'Q'|'d'|'F' => 8,
            'D' => 16,
            _ => return Err(invalid("bad char in struct format")),
        };
        let boundary = if !system {1} else { match letter { b'F'=>4, b'D'=>8, _=>size } };
        position = position.checked_add((boundary-position%boundary)%boundary).ok_or_else(|| invalid("total struct size too long"))?;
        parts.push(Piece(letter, position, repetitions, size));
        position = repetitions.checked_mul(size).and_then(|v| position.checked_add(v)).filter(|v| *v<=isize::MAX as usize).ok_or_else(|| invalid("total struct size too long"))?;
    }
    Ok((position, reverse, parts))
}
type Description = (usize, bool, Vec<Piece>);
thread_local! {
    static SAVED: RefCell<std::collections::BTreeMap<String, Description>> = RefCell::new(std::collections::BTreeMap::new());
}
fn arrangement(spelling: &str) -> Result<Description, String> {
    let old = SAVED.with(|saved| saved.borrow().get(spelling).cloned());
    match old {
        Some(description) => Ok(description),
        None => {
            let description = describe(spelling)?;
            SAVED.with(|saved| {
                let mut book = saved.borrow_mut();
                if book.len() == 100 { book.clear(); }
                book.insert(spelling.into(), description.clone());
            });
            Ok(description)
        }
    }
}
fn decimal(v: &Value) -> Result<f64, String> {
    match v {
        Value::Frac(r) => { let number=nearest_binary(&r.above,&r.beneath); Ok(if r.under && r.above.is_zero() {-number.abs()} else {number}) },
        Value::Small(v) => Ok(*v as f64),
        Value::Huge(v) => v.to_f64().ok_or_else(|| invalid("required argument is not a float")),
        _ => Err(invalid("required argument is not a float")),
    }
}
fn real_bytes(n: f64, symbol: u8) -> Result<Vec<u8>, String> {
    if symbol==b'd' || symbol==b'D' {return Ok(n.to_le_bytes().to_vec());}
    if symbol==b'f' || symbol==b'F' {
        let narrowed=n as f32;
        if n.is_finite() && narrowed.is_infinite() {return Err("OverflowError: float too large to pack with f format".to_owned());}
        return Ok(narrowed.to_le_bytes().to_vec());
    }
    let magnitude=n.abs();
    let sign=u16::from(n.is_sign_negative())<<15;
    let encoded = if magnitude.is_nan() {0x7e00} else if magnitude.is_infinite() {0x7c00} else {
        if magnitude>=65520.0 {return Err("OverflowError: float too large to pack with e format".to_owned());}
        if magnitude<1.0/16384.0 {(magnitude*16777216.0).round_ties_even() as u16} else {
            let power=((magnitude.to_bits()>>52)&0x7ff) as i32-1023;
            ((power+14) as u16*1024)+(magnitude*2.0f64.powi(10-power)).round_ties_even() as u16
        }
    };
    Ok((sign|encoded).to_le_bytes().to_vec())
}
fn read_real(input: &[u8]) -> f64 {
    if input.len()==8 {return f64::from_le_bytes(input.try_into().unwrap());}
    if input.len()==4 {return f32::from_le_bytes(input.try_into().unwrap()) as f64;}
    let word=u16::from_le_bytes(input.try_into().unwrap());
    let e=(word/1024)%32;let m=word%1024;
    let positive=match e {0=>f64::from(m)/16777216.0,31=>if m==0 {f64::INFINITY} else {f64::NAN},_=>f64::from(m+1024)*2.0f64.powi(i32::from(e)-25)};
    if word&32768==0 {positive} else {-positive}
}
fn integer(v: &Value, symbol: u8, size: usize) -> Result<Vec<u8>, String> {
    let n=match v {Value::Huge(n)=>(**n).clone(),Value::Small(n)=>BigInt::from(*n),Value::Flag(b)=>BigInt::from(i32::from(*b)),_=>return Err(invalid("required argument is not an integer"))};
    let signed=b"bhilqn".contains(&symbol);
    let upper=BigInt::from(1u8)<<(8*size-usize::from(signed));
    let minimum=if signed {-&upper} else {BigInt::zero()};
    if symbol==b'P' {
        if n < -(BigInt::from(1u8)<<(size*8-1)) || n >= (BigInt::from(1u8)<<(size*8)) {return Err(invalid("int too large to convert"));}
    } else if n<minimum || n>=upper {return Err(invalid(format!("'{}' format requires {} <= number <= {}",symbol as char,minimum,&upper-1)));}
    let mut result=n.to_signed_bytes_le();
    result.resize(size, if n.is_negative() {0xff} else {0});
    result.truncate(size);
    Ok(result)
}
fn reorder(row: &mut [u8], complex: bool) {
    if complex {let size=row.len()/2;row[..size].reverse();row[size..].reverse();} else {row.reverse();}
}
pub fn perform(given: &[Value]) -> Result<Value,String> {
    let Value::Text(spelling)=&given[1] else {return Err("TypeError: Struct() argument 1 must be a str or bytes object".to_owned());};
    if matches!(given[0].settled(), Value::Small(4)) { SAVED.with(|saved| saved.borrow_mut().clear()); return Ok(Value::Nil); }
    let (size, reverse, pieces)=arrangement(spelling)?;
    let task=given[0].settled();
    if matches!(task, Value::Small(0)) {
        let descriptions=pieces.iter().map(|Piece(c,_,n,_)| Value::tuple(vec![Value::text(&char::from(*c).to_string()),Value::Small(*n as i64)])).collect();
        return Ok(Value::tuple(vec![Value::Small(size as i64),Value::tuple(descriptions)]));
    }
    let payload=given.get(2).ok_or_else(|| invalid("missing struct data"))?.settled();
    if matches!(task,Value::Small(6)) {
        return match payload {
            Value::Octets {cell,..}=>Ok(Value::Flag(crate::exec::OctetLease::held(&cell))),
            _=>Err("TypeError: a bytes-like object is required".to_owned()),
        };
    }
    if let Value::Small(operation @ 7..=9)=task {
        if size==0 {return Err(invalid("zero array item size"));}
        let mut positions=None;
        let storage=if operation==9 {
            let Value::Tuple(values)=payload else {return Err(invalid("invalid array slice"));};
            let mut indices=Vec::new();
            for value in values.iter().skip(1) {
                let index=match value {
                    Value::Small(n)=>*n,
                    Value::Huge(n)=>n.to_i64().unwrap_or(if n.is_negative() {i64::MIN} else {i64::MAX}),
                    _=>return Err(invalid("invalid slice index")),
                };
                indices.push(index);
            }
            positions=Some(indices);
            values[0].settled()
        } else {payload};
        let Value::Octets {cell,..}=storage else {return Err("TypeError: a bytes-like object is required".to_owned());};
        let original=cell.borrow();
        if original.len()%size!=0 {return Err(invalid("bytes length not a multiple of item size"));}
        let mut rewritten=Vec::new();
        match positions {
            Some(indices)=>{
                if indices.len()!=3 || indices[2]==0 {return Err(invalid("invalid array slice"));}
                let (mut at, limit, stride)=(indices[0],indices[1],indices[2]);
                while (stride>0 && at<limit) || (stride<0 && at>limit) {
                    let beginning=usize::try_from(at).ok().and_then(|v|v.checked_mul(size)).ok_or_else(||invalid("invalid array slice"))?;
                    let item=original.get(beginning..beginning+size).ok_or_else(||invalid("invalid array slice"))?;
                    rewritten.extend_from_slice(item);
                    at=at.saturating_add(stride);
                }
            },
            None=>{
                rewritten=original.to_vec();
                if operation==8 {
                    rewritten.chunks_exact_mut(size).for_each(|item| item.reverse());
                } else {
                    let count=rewritten.len()/size;
                    for n in 0..count/2 {
                        let (left,right)=rewritten.split_at_mut((count-n-1)*size);
                        left[n*size..(n+1)*size].swap_with_slice(&mut right[..size]);
                    }
                }
            },
        }
        return Ok(octets(&rewritten));
    }
    if let Value::Small(3)=task {
        if let Value::Octets {cell,..}=payload {
            let content=cell.borrow();
            let location=if content.is_empty() {0} else {content.as_ptr() as usize as i64};
            return Ok(Value::tuple(vec![Value::Small(location),Value::Small(content.len() as i64)]));
        }
        return Err("TypeError: a bytes-like object is required".to_owned());
    }
    if matches!(task, Value::Small(1)) {
        let Value::Tuple(entries)=payload else {return Err(invalid("missing pack arguments"));};
        let need:usize=pieces.iter().map(|Piece(c,_,n,_)| match c {b'x'=>0,b's'|b'p'=>1,_=>*n}).sum();
        if need!=entries.len() {return Err(invalid(format!("pack expected {need} items for packing (got {})",entries.len())));}
        let mut result=Vec::new();result.try_reserve_exact(size).map_err(|_| "MemoryError: ".to_owned())?;result.resize(size,0);
        let mut entry=entries.iter();
        for Piece(symbol, start, repeat, width) in pieces {
            if symbol==b'x' {continue;}
            if symbol==b's' || symbol==b'p' {
                let value=entry.next().unwrap().settled();
                let Value::Octets {cell: data,..}=value else {return Err(invalid(format!("argument for '{}' must be a bytes object",symbol as char)));};
                let data=data.borrow();
                if repeat>0 {
                    let shift=usize::from(symbol==b'p');let length=data.len().min(repeat-shift);
                    result[start+shift..start+shift+length].copy_from_slice(&data[..length]);
                    if shift>0 {result[start]=length.min(255) as u8;}
                }
                continue;
            }
            for offset in (start..start+repeat*width).step_by(width) {
                let member=entry.next().unwrap().settled();
                let mut row=match symbol {
                    b'?' => vec![u8::from(member.is_true())],
                    b'c' => match member {Value::Octets {cell: data,..} if data.borrow().len()==1=>data.borrow().clone(),_=>return Err(invalid("char format requires a bytes object of length 1"))},
                    b'e'|b'f'|b'd' => real_bytes(decimal(&member)?,symbol)?,
                    b'F'|b'D' => {let Value::Tuple(pair)=member else {return Err(invalid("required argument is not a complex"));};let mut pairbytes=real_bytes(decimal(&pair[0])?,symbol)?;pairbytes.extend(real_bytes(decimal(&pair[1])?,symbol)?);pairbytes},
                    _=>integer(&member,symbol,width)?,
                };
                if reverse {reorder(&mut row, symbol==b'F'||symbol==b'D');}
                result[offset..offset+width].copy_from_slice(&row);
            }
        }
        return Ok(octets(&result));
    }
    let Value::Octets {cell: data,..}=payload else {return Err("TypeError: a bytes-like object is required".to_owned());};
    let input=data.borrow();
    if input.len()!=size {return Err(invalid(format!("unpack requires a buffer of {size} bytes")));}
    let mut results=Vec::new();
    for Piece(symbol, start, repeat, width) in pieces {
        match symbol {
            b'x'=>continue,
            b's'=>{results.push(octets(&input[start..start+repeat]));continue;},
            b'p'=>{let length=if repeat==0 {0} else {usize::from(input[start]).min(repeat-1)};let beginning=start+usize::from(repeat>0);results.push(octets(&input[beginning..beginning+length]));continue;},
            _=>(),
        }
        for offset in (start..start+repeat*width).step_by(width) {
            let mut chunk=input[offset..offset+width].to_vec();
            if reverse {reorder(&mut chunk, symbol==b'F'||symbol==b'D');}
            let decoded=match symbol {
                b'?' => Value::Flag(chunk[0]!=0),
                b'c' => octets(&chunk),
                b'e'|b'f'|b'd' => worth_of_binary(read_real(&chunk),17),
                b'F'|b'D' => Value::tuple(vec![worth_of_binary(read_real(&chunk[..width/2]),17),worth_of_binary(read_real(&chunk[width/2..]),17)]),
                _ => {let number=if b"bhilqn".contains(&symbol) {BigInt::from_signed_bytes_le(&chunk)} else {BigInt::from_bytes_le(num_bigint::Sign::Plus,&chunk)};Value::from_big(number)},
            };
            results.push(decoded);
        }
    }
    Ok(Value::tuple(results))
}
