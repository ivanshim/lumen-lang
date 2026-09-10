// Methods belonging to the values themselves. The spelling is supplied
// by the definition; the receiver keeps its own mutable holding place.

use std::rc::Rc;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};
use crate::value::{Value, Wording};

type Answer = Result<Value, String>;

fn alike(a: &Value, b: &Value) -> bool {
    match (a,b) { (Value::Real(x),Value::Real(y)) if Rc::ptr_eq(x,y) => true, _ => a.equals(b) }
}

fn shown(value: &Value, words: &Wording) -> String {
    if let Value::Real(r) = value {
        let mut number=crate::value::as_binary(&r.p,&r.q);
        if number==0.0 && r.below {number = -0.0;}
        if !number.is_finite() {return if number.is_nan(){"nan".into()}else if number.is_sign_negative(){"-inf".into()}else{"inf".into()};}
        let scientific=format!("{number:e}");
        let (mantissa,power)=scientific.split_once('e').unwrap();let power=power.parse::<i32>().unwrap();
        if !(-4..16).contains(&power){return format!("{mantissa}e{}{:02}",if power<0{"-"}else{"+"},power.abs());}
        let mut plain=number.to_string();if !plain.contains('.') {plain.push_str(".0");}return plain;
    }
    value.representation(words)
}

fn reaches(value: &Value, cell: &Rc<std::cell::RefCell<Value>>, depth: usize) -> bool {
    if depth > 100 { return true; }
    match value {
        Value::Collection(held, _) => Rc::ptr_eq(held,cell) || reaches(&held.borrow(),cell,depth+1),
        Value::Array(row) | Value::Tuple(row) => row.iter().any(|v|reaches(v,cell,depth+1)),
        Value::Map(entries) => entries.iter().any(|(k,v)|reaches(k,cell,depth+1)||reaches(v,cell,depth+1)),
        Value::View(view) => reaches(&view.0,cell,depth+1),
        _ => false,
    }
}


pub fn members(v: &Value, fault: &dyn Fn(&str) -> String) -> Result<Vec<Value>, String> {
    match v.contents() {
        Value::Array(a) | Value::Tuple(a) => Ok(a.as_ref().clone()),
        Value::Map(p) => Ok(p.iter().map(|(k, _)| k.clone()).collect()),
        Value::Text(s) => Ok(s.chars().map(|c| Value::text(&c.to_string())).collect()),
        Value::Counted(r) => {
            let count = r.length().to_usize().ok_or_else(|| fault("unready"))?;
            (0..count).map(|i| r.at(BigInt::from(i)).ok_or_else(|| fault("arguments"))).collect()
        }
        _ => Err(fault("arguments")),
    }
}

fn integer(v: &Value, fault: &dyn Fn(&str) -> String) -> Result<i64, String> {
    match v.contents() {
        Value::Small(n) => Ok(n), Value::Huge(n) => Ok(n.to_i64().unwrap_or(if n.is_negative() { i64::MIN } else { i64::MAX })),
        Value::Flag(b) => Ok(i64::from(b)), _ => Err(fault("arguments")),
    }
}

fn text(v: &Value, fault: &dyn Fn(&str) -> String) -> Result<String, String> {
    if let Value::Text(s) = v.contents() { Ok(s.to_string()) } else { Err(fault("arguments")) }
}

fn bound(n: i64, length: usize) -> usize {
    if n < 0 { (length as i64).saturating_add(n).max(0) as usize } else { (n as usize).min(length) }
}

