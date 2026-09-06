// Values of the stack machine.
//
// Integers that fit a machine word are kept unboxed; larger ones, exact
// fractions and reals carry big integers behind a reference count, as do
// strings, arrays and programs, so moving a value on and off the stack
// never copies more than a pointer. Arrays have value semantics: a write
// through a shared reference copies first.

use std::fmt::Write as _;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::code::Program;

/// Kind meta-values, the closed set of value categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Integer,
    Rational,
    Real,
    Text,
    Boolean,
    List,
    Nothing,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Integer => "INTEGER",
            Kind::Rational => "RATIONAL",
            Kind::Real => "REAL",
            Kind::Text => "STRING",
            Kind::Boolean => "BOOLEAN",
            Kind::List => "ARRAY",
            Kind::Nothing => "NULL",
        }
    }
}

/// An exact fraction in lowest terms with a positive denominator greater than one.
#[derive(Debug, Clone)]
pub struct Fraction {
    pub top: BigInt,
    pub bottom: BigInt,
}

/// A real: an exact fraction plus the number of significant digits it shows.
#[derive(Debug, Clone)]
pub struct RealNumber {
    pub top: BigInt,
    pub bottom: BigInt,
    pub digits: usize,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Big(Rc<BigInt>),
    Ratio(Rc<Fraction>),
    Real(Rc<RealNumber>),
    Text(Rc<str>),
    Bool(bool),
    Null,
    List(Rc<Vec<Value>>),
    Program(Rc<Program>),
    Kind(Kind),
    /// A slot that holds nothing yet; never a program's value.
    Unset,
}

/// The words a language renders booleans and null with.
pub struct Spelling<'a> {
    pub yes: &'a str,
    pub no: &'a str,
    pub none: &'a str,
}

impl Value {
    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn integer(n: BigInt) -> Value {
        match n.to_i64() {
            Some(small) => Value::Int(small),
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
            Value::Text(_) => Kind::Text,
            Value::Bool(_) => Kind::Boolean,
            Value::List(_) => Kind::List,
            Value::Null | Value::Kind(_) => Kind::Nothing,
            Value::Program(_) | Value::Unset => return None,
        })
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null | Value::Unset => false,
            Value::Int(n) => *n != 0,
            Value::Big(n) => !n.is_zero(),
            Value::Ratio(_) => true,
            Value::Real(r) => !r.top.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::List(_) | Value::Program(_) | Value::Kind(_) => true,
        }
    }

    /// The integer a value obviously stands for, for the closed operations
    /// on booleans and null and for comparisons of non-numbers.
    pub fn as_integer(&self) -> Result<BigInt, String> {
        match self {
            Value::Int(n) => Ok(BigInt::from(*n)),
            Value::Big(n) => Ok((**n).clone()),
            Value::Ratio(_) => Err("Cannot coerce rational to integer".to_string()),
            Value::Real(r) => Ok(&r.top / &r.bottom),
            Value::Bool(b) => Ok(BigInt::from(*b as i64)),
            Value::Null | Value::Unset => Ok(BigInt::zero()),
            Value::Text(s) => s.parse::<BigInt>().map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::List(_) => Err("Cannot coerce array to number".to_string()),
            Value::Program(_) => Err("Cannot coerce function to number".to_string()),
            Value::Kind(_) => Err("Cannot coerce kind meta-value to number".to_string()),
        }
    }

    /// Equality: numbers by exact value whatever their kind, arrays element
    /// by element, programs by identity, everything else by content.
    pub fn same(&self, other: &Value) -> bool {
        use crate::number::Number;
        if let (Some(a), Some(b)) = (Number::of(self), Number::of(other)) {
            return a.cmp(&b) == std::cmp::Ordering::Equal;
        }
        match (self, other) {
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::List(a), Value::List(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.same(y)),
            (Value::Program(a), Value::Program(b)) => Rc::ptr_eq(a, b),
            (Value::Kind(a), Value::Kind(b)) => a == b,
            _ => false,
        }
    }

    /// The text `print` shows, with the language's own words for the
    /// literals and the machine's own form for everything else.
    pub fn render(&self, spelling: &Spelling) -> String {
        match self {
            Value::Bool(true) => spelling.yes.to_string(),
            Value::Bool(false) => spelling.no.to_string(),
            Value::Null | Value::Unset => spelling.none.to_string(),
            Value::List(items) => {
                let mut out = String::from("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&item.render(spelling));
                }
                out.push(']');
                out
            }
            other => other.plain(),
        }
    }

    /// The text of a value with the machine's own words.
    pub fn plain(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Big(n) => n.to_string(),
            Value::Ratio(f) => format!("{}/{}", f.top, f.bottom),
            Value::Real(r) => decimal(&r.top, &r.bottom, r.digits),
            Value::Text(s) => s.to_string(),
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Null | Value::Unset => "null".to_string(),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(Value::plain).collect();
                format!("[{}]", inner.join(", "))
            }
            Value::Program(p) => format!("<function({})>", p.params.join(", ")),
            Value::Kind(k) => k.label().to_string(),
        }
    }

    /// A fingerprint for the call cache: kind and content.
    pub fn fingerprint(&self, out: &mut String) {
        match self {
            Value::Text(s) => {
                let _ = write!(out, "s{}:{}", s.len(), s);
            }
            Value::List(items) => {
                out.push('[');
                for item in items.iter() {
                    item.fingerprint(out);
                    out.push(',');
                }
                out.push(']');
            }
            Value::Program(p) => {
                let _ = write!(out, "p{:p}", Rc::as_ptr(p));
            }
            other => {
                let _ = write!(out, "{}", other.plain());
            }
        }
        out.push('|');
    }
}

/// A real written to its significant digits: the integer part in full,
/// then as many fraction digits as the precision leaves, without padding.
pub fn decimal(top: &BigInt, bottom: &BigInt, digits: usize) -> String {
    let whole = top / bottom;
    let mut left = top - &whole * bottom;
    if left.is_zero() {
        return whole.to_string();
    }
    let used = whole.to_string().trim_start_matches('-').len();
    let mut room = digits.saturating_sub(used);
    left = left.abs();
    let mut out = String::new();
    if top.is_negative() && whole.is_zero() {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    out.push('.');
    while room > 0 && !left.is_zero() {
        left *= 10;
        let digit = &left / bottom;
        out.push_str(&digit.to_string());
        left -= &digit * bottom;
        room -= 1;
    }
    out
}
