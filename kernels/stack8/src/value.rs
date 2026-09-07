// Values. Small integers are unboxed; everything larger sits behind a
// reference count, so the stack moves pointers. Arrays copy when written
// through a shared reference, which a taking load avoids.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use crate::code::Routine;

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
    pub fn tag(self) -> &'static str {
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

/// p/q in lowest terms, q > 1.
#[derive(Debug, Clone)]
pub struct Frac {
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
    Small(i64),
    Huge(Rc<BigInt>),
    Frac(Rc<Frac>),
    Real(Rc<Real>),
    Text(Rc<str>),
    Flag(bool),
    Null,
    Array(Rc<Vec<Value>>),
    /// Keys and their values, in the order they were put there.
    Map(Rc<Vec<(Value, Value)>>),
    Class(Rc<Class>),
    Object(Rc<Instance>),
    /// A key and a value written together (`k => v`), waiting to be
    /// gathered into a map.
    Tie(Rc<(Value, Value)>),
    Routine(Rc<Routine>),
    SortOf(Sort),
    /// A slot nothing was stored in.
    Blank,
    /// A slot whose value a taking load moved out; the next store fills it.
    Gap,
    /// The bottom of an array literal being gathered.
    Fence,
}

/// How a language spells the literal values when printing.
pub struct Wording<'a> {
    pub true_word: &'a str,
    pub false_word: &'a str,
    pub null_word: &'a str,
}

impl Value {
    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn of_big(n: BigInt) -> Value {
        match n.to_i64() {
            Some(i) => Value::Small(i),
            None => Value::Huge(Rc::new(n)),
        }
    }

    pub fn array(items: Vec<Value>) -> Value {
        Value::Array(Rc::new(items))
    }

    pub fn sort(&self) -> Option<Sort> {
        Some(match self {
            Value::Small(_) | Value::Huge(_) => Sort::Integer,
            Value::Frac(_) => Sort::Rational,
            Value::Real(_) => Sort::Real,
            Value::Text(_) => Sort::Text,
            Value::Flag(_) => Sort::Boolean,
            Value::Array(_) | Value::Map(_) => Sort::Array,
            Value::Class(_) | Value::Object(_) => return None,
            Value::Null | Value::SortOf(_) => Sort::Null,
            _ => return None,
        })
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Flag(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Huge(n) => !n.is_zero(),
            Value::Real(r) => !r.p.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => false,
            Value::Frac(_) | Value::Array(_) | Value::Map(_) | Value::Tie(_) | Value::Routine(_) | Value::SortOf(_) => true,
            Value::Class(_) | Value::Object(_) => true,
        }
    }

    /// The integer a non-number stands in for: booleans and null count,
    /// text is parsed, the rest refuse.
    pub fn as_big(&self) -> Result<BigInt, String> {
        match self {
            Value::Small(n) => Ok(BigInt::from(*n)),
            Value::Huge(n) => Ok((**n).clone()),
            Value::Real(r) => Ok(&r.p / &r.q),
            Value::Flag(b) => Ok(BigInt::from(*b as i64)),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => Ok(BigInt::zero()),
            Value::Text(s) => s.parse::<BigInt>().map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::Frac(_) => Err("Cannot coerce rational to integer".to_string()),
            Value::Array(_) | Value::Map(_) | Value::Tie(_) => Err("Cannot coerce array to number".to_string()),
            Value::Class(_) | Value::Object(_) => Err("Cannot coerce object to number".to_string()),
            Value::Routine(_) => Err("Cannot coerce function to number".to_string()),
            Value::SortOf(_) => Err("Cannot coerce kind meta-value to number".to_string()),
        }
    }

    /// Equal: numbers by value across kinds, arrays elementwise, programs
    /// by identity, the rest by content.
    pub fn equals(&self, other: &Value) -> bool {
        if let Some(order) = crate::arith::order_values(self, other) {
            return order == std::cmp::Ordering::Equal;
        }
        match (self, other) {
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Flag(a), Value::Flag(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::SortOf(a), Value::SortOf(b)) => a == b,
            (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a, b),
            (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Map(a), Value::Map(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.equals(k) && x.equals(y))
            }
            (Value::Tie(a), Value::Tie(b)) => a.0.equals(&b.0) && a.1.equals(&b.1),
            // Two names for one object are the same object; two objects
            // of one class are not.
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => a.name == b.name,
            _ => false,
        }
    }