pub fn call(receiver: &Value, op: &str, args: &[Value], names: &[(String, Value)], words: &Wording, fault: &dyn Fn(&str) -> String) -> Answer {
    let mut supplied = args.to_vec();
    if !names.is_empty() && !matches!(op, "format" | "update" | "encode") {
        let slots: &[&str] = match op { "split" | "rsplit" => &["sep", "maxsplit"], _ => &[] };
        for (name, v) in names {
            let Some(at) = slots.iter().position(|s| s == name) else { return Err(fault("arguments")); };
            if at < args.len() { return Err(fault("arguments")); }
            while supplied.len() <= at { supplied.push(Value::Null); }
            supplied[at] = v.clone();
        }
    }
    let a = supplied.as_slice();
    let arity = |lo, hi| if a.len() >= lo && a.len() <= hi { Ok(()) } else { Err(fault("arguments")) };
    let held = receiver.contents();
    let store = |v: Value| -> Answer {
        if let Value::Collection(cell, _) = receiver { if reaches(&v,cell,0) {return Err(fault("unready"));} *cell.borrow_mut() = v; Ok(Value::Null) } else { Err(fault("unready")) }
    };
    match &held {
        Value::Text(s) => {
            if op == "encode" { return Err(fault("bytes")); }
            if op == "format" { return format_fields(s, a, names, words, fault).map(|s| Value::text(&s)); }
            let answer = match op {
                "upper" | "lower" => { arity(0, 0)?; if op == "upper" { s.to_uppercase() } else { s.to_lowercase() } }
                "title" | "capitalize" => {
                    arity(0, 0)?;
                    if !s.is_ascii() { return Err(fault("unicode")); }
                    let mut begin = true;
                    s.chars().enumerate().map(|(i,c)| {
                        let next = if (op == "capitalize" && i == 0) || (op == "title" && begin) { c.to_ascii_uppercase() } else { c.to_ascii_lowercase() };
                        begin = !c.is_ascii_alphabetic(); next
                    }).collect()
                }
                "strip" | "lstrip" | "rstrip" => {
                    arity(0,1)?;
                    let chars = a.first().filter(|v| !matches!(v, Value::Null)).map(|v| text(v,fault)).transpose()?;
                    let trim = |c: char| chars.as_ref().map_or(c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c), |set| set.contains(c));
                    match op { "lstrip" => s.trim_start_matches(trim), "rstrip" => s.trim_end_matches(trim), _ => s.trim_matches(trim) }.to_string()
                }
                "split" | "rsplit" => {
                    arity(0,2)?;
                    let limit = a.get(1).filter(|x| !matches!(x,Value::Null)).map(|x| integer(x,fault)).transpose()?.unwrap_or(-1);
                    let limit = if limit < 0 { usize::MAX } else { limit as usize };
                    let sep = a.first().filter(|x| !matches!(x,Value::Null)).map(|x| text(x,fault)).transpose()?;
                    let mut parts: Vec<String> = Vec::new();
                    if let Some(sep) = sep {
                        if sep.is_empty() { return Err(fault("separator")); }
                        if op == "rsplit" { parts = s.rsplitn(limit.saturating_add(1), &sep).map(str::to_string).collect(); parts.reverse(); }
                        else { parts = s.splitn(limit.saturating_add(1), &sep).map(str::to_string).collect(); }
                    } else {
                        let whitespace = |c: char| c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c);
                        let mut rest: &str = s;
                        while !rest.is_empty() {
                            rest = if op == "rsplit" { rest.trim_end_matches(whitespace) } else { rest.trim_start_matches(whitespace) };
                            if rest.is_empty() { break; }
                            if parts.len() == limit { parts.push(rest.to_string()); break; }
                            let at = if op == "rsplit" { rest.rfind(whitespace) } else { rest.find(whitespace) };
                            match at {
                                None => { parts.push(rest.to_string()); break; }
                                Some(at) if op == "rsplit" => { let size = rest[at..].chars().next().unwrap().len_utf8(); parts.push(rest[at+size..].to_string()); rest = &rest[..at]; }
                                Some(at) => { parts.push(rest[..at].to_string()); rest = &rest[at..]; }
                            }
                        }
                        if op == "rsplit" { parts.reverse(); }
                    }
                    return Ok(Value::array(parts.iter().map(|x| Value::text(x)).collect()).held(true));
                }
                "join" => { arity(1,1)?; members(&a[0],fault)?.iter().map(|x| text(x,fault)).collect::<Result<Vec<_>,_>>()?.join(s) }
                "replace" => { arity(2,3)?; let n = a.get(2).map(|v| integer(v,fault)).transpose()?.unwrap_or(-1); s.replacen(&text(&a[0],fault)?, &text(&a[1],fault)?, if n < 0 { usize::MAX } else { n as usize }) }
                "find" | "rfind" | "index" | "count" | "startswith" | "endswith" => {
                    arity(1,3)?;
                    let chars: Vec<char> = s.chars().collect();
                    let raw = a.get(1).filter(|v| !matches!(v,Value::Null)).map(|v| integer(v,fault)).transpose()?.unwrap_or(0);
                    let start = bound(raw,chars.len());
                    let stop = a.get(2).filter(|v| !matches!(v,Value::Null)).map(|v| integer(v,fault)).transpose()?.map_or(chars.len(), |n| bound(n,chars.len()));
                    let valid = raw <= chars.len() as i64 && start <= stop;
                    let piece: String = chars[start..stop.max(start)].iter().collect();
                    if matches!(op,"startswith"|"endswith") {
                        let patterns = match &a[0] { Value::Tuple(row) => row.as_ref().clone(), v => vec![v.clone()] };
                        for pattern in patterns { let t=text(&pattern,fault)?; if valid && if op == "startswith" { piece.starts_with(&t) } else { piece.ends_with(&t) } { return Ok(Value::Flag(true)); } }
                        return Ok(Value::Flag(false));
                    }
                    let needle = text(&a[0],fault)?;
                    if op == "count" { return Ok(Value::Small(if !valid {0} else if needle.is_empty() {piece.chars().count() as i64+1} else {piece.matches(&needle).count() as i64})); }
                    let found = if !valid { None } else if op == "rfind" { piece.rfind(&needle) } else { piece.find(&needle) };
                    if op == "index" && found.is_none() { return Err(fault("substring")); }
                    return Ok(Value::Small(found.map_or(-1,|i| (start+piece[..i].chars().count()) as i64)));
                }
                "isdigit" | "isalpha" | "isalnum" | "isspace" | "islower" | "isupper" => {
                    arity(0,0)?;
                    if !s.is_ascii() { return Err(fault("unicode")); }
                    let valid = !s.is_empty() && match op {
                        "isdigit" => s.chars().all(|c| c.is_ascii_digit()), "isalpha" => s.chars().all(|c| c.is_ascii_alphabetic()),
                        "isalnum" => s.chars().all(|c| c.is_ascii_alphanumeric()), "isspace" => s.chars().all(|c| c.is_ascii_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)),
                        "islower" => s.chars().any(|c| c.is_ascii_lowercase()) && !s.chars().any(|c| c.is_ascii_uppercase()),
                        _ => s.chars().any(|c| c.is_ascii_uppercase()) && !s.chars().any(|c| c.is_ascii_lowercase()),
                    };
                    return Ok(Value::Flag(valid));
                }
                "center" | "ljust" | "rjust" | "zfill" => {
                    arity(1,if op=="zfill" {1} else {2})?;
                    let width = integer(&a[0],fault)?.max(0) as usize;
                    if width > 1_000_000 { return Err(fault("unready")); }
                    let fill = a.get(1).map(|v|text(v,fault)).transpose()?.unwrap_or_else(|| " ".to_string());
                    if fill.chars().count()!=1 { return Err(fault("fill")); }
                    let extra = width.saturating_sub(s.chars().count());
                    if op=="zfill" {
                        let (sign, rest) = if s.starts_with(['+','-']) { (&s[..1],&s[1..]) } else { ("",s.as_ref()) };
                        format!("{sign}{}{rest}", "0".repeat(extra))
                    } else {
                        let before = match op { "rjust" => extra, "center" => extra/2+(extra%2)*(width%2), _=>0 };
                        format!("{}{}{}",fill.repeat(before),s,fill.repeat(extra-before))
                    }
                }
                _ => return Err(fault("attribute")),
            };
            Ok(Value::text(&answer))
        }
        Value::Array(row) => {
            let mut row = row.as_ref().clone();
            match op {
                "append" => { arity(1,1)?; row.push(a[0].clone()); }
                "extend" => { arity(1,1)?; row.extend(members(&a[0],fault)?); }
                "insert" => { arity(2,2)?; let at=bound(integer(&a[0],fault)?,row.len()); row.insert(at,a[1].clone()); }
                "pop" => { arity(0,1)?; if row.is_empty() { return Err(fault("pop")); } let n=a.first().map(|v|integer(v,fault)).transpose()?.unwrap_or(-1); let at=if n<0 { n.saturating_add(row.len() as i64) } else {n}; if at<0 || at as usize>=row.len() {return Err(fault("index"));} let item=row.remove(at as usize); store(Value::array(row))?; return Ok(item); }
                "index" | "count" | "remove" => {
                    arity(1,if op=="index" {3} else {1})?;
                    let lo=a.get(1).map(|v|integer(v,fault)).transpose()?.map_or(0,|n|bound(n,row.len()));
                    let hi=a.get(2).map(|v|integer(v,fault)).transpose()?.map_or(row.len(),|n|bound(n,row.len()));
                    let hits:Vec<usize>=(lo..hi).filter(|i|alike(&row[*i],&a[0])).collect();
                    if op=="count" {return Ok(Value::Small(hits.len() as i64));}
                    let at=*hits.first().ok_or_else(||fault(if op=="remove" {"remove"} else {"list_index"}))?;
                    if op=="index" {return Ok(Value::Small(at as i64));} row.remove(at);
                }
                "reverse" => {arity(0,0)?;row.reverse();}
                "clear" => {arity(0,0)?;row.clear();}
                "copy" => {arity(0,0)?;return Ok(Value::array(row).held(true));}
                _ => return Err(fault("attribute")),
            }
            store(Value::array(row))
        }
        Value::Map(pairs) => {
            let mut pairs=pairs.as_ref().clone();
            match op {
                "get" | "setdefault" | "pop" => {
                    arity(1,2)?;
                    if matches!(a[0].contents(), Value::Array(_) | Value::Map(_)) {return Err(fault("arguments"));}
                    let at=pairs.iter().position(|(k,_)| alike(k,&a[0]));
                    if let Some(at)=at {let value=pairs[at].1.clone();if op=="pop" {pairs.remove(at);store(Value::Map(Rc::new(pairs)))?;} return Ok(value);}
                    let value=a.get(1).cloned().unwrap_or(Value::Null);
                    if op=="pop" && a.len()==1 {return Err(fault("key")+&a[0].representation(words));}
                    if op=="setdefault" {pairs.push((a[0].clone(),value.clone()));store(Value::Map(Rc::new(pairs)))?;} return Ok(value);
                }
                "keys" | "values" | "items" => {arity(0,0)?;return Ok(Value::View(Rc::new((receiver.clone(),op.to_string()))));}
                "copy" => {arity(0,0)?;return Ok(Value::Map(Rc::new(pairs)).held(true));}
                "clear" => {arity(0,0)?;pairs.clear();}
                "update" => {
                    arity(0,1)?;let mut entries=Vec::new();
                    if let Some(v)=a.first() {entries=match v.contents() {Value::Map(p)=>p.as_ref().clone(),v=>{let mut out=Vec::new();for item in members(&v,fault)? {let pair=members(&item,fault)?;if pair.len()!=2{return Err(fault("arguments"));}out.push((pair[0].clone(),pair[1].clone()));}out}};}
                    entries.extend(names.iter().map(|(k,v)|(Value::text(k),v.clone())));
                    for (k,v) in entries {if let Some(at)=pairs.iter().position(|(key,_)|key.equals(&k)){pairs[at].1=v;}else{pairs.push((k,v));}}
                }
                _ => return Err(fault("attribute")),
            }
            store(Value::Map(Rc::new(pairs)))
        }
        Value::Small(_) | Value::Huge(_) | Value::Real(_) => {
            arity(0,0)?;
            match op {
                "bit_count" if !matches!(held, Value::Real(_)) => {
                    let count: u64 = held.as_big()?.to_bytes_le().1.iter().map(|byte| u64::from(byte.count_ones())).sum();
                    Ok(Value::of_big(BigInt::from(count)))
                },
                "bit_length" if !matches!(held,Value::Real(_)) => Ok(Value::Small(held.as_big()?.bits() as i64)),
                "is_integer" => Ok(Value::Flag(match &held {Value::Real(r)=>!r.outside() && (&r.p % &r.q).is_zero(),_=>true})),
                "as_integer_ratio" => {let (p,q)=match &held {Value::Real(r) if !r.outside()=>crate::value::from_binary(crate::value::as_binary(&r.p,&r.q)).ok_or_else(||fault("unready"))?,Value::Real(_)=>return Err(fault("unready")),_=>(held.as_big()?,BigInt::from(1))};let divisor=p.gcd(&q);Ok(Value::Tuple(Rc::new(vec![Value::of_big(p/&divisor),Value::of_big(q/divisor)])))},
                "hex" if matches!(held,Value::Real(_)) => {
                    let Value::Real(r) = &held else { unreachable!() };
                    let n=crate::value::as_binary(&r.p,&r.q);
                    let bits=n.to_bits();let minus=if n.is_sign_negative() || r.below { "-" } else { "" };
                    let shown=if n.is_nan() {"nan".to_string()} else if n.is_infinite() {format!("{minus}inf")} else if n==0.0 {format!("{minus}0x0.0p+0")}
                        else {let power=((bits>>52)&0x7ff) as i32;let (lead,exponent)=if power==0 {(0,-1022)}else{(1,power-1023)};format!("{minus}0x{lead}.{:013x}p{exponent:+}",bits&0xfffffffffffff)};
                    Ok(Value::text(&shown))
                },
                _=>Err(fault("attribute")),
            }
        }
        _ => Err(fault("attribute")),
    }
}

