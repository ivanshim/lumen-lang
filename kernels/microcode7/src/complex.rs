// Two coordinates give a complex worth. Each reckoning returns a fresh
// pair; neither coordinate may be written through a member name.

use crate::{data, math};
use crate::data::Value;
use crate::form::Prim;
use crate::table::Table;
use std::rc::Rc;
use num_traits::ToPrimitive;

pub fn pair(t: &Table, x: f64, y: f64) -> Value {
    Value::Complex(Rc::new((x,y,Rc::from(complaint(t,"integer")))))
}

pub fn coordinates(v: &Value) -> Option<(f64,f64)> {
    match v {
        Value::Complex(p) => Some((p.0,p.1)),
        Value::Flag(b) => Some((if *b { 1.0 } else { 0.0 },0.0)),
        Value::Huge(n) => n.to_f64().filter(|number| number.is_finite()).map(|x| (x,0.0)),
        Value::Small(n) => Some((*n as f64,0.0)),
        Value::Frac(r) if r.places.is_some() => {
            let x = if r.under && r.above == 0.into() && r.beneath != 0.into() { -0.0 } else { data::nearest_binary(&r.above,&r.beneath) };
            Some((x,0.0))
        }
        _ => None,
    }
}

pub fn written(p: &(f64,f64,Rc<str>)) -> String {
    let coefficient = |x: f64| if x.is_nan() { String::from("nan") } else { data::brief_decimal(x) };
    let negative = p.1 < 0.0 || p.1 == 0.0 && p.1.is_sign_negative();
    let imaginary = coefficient(p.1.abs()) + "j";
    match (p.0 == 0.0, p.0.is_sign_negative()) {
        (true,false) => if negative { "-".to_owned()+&imaginary } else { imaginary },
        _ => format!("({}{}{})",coefficient(p.0),if negative { '-' } else { '+' },imaginary),
    }
}

pub fn complaint(t: &Table, ending: &str) -> String {
    if ending == "arguments" {
        return t.around("ext.builtin.core.arity").map_or_else(String::new, |(before,after)| {
            [before, t.single("ext.builtin.complex").unwrap_or_default(), after].concat()
        });
    }
    t.single(&format!("ext.builtin.complex.{ending}")).unwrap_or_default().to_owned()
}

pub fn floor(t: &Table, left: &Value, right: &Value, symbol: &str) -> String {
    let pieces = t.strings("ext.stmt.class.binary.amiss");
    [pieces[0].as_str(), symbol, pieces[1].as_str(), &left.kind_word(), pieces[2].as_str(), &right.kind_word(), pieces[3].as_str()].concat()
}

fn boxed(number: f64) -> f64 {
    let size: f64 = if number.is_infinite() { 1.0 } else { 0.0 };
    size.copysign(number)
}

fn quotient(top: (f64,f64), bottom: (f64,f64), scalar: bool) -> (f64,f64) {
    let (r,i) = bottom;
    let computed;
    if r.abs() >= i.abs() {
        let scale = i/r;
        let divisor = r + i*scale;
        computed = if scalar { (top.0/divisor, -top.0*scale/divisor) }
            else { ((top.0+top.1*scale)/divisor,(top.1-top.0*scale)/divisor) };
    } else if i.abs() >= r.abs() {
        let scale = r/i;
        let divisor = i + r*scale;
        computed = if scalar { (top.0*scale/divisor, -top.0/divisor) }
            else { ((top.0*scale+top.1)/divisor,(top.1*scale-top.0)/divisor) };
    } else { computed = (f64::NAN,f64::NAN); }
    if !(computed.0.is_nan() && computed.1.is_nan()) { return computed; }
    let (a,b) = top;
    match (scalar, a.is_infinite() || b.is_infinite(), r.is_finite() && i.is_finite()) {
        (false,true,true) => (f64::INFINITY*(boxed(a)*r+boxed(b)*i), f64::INFINITY*(boxed(b)*r-boxed(a)*i)),
        _ if a.is_finite() && b.is_finite() && (r.is_infinite() || i.is_infinite()) => {
            if scalar { (0.0*(a*boxed(r)), 0.0*(-a*boxed(i))) }
            else { (0.0*(a*boxed(r)+b*boxed(i)), 0.0*(b*boxed(r)-a*boxed(i))) }
        }
        _ => computed,
    }
}

