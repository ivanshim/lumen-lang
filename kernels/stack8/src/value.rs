// Values. Small integers are unboxed; everything larger sits behind a
// reference count, so the stack moves pointers. Arrays copy when written
// through a shared reference, which a taking load avoids.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
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
    Set,
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
            Sort::Set => "SET",
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
///
/// A real of a width also holds two things no ratio does: the one no
/// number answers to, itself included, and the one lying past every
/// number on either side. Both are written here with nought beneath,
/// which no ratio in lowest terms ever is; the top then tells them
/// apart, being nought for the first and its sign for the second. It is
/// held so because a whole new kind of value would have to be answered
/// for everywhere a value is looked at, while nought beneath is answered
/// for where numbers are worked and set against one another and nowhere
/// else.
#[derive(Debug, Clone)]
pub struct Real {
    pub p: BigInt,
    pub q: BigInt,
    pub places: usize,
    /// A nought that came of working with a number below nought keeps
    /// the minus, since a real of a width has two noughts and a language
    /// holding reals to a width writes them apart.
    pub below: bool,
    /// A real read by the newer number forms keeps its point when
    /// printed whole. Older forms keep the spelling they had before.
    pub point: bool,
}

impl Real {
    /// Whether this stands outside the numbers altogether.
    pub fn outside(&self) -> bool {
        self.q.is_zero()
    }

    /// Whether this is the one no number answers to.
    pub fn no_number(&self) -> bool {
        self.q.is_zero() && self.p.is_zero()
    }

    /// How such a value is written, which is one of three ways.
    pub fn spelled(&self) -> &'static str {
        if !self.q.is_zero() {
            return "";
        }
        match (self.p.is_zero(), self.p.is_negative()) {
            (true, _) => "NAN",
            (_, true) => "-INF",
            _ => "INF",
        }
    }
}

/// A counted walk keeps its bounds rather than all its places.
#[derive(Debug, Clone)]
pub struct Counted {
    pub start: BigInt,
    pub stop: BigInt,
    pub step: BigInt,
    pub name: String,
}

impl Counted {
    pub fn length(&self) -> BigInt {
        let distance = if self.step.is_positive() { &self.stop - &self.start } else { &self.start - &self.stop };
        if distance <= BigInt::zero() { BigInt::zero() }
        else { (distance - 1) / self.step.abs() + 1 }
    }

    pub fn at(&self, mut index: BigInt) -> Option<Value> {
        let length = self.length();
        if index.is_negative() { index += &length; }
        (index >= BigInt::zero() && index < length).then(|| Value::of_big(&self.start + index * &self.step))
    }
}

/// A walk keeps its own cells and the part of the stack still wanted.
#[derive(Debug)]
pub struct Generator {
    pub program: Option<Rc<Routine>>,
    pub frame: Vec<Value>,
    pub stack: Vec<Value>,
    pub pc: usize,
    pub started: bool,
    pub closed: bool,
    pub waiting: bool,
    pub handed: Option<Value>,
    pub returned: Value,
    pub delegate: Option<Value>,
    pub sent: Value,
    pub items: Vec<Value>,
    pub current: Option<Value>,
}

impl Generator {
    pub fn new(program: Option<Rc<Routine>>, frame: Vec<Value>, items: Vec<Value>) -> Self {
        Self { program, frame, items, stack: Vec::new(), pc: 0, started: false,
            closed: false, waiting: false, handed: None, returned: Value::Null,
            delegate: None, sent: Value::Null, current: None }
    }
}