    /// What print shows: the language's words for the literals, the
    /// machine's own form for the rest.
    pub fn display(&self, sp: &Wording) -> String {
        match self {
            Value::Flag(true) => sp.true_word.to_string(),
            Value::Flag(false) => sp.false_word.to_string(),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => sp.null_word.to_string(),
            Value::Array(items) => {
                let shown: Vec<String> = items.iter().map(|v| v.display(sp)).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Map(pairs) => {
                let shown: Vec<String> = pairs.iter().map(|(k, v)| format!("{} => {}", k.display(sp), v.display(sp))).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Tie(pair) => format!("{} => {}", pair.0.display(sp), pair.1.display(sp)),
            other => other.plain(),
        }
    }

    /// The machine's own text for a value.
    pub fn plain(&self) -> String {
        match self {
            Value::Small(n) => n.to_string(),
            Value::Huge(n) => n.to_string(),
            Value::Frac(r) => format!("{}/{}", r.p, r.q),
            Value::Real(r) => decimal_string(&r.p, &r.q, r.places),
            Value::Text(s) => s.to_string(),
            Value::Flag(b) => (if *b { "true" } else { "false" }).to_string(),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => "null".to_string(),
            Value::Array(items) => {
                let shown: Vec<String> = items.iter().map(Value::plain).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Map(pairs) => {
                let shown: Vec<String> = pairs.iter().map(|(k, v)| format!("{} => {}", k.plain(), v.plain())).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Tie(pair) => format!("{} => {}", pair.0.plain(), pair.1.plain()),
            Value::Routine(p) => format!("<function({})>", p.formals.join(", ")),
            Value::Class(c) => format!("<class {}>", c.name),
            Value::Object(o) => format!("<object {}>", o.class.name),
            Value::SortOf(k) => k.tag().to_string(),
        }
    }

    /// A key for the call cache: kind and content, nested for arrays.
    pub fn memo_key(&self, into: &mut String) {
        match self {
            Value::Text(s) => {
                let _ = write!(into, "s{}:{}", s.len(), s);
            }
            Value::Array(items) => {
                into.push('[');
                for v in items.iter() {
                    v.memo_key(into);
                    into.push(',');
                }
                into.push(']');
            }
            Value::Map(pairs) => {
                into.push('{');
                for (k, v) in pairs.iter() {
                    k.memo_key(into);
                    v.memo_key(into);
                    into.push(',');
                }
                into.push('}');
            }
            Value::Tie(pair) => {
                into.push('(');
                pair.0.memo_key(into);
                pair.1.memo_key(into);
                into.push(')');
            }
            Value::Routine(p) => {
                let _ = write!(into, "p{:p}", Rc::as_ptr(p));
            }
            Value::Object(o) => {
                let _ = write!(into, "o{:p}", Rc::as_ptr(o));
            }
            Value::Class(c) => {
                let _ = write!(into, "c{}", c.name);
            }
            other => into.push_str(&other.plain()),
        }
        into.push('|');
    }
}

/// p/q to `places` significant digits: the whole part in full, then the
/// fraction digits the precision leaves, none of them padding.
pub fn decimal_string(p: &BigInt, q: &BigInt, places: usize) -> String {
    let int_part = p / q;
    let mut remainder = (p - &int_part * q).abs();
    if remainder.is_zero() {
        return int_part.to_string();
    }
    let int_text = int_part.to_string();
    let mut left = places.saturating_sub(int_text.trim_start_matches('-').len());
    let mut s = String::new();
    if p.is_negative() && int_part.is_zero() {
        s.push('-');
    }
    s.push_str(&int_text);
    s.push('.');
    while left > 0 && !remainder.is_zero() {
        remainder *= 10;
        let d = &remainder / q;
        s.push_str(&d.to_string());
        remainder -= &d * q;
        left -= 1;
    }
    s
}

/// A class: what it is called, what it stands on, the properties an
/// object of it begins with, the programs it answers to, its constants
/// and the values it keeps for itself.
#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub base: Option<Rc<Class>>,
    pub fields: Vec<(String, Value)>,
    pub methods: Vec<(String, Rc<Routine>)>,
    pub constants: Vec<(String, Value)>,
    pub shared: RefCell<Vec<(String, Value)>>,
}

impl Class {
    /// The program of that name, in this class or the nearest one
    /// beneath it that has one.
    pub fn method(&self, name: &str) -> Option<&Rc<Routine>> {
        match self.methods.iter().find(|(n, _)| n == name) {
            Some((_, p)) => Some(p),
            None => self.base.as_ref().and_then(|b| b.method(name)),
        }
    }

    /// The constant of that name, looked for the same way.
    pub fn constant(&self, name: &str) -> Option<&Value> {
        match self.constants.iter().find(|(n, _)| n == name) {
            Some((_, v)) => Some(v),
            None => self.base.as_ref().and_then(|b| b.constant(name)),
        }
    }

    /// The class holding a value of that name for itself.
    pub fn holder(&self, name: &str) -> Option<&Class> {
        if self.shared.borrow().iter().any(|(n, _)| n == name) {
            return Some(self);
        }
        self.base.as_ref().and_then(|b| b.holder(name))
    }

    /// Whether this class is that one, or stands on it.
    pub fn descends_from(&self, name: &str) -> bool {
        self.name == name || self.base.as_ref().map_or(false, |b| b.descends_from(name))
    }

    /// Every property an object of this class begins with, those it
    /// stands on first, so a class of its own overrides them.
    pub fn all_fields(&self) -> Vec<(String, Value)> {
        let mut all = self.base.as_ref().map_or_else(Vec::new, |b| b.all_fields());
        for (name, value) in &self.fields {
            match all.iter_mut().find(|(n, _)| n == name) {
                Some(place) => place.1 = value.clone(),
                None => all.push((name.clone(), value.clone())),
            }
        }
        all
    }
}

/// One object: the class that made it and what it holds. An object is a
/// handle, so two names for it see one another's writes.
#[derive(Debug)]
pub struct Instance {
    pub class: Rc<Class>,
    pub fields: RefCell<Vec<(String, Value)>>,
}