fn product(x: (f64,f64), y: (f64,f64)) -> (f64,f64) {
    let terms = [x.0*y.0, x.1*y.1, x.0*y.1, x.1*y.0];
    let answer = (terms[0]-terms[1], terms[2]+terms[3]);
    if answer.0.is_nan() && answer.1.is_nan() {
        let mut values = [x.0,x.1,y.0,y.1];
        let mut redo = false;
        for offset in [0,2] {
            if values[offset].is_infinite() || values[offset+1].is_infinite() {
                values[offset] = boxed(values[offset]);
                values[offset+1] = boxed(values[offset+1]);
                redo = true;
            }
        }
        redo |= terms.iter().any(|term| term.is_infinite());
        if redo {
            for v in &mut values { if v.is_nan() { *v = 0.0_f64.copysign(*v); } }
            let [a,b,c,d] = values;
            return (f64::INFINITY*(a*c-b*d),f64::INFINITY*(a*d+b*c));
        }
    }
    answer
}

pub fn reckon(t: &Table, op: Prim, values: &[Value]) -> Result<Value,String> {
    let first = &values[0];
    if matches!(op,Prim::Positive | Prim::NumberAlone) { return Ok(first.clone()); }
    if op == Prim::Negate {
        let (r,i) = coordinates(first).ok_or_else(|| number_error(t,first))?;
        return Ok(pair(t, -r,-i));
    }
    let second = values.get(1).ok_or_else(|| complaint(t,"unready"))?;
    if matches!(op,Prim::IntDiv | Prim::Mod) { return Err(floor(t,first,second, match op { Prim::Mod => "%", _ => "//" })); }
    if matches!(op,Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
        let symbol = match op { Prim::Ge => ">=", Prim::Gt => ">", Prim::Le => "<=", _ => "<" };
        let words = t.strings("ext.builtin.complex.order");
        return Err([words[0].as_str(),symbol,words[1].as_str(),&first.kind_word(),words[2].as_str(),&second.kind_word(),words[3].as_str()].concat());
    }
    let x = coordinates(first).ok_or_else(|| number_error(t,first))?;
    let y = coordinates(second).ok_or_else(|| number_error(t,second))?;
    let lhs_complex = matches!(first, Value::Complex(_));
    let rhs_complex = matches!(second, Value::Complex(_));
    let finite = x.0.is_finite() && x.1.is_finite() && y.0.is_finite() && y.1.is_finite();
    let result = match op {
        Prim::Plus => (x.0+y.0, if !rhs_complex { x.1 } else if !lhs_complex { y.1 } else { x.1+y.1 }),
        Prim::Minus => (x.0-y.0, if !rhs_complex { x.1 } else if !lhs_complex { -y.1 } else { x.1-y.1 }),
        Prim::Times => if !rhs_complex { (x.0*y.0,x.1*y.0) } else if !lhs_complex { (x.0*y.0,x.0*y.1) } else { product(x,y) },
        Prim::Over | Prim::OverReal => {
            if y == (0.0,0.0) { return Err(complaint(t,"zero")); }
            if rhs_complex { quotient(x,y,!lhs_complex) } else { (x.0/y.0,x.1/y.0) }
        }
        Prim::Power => {
            let exponent = y.0;
            if y == (0.0,0.0) { (1.0,0.0) }
            else if y == (1.0,0.0) { x }
            else if y.1 == 0.0 && exponent.abs() <= 100.0 && exponent == exponent.trunc() {
                if x == (0.0,0.0) && exponent < 0.0 { return Err(complaint(t,"power.zero")); }
                let mut factor = x;
                let mut count = exponent.abs() as u64;
                while count % 2 == 0 { factor = product(factor,factor); count /= 2; }
                let mut answer = factor;
                while count > 1 {
                    count /= 2;
                    factor = product(factor,factor);
                    if count % 2 == 1 { answer = product(answer,factor); }
                }
                if exponent.is_sign_negative() { quotient((1.0,0.0),answer,true) } else { answer }
            } else if x == (0.0,0.0) {
                if exponent < 0.0 || y.1 != 0.0 { return Err(complaint(t,"power.zero")); }
                (0.0,0.0)
            } else {
                let distance = x.0.hypot(x.1);
                let bearing = x.1.atan2(x.0);
                let (mut radius,mut direction) = (distance.powf(exponent),exponent*bearing);
                if y.1 != 0.0 {
                    radius *= (-y.1*bearing).exp();
                    direction += distance.ln()*y.1;
                }
                (radius*direction.cos(),radius*direction.sin())
            }
        }
        _ => return Err(complaint(t,"unready")),
    };
    if finite && op == Prim::Power && (result.0.is_infinite() || result.1.is_infinite()) {
        return Err(complaint(t,"power.overflow"));
    }
    Ok(pair(t, result.0,result.1))
}