/// A cursor keeps one pending item for a loop's question about its end.
#[derive(Debug, Clone)]
pub struct CursorState {
    pub source: CursorSource,
    pub pending: Option<Value>,
    pub finished: bool,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub enum CursorSource {
    Items(Vec<Value>, usize),
    Numbered(Value, BigInt),
    Combined(Vec<Value>, Option<Value>),
    Selected(Value, Value),
}

#[derive(Debug, Clone)]
pub enum Value {
    Collection(Rc<RefCell<Value>>, bool),
    ValueMethod(Rc<(Value, String)>),
    View(Rc<(Value, String)>),
    Native(crate::code::Builtin, Rc<str>),
    Cursor(Rc<RefCell<CursorState>>),
    Stream(bool),
    Counted(Rc<Counted>),
    Small(i64),
    Huge(Rc<BigInt>),
    Frac(Rc<Frac>),
    Real(Rc<Real>),
    /// An imaginary literal and the words for working with it too soon.
    Imaginary(f64, Rc<str>),
    Text(Rc<str>),
    Flag(bool),
    Null,
    Ellipsis,
    Array(Rc<Vec<Value>>),
    Tuple(Rc<Vec<Value>>),
    Generator(Rc<RefCell<Generator>>),
    Set(Rc<RefCell<Members>>),
    SetWalk(Rc<RefCell<Members>>, usize),
    /// Bounds of an index span; nothing stands for an omitted bound.
    Slice(Rc<[Value; 3]>),
    /// Keys and their values, in the order they were put there.
    Map(Rc<Vec<(Value, Value)>>),
    /// A cell two or more names share: a write through any of them is a
    /// write all of them see. Where calls bind by name, it also holds
    /// a collection whose items may change whilst its names stay apart.
    Bond(Rc<RefCell<Value>>),
    /// A lexical binding, independent of any collection it currently holds.
    Binding(Rc<RefCell<Value>>),
    Class(Rc<Class>),
    Object(Rc<Instance>),
    /// A key and a value written together (`k => v`), waiting to be
    /// gathered into a map.
    Tie(Rc<(Value, Value)>),
    Routine(Rc<Routine>),
    Method(Rc<Instance>, Rc<Routine>),
    Descriptor(Rc<Descriptor>),
    SortOf(Sort),
    /// A slot nothing was stored in.
    Blank,
    /// A slot whose value a taking load moved out; the next store fills it.
    Gap,
    /// The bottom of an array literal being gathered.
    Fence,
}

/// What a decorated member does when reached or called.
#[derive(Debug, Clone)]
pub enum Descriptor {
    Static(Value),
    Class(Value),
    Property(Value, Option<Value>),
    Bound(Value, Value),
}

/// The hash finds a member; the row remembers when it first came.
#[derive(Debug, Clone)]
pub struct Members {
    pub row: Vec<String>,
    pub held: std::collections::HashMap<String, Value>,
    pub word: String,
}

impl Members {
    pub fn empty(word: String) -> Self {
        Self { row: Vec::new(), held: std::collections::HashMap::new(), word }
    }

    pub fn insert(&mut self, key: String, value: Value) {
        if !self.held.contains_key(&key) {
            self.row.push(key.clone());
            self.held.insert(key, value);
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        let found = self.held.remove(key);
        if found.is_some() { self.row.retain(|k| k != key); }
        found
    }

    pub fn items(&self) -> Vec<Value> {
        self.row.iter().map(|k| self.held[k].clone()).collect()
    }

    pub fn beneath(&self, other: &Self) -> bool {
        self.held.keys().all(|k| other.held.contains_key(k))
    }

    pub fn combine(&self, other: &Self, how: u8) -> Self {
        let mut result = Self::empty(self.word.clone());
        for key in &self.row {
            let shared = other.held.contains_key(key);
            if how == 0 || (how == 1 && shared) || (how >= 2 && !shared) {
                result.insert(key.clone(), self.held[key].clone());
            }
        }
        if how == 0 || how == 3 {
            for key in &other.row {
                if !self.held.contains_key(key) { result.insert(key.clone(), other.held[key].clone()); }
            }
        }
        result
    }

    pub fn show(&self, shown: impl Fn(&Value) -> String) -> String {
        if self.row.is_empty() { return format!("{}()", self.word); }
        format!("{{{}}}", self.row.iter().map(|k| shown(&self.held[k])).collect::<Vec<_>>().join(", "))
    }
}

/// How a language spells the literal values when printing.
pub struct Wording<'a> {
    pub true_word: &'a str,
    pub false_word: &'a str,
    pub null_word: &'a str,
    /// Whether a flag becomes text as the number it stands for: one
    /// holding true becomes `1`, one holding false nothing at all.
    pub flag_counts: bool,
    /// Where a language's reals are binary numbers of a fixed width,
    /// how many significant digits one shows when simply written out.
    /// Where it says nothing, a real is shown to its own precision.
    pub real_digits: Option<usize>,
    /// The words for a member the class shares only with those standing
    /// on it, and for one it keeps to itself, as they are marked beside
    /// the name where a thing is shown.
    /// Whether text is held as the bytes it was written in, so that the
    /// width of a piece of text is the count of its characters and not
    /// the count of bytes the letters they spell would take.
    pub text_is_bytes: bool,
    pub guarded_word: Option<&'a str>,
    pub hidden_word: Option<&'a str>,
}

impl Value {
    pub fn keeps_point(&self) -> bool {
        match self {
            Value::Real(r) => r.point,
            Value::Bond(cell) | Value::Binding(cell) => cell.borrow().keeps_point(),
            _ => false,
        }
    }

    pub fn with_point(mut self, keep: bool) -> Self {
        if keep {
            if let Value::Real(r) = &mut self {
                Rc::make_mut(r).point = true;
            }
        }
        self
    }

    pub fn contents(&self) -> Value {
        match self {
            Value::Collection(cell, _) | Value::Bond(cell) => cell.borrow().contents(),
            Value::View(view) => {
                let Value::Map(pairs) = view.0.contents() else { return Value::array(Vec::new()); };
                Value::array(pairs.iter().map(|(k,v)| match view.1.as_str() {
                    "keys" => k.clone(), "values" => v.clone(), _ => Value::Tuple(Rc::new(vec![k.clone(),v.clone()])),
                }).collect())
            }
            _ => self.clone(),
        }
    }

