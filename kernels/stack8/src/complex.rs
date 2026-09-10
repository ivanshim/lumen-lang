// A number with two real parts. The writer borrows the real writer's
// choice of figures, retaining the sign of either nought.

use std::rc::Rc;
use crate::value::{Value, real_of, as_binary, shortest_real};
use crate::code::Action;
use crate::lang::Lang;
use num_traits::ToPrimitive;

#[derive(Debug)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
    pub integer_fault: Rc<str>,
}

pub fn made(lang: &Lang, real: f64, imag: f64) -> Value {
    Value::Complex(Rc::new(Complex { real, imag, integer_fault: Rc::from(fault(lang, "integer")) }))
}

pub fn parts(value: &Value) -> Option<(f64, f64)> {
    Some(match value {
        Value::Complex(z) => (z.real, z.imag),
        Value::Small(n) => (*n as f64, 0.0),
        Value::Huge(n) => (n.to_f64().filter(|x| x.is_finite())?, 0.0),
        Value::Flag(b) => (u8::from(*b) as f64, 0.0),
        Value::Real(r) => (if r.p == 0.into() && r.q != 0.into() && r.below { -0.0 } else { as_binary(&r.p, &r.q) }, 0.0),
        _ => return None,
    })
}

pub fn shown(z: &Complex) -> String {
    let imag = if z.imag.is_nan() { "nan".into() } else { shortest_real(z.imag.abs()) };
    let minus = z.imag.is_sign_negative() && !z.imag.is_nan();
    if z.real == 0.0 && !z.real.is_sign_negative() {
        format!("{}{}j", if minus { "-" } else { "" }, imag)
    } else {
        let real = if z.real.is_nan() { "nan".into() } else { shortest_real(z.real) };
        format!("({}{}{}j)", real, if minus { "-" } else { "+" }, imag)
    }
}

pub fn fault(lang: &Lang, tail: &str) -> String {
    if tail == "arguments" {
        let words = &lang.core_words["core.arity"];
        let name = lang.complex_words["ext.builtin.complex"].first().map_or("", String::as_str);
        return match words.as_slice() { [head, end, ..] => format!("{head}{name}{end}"), _ => String::new() };
    }
    lang.complex_words.get(&format!("ext.builtin.complex.{tail}")).and_then(|w| w.first()).cloned().unwrap_or_default()
}

pub fn floor_fault(lang: &Lang, a: &Value, b: &Value) -> String {
    let words = &lang.complex_words["ext.builtin.complex.floor"];
    format!("{}{}{}{}{}", words[0], a.core_kind(), words[1], b.core_kind(), words[2])
}

fn divided(a: f64, b: f64, c: f64, d: f64) -> (f64, f64) {
    if c.abs() >= d.abs() {
        let ratio = d / c;
        let denom = c + d * ratio;
        ((a + b * ratio) / denom, (b - a * ratio) / denom)
    } else {
        let ratio = c / d;
        let denom = c * ratio + d;
        ((a * ratio + b) / denom, (b * ratio - a) / denom)
    }
}

pub fn work(lang: &Lang, op: &Action, left: &Value, right: &Value) -> Result<Value, String> {
    if matches!(op, Action::Lt | Action::Le | Action::Gt | Action::Ge) {
        let sign = match op { Action::Lt => "<", Action::Le => "<=", Action::Gt => ">", _ => ">=" };
        let w = &lang.complex_words["ext.builtin.complex.order"];
        return Err(format!("{}{}{}{}{}{}{}", w[0], sign, w[1], left.core_kind(), w[2], right.core_kind(), w[3]));
    }
    if matches!(op, Action::IntDiv | Action::Mod) { return Err(floor_fault(lang, left, right)); }
    let (a,b) = parts(left).ok_or_else(|| fault(lang, "unready"))?;
    let (c,d) = parts(right).ok_or_else(|| fault(lang, "unready"))?;
    let l = matches!(left, Value::Complex(_));
    let r = matches!(right, Value::Complex(_));
    if [a,b,c,d].iter().any(|x| !x.is_finite())
        && (matches!(op, Action::Mul) && l && r || matches!(op, Action::Div | Action::DivReal) && r
            || matches!(op, Action::Power) && (d != 0.0 || c != 0.0 && c != 1.0)) { return Err(fault(lang, "unready")); }
    let (x,y) = match op {
        Action::Add if !r => (a+c,b),
        Action::Add if !l => (a+c,d),
        Action::Sub if !r => (a-c,b),
        Action::Sub if !l => (a-c,-d),
        Action::Mul if !r => (a*c,b*c),
        Action::Mul if !l => (a*c,a*d),
        Action::Div | Action::DivReal if !r => {
            if c == 0.0 { return Err(fault(lang, "zero")); }
            (a/c,b/c)
        }
        Action::Add => (a+c, b+d),
        Action::Sub => (a-c, b-d),
        Action::Mul => (a*c-b*d, a*d+b*c),
        Action::Div | Action::DivReal => {
            if c == 0.0 && d == 0.0 { return Err(fault(lang, "zero")); }
            divided(a,b,c,d)
        }
        Action::Power => {
            if c == 0.0 && d == 0.0 { (1.0, 0.0) }
            else if c == 1.0 && d == 0.0 { (a,b) }
            else if a == 0.0 && b == 0.0 {
                if c < 0.0 || d != 0.0 { return Err(fault(lang, "power.zero")); }
                (0.0,0.0)
            } else if d == 0.0 && c.fract() == 0.0 && c.abs() <= 100.0 {
                let (mut x,mut y) = (1.0,0.0);
                let (mut u,mut v) = (a,b);
                let mut n = c.abs() as u32;
                while n != 0 {
                    if n & 1 != 0 { (x,y) = (x*u-y*v, x*v+y*u); }
                    n >>= 1;
                    if n != 0 { (u,v) = (u*u-v*v, 2.0*u*v); }
                }
                if c < 0.0 { divided(1.0,0.0,x,y) } else { (x,y) }
            } else {
                let magnitude = a.hypot(b);
                let argument = b.atan2(a);
                let mut length = magnitude.powf(c);
                let mut angle = argument*c;
                if d != 0.0 {
                    length *= (-argument*d).exp();
                    angle += d*magnitude.ln();
                }
                (length*angle.cos(), length*angle.sin())
            }
        }
        _ => return Err(fault(lang, "unready")),
    };
    if matches!(op, Action::Power) && [a,b,c,d].iter().all(|n| n.is_finite()) && (!x.is_finite() || !y.is_finite()) { return Err(fault(lang, "unready")); }
    Ok(made(lang, x,y))
}

