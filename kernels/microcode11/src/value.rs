// Values: the closed set the tree computes with.
//
// Small integers are immediate; everything with a heap body sits behind a
// reference count so a value is cheap to pass and an array is copied only
// when a shared one is written.

use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::tree::Program;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tag {
    Integer,
    Rational,
    Real,
    Text,
    Boolean,
    Array,
    Null,
}

impl Tag {
    pub fn name(self) -> &'static str {
        match self {
            Tag::Integer => "INTEGER",
            Tag::Rational => "RATIONAL",
            Tag::Real => "REAL",
            Tag::Text => "STRING",
            Tag::Boolean => "BOOLEAN",
            Tag::Array => "ARRAY",
            Tag::Null => "NULL",
        }
    }
}

/// An exact ratio in lowest terms; `digits` is set for a real.
#[derive(Debug, Clone)]
pub struct Ratio {
    pub num: BigInt,
    pub den: BigInt,
    pub digits: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Small(i64),
    Large(Rc<BigInt>),
    /// A rational (digits None) or a real (digits Some).
    Fraction(Rc<Ratio>),
    Text(Rc<str>),
    Truth(bool),
    Nothing,
    Array(Rc<Vec<Value>>),
    Routine(Rc<Program>),
    Tag(Tag),
    /// An empty slot; never a value a program sees.
    Empty,
}

/// How a language spells the literal values when printing.
#[derive(Clone, Copy)]
pub struct Literals<'a> {
    pub yes: &'a str,
    pub no: &'a str,
    pub none: &'a str,
}

impl Value {
    pub fn from_big(n: BigInt) -> Value {
        match n.to_i64() {
            Some(i) => Value::Small(i),
            None => Value::Large(Rc::new(n)),
        }
    }

    pub fn from_text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn tag(&self) -> Option<Tag> {
        Some(match self {
            Value::Small(_) | Value::Large(_) => Tag::Integer,
            Value::Fraction(f) => if f.digits.is_some() { Tag::Real } else { Tag::Rational },
            Value::Text(_) => Tag::Text,
            Value::Truth(_) => Tag::Boolean,
            Value::Array(_) => Tag::Array,
            Value::Nothing | Value::Tag(_) => Tag::Null,
            Value::Routine(_) | Value::Empty => return None,
        })
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Truth(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Large(n) => !n.is_zero(),
            Value::Fraction(f) => !f.num.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::Nothing | Value::Empty => false,
            _ => true,
        }
    }

    /// The whole number a value stands for where one is obvious.
    pub fn whole(&self) -> Result<BigInt, String> {
        Ok(match self {
            Value::Small(n) => BigInt::from(*n),
            Value::Large(n) => (**n).clone(),
            Value::Fraction(f) if f.digits.is_some() => &f.num / &f.den,
            Value::Fraction(_) => return Err("Cannot coerce rational to integer".to_string()),
            Value::Truth(b) => BigInt::from(*b as i64),
            Value::Nothing | Value::Empty => BigInt::zero(),
            Value::Text(s) => s.parse().map_err(|_| format!("Cannot coerce '{}' to number", s))?,
            Value::Array(_) => return Err("Cannot coerce array to number".to_string()),
            Value::Routine(_) => return Err("Cannot coerce function to number".to_string()),
            Value::Tag(_) => return Err("Cannot coerce kind meta-value to number".to_string()),
        })
    }

    pub fn equals(&self, other: &Value) -> bool {
        if let (Some(a), Some(b)) = (crate::numeric::ratio(self), crate::numeric::ratio(other)) {
            return a.num * b.den == b.num * a.den;
        }
        match (self, other) {
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Truth(a), Value::Truth(b)) => a == b,
            (Value::Nothing, Value::Nothing) => true,
            (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a, b),
            (Value::Tag(a), Value::Tag(b)) => a == b,
            _ => false,
        }
    }

    /// The text `print` shows.
    pub fn show(&self, lit: Literals) -> String {
        match self {
            Value::Truth(true) => lit.yes.to_string(),
            Value::Truth(false) => lit.no.to_string(),
            Value::Nothing | Value::Empty => lit.none.to_string(),
            Value::Array(items) => {
                let parts: Vec<String> = items.iter().map(|v| v.show(lit)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Small(n) => n.to_string(),
            Value::Large(n) => n.to_string(),
            Value::Fraction(f) => match f.digits {
                Some(d) => real_text(&f.num, &f.den, d),
                None => format!("{}/{}", f.num, f.den),
            },
            Value::Text(s) => s.to_string(),
            Value::Routine(p) => format!("<function({})>", p.params.join(", ")),
            Value::Tag(t) => t.name().to_string(),
        }
    }

    /// A key for the call cache.
    pub fn key(&self, into: &mut String) {
        match self {
            Value::Text(s) => {
                into.push('"');
                into.push_str(s);
                into.push('"');
            }
            Value::Array(items) => {
                into.push('[');
                for v in items.iter() {
                    v.key(into);
                    into.push(';');
                }
                into.push(']');
            }
            Value::Routine(p) => into.push_str(&format!("@{:p}", Rc::as_ptr(p))),
            other => into.push_str(&other.show(Literals { yes: "true", no: "false", none: "null" })),
        }
        into.push(',');
    }
}

/// A real's decimal text: the whole part, then fraction digits up to the
/// precision counted in significant digits, no padding.
pub fn real_text(num: &BigInt, den: &BigInt, digits: usize) -> String {
    let whole = num / den;
    let mut rest = (num - &whole * den).abs();
    if rest.is_zero() {
        return whole.to_string();
    }
    let sign = if num.is_negative() && whole.is_zero() { "-" } else { "" };
    let mut out = format!("{}{}.", sign, whole);
    let mut room = digits.saturating_sub(whole.to_string().trim_start_matches('-').len());
    while room > 0 && !rest.is_zero() {
        rest *= 10;
        let d = &rest / den;
        out.push_str(&d.to_string());
        rest -= &d * den;
        room -= 1;
    }
    out
}
