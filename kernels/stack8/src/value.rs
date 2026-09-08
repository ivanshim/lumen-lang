// Values. Small integers are unboxed; everything larger sits behind a
// reference count, so the stack moves pointers. Arrays copy when written
// through a shared reference, which a taking load avoids.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

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
    /// A nought that came of working with a number below nought keeps
    /// the minus, since a real of a width has two noughts and a language
    /// holding reals to a width writes them apart.
    pub below: bool,
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
    /// A cell two or more names share: a write through any of them is a
    /// write all of them see. Never a value a program can hold itself.
    Bond(Rc<RefCell<Value>>),
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
    /// Where a language's reals are binary numbers of a fixed width,
    /// how many significant digits one shows when simply written out.
    /// Where it says nothing, a real is shown to its own precision.
    pub real_digits: Option<usize>,
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
            Value::Bond(shared) => return shared.borrow().sort(),
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
            Value::Bond(shared) => shared.borrow().is_true(),
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
            Value::Bond(shared) => shared.borrow().as_big(),
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

    /// Whether two values are the very same. Equal is not enough: they
    /// must be of one kind, so a whole number and a real that stand for
    /// the same amount are equal but not the same. An array is the same
    /// as another when it holds the same keys in the same order, each
    /// with a value that is itself the same.
    pub fn identical(&self, other: &Value) -> bool {
        if let Value::Bond(shared) = self {
            let held = shared.borrow().clone();
            return held.identical(other);
        }
        if let Value::Bond(shared) = other {
            let held = shared.borrow().clone();
            return self.identical(&held);
        }
        match (self, other) {
            (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.identical(y)),
            (Value::Map(a), Value::Map(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.identical(k) && x.identical(y))
            }
            // An array written with keys and one written without are of
            // one kind, so the keys themselves decide.
            (Value::Array(a), Value::Map(b)) | (Value::Map(b), Value::Array(a)) => {
                a.len() == b.len()
                    && a.iter().zip(b.iter()).enumerate().all(|(at, (x, (k, y)))| k.equals(&Value::Small(at as i64)) && x.identical(y))
            }
            _ => match (self.sort(), other.sort()) {
                (Some(one), Some(two)) => one == two && self.equals(other),
                _ => self.equals(other),
            },
        }
    }

    /// What print shows: the language's words for the literals, the
    /// machine's own form for the rest.
    pub fn display(&self, sp: &Wording) -> String {
        match self {
            // A cell two names share is written as what it holds: the
            // sharing is between the names and not in the value.
            Value::Bond(shared) => shared.borrow().display(sp),
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
            // A language whose reals are binary numbers writes one out
            // to its own count of significant figures.
            Value::Real(r) if r.below && r.p.is_zero() => "-0".to_string(),
            Value::Real(r) if sp.real_digits.is_some() => binary_string(as_binary(&r.p, &r.q), sp.real_digits),
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
            Value::Bond(shared) => shared.borrow().plain(),
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
            Value::Bond(shared) => shared.borrow().memo_key(into),
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
    /// The classes of method names only that this one answers to.
    pub answers: Vec<Rc<Class>>,
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

    /// Whether this class is that one, stands on it, or answers to it.
    pub fn descends_from(&self, name: &str) -> bool {
        self.name == name
            || self.base.as_ref().map_or(false, |b| b.descends_from(name))
            || self.answers.iter().any(|a| a.descends_from(name))
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
    /// Which object this is by the order it was made, counting from
    /// one: what a language that names objects when showing them shows.
    pub mark: usize,
}

/// A real as the nearest binary number of sixty-four bits. A number too
/// large for one to hold stands beyond every one of them, which is what
/// such a language means by an unbounded number.
pub fn as_binary(p: &BigInt, q: &BigInt) -> f64 {
    let beyond = || if p.is_negative() { f64::NEG_INFINITY } else { f64::INFINITY };
    if q.is_one() {
        return p.to_f64().unwrap_or_else(beyond);
    }
    match (p.to_f64(), q.to_f64()) {
        (Some(a), Some(b)) if a.is_finite() && b.is_finite() && b != 0.0 => a / b,
        // Too large for the division to be done straight off: bring
        // both down by the same power of two and divide those.
        _ => {
            let shift = p.bits().max(q.bits()).saturating_sub(900);
            let (a, b) = (p >> shift, q >> shift);
            match (a.to_f64(), b.to_f64()) {
                (Some(a), Some(b)) if b != 0.0 => a / b,
                _ => beyond(),
            }
        }
    }
}

/// What a binary real is worth, held exactly: the fewest digits that
/// read back as the same number are what the number stands for.
pub fn from_binary(x: f64) -> Option<(BigInt, BigInt)> {
    if !x.is_finite() {
        return None;
    }
    let written = format!("{:e}", x);
    let (mantissa, power) = written.split_once('e')?;
    let power: i32 = power.parse().ok()?;
    let negative = mantissa.starts_with('-');
    let figures: String = mantissa.trim_start_matches('-').chars().filter(|c| *c != '.').collect();
    let scale = power - (figures.len() as i32 - 1);
    let mut p: BigInt = figures.parse().ok()?;
    if negative {
        p = -p;
    }
    Some(match scale >= 0 {
        true => (p * BigInt::from(10).pow(scale as u32), BigInt::one()),
        false => (p, BigInt::from(10).pow(scale.unsigned_abs())),
    })
}

/// A binary real written out the way such a language writes one: the
/// fewest digits that read back as the same number, with a power of ten
/// after them where the number is very large or very small. `digits`
/// caps the significant figures, as a language's own setting does when
/// a number is simply written out rather than shown with its kind.
pub fn binary_string(x: f64, digits: Option<usize>) -> String {
    if x.is_nan() {
        return "NAN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-INF".to_string() } else { "INF".to_string() };
    }
    // The shortest run of digits that reads back as this number, with
    // the power of ten it stands at, is what the machine's own writing
    // gives when asked for a power of ten.
    let written = match digits {
        Some(n) => format!("{:.*e}", n.saturating_sub(1), x),
        None => format!("{:e}", x),
    };
    let (mantissa, power) = written.split_once('e').expect("a power of ten was asked for");
    let power: i32 = power.parse().unwrap_or(0);
    let mut figures = mantissa.trim_start_matches('-').replace('.', "");
    if digits.is_some() {
        while figures.len() > 1 && figures.ends_with('0') {
            figures.pop();
        }
    }
    let sign = if mantissa.starts_with('-') { "-" } else { "" };
    // Written plainly while the power is small, and with the power
    // spelled out beyond that, which is where such a language changes.
    if (-5..15).contains(&power) {
        return format!("{}{}", sign, laid_flat(&figures, power));
    }
    let rest = &figures[1..];
    let after = if rest.is_empty() { "0".to_string() } else { rest.to_string() };
    let mark = if power < 0 { "-" } else { "+" };
    format!("{}{}.{}E{}{}", sign, &figures[..1], after, mark, power.abs())
}

/// A run of significant figures written out plainly at the power of ten
/// it stands at: 123 at power 1 is 12.3, at power -2 is 0.0123.
fn laid_flat(figures: &str, power: i32) -> String {
    let point = power + 1;
    if point <= 0 {
        return format!("0.{}{}", "0".repeat(-point as usize), figures);
    }
    let point = point as usize;
    if point >= figures.len() {
        return format!("{}{}", figures, "0".repeat(point - figures.len()));
    }
    format!("{}.{}", &figures[..point], &figures[point..])
}