fn format_fields(template: &str, args: &[Value], names: &[(String,Value)], words: &Wording, fault: &dyn Fn(&str)->String) -> Result<String,String> {
    let mut out=String::new();let mut chars=template.chars().peekable();let mut next=0;let mut numbered=false;let mut automatic=false;
    while let Some(c)=chars.next() {
        if c!='{' && c!='}' {out.push(c);continue;}
        if chars.peek()==Some(&c) {chars.next();out.push(c);continue;}
        if c=='}' {return Err(fault("format"));}
        let mut field=String::new();let mut closed=false;
        for c in chars.by_ref() {if c=='}' {closed=true;break;}if c=='{' {return Err(fault("spec"));}field.push(c);}
        if !closed {return Err(fault("format"));}
        let (key,spec)=field.split_once(':').unwrap_or((&field,""));
        let (key,conversion)=key.split_once('!').unwrap_or((key,""));
        if !matches!(conversion,""|"s"|"r"|"a") {return Err(fault("format"));}
        let value=if key.is_empty() {if numbered{return Err(fault("mixed"));}automatic=true;let at=next;next+=1;args.get(at).ok_or_else(||fault("missing"))?}
            else if let Ok(at)=key.parse::<usize>() {if automatic{return Err(fault("mixed"));}numbered=true;args.get(at).ok_or_else(||fault("missing"))?}
            else {if key.contains(['.','[']) {return Err(fault("unready"));} &names.iter().find(|(n,_)|n==key).ok_or_else(||fault("key")+&Value::text(key).representation(words))?.1};
        let value=value.contents();
        let rendered=if spec=="x" {match &value {Value::Small(_) | Value::Huge(_)=>value.as_big()?.to_str_radix(16),_=>return Err(fault("arguments"))}}
            else if spec=="," {let raw=match &value {Value::Small(_) | Value::Huge(_)=>value.as_big()?.to_string(),_=>return Err(fault("spec"))};let (sign,digits)=raw.strip_prefix('-').map_or(("",raw.as_str()),|n|("-",n));let mut grouped=String::new();for (i,c) in digits.chars().enumerate(){if i>0 && (digits.len()-i)%3==0 {grouped.push(',');}grouped.push(c);}format!("{sign}{grouped}")}
            else {let letters:Vec<char>=spec.chars().collect();let align=if letters.first().map_or(false,|c|matches!(c,'<'|'>'|'^')) {1}else if letters.get(1).map_or(false,|c|matches!(c,'<'|'>'|'^')){2}else{0};let valid=spec.is_empty() || align>0 && letters[align..].iter().collect::<String>().parse::<usize>().map_or(false,|n|n<=100000) || spec.strip_prefix('.').and_then(|s|s.strip_suffix('f')).and_then(|s|s.parse::<usize>().ok()).map_or(false,|n|n<=1000)&&matches!(value,Value::Small(_)|Value::Huge(_)|Value::Real(_));if !valid{return Err(fault("spec"));}if spec.ends_with('f') && conversion.is_empty() {value.string_field(words,spec,conversion).ok_or_else(|| fault("spec"))?} else {
                let base=if let Value::Text(_) = value {value.string_field(words,"",conversion).ok_or_else(|| fault("spec"))?} else {shown(&value,words)};
                Value::text(&base).string_field(words,spec,"").ok_or_else(|| fault("spec"))?
            }};
        out.push_str(&rendered);
    }
    Ok(out)
}

