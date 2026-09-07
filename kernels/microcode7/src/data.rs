// Values. A program value is a closure: the program and the frame it was
// made in, so a nested program sees the bindings around it.

use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::form::Routine;

/// A run-time frame: slots, and the frame the program was made in.
pub struct Env {
    pub cells: RefCell<Vec<Value>>,
    pub outer: Option<Rc<Env>>,
}

impl Env {
    pub fn make(size: usize, parent: Option<Rc<Env>>) -> Rc<Env> {
        Rc::new(Env { cells: RefCell::new(vec![Value::Unset; size]), outer: parent })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Whole,
    Fraction,
    Decimal,
    Chars,
    Truth,
    Vector,
    Nothing,
}

impl Kind {
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Whole => "INTEGER",
            Kind::Fraction => "RATIONAL",
            Kind::Decimal => "REAL",
            Kind::Chars => "STRING",
            Kind::Truth => "BOOLEAN",
            Kind::Vector => "ARRAY",
            Kind::Nothing => "NULL",
        }
    }
}

/// above/beneath in lowest terms; a real carries the places it shows.
#[derive(Debug, Clone)]
pub struct Ratio {
    pub above: BigInt,
    pub beneath: BigInt,
    pub places: Option<usize>,
}

#[derive(Clone)]
pub enum Value {
    Small(i64),
    Huge(Rc<BigInt>),
    Frac(Rc<Ratio>),
    Text(Rc<str>),
    Flag(bool),
    Nil,
    Vector(Rc<Vec<Value>>),
    /// Keys with their values, kept in the order they were written.
    Dict(Rc<Vec<(Value, Value)>>),
    /// A key written together with its value (`k => v`), until a
    /// literal takes it in.
    Couple(Rc<(Value, Value)>),
    /// A program not yet bound to a frame: only inside the tree.
    Routine(Rc<Routine>),
    /// A program bound to the frame it was made in.
    Bound(Rc<Routine>, Rc<Env>),
    KindOf(Kind),
    Unset,
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bare())
    }
}

#[derive(Clone, Copy)]
pub struct Names<'a> {
    pub truth: &'a str,
    pub falsity: &'a str,
    pub nil: &'a str,
}

impl Value {
    pub fn from_big(n: BigInt) -> Value {
        match n.to_i64() {
            Some(i) => Value::Small(i),
            None => Value::Huge(Rc::new(n)),
        }
    }

    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn kind(&self) -> Option<Kind> {
        Some(match self {
            Value::Small(_) | Value::Huge(_) => Kind::Whole,
            Value::Frac(e) => if e.places.is_some() { Kind::Decimal } else { Kind::Fraction },
            Value::Text(_) => Kind::Chars,
            Value::Flag(_) => Kind::Truth,
            Value::Vector(_) | Value::Dict(_) => Kind::Vector,
            Value::Nil | Value::KindOf(_) => Kind::Nothing,
            Value::Couple(_) | Value::Routine(_) | Value::Bound(..) | Value::Unset => return None,
        })
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Flag(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Huge(n) => !n.is_zero(),
            Value::Frac(e) => !e.above.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::Nil | Value::Unset => false,
            _ => true,
        }
    }

    pub fn as_big(&self) -> Result<BigInt, String> {
        Ok(match self {
            Value::Small(n) => BigInt::from(*n),
            Value::Huge(n) => (**n).clone(),
            Value::Frac(e) if e.places.is_some() => &e.above / &e.beneath,
            Value::Frac(_) => return Err("Cannot coerce rational to integer".to_string()),
            Value::Flag(b) => BigInt::from(*b as i64),
            Value::Nil | Value::Unset => BigInt::zero(),
            Value::Text(s) => s.parse().map_err(|_| format!("Cannot coerce '{}' to number", s))?,
            Value::Vector(_) | Value::Dict(_) | Value::Couple(_) => return Err("Cannot coerce array to number".to_string()),
            Value::Routine(_) | Value::Bound(..) => return Err("Cannot coerce function to number".to_string()),
            Value::KindOf(_) => return Err("Cannot coerce kind meta-value to number".to_string()),
        })
    }