fn coefficient(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    for (i,b) in bytes.iter().enumerate() {
        if *b == b'_' && (i == 0 || i+1 == bytes.len() || !bytes[i-1].is_ascii_digit() || !bytes[i+1].is_ascii_digit()) { return None; }
    }
    s.replace('_', "").parse().ok()
}

fn read(text: &str) -> Option<(f64,f64)> {
    let text = text.trim();
    let text = if text.starts_with('(') { text.strip_prefix('(')?.strip_suffix(')')?.trim() } else { text };
    if text.chars().any(char::is_whitespace) { return None; }
    if !text.ends_with(['j','J']) { return Some((coefficient(text)?,0.0)); }
    let body = &text[..text.len()-1];
    let split = body.char_indices().skip(1).find(|(at,c)| (*c == '+' || *c == '-') && !matches!(body.as_bytes()[at-1], b'e' | b'E'));
    let (real, imag) = if let Some((at,_)) = split { (coefficient(&body[..at])?, &body[at..]) } else { (0.0, body) };
    let imag = match imag { "" | "+" => 1.0, "-" => -1.0, s => coefficient(s)? };
    Some((real,imag))
}

pub fn construct(lang: &Lang, values: &[Value]) -> Result<Value,String> {
    if values.is_empty() { return Ok(made(lang, 0.0,0.0)); }
    if values.len() > 2 { return Err(fault(lang, "arguments")); }
    if let Value::Text(text) = &values[0] {
        if values.len() != 1 { return Err(fault(lang, "arguments")); }
        let (a,b) = read(text).ok_or_else(|| fault(lang, "invalid"))?;
        return Ok(made(lang, a,b));
    }
    let (a,b) = parts(&values[0]).ok_or_else(|| fault(lang, "arguments"))?;
    if values.len() == 1 {
        return Ok(if matches!(values[0], Value::Complex(_)) { values[0].clone() } else { made(lang, a,b) });
    }
    let (c,d) = parts(&values[1]).ok_or_else(|| fault(lang, "arguments"))?;
    Ok(made(lang, if matches!(values[1], Value::Complex(_)) { a-d } else { a },
        if matches!(values[0], Value::Complex(_)) { b+c } else { c }))
}

pub fn real(n: f64) -> Value {
    let mut value = real_of(n, crate::arith::DEFAULT_PLACES);
    if let Value::Real(r) = &mut value { Rc::make_mut(r).point = true; }
    value
}

/// A complaint already bearing its kind needs no outer banner.
pub fn says(lang: &Lang, message: &str) -> bool {
    if lang.complex_words["ext.builtin.complex"].is_empty() { return false; }
    ["arguments", "invalid", "integer", "zero", "power.zero", "unready"].iter()
        .any(|tail| { let words = fault(lang, tail); !words.is_empty() && message == words })
        || ["order", "floor"].iter().any(|tail| {
            let Some(words) = lang.complex_words.get(&format!("ext.builtin.complex.{tail}")) else { return false };
            words.len() > 1 && message.starts_with(&words[0]) && message.ends_with(words.last().unwrap())
                && message.contains("'complex'")
        })
}