fn number_error(t: &Table, v: &Value) -> String {
    match v {
        Value::Huge(_) => complaint(t,"integer.overflow"),
        _ => t.single("ext.system.fault.operands").unwrap_or_default().to_owned(),
    }
}

fn decimal(source: &str) -> Option<f64> {
    let mut clean = String::new();
    let mut letters = source.chars().peekable();
    let mut digit = false;
    while let Some(c) = letters.next() {
        if c == '_' {
            if !digit || !letters.peek().map_or(false, |n| n.is_ascii_digit()) { return None; }
        } else { clean.push(c); }
        digit = c.is_ascii_digit();
    }
    clean.parse::<f64>().ok()
}

fn from_chars(chars: &str) -> Option<(f64,f64)> {
    let mut text = chars.trim();
    if text.starts_with('(') && text.ends_with(')') { text = text[1..text.len()-1].trim(); }
    if text.chars().any(char::is_whitespace) { return None; }
    match text.chars().last() {
        Some('j' | 'J') => {
            let coefficient = &text[..text.len()-1];
            let mut before = '\0';
            let mut dividing = None;
            for (offset,c) in coefficient.char_indices() {
                if offset > 0 && matches!(c,'+' | '-') && !matches!(before,'e' | 'E') { dividing = Some(offset); break; }
                before = c;
            }
            let (real, imag) = match dividing {
                Some(at) => (decimal(&coefficient[..at])?,&coefficient[at..]),
                None => (0.0,coefficient),
            };
            Some((real,match imag { "+" | "" => 1.0, "-" => -1.0, s => decimal(s)? }))
        }
        _ => decimal(text).map(|real| (real,0.0)),
    }
}

pub fn create(t: &Table, input: &[Value]) -> Result<Value,String> {
    match input {
        [] => Ok(pair(t, 0.0,0.0)),
        [Value::Text(s)] => {
            let parsed = from_chars(s).ok_or_else(|| complaint(t,"invalid"))?;
            Ok(pair(t, parsed.0,parsed.1))
        }
        [value @ Value::Complex(_)] => Ok(value.clone()),
        [one] => coordinates(one).map(|p| pair(t, p.0,p.1)).ok_or_else(|| complaint(t,"arguments")),
        [one,two] => {
            let left = coordinates(one).ok_or_else(|| complaint(t,"arguments"))?;
            let right = coordinates(two).ok_or_else(|| complaint(t,"arguments"))?;
            let horizontal = if matches!(two, Value::Complex(_)) { left.0-right.1 } else { left.0 };
            let vertical = if matches!(one, Value::Complex(_)) { left.1+right.0 } else { right.0 };
            Ok(pair(t, horizontal,vertical))
        }
        _ => Err(complaint(t,"arguments")),
    }
}

pub fn decimal_value(number: f64) -> Value {
    let value = data::worth_of_binary(number,math::DEFAULT_PLACES);
    if let Value::Frac(mut parts) = value {
        Rc::make_mut(&mut parts).pointed = true;
        Value::Frac(parts)
    } else { value }
}

/// Keep the kind which the definition itself gave the complaint.
pub fn already_named(table: &Table, words: &str) -> bool {
    if !table.has_any("ext.builtin.complex") { return false; }
    for ending in ["invalid", "arguments", "integer", "integer.overflow", "power.zero", "power.overflow", "power.modulo", "zero", "unready"] {
        let expected = complaint(table, ending);
        if !expected.is_empty() && words == expected { return true; }
    }
    if !words.contains("'complex'") { return false; }
    for label in ["ext.builtin.complex.floor", "ext.builtin.complex.order"] {
        let fragments = table.strings(label);
        if let (Some(first), Some(last)) = (fragments.first(), fragments.last()) {
            if !first.is_empty() && words.starts_with(first) && words.ends_with(last) { return true; }
        }
    }
    false
}
