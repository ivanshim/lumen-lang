// Values. Small integers are unboxed; everything larger sits behind a
// reference count, so the stack moves pointers. Arrays copy when written
// through a shared reference, which a taking load avoids.

use std::fmt::Write as _;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::words::Program;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Integer,
    Rational,
    Real,
    Text,
    Boolean,
    Array,
    Null,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Integer => "INTEGER",
            Kind::Rational => "RATIONAL",
            Kind::Real => "REAL",
            Kind::Text => "STRING",
            Kind::Boolean => "BOOLEAN",
            Kind::Array => "ARRAY",
            Kind::Null => "NULL",
        }
    }
}

/// p/q in lowest terms, q > 1.
#[derive(Debug, Clone)]
pub struct Ratio {
    pub p: BigInt,
    pub q: BigInt,
}

/// p/q shown to `places` significant digits.
#[derive(Debug, Clone)]
pub struct Real {
    pub p: BigInt,
    pub q: BigInt,
    pub places: usize,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Big(Rc<BigInt>),
    Ratio(Rc<Ratio>),
    Real(Rc<Real>),
    Str(Rc<str>),
    Bool(bool),
    Null,
    List(Rc<Vec<Value>>),
    Program(Rc<Program>),
    Kind(Kind),
    /// A slot nothing was stored in.
    Empty,
    /// A slot whose value a taking load moved out; the next store fills it.
    Hole,
    /// The bottom of an array literal being gathered.
    Mark,
}

/// How a language spells the literal values when printing.
pub struct Spelling<'a> {
    pub yes: &'a str,
    pub no: &'a str,
    pub none: &'a str,
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn whole(n: BigInt) -> Value {
        match n.to_i64() {
            Some(i) => Value::Int(i),
            None => Value::Big(Rc::new(n)),
        }
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(items))
    }

    pub fn kind(&self) -> Option<Kind> {
        Some(match self {
            Value::Int(_) | Value::Big(_) => Kind::Integer,
            Value::Ratio(_) => Kind::Rational,
            Value::Real(_) => Kind::Real,
            Value::Str(_) => Kind::Text,
            Value::Bool(_) => Kind::Boolean,
            Value::List(_) => Kind::Array,
            Value::Null | Value::Kind(_) => Kind::Null,
            _ => return None,
        })
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Big(n) => !n.is_zero(),
            Value::Real(r) => !r.p.is_zero(),
            Value::Str(s) => !s.is_empty(),
            Value::Null | Value::Empty | Value::Hole | Value::Mark => false,
            Value::Ratio(_) | Value::List(_) | Value::Program(_) | Value::Kind(_) => true,
        }
    }

    /// The integer a non-number stands in for: booleans and null count,
    /// text is parsed, the rest refuse.
    pub fn as_whole(&self) -> Result<BigInt, String> {
        match self {
            Value::Int(n) => Ok(BigInt::from(*n)),
            Value::Big(n) => Ok((**n).clone()),
            Value::Real(r) => Ok(&r.p / &r.q),
            Value::Bool(b) => Ok(BigInt::from(*b as i64)),
            Value::Null | Value::Empty | Value::Hole | Value::Mark => Ok(BigInt::zero()),
            Value::Str(s) => s.parse::<BigInt>().map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::Ratio(_) => Err("Cannot coerce rational to integer".to_string()),
            Value::List(_) => Err("Cannot coerce array to number".to_string()),
            Value::Program(_) => Err("Cannot coerce function to number".to_string()),
            Value::Kind(_) => Err("Cannot coerce kind meta-value to number".to_string()),
        }
    }

    /// Equal: numbers by value across kinds, arrays elementwise, programs
    /// by identity, the rest by content.
    pub fn same(&self, other: &Value) -> bool {
        if let Some(order) = crate::numbers::compare(self, other) {
            return order == std::cmp::Ordering::Equal;
        }
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Kind(a), Value::Kind(b)) => a == b,
            (Value::Program(a), Value::Program(b)) => Rc::ptr_eq(a, b),
            (Value::List(a), Value::List(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.same(y)),
            _ => false,
        }
    }

    /// What print shows: the language's words for the literals, the
    /// machine's own form for the rest.
    pub fn show(&self, sp: &Spelling) -> String {
        match self {
            Value::Bool(true) => sp.yes.to_string(),
            Value::Bool(false) => sp.no.to_string(),
            Value::Null | Value::Empty | Value::Hole | Value::Mark => sp.none.to_string(),
            Value::List(items) => {
                let shown: Vec<String> = items.iter().map(|v| v.show(sp)).collect();
                format!("[{}]", shown.join(", "))
            }
            other => other.bare(),
        }
    }

    /// The machine's own text for a value.
    pub fn bare(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Big(n) => n.to_string(),
            Value::Ratio(r) => format!("{}/{}", r.p, r.q),
            Value::Real(r) => decimal_text(&r.p, &r.q, r.places),
            Value::Str(s) => s.to_string(),
            Value::Bool(b) => (if *b { "true" } else { "false" }).to_string(),
            Value::Null | Value::Empty | Value::Hole | Value::Mark => "null".to_string(),
            Value::List(items) => {
                let shown: Vec<String> = items.iter().map(Value::bare).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Program(p) => format!("<function({})>", p.params.join(", ")),
            Value::Kind(k) => k.label().to_string(),
        }
    }

    /// A key for the call cache: kind and content, nested for arrays.
    pub fn key(&self, into: &mut String) {
        match self {
            Value::Str(s) => {
                let _ = write!(into, "s{}:{}", s.len(), s);
            }
            Value::List(items) => {
                into.push('[');
                for v in items.iter() {
                    v.key(into);
                    into.push(',');
                }
                into.push(']');
            }
            Value::Program(p) => {
                let _ = write!(into, "p{:p}", Rc::as_ptr(p));
            }
            other => into.push_str(&other.bare()),
        }
        into.push('|');
    }
}

/// p/q to `places` significant digits: the whole part in full, then the
/// fraction digits the precision leaves, none of them padding.
pub fn decimal_text(p: &BigInt, q: &BigInt, places: usize) -> String {
    let whole = p / q;
    let mut rest = (p - &whole * q).abs();
    if rest.is_zero() {
        return whole.to_string();
    }
    let whole_text = whole.to_string();
    let mut room = places.saturating_sub(whole_text.trim_start_matches('-').len());
    let mut out = String::new();
    if p.is_negative() && whole.is_zero() {
        out.push('-');
    }
    out.push_str(&whole_text);
    out.push('.');
    while room > 0 && !rest.is_zero() {
        rest *= 10;
        let digit = &rest / q;
        out.push_str(&digit.to_string());
        rest -= &digit * q;
        room -= 1;
    }
    out
}
