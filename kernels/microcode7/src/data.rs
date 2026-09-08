// Values. A program value is a closure: the program and the frame it was
// made in, so a nested program sees the bindings around it.

use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

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
    /// A nought that came of working with something under nought holds
    /// on to the minus: a real of a width has two noughts, and a
    /// language holding reals to a width writes each its own way.
    pub under: bool,
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
    /// A cell more than one name stands for: what one writes, the others
    /// read. Never a value a program holds by itself.
    Shared(Rc<RefCell<Value>>),
    Blueprint(Rc<Blueprint>),
    Thing(Rc<Thing>),
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
    /// Whether a flag becomes text as the number it counts for: one
    /// holding true becomes `1`, one holding false nothing whatever.
    pub flag_counted: bool,
    /// Where a language holds its reals to a width of bits, how many
    /// figures one shows when simply written out; where it says
    /// nothing, a real is shown to the precision it carries.
    pub real_figures: Option<usize>,
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
            Value::Shared(cell) => return cell.borrow().kind(),
            Value::Couple(_) | Value::Blueprint(_) | Value::Thing(_) => return None,
            Value::Routine(_) | Value::Bound(..) | Value::Unset => return None,
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
            Value::Blueprint(_) | Value::Thing(_) => return Err("Cannot coerce object to number".to_string()),
            Value::Shared(cell) => return cell.borrow().as_big(),
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
            // One object is itself and nothing else; two classes are one
            // when they carry the same name.
            (Value::Thing(a), Value::Thing(b)) => Rc::ptr_eq(a, b),
            (Value::Blueprint(a), Value::Blueprint(b)) => a.name == b.name,
            (Value::Bound(a, _), Value::Bound(b, _)) => Rc::ptr_eq(a, b),
            (Value::KindOf(a), Value::KindOf(b)) => a == b,
            _ => false,
        }
    }

    /// One and the same, which asks more than being equal: the two must
    /// also be of one kind, so a whole number and a decimal standing for
    /// the same amount are equal and yet not the same. Values that hold
    /// others are the same when they hold the same keys in the same
    /// order, each holding what is itself the same.
    pub fn selfsame(&self, other: &Value) -> bool {
        if let Value::Shared(cell) = self {
            let held = cell.borrow().clone();
            return held.selfsame(other);
        }
        if let Value::Shared(cell) = other {
            let held = cell.borrow().clone();
            return self.selfsame(&held);
        }
        let alike = |one: &[(Value, Value)], two: &[(Value, Value)]| {
            one.len() == two.len() && one.iter().zip(two.iter()).all(|((j, x), (k, y))| j.selfsame(k) && x.selfsame(y))
        };
        let numbered = |items: &[Value]| -> Vec<(Value, Value)> {
            items.iter().enumerate().map(|(at, x)| (Value::Small(at as i64), x.clone())).collect()
        };
        match (self, other) {
            (Value::Vector(a), Value::Vector(b)) => alike(&numbered(a), &numbered(b)),
            (Value::Dict(a), Value::Dict(b)) => alike(a, b),
            // Written with keys or written without, an array is an
            // array; the keys themselves then say whether it matches.
            (Value::Vector(a), Value::Dict(b)) => alike(&numbered(a), b),
            (Value::Dict(a), Value::Vector(b)) => alike(a, &numbered(b)),
            _ => match (self.kind(), other.kind()) {
                (Some(here), Some(there)) => here == there && self.equals(other),
                _ => self.equals(other),
            },
        }
    }

    pub fn render(&self, w: Names) -> String {
        match self {
            // A cell that names share is written as what it holds.
            Value::Shared(cell) => cell.borrow().render(w),
            Value::Flag(true) if w.flag_counted => "1".to_string(),
            Value::Flag(false) if w.flag_counted => String::new(),
            Value::Flag(true) => w.truth.to_string(),
            Value::Flag(false) => w.falsity.to_string(),
            Value::Nil | Value::Unset => w.nil.to_string(),
            Value::Vector(items) => format!("[{}]", items.iter().map(|v| v.render(w)).collect::<Vec<_>>().join(", ")),
            Value::Dict(entries) => {
                format!("[{}]", entries.iter().map(|(k, v)| format!("{} => {}", k.render(w), v.render(w))).collect::<Vec<_>>().join(", "))
            }
            Value::Couple(e) => format!("{} => {}", e.0.render(w), e.1.render(w)),
            // A nought under nought is written so, at any width.
            Value::Frac(e) if e.under && num_traits::Zero::is_zero(&e.above) => "-0".to_string(),
            // A language whose reals are numbers of bits writes one to
            // its own count of figures.
            Value::Frac(e) if w.real_figures.is_some() => figured(nearest_binary(&e.above, &e.beneath), w.real_figures),
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
            Value::Shared(cell) => cell.borrow().bare(),
            Value::Blueprint(b) => format!("<class {}>", b.name),
            Value::Thing(t) => format!("<object {}>", t.of.name),
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
            Value::Thing(t) => out.push_str(&format!("t{:p}", Rc::as_ptr(t))),
            Value::Shared(cell) => cell.borrow().memo_key(out),
            Value::Blueprint(b) => out.push_str(&format!("b{}", b.name)),
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

/// A class: its name, what it is built on, the properties a thing of it
/// starts with, the programs it answers to, its constants, and the
/// values it keeps for itself rather than for its things.
#[derive(Debug)]
pub struct Blueprint {
    pub name: String,
    pub under: Option<Rc<Blueprint>>,
    /// The classes of method names only that this one answers to.
    pub answers: Vec<Rc<Blueprint>>,
    pub fields: Vec<(String, Value)>,
    pub methods: Vec<(String, Rc<Routine>)>,
    pub constants: Vec<(String, Value)>,
    pub shared: RefCell<Vec<(String, Value)>>,
}

impl Blueprint {
    pub fn program(&self, name: &str) -> Option<&Rc<Routine>> {
        match self.methods.iter().find(|(n, _)| n == name) {
            Some((_, p)) => Some(p),
            None => self.under.as_ref().and_then(|u| u.program(name)),
        }
    }

    pub fn constant(&self, name: &str) -> Option<&Value> {
        match self.constants.iter().find(|(n, _)| n == name) {
            Some((_, v)) => Some(v),
            None => self.under.as_ref().and_then(|u| u.constant(name)),
        }
    }

    /// The class along the line that keeps a value of that name.
    pub fn keeper(&self, name: &str) -> Option<&Blueprint> {
        if self.shared.borrow().iter().any(|(n, _)| n == name) {
            return Some(self);
        }
        self.under.as_ref().and_then(|u| u.keeper(name))
    }

    pub fn built_on(&self, name: &str) -> bool {
        self.goes_by(name, false)
    }

    /// The same, save that letters written large and small may be
    /// counted the one letter where a language asks for that.
    pub fn goes_by(&self, name: &str, either_way: bool) -> bool {
        let it = match either_way {
            true => self.name.eq_ignore_ascii_case(name),
            false => self.name == name,
        };
        it || self.under.as_ref().map_or(false, |u| u.goes_by(name, either_way))
            || self.answers.iter().any(|a| a.goes_by(name, either_way))
    }

    /// Every property a thing of this class starts with, what it is
    /// built on first, so this class has the last word.
    pub fn every_field(&self) -> Vec<(String, Value)> {
        let mut all = self.under.as_ref().map_or_else(Vec::new, |u| u.every_field());
        for (name, value) in &self.fields {
            match all.iter_mut().find(|(n, _)| n == name) {
                Some(place) => place.1 = value.clone(),
                None => all.push((name.clone(), value.clone())),
            }
        }
        all
    }
}

/// One thing: the class it was made from and what it holds. Naming a
/// thing twice names one thing, so a write through either name shows in
/// both.
#[derive(Debug)]
pub struct Thing {
    pub of: Rc<Blueprint>,
    pub holds: RefCell<Vec<(String, Value)>>,
    /// Which thing this is by the turn it was made in, counting from
    /// one, for a language that names them when showing them.
    pub turn: usize,
}

/// A ratio as the nearest binary number of sixty-four bits. One too
/// large for such a number to hold stands past all of them.
pub fn nearest_binary(above: &BigInt, beneath: &BigInt) -> f64 {
    let past = || if above.is_negative() { f64::NEG_INFINITY } else { f64::INFINITY };
    if beneath.is_one() {
        return above.to_f64().unwrap_or_else(past);
    }
    match (above.to_f64(), beneath.to_f64()) {
        (Some(x), Some(y)) if x.is_finite() && y.is_finite() && y != 0.0 => x / y,
        _ => {
            let down = above.bits().max(beneath.bits()).saturating_sub(900);
            let (x, y) = (above >> down, beneath >> down);
            match (x.to_f64(), y.to_f64()) {
                (Some(x), Some(y)) if y != 0.0 => x / y,
                _ => past(),
            }
        }
    }
}

/// What a binary real is worth, held as a ratio: so many halves,
/// quarters and eighths, which is the whole of what such a number is.
/// Holding it that way is what makes the step after it round as the
/// width rounds, and not as the shortest way of writing it would.
pub fn binary_worth(x: f64) -> Option<(BigInt, BigInt)> {
    if !x.is_finite() {
        return None;
    }
    if x == 0.0 {
        return Some((BigInt::zero(), BigInt::one()));
    }
    let held = x.to_bits();
    let under = held >> 63 == 1;
    let step = ((held >> 52) & 0x7ff) as i64;
    let rest = held & 0x000f_ffff_ffff_ffff;
    // The very smallest of the width carry no leading one.
    let (run, halvings) = match step {
        0 => (rest, -1074i64),
        _ => (rest | (1u64 << 52), step - 1075),
    };
    let mut above = BigInt::from(run);
    if under {
        above = -above;
    }
    Some(match halvings >= 0 {
        true => (above << halvings as usize, BigInt::one()),
        false => (above, BigInt::one() << halvings.unsigned_abs() as usize),
    })
}

/// A real brought to the nearest of a width of bits, held exactly.
/// Where the language holds no width, or the number stands past every
/// one of that width, it is left as it is.
pub fn at_binary_width(v: Value, bits: Option<usize>, figures: usize) -> Value {
    if bits.is_none() {
        return v;
    }
    let Value::Frac(e) = &v else { return v };
    match binary_worth(nearest_binary(&e.above, &e.beneath)) {
        // A nought under nought holds its minus at any width.
        Some((above, beneath)) => crate::math::made_number(above, beneath, Some(figures), e.under),
        None => v,
    }
}

/// A binary real written out: the fewest figures that read back as the
/// same number, with a power of ten after them where it stands very
/// high or very low. `figures` caps them, as a language's own setting
/// does where a number is written out rather than shown with its kind.
pub fn figured(x: f64, figures: Option<usize>) -> String {
    if x.is_nan() {
        return "NAN".to_string();
    }
    if x.is_infinite() {
        return if x < 0.0 { "-INF".to_string() } else { "INF".to_string() };
    }
    let shown = match figures {
        Some(n) => format!("{:.*e}", n.saturating_sub(1), x),
        None => format!("{:e}", x),
    };
    let (front, power) = shown.split_once('e').expect("a power of ten was asked for");
    let power: i32 = power.parse().unwrap_or(0);
    let mut run = front.trim_start_matches('-').replace('.', "");
    if figures.is_some() {
        while run.len() > 1 && run.ends_with('0') {
            run.pop();
        }
    }
    let sign = if front.starts_with('-') { "-" } else { "" };
    if (-4..15).contains(&power) {
        let point = power + 1;
        let body = if point <= 0 {
            format!("0.{}{}", "0".repeat(-point as usize), run)
        } else if point as usize >= run.len() {
            format!("{}{}", run, "0".repeat(point as usize - run.len()))
        } else {
            format!("{}.{}", &run[..point as usize], &run[point as usize..])
        };
        return format!("{}{}", sign, body);
    }
    let tail = &run[1..];
    let after = if tail.is_empty() { "0" } else { tail };
    format!("{}{}.{}E{}{}", sign, &run[..1], after, if power < 0 { "-" } else { "+" }, power.abs())
}
