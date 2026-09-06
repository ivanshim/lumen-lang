// Values. A program value is a closure: the program and the frame it was
// made in, so a nested program sees the bindings around it.

use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::tree::Program;

/// A run-time frame: slots, and the frame the program was made in.
pub struct Frame {
    pub slots: RefCell<Vec<Value>>,
    pub parent: Option<Rc<Frame>>,
}

impl Frame {
    pub fn new(size: usize, parent: Option<Rc<Frame>>) -> Rc<Frame> {
        Rc::new(Frame { slots: RefCell::new(vec![Value::Empty; size]), parent })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Integer,
    Rational,
    Real,
    Text,
    Boolean,
    Array,
    Null,
}

impl Sort {
    pub fn word(self) -> &'static str {
        match self {
            Sort::Integer => "INTEGER",
            Sort::Rational => "RATIONAL",
            Sort::Real => "REAL",
            Sort::Text => "STRING",
            Sort::Boolean => "BOOLEAN",
            Sort::Array => "ARRAY",
            Sort::Null => "NULL",
        }
    }
}

/// num/den in lowest terms; a real carries the digits it shows.
#[derive(Debug, Clone)]
pub struct Exact {
    pub num: BigInt,
    pub den: BigInt,
    pub digits: Option<usize>,
}

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Big(Rc<BigInt>),
    Exact(Rc<Exact>),
    Str(Rc<str>),
    Bool(bool),
    Null,
    List(Rc<Vec<Value>>),
    /// A program not yet bound to a frame: only inside the tree.
    Code(Rc<Program>),
    /// A program bound to the frame it was made in.
    Closure(Rc<Program>, Rc<Frame>),
    Sort(Sort),
    Empty,
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.plain())
    }
}

#[derive(Clone, Copy)]
pub struct Words<'a> {
    pub yes: &'a str,
    pub no: &'a str,
    pub none: &'a str,
}

impl Value {
    pub fn big(n: BigInt) -> Value {
        match n.to_i64() {
            Some(i) => Value::Int(i),
            None => Value::Big(Rc::new(n)),
        }
    }

    pub fn str(s: &str) -> Value {
        Value::Str(Rc::from(s))
    }

    pub fn sort(&self) -> Option<Sort> {
        Some(match self {
            Value::Int(_) | Value::Big(_) => Sort::Integer,
            Value::Exact(e) => if e.digits.is_some() { Sort::Real } else { Sort::Rational },
            Value::Str(_) => Sort::Text,
            Value::Bool(_) => Sort::Boolean,
            Value::List(_) => Sort::Array,
            Value::Null | Value::Sort(_) => Sort::Null,
            Value::Code(_) | Value::Closure(..) | Value::Empty => return None,
        })
    }

    pub fn truth(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Big(n) => !n.is_zero(),
            Value::Exact(e) => !e.num.is_zero(),
            Value::Str(s) => !s.is_empty(),
            Value::Null | Value::Empty => false,
            _ => true,
        }
    }

    pub fn integer(&self) -> Result<BigInt, String> {
        Ok(match self {
            Value::Int(n) => BigInt::from(*n),
            Value::Big(n) => (**n).clone(),
            Value::Exact(e) if e.digits.is_some() => &e.num / &e.den,
            Value::Exact(_) => return Err("Cannot coerce rational to integer".to_string()),
            Value::Bool(b) => BigInt::from(*b as i64),
            Value::Null | Value::Empty => BigInt::zero(),
            Value::Str(s) => s.parse().map_err(|_| format!("Cannot coerce '{}' to number", s))?,
            Value::List(_) => return Err("Cannot coerce array to number".to_string()),
            Value::Code(_) | Value::Closure(..) => return Err("Cannot coerce function to number".to_string()),
            Value::Sort(_) => return Err("Cannot coerce kind meta-value to number".to_string()),
        })
    }

    pub fn same(&self, other: &Value) -> bool {
        if let (Some(a), Some(b)) = (crate::arith::exact(self), crate::arith::exact(other)) {
            return a.num * b.den == b.num * a.den;
        }
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::List(a), Value::List(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.same(y)),
            (Value::Closure(a, _), Value::Closure(b, _)) => Rc::ptr_eq(a, b),
            (Value::Sort(a), Value::Sort(b)) => a == b,
            _ => false,
        }
    }

    pub fn text(&self, w: Words) -> String {
        match self {
            Value::Bool(true) => w.yes.to_string(),
            Value::Bool(false) => w.no.to_string(),
            Value::Null | Value::Empty => w.none.to_string(),
            Value::List(items) => format!("[{}]", items.iter().map(|v| v.text(w)).collect::<Vec<_>>().join(", ")),
            other => other.plain(),
        }
    }

    pub fn plain(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Big(n) => n.to_string(),
            Value::Exact(e) => match e.digits {
                Some(d) => decimal_digits(&e.num, &e.den, d),
                None => format!("{}/{}", e.num, e.den),
            },
            Value::Str(s) => s.to_string(),
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Null | Value::Empty => "null".to_string(),
            Value::List(items) => format!("[{}]", items.iter().map(Value::plain).collect::<Vec<_>>().join(", ")),
            Value::Code(p) | Value::Closure(p, _) => format!("<function({})>", p.params.join(", ")),
            Value::Sort(s) => s.word().to_string(),
        }
    }

    pub fn cache_key(&self, out: &mut String) {
        match self {
            Value::Str(s) => out.push_str(&format!("s{:?}", s)),
            Value::List(items) => {
                out.push('[');
                items.iter().for_each(|v| v.cache_key(out));
                out.push(']');
            }
            Value::Closure(p, _) => out.push_str(&format!("f{:p}", Rc::as_ptr(p))),
            other => out.push_str(&other.plain()),
        }
        out.push('|');
    }
}

/// The whole part, then fraction digits while the significant digits last.
pub fn decimal_digits(num: &BigInt, den: &BigInt, digits: usize) -> String {
    let whole = num / den;
    let mut left = (num - &whole * den).abs();
    if left.is_zero() {
        return whole.to_string();
    }
    let mut out = String::new();
    if num.is_negative() && whole.is_zero() {
        out.push('-');
    }
    out.push_str(&whole.to_string());
    out.push('.');
    let mut room = digits.saturating_sub(whole.to_string().trim_start_matches('-').len());
    while room > 0 && !left.is_zero() {
        left *= 10;
        let d = &left / den;
        out.push_str(&d.to_string());
        left -= &d * den;
        room -= 1;
    }
    out
}