    pub fn held(self, quoted: bool) -> Value {
        match self { Value::Bond(cell) => Value::Collection(cell, quoted), Value::Array(_) | Value::Map(_) => Value::Collection(Rc::new(RefCell::new(self)), quoted), _ => self }
    }

    pub fn representation(&self, words: &Wording) -> String {
        match self {
            Value::Collection(cell, _) => cell.borrow().representation(words),
            Value::Text(_) => self.string_field(words, "", "r").unwrap_or_else(|| self.plain()),
            Value::Array(row) => format!("[{}]", row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", ")),
            Value::Tuple(row) => format!("({}{})", row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", "), if row.len() == 1 { "," } else { "" }),
            Value::Map(row) => format!("{{{}}}", row.iter().map(|(k,v)| format!("{}: {}", k.representation(words), v.representation(words))).collect::<Vec<_>>().join(", ")),
            _ => self.display(words),
        }
    }

    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn member_text(&self, words: &Wording) -> String {
        let mut text = self.string_field(words, "", "r").unwrap_or_else(|| self.display(words));
        if matches!(self, Value::Real(r) if !r.outside()) && !text.contains(['.', 'e', 'E']) { text.push_str(".0"); }
        text
    }

    pub fn member_key(&self) -> Result<String, &'static str> {
        if let Value::Tuple(items) = self {
            let keys = items.iter().map(Value::member_key).collect::<Result<Vec<_>, _>>()?;
            return Ok(format!("tuple:{keys:?}"));
        }
        if let Value::Collection(cell, _) | Value::Bond(cell) | Value::Binding(cell) = self { return cell.borrow().member_key(); }
        if let Some((p, q)) = crate::arith::parts(self) {
            if q.is_zero() { return Err(""); }
            let common = p.gcd(&q);
            return Ok(format!("n{}/{}", p / &common, q / common));
        }
        match self {
            Value::Flag(b) => Ok(format!("n{}/1", u8::from(*b))),
            Value::Text(s) => Ok(format!("s{}", s)),
            Value::Null => Ok("nil".into()),
            Value::Ellipsis => Ok("dots".into()),
            Value::Array(_) => Err("list"),
            Value::Map(_) => Err("dict"),
            Value::Set(_) => Err("set"),
            Value::Bond(cell) => cell.borrow().member_key(),
            _ => Err(""),
        }
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
            Value::Array(_) | Value::Map(_) | Value::Tuple(_) => Sort::Array,
            Value::Collection(cell, _) => return cell.borrow().sort(),
            Value::View(_) => Sort::Array,
            Value::Bond(shared) | Value::Binding(shared) => return shared.borrow().sort(),
            Value::Set(_) => Sort::Set,
            Value::Class(_) | Value::Object(_) => return None,
            Value::Null | Value::SortOf(_) => Sort::Null,
            _ => return None,
        })
    }

    /// Whether a value stands outside the numbers: the one no number
    /// answers to, or one past every number.
    pub fn outside_numbers(&self) -> bool {
        match self {
            Value::Real(r) => r.outside(),
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().outside_numbers(),
            _ => false,
        }
    }

    /// Whether a value is the one no number answers to.
    pub fn no_number(&self) -> bool {
        match self {
            Value::Real(r) => r.no_number(),
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().no_number(),
            _ => false,
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Imaginary(n, _) => *n != 0.0,
            Value::Collection(cell, _) => cell.borrow().is_true(),
            Value::ValueMethod(_) => true,
            Value::View(_) => if let Value::Array(row) = self.contents() { !row.is_empty() } else { false },
            Value::Native(..) | Value::Cursor(_) => true,
            Value::Set(s) => !s.borrow().held.is_empty(),
            Value::SetWalk(..) | Value::Stream(_) => true,
            Value::Counted(r) => !r.length().is_zero(),
            Value::Flag(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Huge(n) => !n.is_zero(),
            // Neither what stands outside the numbers is nought, so
            // both count as true, though the top of the one is nought.
            Value::Real(r) => r.outside() || !r.p.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::Tuple(items) => !items.is_empty(),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => false,
            Value::Descriptor(_) | Value::Generator(_) | Value::Frac(_) | Value::Array(_) | Value::Map(_) | Value::Tie(_) | Value::Routine(_) | Value::Method(..) | Value::SortOf(_) => true,
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().is_true(),
            Value::Class(_) | Value::Object(_) | Value::Ellipsis | Value::Slice(_) => true,
        }
    }

    /// The integer a non-number stands in for: booleans and null count,
    /// text is parsed, the rest refuse.
    pub fn as_big(&self) -> Result<BigInt, String> {
        match self {
            Value::Imaginary(_, words) => Err(words.to_string()),
            Value::Small(n) => Ok(BigInt::from(*n)),
            Value::Huge(n) => Ok((**n).clone()),
            // What stands outside the numbers has no whole part; such
            // a language counts it as nought.
            Value::Real(r) if r.outside() => Ok(BigInt::zero()),
            Value::Real(r) => Ok(&r.p / &r.q),
            Value::Flag(b) => Ok(BigInt::from(*b as i64)),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => Ok(BigInt::zero()),
            Value::Text(s) => s.parse::<BigInt>().map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::Frac(_) => Err("Cannot coerce rational to integer".to_string()),
            Value::Set(_) | Value::Tuple(_) | Value::Array(_) | Value::Map(_) | Value::Tie(_) | Value::View(_) => Err("Cannot coerce array to number".to_string()),
            Value::Class(_) | Value::Object(_) => Err("Cannot coerce object to number".to_string()),
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().as_big(),
            Value::Descriptor(_) | Value::Generator(_) | Value::Method(..) | Value::Routine(_) => Err("Cannot coerce function to number".to_string()),
            Value::Collection(cell, _) => cell.borrow().as_big(),
            Value::ValueMethod(_) => Err("Cannot coerce method to number".to_string()),
            Value::SetWalk(..) | Value::Native(..) | Value::Cursor(_) | Value::Stream(_) | Value::Counted(_) => Err("Cannot coerce this value to number".to_string()),
            Value::Ellipsis => Err("Ellipsis is not a number".to_string()),
            Value::Slice(_) => Err("Cannot coerce slice to number".to_string()),
            Value::SortOf(_) => Err("Cannot coerce kind meta-value to number".to_string()),
        }
    }

    /// Equal: numbers by value across kinds, arrays elementwise, programs
    /// by identity, the rest by content.
    pub fn equals(&self, other: &Value) -> bool {
        if let Value::Collection(cell, _) = self { return cell.borrow().equals(&other.contents()); }
        if let Value::Collection(cell, _) = other { return self.equals(&cell.borrow()); }
        if let Some(order) = crate::arith::order_values(self, other) {
            return order == std::cmp::Ordering::Equal;
        }
        match (self, other) {
            (Value::Imaginary(a, _), Value::Imaginary(b, _)) => a == b,
            (Value::Imaginary(a, _), b) | (b, Value::Imaginary(a, _)) => *a == 0.0 && (matches!(b, Value::Flag(false)) || b.equals(&Value::Small(0))),
            (Value::Native(a,_), Value::Native(b,_)) => a == b,
            (Value::Cursor(a), Value::Cursor(b)) => Rc::ptr_eq(a,b),
            (Value::Set(a), Value::Set(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.held.len() == b.held.len() && a.beneath(&b)
            }
            (Value::Stream(a), Value::Stream(b)) => a == b,
            (Value::Counted(a), Value::Counted(b)) => {
                let length = a.length();
                length == b.length() && (length.is_zero() || a.start == b.start && (length.is_one() || a.step == b.step))
            }
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Flag(a), Value::Flag(b)) => a == b,
            (Value::Null, Value::Null) | (Value::Ellipsis, Value::Ellipsis) => true,
            (Value::SortOf(a), Value::SortOf(b)) => a == b,
            (Value::Generator(a), Value::Generator(b)) => Rc::ptr_eq(a, b),
            (Value::Tuple(a), Value::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a, b),
            (Value::Descriptor(a), Value::Descriptor(b)) => Rc::ptr_eq(a, b),
            (Value::Method(a, p), Value::Method(b, q)) => Rc::ptr_eq(a, b) && Rc::ptr_eq(p, q),
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
            (Value::Tuple(a), Value::Tuple(b)) | (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.identical(y)),
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
            Value::Collection(cell, quote) => if *quote { cell.borrow().representation(sp) } else { cell.borrow().display(sp) },
            Value::View(view) => format!("dict_{}({})", view.1, self.contents().representation(sp)),
            // A cell two names share is written as what it holds: the
            // sharing is between the names and not in the value.
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().display(sp),
            Value::Set(s) => s.borrow().show(|v| v.member_text(sp)),

            Value::Flag(true) => match sp.flag_counts {
                true => "1".to_string(),
                false => sp.true_word.to_string(),
            },
            Value::Flag(false) => match sp.flag_counts {
                true => String::new(),
                false => sp.false_word.to_string(),
            },
            Value::Null | Value::Blank | Value::Gap | Value::Fence => sp.null_word.to_string(),
            Value::Tuple(items) => {
                let shown = items.iter().map(|v| v.string_field(sp, "", "r").unwrap_or_else(|| v.plain())).collect::<Vec<_>>().join(", ");
                format!("({}{})", shown, if items.len() == 1 { "," } else { "" })
            }
            Value::Array(items) => {
                let shown: Vec<String> = items.iter().map(|v| v.display(sp)).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Map(pairs) => {
                let shown: Vec<String> = pairs.iter().map(|(k, v)| format!("{} => {}", k.display(sp), v.display(sp))).collect();
                format!("[{}]", shown.join(", "))
            }
            Value::Tie(pair) => format!("{} => {}", pair.0.display(sp), pair.1.display(sp)),
            // What stands outside the numbers is written by its name at
            // any width, since there are no figures to write.
            Value::Real(r) if r.outside() => r.spelled().to_string(),
            // A language whose reals are binary numbers writes one out
            // to its own count of significant figures.
            Value::Real(r) if r.below && r.p.is_zero() => "-0".to_string(),
            Value::Real(r) if sp.real_digits.is_some() => written_out(as_binary(&r.p, &r.q), figures_now(false).unwrap_or(sp.real_digits)),
            other => other.plain(),
        }
    }

    /// A field is rendered after its specification has itself been
    /// worked out. The small common formats are honoured here; the
    /// rest say that the run has no rule for them.
    pub fn string_field(&self, words: &Wording, spec: &str, conversion: &str) -> Option<String> {
        if !matches!(self, Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Real(_) | Value::Text(_) | Value::Flag(_) | Value::Null) { return None; }
        if !spec.is_empty() && conversion.is_empty() && matches!(self, Value::Flag(_) | Value::Null) { return None; }
        let mut shown = self.display(words);
        if let Value::Real(real) = self {
            let number = if real.below && real.p.is_zero() { -0.0 } else { as_binary(&real.p, &real.q) };
            shown = format!("{number:?}").to_ascii_lowercase();
            if let Some((mantissa, exponent)) = shown.split_once('e') {
                let power = exponent.parse::<i32>().ok()?;
                shown = format!("{mantissa}e{power:+03}");
            }
        }
        if let Value::Text(text) = self {
            if conversion == "r" || conversion == "a" {
                let quote = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
                shown = String::from(quote);
                for c in text.chars() {
                    match c {
                        '\\' => shown.push_str("\\\\"),
                        '\n' => shown.push_str("\\n"), '\r' => shown.push_str("\\r"), '\t' => shown.push_str("\\t"),
                        c if c == quote => { shown.push('\\'); shown.push(c); }
                        c if c.is_control() || conversion == "a" && !c.is_ascii() => {
                            let n = c as u32;
                            if n <= 255 { shown.push_str(&format!("\\x{n:02x}")); }
                            else if n <= 65535 { shown.push_str(&format!("\\u{n:04x}")); }
                            else { shown.push_str(&format!("\\U{n:08x}")); }
                        }
                        c => shown.push(c),
                    }
                }
                shown.push(quote);
            }
        }
        if conversion.is_empty() && matches!(self, Value::Small(_) | Value::Huge(_)) {
            let (radix, prefix) = match spec {
                "x" | "X" => (16, ""), "#x" => (16, "0x"), "#X" => (16, "0X"),
                "o" => (8, ""), "#o" => (8, "0o"), "b" => (2, ""), "#b" => (2, "0b"),
                _ => (0, ""),
            };
            if radix != 0 {
                let mut digits = self.as_big().ok()?.to_str_radix(radix);
                if spec.ends_with('X') { digits.make_ascii_uppercase(); }
                return Some(match digits.strip_prefix('-') {
                    Some(body) => format!("-{prefix}{body}"),
                    None => format!("{prefix}{digits}"),
                });
            }
        }
        if conversion.is_empty() && matches!(self, Value::Small(_) | Value::Huge(_)) {
            if let Some(digits) = spec.strip_suffix('d') {
                if digits.chars().all(|c| c.is_ascii_digit()) {
                    let width = if digits.is_empty() { Some(0) } else { digits.parse::<usize>().ok() };
                    if let Some(width) = width.filter(|n| *n <= 100000) {
                        let padding = width.saturating_sub(shown.len());
                        if digits.starts_with('0') {
                            return Some(if let Some(body) = shown.strip_prefix('-') { format!("-{}{body}", "0".repeat(padding)) }
                                else { format!("{}{shown}", "0".repeat(padding)) });
                        }
                        return Some(format!("{}{shown}", " ".repeat(padding)));
                    }
                }
            }
        }
        if conversion.is_empty() && !matches!(self, Value::Text(_)) {
            if let Some(places) = spec.strip_prefix('.').and_then(|s| s.strip_suffix('f')).and_then(|s| s.parse::<usize>().ok()).filter(|n| *n <= 1000) {
                if let Ok(number) = shown.parse::<f64>() { return Some(format!("{number:.places$}")); }
            }
        }
        if spec.is_empty() { return Some(shown); }
        let letters: Vec<char> = spec.chars().collect();
        let (fill, align, offset) = if letters.len() > 1 && matches!(letters[1], '<' | '>' | '^') {
            (letters[0], letters[1], 2)
        } else if letters.first().map_or(false, |c| matches!(c, '<' | '>' | '^')) { (' ', letters[0], 1) }
        else if letters.iter().all(char::is_ascii_digit) && letters.first() != Some(&'0') {
            (' ', if conversion.is_empty() && !matches!(self, Value::Text(_)) { '>' } else { '<' }, 0)
        } else { return None; };
        let width = if offset == letters.len() { Some(0) } else { letters[offset..].iter().collect::<String>().parse::<usize>().ok().filter(|n| *n <= 100000) };
        {
            let width = width?;
            let spaces = width.saturating_sub(shown.chars().count());
            let left = match align { '>' => spaces, '^' => spaces / 2, _ => 0 };
            shown = format!("{}{}{}", fill.to_string().repeat(left), shown, fill.to_string().repeat(spaces - left));
        }
        Some(shown)
    }

    /// The machine's own text for a value.
    pub fn plain(&self) -> String {
        match self {
            Value::Imaginary(n, _) => format!("{}j", shortest_real(*n)),
            Value::Collection(cell, _) => cell.borrow().plain(),
            Value::ValueMethod(_) => "<built-in method>".to_string(),
            Value::View(view) => format!("dict_{}({})", view.1, self.contents().plain()),
            Value::Native(_, word) => format!("<built-in function {}>", word),
            Value::Cursor(_) => "<iterator>".to_string(),
            Value::SetWalk(..) => "<set walk>".into(),
            Value::Set(s) => s.borrow().show(Value::plain),
            Value::Stream(error) => format!("<{} stream>", if *error { "error" } else { "output" }),
            Value::Counted(r) => if r.step.is_one() { format!("{}({}, {})", r.name, r.start, r.stop) }
                else { format!("{}({}, {}, {})", r.name, r.start, r.stop, r.step) },
            Value::Ellipsis => "Ellipsis".to_string(),
            Value::Small(n) => n.to_string(),
            Value::Huge(n) => n.to_string(),
            Value::Frac(r) => format!("{}/{}", r.p, r.q),
            Value::Real(r) if r.outside() => r.spelled().to_string(),
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
            Value::Generator(_) => "<generator>".to_string(),
            Value::Tuple(items) => format!("({}{})", items.iter().map(Value::plain).collect::<Vec<_>>().join(", "), if items.len() == 1 { "," } else { "" }),
            Value::Descriptor(_) => "<descriptor>".to_string(),
            Value::Routine(p) | Value::Method(_, p) => format!("<function({})>", p.formals.join(", ")),
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().plain(),
            Value::Class(c) => format!("<class {}>", c.name),
            Value::Object(o) => format!("<object {}>", o.class.name),
            Value::SortOf(k) => k.tag().to_string(),
            Value::Slice(parts) => format!("slice({}, {}, {})", parts[0].plain(), parts[1].plain(), parts[2].plain()),
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
            Value::Descriptor(d) => { let _ = write!(into, "d{:p}", Rc::as_ptr(d)); }
            Value::Method(o, p) => {
                let _ = write!(into, "m{:p}:{:p}", Rc::as_ptr(o), Rc::as_ptr(p));
            }
            Value::Routine(p) => {
                let _ = write!(into, "p{:p}", Rc::as_ptr(p));
            }
            Value::Object(o) => {
                let _ = write!(into, "o{:p}", Rc::as_ptr(o));
            }
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().memo_key(into),
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

/// How far a member of a class may be reached from: from anywhere, from
/// the class and those standing on it, or from the class alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reach {
    Open,
    Guarded,
    Hidden,
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
    /// How far each of those may be reached from, place for place.
    pub reaches: Vec<Reach>,
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
    /// Loosely, how the name is written is not part of it.
    pub fn named(&self, name: &str, loosely: bool) -> bool {
        let same = match loosely {
            true => self.name.eq_ignore_ascii_case(name),
            false => self.name == name,
        };
        same
            || self.base.as_ref().map_or(false, |b| b.named(name, loosely))
            || self.answers.iter().any(|a| a.named(name, loosely))
    }

    /// Whether this class is that one, stands on it, or answers to it,
    /// the name written just as it is.
    pub fn descends_from(&self, name: &str) -> bool {
        self.named(name, false)
    }

    /// How far a member of that name may be reached from, and the class
    /// that says so: the nearest one declaring it, this class first.
    pub fn reach_of(&self, name: &str) -> Option<(Reach, &str)> {
        match self.fields.iter().position(|(n, _)| n == name) {
            Some(at) => Some((self.reaches.get(at).copied().unwrap_or(Reach::Open), self.name.as_str())),
            None => self.base.as_ref().and_then(|b| b.reach_of(name)),
        }
    }

    /// Every property an object of this class begins with, those it
    /// stands on first, so a class of its own overrides them. A property
    /// the class keeps to itself is filed under its own name and the
    /// class's together, so that a class standing on it may declare one
    /// of the same name without the two becoming one.
    pub fn all_fields(&self) -> Vec<(String, Value)> {
        let mut all = self.base.as_ref().map_or_else(Vec::new, |b| b.all_fields());
        for (at, (name, value)) in self.fields.iter().enumerate() {
            let alone = self.reaches.get(at) == Some(&Reach::Hidden);
            let filed = match alone {
                true => kept_alone(name, &self.name),
                false => name.clone(),
            };
            match all.iter_mut().find(|(n, _)| *n == filed) {
                Some(place) => place.1 = value.clone(),
                None => all.push((filed, value.clone())),
            }
        }
        all
    }
}

/// A property a class keeps to itself, filed under its own name and the
/// class's together, so that two classes along one line may each have a
/// property of that name and neither be the other's.
pub fn kept_alone(name: &str, owner: &str) -> String {
    format!("{}\0{}", name, owner)
}

/// The name a property is filed under, taken apart: what it is called,
/// and the class that keeps it to itself where one does.
pub fn who_keeps(filed: &str) -> (&str, Option<&str>) {
    match filed.split_once('\0') {
        Some((name, owner)) => (name, Some(owner)),
        None => (filed, None),
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
    // Nought beneath is no ratio at all but the mark of a value
    // standing outside the numbers, and the width has one for each.
    if q.is_zero() {
        return match p.is_zero() {
            true => f64::NAN,
            false => beyond(),
        };
    }
    if q.is_one() {
        return p.to_f64().unwrap_or_else(beyond);
    }
    let (top, bottom) = (p.abs(), q.abs());
    let below = p.is_negative() != q.is_negative();
    let signed = |x: f64| if below { -x } else { x };
    // A bottom that is a power of two, which is what every real of the
    // width itself comes to, asks only for so many halvings, and a top
    // of that width is held to the last bit as it stands.
    if bottom.trailing_zeros() == Some(bottom.bits() - 1) && top.bits() <= 53 {
        return signed(by_twos(top.to_f64().unwrap_or(0.0), 1 - bottom.bits() as i64));
    }
    // Where both sides are held to the last bit by a real of the width,
    // dividing them gives the nearest real to the ratio outright.
    if top.bits() <= 53 && bottom.bits() <= 53 {
        return signed(top.to_f64().unwrap_or(0.0) / bottom.to_f64().unwrap_or(1.0));
    }
    // Otherwise the division is done on the whole numbers themselves,
    // since rounding each side to the width first and dividing after
    // rounds twice and need not land where rounding once lands. The top
    // is raised by as many twos as it takes for the answer to keep more
    // bits than the width holds, and they are taken off it again.
    let raise = bottom.bits() as i64 + 128 - top.bits() as i64;
    let raised = match raise >= 0 {
        true => top << raise as usize,
        false => top >> (-raise) as usize,
    };
    let (mut whole, left) = raised.div_rem(&bottom);
    // A bit set where the division did not come out even keeps the
    // rounding from falling the wrong way where the answer would
    // otherwise sit halfway between two reals of the width.
    if !left.is_zero() {
        whole.set_bit(0, true);
    }
    signed(by_twos(whole.to_f64().unwrap_or(f64::INFINITY), -raise))
}

/// A real of the width taken 2^n times, in steps small enough that only
/// the last of them can fall past what the width holds, so that the
/// answer is rounded once and no more.
fn by_twos(x: f64, n: i64) -> f64 {
    let mut worth = x;
    let mut left = n;
    while left != 0 && worth != 0.0 && worth.is_finite() {
        let step = left.clamp(-500, 500);
        worth *= (2.0f64).powi(step as i32);
        left -= step;
    }
    worth
}

/// What a binary real is worth, held exactly: a whole number of halves,
/// quarters and so on, which is all such a number ever is. Holding it
/// so is what makes the next step round as the width rounds, rather
/// than as the shortest way of writing it would.
pub fn from_binary(x: f64) -> Option<(BigInt, BigInt)> {
    if !x.is_finite() {
        return None;
    }
    if x == 0.0 {
        return Some((BigInt::zero(), BigInt::one()));
    }
    let bits = x.to_bits();
    let below = bits >> 63 == 1;
    let power = ((bits >> 52) & 0x7ff) as i64;
    let part = bits & 0x000f_ffff_ffff_ffff;
    // The smallest numbers of the width carry no leading one.
    let (whole, twos) = match power {
        0 => (part, -1074i64),
        _ => (part | (1u64 << 52), power - 1075),
    };
    let mut p = BigInt::from(whole);
    if below {
        p = -p;
    }
    Some(match twos >= 0 {
        true => (p << twos as usize, BigInt::one()),
        false => (p, BigInt::one() << twos.unsigned_abs() as usize),
    })
}

/// A binary real of the width as a value: held exactly where it is a
/// number, and as what stands outside the numbers where it is not. This
/// is the way back from working at the width, which every real-valued
/// reckoning must come home by.
pub fn real_of(x: f64, places: usize) -> Value {
    match from_binary(x) {
        // A nought that came out below nought keeps its minus.
        Some((p, q)) => crate::arith::shape_signed(p, q, Some(places), x.is_sign_negative()),
        None => outside_number(x, places),
    }
}

/// The value standing for what no number answers to, or for what lies
/// past every number on the side the sign says.
pub fn outside_number(x: f64, places: usize) -> Value {
    let p = match (x.is_nan(), x.is_sign_negative()) {
        (true, _) => BigInt::zero(),
        (_, true) => -BigInt::one(),
        _ => BigInt::one(),
    };
    Value::Real(Rc::new(Real { p, q: BigInt::zero(), places, below: false, point: false }))
}

/// A real brought to the nearest one of a width of bits, held exactly.
/// Where the language holds no width, or the number is past every one
/// of that width, it is left as it stands.
pub fn to_binary_width(v: Value, bits: Option<usize>, places: usize) -> Value {
    if bits.is_none() {
        return v;
    }
    let (p, q, below) = match &v {
        Value::Real(r) => (r.p.clone(), r.q.clone(), r.below),
        Value::Frac(r) => (r.p.clone(), r.q.clone(), false),
        _ => return v,
    };
    match from_binary(as_binary(&p, &q)) {
        // A nought below nought keeps its minus at any width.
        Some((p, q)) => crate::arith::shape_signed(p, q, Some(places), below).with_point(v.keeps_point()),
        None => v,
    }
}

thread_local! {
    /// The cells a run keeps its counts of figures in, where the
    /// language gives those counts a name of their own: how many
    /// figures a real written plainly carries, and how many one shown
    /// with its kind carries. Both stand empty for a language that
    /// keeps no such count.
    static FIGURES: RefCell<(Option<Rc<RefCell<Value>>>, Option<Rc<RefCell<Value>>>)> = const { RefCell::new((None, None)) };
}

/// Hand the kernel the cells the run keeps its counts of figures in.
/// The run writes into them by the names the definition gives, so a
/// count set while the run goes is the count the next real written out
/// follows.
pub fn figures_kept_in(plainly: Option<Rc<RefCell<Value>>>, with_kind: Option<Rc<RefCell<Value>>>) {
    FIGURES.with(|held| *held.borrow_mut() = (plainly, with_kind));
}

/// What the run's count of figures stands at now. Nothing at all where
/// the run keeps no count of its own; within that, a count of figures,
/// or nothing again where the count is below nought, by which the run
/// asks for the fewest figures that read back as the same number.
fn figures_now(with_kind: bool) -> Option<Option<usize>> {
    FIGURES.with(|held| {
        let held = held.borrow();
        let cell = match with_kind {
            true => held.1.as_ref()?,
            false => held.0.as_ref()?,
        };
        let said = cell.borrow();
        let count = match &*said {
            Value::Small(n) => *n,
            Value::Huge(n) => n.to_i64().unwrap_or(0),
            Value::Text(s) => s.trim().parse().unwrap_or(0),
            _ => 0,
        };
        // No real of the width spells out more figures than the widest
        // of them needs, so a greater count asks only for noughts, and
        // those are dropped again below.
        Some(match count >= 0 {
            true => Some(count.min(1100) as usize),
            false => None,
        })
    })
}

/// A binary real written out the way such a language writes one: the
/// fewest digits that read back as the same number, with a power of ten
/// after them where the number is very large or very small. `digits`
/// caps the significant figures, as a language's own setting does when
/// a number is simply written out rather than shown with its kind.
pub fn written_out(x: f64, digits: Option<usize>) -> String {
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
    // spelled out beyond that, which is where such a language changes:
    // at as many figures as are being shown, which is the count the
    // language sets when it is set and the whole of what tells one such
    // number from its neighbours when it is not.
    if (-4..digits.unwrap_or(17) as i32).contains(&power) {
        return format!("{}{}", sign, laid_flat(&figures, power));
    }
    let rest = &figures[1..];
    let after = if rest.is_empty() { "0".to_string() } else { rest.to_string() };
    let mark = if power < 0 { "-" } else { "+" };
    format!("{}{}.{}E{}{}", sign, &figures[..1], after, mark, power.abs())
}

/// A binary real written out as the run shows one with its kind: to the
/// count of figures asked for, else to the count the run keeps for
/// showing one, else in the fewest that read back as the same number.
pub fn binary_string(x: f64, digits: Option<usize>) -> String {
    written_out(x, digits.or_else(|| figures_now(true).flatten()))
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


/// Write an imaginary coefficient in the fewest figures, with a sign
/// and two places at least for the power of ten.
fn shortest_real(x: f64) -> String {
    if !x.is_finite() { return x.to_string().to_ascii_lowercase(); }
    let scientific = format!("{:e}", x);
    let (mantissa, exponent) = scientific.split_once('e').expect("a power follows");
    let power: i32 = exponent.parse().expect("a whole power");
    if x != 0.0 && !(-4..16).contains(&power) {
        return format!("{}e{}{:02}", mantissa, if power < 0 { "-" } else { "+" }, power.unsigned_abs());
    }
    x.to_string()
}