/// Gather hexadecimal figures before rounding their exact worth once.
pub fn from_hex(args: &[Value], fault: &dyn Fn(&str) -> String) -> Answer {
    let [Value::Text(source)] = args else { return Err(fault("arguments")); };
    let text = source.trim_matches(|c: char| c.is_ascii_whitespace()).to_ascii_lowercase();
    let negative = text.starts_with('-');
    let text = text.strip_prefix(['+', '-']).unwrap_or(&text);
    let number = if matches!(text, "inf" | "infinity") { f64::INFINITY }
        else if text == "nan" { f64::NAN }
        else {
            let text = text.strip_prefix("0x").unwrap_or(text);
            let (digits, exponent) = text.split_once('p').unwrap_or((text, "0"));
            let exponent_digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            if exponent_digits.is_empty() || !exponent_digits.bytes().all(|c| c.is_ascii_digit()) { return Err(fault("hex")); }
            let power = exponent.parse::<i64>().unwrap_or(if exponent.starts_with('-') { i64::MIN } else { i64::MAX });
            let mut fraction = 0i64;
            let mut point = false;
            let mut figures = String::new();
            for c in digits.chars() {
                if c == '.' && !point { point = true; }
                else if c.is_ascii_hexdigit() { figures.push(c); if point { fraction += 4; } }
                else { return Err(fault("hex")); }
            }
            let whole = BigInt::parse_bytes(figures.as_bytes(), 16).ok_or_else(|| fault("hex"))?;
            let scale = power.saturating_sub(fraction);
            let highest = scale.saturating_add(whole.bits() as i64);
            if whole.is_zero() || highest < -1075 { 0.0 }
            else if highest > 1025 { return Err(fault("hex_overflow")); }
            else {
                let (top, bottom) = if scale >= 0 { (whole << scale as usize, BigInt::from(1)) }
                    else { (whole, BigInt::from(1) << scale.unsigned_abs() as usize) };
                let value = crate::value::as_binary(&top, &bottom);
                if value.is_infinite() { return Err(fault("hex_overflow")); }
                value
            }
        };
    let number = if negative { -number } else { number };
    let (p, q) = crate::value::from_binary(number).unwrap_or_else(|| (BigInt::from(if number.is_nan() { 0 } else if negative { -1 } else { 1 }), BigInt::from(0)));
    Ok(Value::Real(Rc::new(crate::value::Real { p, q, places: 16, below: negative, point: true })))
}