    pub fn equals(&self, other: &Value) -> bool {
        if let (Some(a), Some(b)) = (crate::math::ratio_of(self), crate::math::ratio_of(other)) {
            return a.above * b.beneath == b.above * a.beneath;
        }
        match (self, other) {
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Flag(a), Value::Flag(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Vector(a), Value::Vector(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Dict(a), Value::Dict(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.equals(k) && x.equals(y))
            }
            (Value::Couple(a), Value::Couple(b)) => a.0.equals(&b.0) && a.1.equals(&b.1),
            (Value::Bound(a, _), Value::Bound(b, _)) => Rc::ptr_eq(a, b),
            (Value::KindOf(a), Value::KindOf(b)) => a == b,
            _ => false,
        }
    }

    pub fn render(&self, w: Names) -> String {
        match self {
            Value::Flag(true) => w.truth.to_string(),
            Value::Flag(false) => w.falsity.to_string(),
            Value::Nil | Value::Unset => w.nil.to_string(),
            Value::Vector(items) => format!("[{}]", items.iter().map(|v| v.render(w)).collect::<Vec<_>>().join(", ")),
            Value::Dict(entries) => {
                format!("[{}]", entries.iter().map(|(k, v)| format!("{} => {}", k.render(w), v.render(w))).collect::<Vec<_>>().join(", "))
            }
            Value::Couple(e) => format!("{} => {}", e.0.render(w), e.1.render(w)),
            other => other.bare(),
        }
    }

    pub fn bare(&self) -> String {
        match self {
            Value::Small(n) => n.to_string(),
            Value::Huge(n) => n.to_string(),
            Value::Frac(e) => match e.places {
                Some(d) => decimal_string(&e.above, &e.beneath, d),
                None => format!("{}/{}", e.above, e.beneath),
            },
            Value::Text(s) => s.to_string(),
            Value::Flag(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Nil | Value::Unset => "null".to_string(),
            Value::Vector(items) => format!("[{}]", items.iter().map(Value::bare).collect::<Vec<_>>().join(", ")),
            Value::Dict(entries) => {
                format!("[{}]", entries.iter().map(|(k, v)| format!("{} => {}", k.bare(), v.bare())).collect::<Vec<_>>().join(", "))
            }
            Value::Couple(e) => format!("{} => {}", e.0.bare(), e.1.bare()),
            Value::Routine(p) | Value::Bound(p, _) => format!("<function({})>", p.formals.join(", ")),
            Value::KindOf(s) => s.tag().to_string(),
        }
    }

    pub fn memo_key(&self, out: &mut String) {
        match self {
            Value::Text(s) => out.push_str(&format!("s{:?}", s)),
            Value::Vector(items) => {
                out.push('[');
                items.iter().for_each(|v| v.memo_key(out));
                out.push(']');
            }
            Value::Dict(entries) => {
                out.push('{');
                entries.iter().for_each(|(k, v)| {
                    k.memo_key(out);
                    v.memo_key(out);
                });
                out.push('}');
            }
            Value::Couple(e) => {
                out.push('(');
                e.0.memo_key(out);
                e.1.memo_key(out);
                out.push(')');
            }
            Value::Bound(p, _) => out.push_str(&format!("f{:p}", Rc::as_ptr(p))),
            other => out.push_str(&other.bare()),
        }
        out.push('|');
    }
}

/// The whole part, then fraction places while the significant places last.
pub fn decimal_string(above: &BigInt, beneath: &BigInt, places: usize) -> String {
    let whole = above / beneath;
    let mut left = (above - &whole * beneath).abs();
    if left.is_zero() {
        return whole.to_string();
    }
    let mut out = String::new();
    if above.is_negative() && whole.is_zero() {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    out.push('.');
    let mut room = places.saturating_sub(whole.to_string().trim_start_matches('-').len());
    while room > 0 && !left.is_zero() {
        left *= 10;
        let d = &left / beneath;
        out.push_str(&d.to_string());
        left -= &d * beneath;
        room -= 1;
    }
    out
}
