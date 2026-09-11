// Values. A program value is a closure: the program and the frame it was
// made in, so a nested program sees the bindings around it.

use std::cell::RefCell;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
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
    Set,
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
            Kind::Set => "SET",
            Kind::Nothing => "NULL",
        }
    }
}

/// above/beneath in lowest terms; a real carries the places it shows.
///
/// A real of a width carries two worths besides that no ratio does: the
/// one nothing whatever is equal to, itself included, and the one lying
/// past every number on either hand. Both are written with nought
/// beneath, which no ratio brought to lowest terms ever is; the top then
/// says which, being nought for the first and its sign for the second.
/// They are kept so rather than as a kind of their own because a new
/// kind must be answered for wherever a worth is looked at, while nought
/// beneath is answered for where numbers are worked and weighed against
/// each other and in no other place.
#[derive(Debug, Clone)]
pub struct Ratio {
    pub float_style: bool,
    pub above: BigInt,
    pub beneath: BigInt,
    pub places: Option<usize>,
    /// A nought that came of working with something under nought holds
    /// on to the minus: a real of a width has two noughts, and a
    /// language holding reals to a width writes each its own way.
    pub under: bool,
    /// Whether this real came through a form which writes the point
    /// even when no figures follow it but nought.
    pub pointed: bool,
}

impl Ratio {
    /// Whether this worth stands past the numbers, either way.
    pub fn past_numbers(&self) -> bool {
        self.beneath.is_zero()
    }

    /// Whether it is the one nothing whatever is equal to.
    pub fn answers_none(&self) -> bool {
        self.beneath.is_zero() && self.above.is_zero()
    }

    /// How such a worth is written, which is one of three ways.
    pub fn written(&self) -> &'static str {
        match (self.past_numbers(), self.above.is_zero(), self.above.is_negative()) {
            (false, _, _) => "",
            (_, true, _) => "NAN",
            (_, _, true) => "-INF",
            _ => "INF",
        }
    }
}

/// Three numbers suffice for a walk, however far its end stands.
#[derive(Clone)]
pub struct Progression {
    pub first: BigInt,
    pub limit: BigInt,
    pub stride: BigInt,
    pub word: String,
}

impl Progression {
    /// Where the three bounds each sit inside a machine word, the size
    /// of the walk and every member of it can be reckoned in the wider
    /// word, sparing the great numbers. A stride of nought is refused
    /// here and left to the older road, which answers as it always did.
    fn plain_bounds(&self) -> Option<(i128, i128, i128)> {
        let stride = i128::from(self.stride.to_i64()?);
        if stride == 0 { return None; }
        let first = i128::from(self.first.to_i64()?);
        let limit = i128::from(self.limit.to_i64()?);
        let span = if stride < 0 { first - limit } else { limit - first };
        let size = if span > 0 { (span - 1) / stride.abs() + 1 } else { 0 };
        Some((first, stride, size))
    }

    pub fn count(&self) -> BigInt {
        if let Some((_, _, size)) = self.plain_bounds() { return BigInt::from(size); }
        let forward = self.stride > BigInt::zero();
        if (forward && self.first >= self.limit) || (!forward && self.first <= self.limit) {
            return BigInt::zero();
        }
        ((&self.limit - &self.first).abs() - BigInt::one()) / self.stride.abs() + BigInt::one()
    }

    pub fn item(&self, position: &BigInt) -> Option<Value> {
        // A loop over a progression asks this at every turn, so the
        // plain answer is given without a great number being made.
        if let Some((first, stride, size)) = self.plain_bounds() {
            if let Some(asked) = position.to_i64() {
                let mut offset = i128::from(asked);
                if offset < 0 { offset += size; }
                if offset < 0 || offset >= size { return None; }
                return Some(Value::Small((first + stride * offset) as i64));
            }
        }
        let count = self.count();
        let offset = if position < &BigInt::zero() { position + &count } else { position.clone() };
        if offset < BigInt::zero() || offset >= count { return None; }
        Some(Value::from_big(&self.first + &self.stride * offset))
    }
}

/// A walk may keep a member aside while a loop asks whether it has more.
#[derive(Clone)]
pub struct IteratorState {
    pub kind: IteratorKind,
    pub peek: Option<Value>,
    pub done: bool,
}

#[derive(Clone)]
pub enum IteratorKind {
    Stored(std::collections::VecDeque<Value>),
    /// A progression stepped through one place at a time.
    Stepping(Rc<Progression>, BigInt),
    /// A list read through its cell at every step, so that members put
    /// in before the end are walked as well.
    Living(Rc<RefCell<Value>>, usize),
    /// A window upon a dictionary, and the size the dictionary had at
    /// the start: a different size later stops the walk.
    Watching { window: Value, at: usize, size: usize },
    /// A thing read place by place from nought, until the reading fails.
    Placed(Value, BigInt),
    /// A callable summoned for each member until it answers the sentinel.
    Summoned { work: Value, stop: Value },
    Count(Value, BigInt),
    /// Inputs walked abreast, mapped where a mapper is given, and made
    /// to end together where exactness is demanded.
    Parallel { inputs: Vec<Value>, mapper: Option<Value>, exact: bool },
    Select(Value, Value),
    Busy,
}

#[derive(Clone)]
pub enum Value {
    Mutable(Rc<RefCell<Value>>, bool),
    Member(Rc<Value>, String),
    Window(Rc<Value>, char),
    Row(Rc<Vec<Value>>),
    Intrinsic(Rc<str>),
    Iterator(Rc<RefCell<IteratorState>>),
    Backtrace(Rc<str>),
    Keyed(Rc<Value>, Rc<Value>),
    Attributes(Rc<Thing>),
    Traversal(Rc<Value>, Rc<RefCell<Option<Value>>>),
    Refusal(Rc<str>),
    Cursor(Rc<RefCell<std::collections::VecDeque<Value>>>),
    Arguments(Rc<Vec<Value>>),
    Octets { cell: Rc<RefCell<Vec<u8>>>, changeable: bool, lead: Rc<str> },
    OctetKind { changeable: bool, shown: Rc<str> },
    Wrapped(u8, Rc<Vec<Value>>),
    Channel(u8),
    Progression(Rc<Progression>),
    Small(i64),
    Huge(Rc<BigInt>),
    Frac(Rc<Ratio>),
    /// The coefficient of an imaginary literal, with its unready words.
    Imaginary { coefficient: f64, unready: Rc<str> },
    Complex(Rc<(f64, f64, Rc<str>)>),
    Text(Rc<str>),
    TextRow(Rc<Vec<String>>, bool),
    TextCall { subject: Rc<str>, work: crate::text::Work, name: Rc<str> },
    Flag(bool),
    Nil,
    Ellipsis,
    Vector(Rc<Vec<Value>>),
    Tuple(Rc<Vec<Value>>),
    Generator(Rc<RefCell<crate::exec::Suspension>>),
    Set(Rc<RefCell<SetStore>>),
    SetCursor { source: Rc<RefCell<SetStore>>, count: usize },
    /// A span awaiting the length of what it is to read.
    Span(Rc<Vec<Value>>),
    /// Keys with their values, kept in the order they were written.
    Dict(Rc<Vec<(Value, Value)>>),
    /// A key written together with its value (`k => v`), until a
    /// literal takes it in.
    Couple(Rc<(Value, Value)>),
    /// A cell more than one name stands for: what one writes, the others
    /// read. A language keeping collections between calls uses this
    /// cell for the collection, without fastening its names together.
    Shared(Rc<RefCell<Value>>),
    Blueprint(Rc<Blueprint>),
    Thing(Rc<Thing>),
    /// A program not yet bound to a frame: only inside the tree.
    Routine(Rc<Routine>),
    Method(Rc<Routine>, Rc<Thing>),
    Adorned(Rc<Adornment>),
    /// A program bound to the frame it was made in.
    Bound(Rc<Routine>, Rc<Env>),
    KindOf(Kind),
    Unset,
}

/// A member keeps its callable and, where needed, its writer or receiver.
pub struct Adornment {
    pub manner: char,
    pub target: Value,
    pub extra: Option<Value>,
}

/// Members keep both their hash address and their place in the telling.
#[derive(Clone, Debug)]
pub struct SetStore {
    pub entries: Vec<(String, Value)>,
    pub keys: std::collections::HashSet<String>,
    pub spelling: String,
}

impl SetStore {
    pub fn new(spelling: &str) -> SetStore {
        SetStore { entries: vec![], keys: Default::default(), spelling: spelling.into() }
    }

    pub fn put(&mut self, address: String, item: Value) {
        if self.keys.insert(address.clone()) { self.entries.push((address, item)); }
    }

    pub fn take(&mut self, address: &str) -> Option<Value> {
        if !self.keys.remove(address) { return None; }
        let place = self.entries.iter().position(|(k, _)| k == address).unwrap();
        Some(self.entries.remove(place).1)
    }

    pub fn values(&self) -> Vec<Value> {
        self.entries.iter().map(|(_, v)| v.clone()).collect()
    }

    pub fn merge(&self, rhs: &SetStore, rule: u8) -> SetStore {
        let mut answer = SetStore::new(&self.spelling);
        for (key, item) in self.entries.iter().chain(rhs.entries.iter()) {
            let left = self.keys.contains(key);
            let right = rhs.keys.contains(key);
            let keep = match rule { 0 => left || right, 1 => left && right, 2 => left && !right, _ => left != right };
            if keep { answer.put(key.clone(), item.clone()); }
        }
        answer
    }

    pub fn written(&self, item_text: impl Fn(&Value) -> String) -> String {
        match self.entries.len() {
            0 => self.spelling.clone() + "()",
            _ => {
                let words = self.entries.iter().map(|(_, item)| item_text(item)).collect::<Vec<_>>();
                "{".to_owned() + &words.join(", ") + "}"
            }
        }
    }
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
    pub bit_reals: bool,
    pub brief_reals: bool,
    /// The words for a member a class shares only with those built on
    /// it, and for one it keeps to itself, as they are written beside
    /// the name where a thing is shown.
    pub within_word: Option<&'a str>,
    pub alone_word: Option<&'a str>,
    /// Whether text is kept as bytes, in which case a character of it
    /// is one byte and the width of a piece of text is how many
    /// characters it has rather than what the letters would take.
    pub kept_as_bytes: bool,
    /// Whether a map's keys are compared by worth: a flag as the number
    /// it counts for, a whole number as one key with its real.
    pub keys_by_worth: bool,
}

impl Value {
    pub fn point_kept(&self) -> bool {
        match self {
            Self::Frac(e) => e.pointed,
            Self::Shared(held) => held.borrow().point_kept(),
            _ => false,
        }
    }

    pub fn keeping_point(mut self, wanted: bool) -> Value {
        match &mut self {
            Self::Frac(e) if wanted && e.places.is_some() => Rc::make_mut(e).pointed = true,
            _ => {}
        }
        self
    }

    pub fn settled(&self) -> Value {
        if let Value::Mutable(place, _) | Value::Shared(place) = self { return place.borrow().settled(); }
        if let Value::Window(owner, portion) = self {
            let mut items=Vec::new();
            if let Value::Dict(entries)=owner.settled() {
                for (key,value) in entries.iter() {
                    items.push(if *portion=='k' {key.clone()} else if *portion=='v' {value.clone()} else {Value::Row(Rc::new(vec![key.clone(),value.clone()]))});
                }
            }
            return Value::Vector(Rc::new(items));
        }
        self.clone()
    }

    pub fn keep(self, quoted: bool) -> Value {
        if let Value::Shared(cell) = self { return Value::Mutable(cell, quoted); }
        if matches!(self, Value::Vector(_) | Value::Dict(_)) {
            Value::Mutable(Rc::new(RefCell::new(self)), quoted)
        } else { self }
    }

    pub fn repr(&self, names: &Names) -> String {
        if let Value::Mutable(cell, _) | Value::Shared(cell) = self { return within_cell(cell, |inner| inner.repr(names)); }
        let settled = self.settled();
        match &settled {
            Value::Text(_) => settled.in_field(*names, "", "r").unwrap_or_else(|| settled.bare()),
            Value::Vector(v) | Value::Row(v) => {
                let body = v.iter().map(|item| item.repr(names)).collect::<Vec<_>>().join(", ");
                if matches!(settled, Value::Row(_)) { format!("({body}{})", if v.len() == 1 { "," } else { "" }) }
                else { format!("[{body}]") }
            }
            Value::Dict(entries) => {
                let body = entries.iter().map(|entry| entry.0.repr(names) + ": " + &entry.1.repr(names)).collect::<Vec<_>>().join(", ");
                format!("{{{body}}}")
            }
            _ => settled.render(*names),
        }
    }

    pub fn set_member_spelling(&self, names: Names) -> String {
        let shown = self.in_field(names, "", "r").unwrap_or_else(|| self.render(names));
        match self {
            Value::Frac(r) if r.places.is_some() && !r.past_numbers() && !shown.chars().any(|c| matches!(c, '.' | 'E' | 'e')) => shown + ".0",
            _ => shown,
        }
    }

    pub fn hash_address(&self) -> Result<String, &'static str> {
        match self {
            Value::Tuple(items) | Value::Row(items) => {
                let parts = items.iter().map(Value::hash_address).collect::<Result<Vec<_>, _>>()?;
                Ok(format!("row:{parts:?}"))
            }
            Value::Shared(slot) | Value::Mutable(slot, _) => slot.borrow().hash_address(),
            Value::Vector(_) => Err("list"),
            Value::Dict(_) => Err("dict"),
            Value::Set(_) => Err("set"),
            Value::Text(word) => Ok(format!("text:{word}")),
            Value::Flag(b) => Ok(format!("number:{}:1", u8::from(*b))),
            Value::Nil => Ok("nothing".to_owned()),
            Value::Ellipsis => Ok("ellipsis".to_owned()),
            value => {
                let Some(ratio) = crate::math::ratio_of(value) else { return Err(""); };
                if ratio.beneath.is_zero() { return Err(""); }
                let divisor = ratio.above.gcd(&ratio.beneath);
                Ok(format!("number:{}:{}", ratio.above / &divisor, ratio.beneath / divisor))
            }
        }
    }

    pub fn representation(&self, words: Names) -> String {
        if let Value::Text(text) = self {
            let mark = if text.contains('\'') && !text.contains('"') { '"' } else { '\'' };
            let letters = text.chars().map(|ch| match ch {
                '\n' => "\\n".into(), '\t' => "\\t".into(), '\r' => "\\r".into(),
                '\\' => "\\\\".into(),
                x if x == mark => format!("\\{x}"),
                x if x.is_control() => format!("\\x{:02x}", x as u32),
                x => x.to_string(),
            }).collect::<String>();
            return format!("{mark}{letters}{mark}");
        }
        match self {
            Value::Arguments(row) => Self::argument_text(row, words),
            Value::Thing(thing) => match self.arguments_held() {
                Some(row) => format!("{}({})", thing.of.name, row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", ")),
                None => self.render(words),
            },
            Value::Vector(row) => format!("[{}]", row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", ")),
            _ => self.render(words),
        }
    }

    fn argument_text(row: &[Value], words: Names) -> String {
        let contents = row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", ");
        if row.len() == 1 { format!("({contents},)") } else { format!("({contents})") }
    }

    fn arguments_held(&self) -> Option<Vec<Value>> {
        if let Value::Thing(thing) = self {
            if thing.of.every_field().iter().any(|(key, _)| key == "\0fault-kind") {
                let holds = thing.holds.borrow();
                return Some(match holds.iter().find(|(key, _)| key == "\0raised-values") {
                    Some((_, Value::Arguments(row))) => row.to_vec(),
                    _ => holds.iter().filter(|(key, value)| key == "message" && !matches!(value, Value::Text(s) if s.is_empty())).map(|(_, value)| value.clone()).collect(),
                });
            }
        }
        None
    }

    pub fn raised_words(&self, words: Names) -> Option<String> {
        let row = self.arguments_held()?;
        let Value::Thing(thing) = self else { return None };
        // A gatherer, or a system fault with its number, was given the
        // words to show itself with when it was made.
        if let Some((_, Value::Text(told))) = thing.holds.borrow().iter().find(|(key, _)| key == "\0told-as") { return Some(told.to_string()); }
        Some(if row.is_empty() { String::new() }
            else if row.len() > 1 { Self::argument_text(&row, words) }
            else if thing.of.every_field().iter().any(|(key, _)| key == "\0key-fault") { row[0].representation(words) }
            else { row[0].render(words) })
    }

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
            Value::TextRow(..) | Value::Vector(_) | Value::Dict(_) | Value::Row(_) => Kind::Vector,
            Value::Mutable(place, _) => return place.borrow().kind(),
            Value::Member(..) | Value::Complex(_) => return None,
            Value::Window(..) => Kind::Vector,
            Value::Nil | Value::KindOf(_) => Kind::Nothing,
            Value::Shared(cell) => return cell.borrow().kind(),
            Value::Wrapped(..) | Value::Octets { .. } | Value::OctetKind { .. } | Value::Arguments(_) | Value::Backtrace(_) | Value::Keyed(..) | Value::Attributes(_) | Value::Traversal(..) | Value::Refusal(_) | Value::Cursor(_) | Value::SetCursor { .. } | Value::Couple(_) | Value::Blueprint(_) | Value::Thing(_) => return None,
            Value::TextCall { .. } | Value::Intrinsic(_) | Value::Iterator(_) | Value::Adorned(_) | Value::Generator(_) | Value::Tuple(_) | Value::Imaginary { .. } | Value::Ellipsis | Value::Method(..) | Value::Routine(_) | Value::Bound(..) | Value::Unset | Value::Span(_) | Value::Channel(_) | Value::Progression(_) => return None,
            Value::Set(_) => Kind::Set,
        })
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Complex(pair) => pair.0 != 0.0 || pair.1 != 0.0,
            Value::Imaginary { coefficient, .. } => *coefficient != 0.0,
            Value::Mutable(cell, _) => cell.borrow().is_true(),
            Value::Row(items) => !items.is_empty(),
            Value::Window(..) => match self.settled() {Value::Vector(items)=>!items.is_empty(),_=>false},
            Value::Set(items) => !items.borrow().keys.is_empty(),
            Value::Arguments(row) => !row.is_empty(),
            Value::Octets { cell, .. } => cell.borrow().len() > 0,
            Value::Progression(walk) => walk.count() != BigInt::zero(),
            Value::Flag(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Huge(n) => !n.is_zero(),
            // Neither worth standing past the numbers is nought, so
            // both count as true, though the top of the one is nought.
            Value::Frac(e) => e.past_numbers() || !e.above.is_zero(),
            Value::TextRow(words, _) => !words.is_empty(),
            Value::Text(s) => !s.is_empty(),
            Value::Tuple(parts) => !parts.is_empty(),
            Value::Nil | Value::Unset => false,
            _ => true,
        }
    }

    pub fn as_big(&self) -> Result<BigInt, String> {
        Ok(match self {
            Value::Complex(pair) => return Err(pair.2.to_string()),
            Value::Imaginary { unready, .. } => return Err(unready.to_string()),
            Value::Small(n) => BigInt::from(*n),
            Value::Huge(n) => (**n).clone(),
            // A worth past the numbers has no whole part; a language
            // holding reals to a width counts it as nought.
            Value::Frac(e) if e.past_numbers() => BigInt::zero(),
            Value::Frac(e) if e.places.is_some() => &e.above / &e.beneath,
            Value::Frac(_) => return Err("Cannot coerce rational to integer".to_string()),
            Value::Flag(b) => BigInt::from(*b as i64),
            Value::Nil | Value::Unset => BigInt::zero(),
            Value::Text(s) => s.parse().map_err(|_| format!("Cannot coerce '{}' to number", s))?,
            Value::TextRow(..) | Value::Arguments(_) | Value::Set(_) | Value::Tuple(_) | Value::Vector(_) | Value::Dict(_) | Value::Couple(_) | Value::Row(_) | Value::Window(..) => return Err("Cannot coerce array to number".to_string()),
            Value::Wrapped(..) | Value::Blueprint(_) | Value::Thing(_) => return Err("Cannot coerce object to number".to_string()),
            Value::Shared(cell) => return cell.borrow().as_big(),
            Value::TextCall { .. } | Value::Adorned(_) | Value::Generator(_) | Value::Method(..) | Value::Routine(_) | Value::Bound(..) => return Err("Cannot coerce function to number".to_string()),
            Value::Mutable(place, _) => return place.borrow().as_big(),
            Value::Member(..) => return Err("Cannot coerce method to number".to_string()),
            Value::Octets { .. } | Value::OctetKind { .. } | Value::Backtrace(_) | Value::Keyed(..) | Value::Attributes(_) | Value::Traversal(..) | Value::Refusal(_) | Value::Cursor(_) | Value::SetCursor { .. } | Value::Intrinsic(_) | Value::Iterator(_) | Value::Channel(_) | Value::Progression(_) => return Err("Cannot coerce this value to number".into()),
            Value::Ellipsis => return Err("Ellipsis is not a number".to_string()),
            Value::Span(_) => return Err("Cannot coerce slice to number".to_string()),
            Value::KindOf(_) => return Err("Cannot coerce kind meta-value to number".to_string()),
        })
    }

    pub fn equals(&self, other: &Value) -> bool {
        // Most askings are of two small numbers, two great ones, or two
        // pieces of text, and all three can be settled here and now.
        // Left to the ratios below, a pair of small numbers would have
        // two great numbers made of it and then be cross-multiplied.
        if let (Value::Small(here), Value::Small(there)) = (self, other) { return here == there; }
        if let (Value::Huge(here), Value::Huge(there)) = (self, other) { return here.as_ref() == there.as_ref(); }
        if let (Value::Text(here), Value::Text(there)) = (self, other) { return here.as_ref() == there.as_ref(); }
        if let Value::Mutable(cell, _) = self { return cell.borrow().equals(&other.settled()); }
        if let Value::Mutable(cell, _) = other { return self.equals(&cell.borrow()); }
        match (self, other) {
            (Value::TextRow(a, fixed), Value::TextRow(b, closed)) => return fixed == closed && a == b,
            (Value::Span(a), Value::Span(b)) => return a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::TextRow(words, false), Value::Vector(values)) | (Value::Vector(values), Value::TextRow(words, false)) => {
                return words.len() == values.len() && words.iter().zip(values.iter()).all(|(word, value)| match value { Value::Text(s) => word.as_str() == s.as_ref(), _ => false });
            }
            _ => (),
        }
        if let (Some(a), Some(b)) = (crate::math::ratio_of(self), crate::math::ratio_of(other)) {
            // Nought beneath is no ratio to cross-multiply: what lies
            // past every number is equal to another only where both lie
            // past on the same hand, and the worth nothing is equal to
            // is equal to nothing, itself least of all.
            if a.past_numbers() || b.past_numbers() {
                return a.past_numbers()
                    && b.past_numbers()
                    && !a.answers_none()
                    && !b.answers_none()
                    && a.above.is_negative() == b.above.is_negative();
            }
            return a.above * b.beneath == b.above * a.beneath;
        }
        match (self, other) {
            (Value::Complex(left), Value::Complex(right)) => left.0 == right.0 && left.1 == right.1,
            (Value::Complex(pair), rhs) | (rhs, Value::Complex(pair)) => {
                let scalar = match rhs { Value::Flag(true) => Value::Small(1), Value::Flag(false) => Value::Small(0), _ => rhs.clone() };
                pair.1 == 0.0 && crate::complex::decimal_value(pair.0).equals(&scalar)
            },
            (Value::Imaginary { coefficient: x, .. }, Value::Imaginary { coefficient: y, .. }) => x == y,
            (Value::Imaginary { coefficient, .. }, other) | (other, Value::Imaginary { coefficient, .. }) => {
                *coefficient == 0.0 && (matches!(other, Value::Flag(false)) || other.equals(&Value::Small(0)))
            }
            (Value::Intrinsic(left), Value::Intrinsic(right)) => left == right,
            (Value::Iterator(left), Value::Iterator(right)) => Rc::ptr_eq(left,right),
            (Value::Set(left), Value::Set(right)) => left.borrow().keys == right.borrow().keys,
            (Value::Arguments(one), Value::Arguments(two)) => one.len() == two.len() && one.iter().zip(two.iter()).all(|(a, b)| a.equals(b)),
            (Value::Octets { cell: x, .. }, Value::Octets { cell: y, .. }) => x.borrow().as_slice() == y.borrow().as_slice(),
            (Value::OctetKind { changeable: x, .. }, Value::OctetKind { changeable: y, .. }) => x == y,
            (Value::Channel(left), Value::Channel(right)) => left == right,
            // A method read off a value twice is one method, so long as
            // the word is the same word and the value the same value —
            // not another one merely equal to it.
            (Value::Member(one, first), Value::Member(two, second)) => first == second && one.one_and_same(two),
            (Value::Progression(left), Value::Progression(right)) => {
                if left.count() != right.count() { return false; }
                match left.count().to_u8() {
                    Some(0) => true,
                    Some(1) => left.first == right.first,
                    _ => left.first == right.first && left.stride == right.stride,
                }
            }
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Flag(a), Value::Flag(b)) => a == b,
            (Value::Nil, Value::Nil) | (Value::Ellipsis, Value::Ellipsis) => true,
            (Value::Tuple(a), Value::Tuple(b)) | (Value::Vector(a), Value::Vector(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Dict(a), Value::Dict(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.equals(k) && x.equals(y))
            }
            (Value::Couple(a), Value::Couple(b)) => a.0.equals(&b.0) && a.1.equals(&b.1),
            // One object is itself and nothing else; two classes are one
            // when they carry the same name.
            (Value::Adorned(x), Value::Adorned(y)) => Rc::ptr_eq(x, y),
            (Value::Method(p, a), Value::Method(q, b)) => Rc::ptr_eq(p, q) && Rc::ptr_eq(a, b),
            (Value::Thing(a), Value::Thing(b)) => Rc::ptr_eq(a, b),
            (Value::Blueprint(a), Value::Blueprint(b)) => if a.presentation.is_none() { a.name == b.name } else { Rc::ptr_eq(a,b) },
            (Value::Generator(x), Value::Generator(y)) => Rc::ptr_eq(x, y),
            (Value::Wrapped(k,x), Value::Wrapped(l,y)) => k == l && Rc::ptr_eq(x,y),
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
    /// Whether two values occupy one place: one cell or allocation, or
    /// one worth for a value held by worth alone; no for kinds without a
    /// place of their own.
    pub fn one_place(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Shared(x), _) | (Value::Mutable(x, _), _) => x.borrow().one_place(other),
            (_, Value::Shared(y)) | (_, Value::Mutable(y, _)) => self.one_place(&y.borrow()),
            (Value::Frac(x), Value::Frac(y)) => Rc::ptr_eq(x, y),
            (Value::Huge(x), Value::Huge(y)) => Rc::ptr_eq(x, y),
            (Value::Vector(x), Value::Vector(y)) | (Value::Tuple(x), Value::Tuple(y)) => Rc::ptr_eq(x, y),
            (Value::Dict(x), Value::Dict(y)) => Rc::ptr_eq(x, y),
            (Value::Thing(x), Value::Thing(y)) => Rc::ptr_eq(x, y),
            (Value::Small(x), Value::Small(y)) => x == y,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }

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
        if let Some(words) = self.raised_words(w) { return words; }
        match self {
            Value::Mutable(cell, true) => within_cell(cell, |inner| inner.repr(&w)),
            Value::Mutable(cell, false) => within_cell(cell, |inner| inner.render(w)),
            Value::Row(_) => self.repr(&w),
            Value::Window(_, portion) => format!("dict_{}({})", match portion { 'k'=>"keys",'v'=>"values",_=>"items" }, self.settled().repr(&w)),
            Value::Arguments(row) => Self::argument_text(row, w),
            // A cell that names share is written as what it holds.
            Value::Shared(cell) => within_cell(cell, |inner| inner.render(w)),
            Value::Set(items) => items.borrow().written(|item| item.set_member_spelling(w)),

            Value::Flag(true) if w.flag_counted => "1".to_string(),
            Value::Flag(false) if w.flag_counted => String::new(),
            Value::Flag(true) => w.truth.to_string(),
            Value::Flag(false) => w.falsity.to_string(),
            Value::Nil | Value::Unset => w.nil.to_string(),
            Value::Tuple(parts) => {
                let inside = parts.iter().map(|part| part.in_field(w, "", "r").unwrap_or_else(|| part.bare())).collect::<Vec<_>>().join(", ");
                format!("({}{})", inside, if parts.len() == 1 { "," } else { "" })
            }
            Value::Vector(items) => format!("[{}]", items.iter().map(|v| v.render(w)).collect::<Vec<_>>().join(", ")),
            Value::Dict(entries) => {
                format!("[{}]", entries.iter().map(|(k, v)| format!("{} => {}", k.render(w), v.render(w))).collect::<Vec<_>>().join(", "))
            }
            Value::Couple(e) => format!("{} => {}", e.0.render(w), e.1.render(w)),
            // A worth past the numbers is written by its name at any
            // width, there being no figures in it to write.
            // A real a working gave is written as any real is where the
            // table asks the shortest spelling, and in the library's own
            // spelling elsewhere.
            Value::Frac(e) if e.float_style && !w.brief_reals => {
                let number = if e.under && e.above.is_zero() { -0.0 } else { nearest_binary(&e.above, &e.beneath) };
                format!("{number:?}").to_lowercase()
            }
            Value::Frac(e) if w.brief_reals && e.places.is_some() => decimal_roundtrip(if e.under && e.above.is_zero() && !e.past_numbers() { -0.0 } else { nearest_binary(&e.above, &e.beneath) }),
            Value::Frac(e) if e.past_numbers() => e.written().to_string(),
            // A nought under nought is written so, at any width.
            Value::Frac(e) if e.under && num_traits::Zero::is_zero(&e.above) => "-0".to_string(),
            // A language whose reals are numbers of bits writes one to
            // its own count of figures.
            Value::Frac(e) if w.real_figures.is_some() => spelled_out(nearest_binary(&e.above, &e.beneath), figures_asked(false).unwrap_or(w.real_figures)),
            Value::Frac(e) if w.bit_reals && e.places.is_some() => ordinary_real(nearest_binary(&e.above, &e.beneath), e.places.unwrap()),
            other => other.bare(),
        }
    }

    /// Common field presentations, with the ordinary spelling kept for
    /// those whose further rules the machine does not yet know.
    pub fn in_field(&self, names: Names, pattern: &str, manner: &str) -> Option<String> {
        match self {
            Value::Text(_) | Value::Small(_) | Value::Huge(_) | Value::Frac(_) => {},
            Value::Flag(_) | Value::Nil if pattern.is_empty() || !manner.is_empty() => {},
            _ => return None,
        }
        let mut result = self.render(names);
        if let Self::Frac(ratio) = self {
            if ratio.places.is_some() && !names.brief_reals {
                let mut worth = nearest_binary(&ratio.above, &ratio.beneath);
                if ratio.under && worth == 0.0 { worth = -0.0; }
                let raw = format!("{:?}", worth).to_lowercase();
                result = match raw.find('e') {
                    None => raw,
                    Some(cut) => {
                        let power: i32 = raw[cut + 1..].parse().ok()?;
                        format!("{}e{:+03}", &raw[..cut], power)
                    },
                };
            }
        }
        if matches!(manner, "a" | "r") {
            if let Value::Text(chars) = self {
                let delimiter = match (chars.contains('\''), chars.contains('"')) { (true, false) => '"', _ => '\'' };
                let mut body = String::new();
                for letter in chars.chars() {
                    if letter == delimiter || letter == '\\' { body.push('\\'); body.push(letter); continue; }
                    let escaped = match letter { '\n' => Some("\\n"), '\t' => Some("\\t"), '\r' => Some("\\r"), _ => None };
                    if let Some(escape) = escaped { body.push_str(escape); continue; }
                    if letter.is_control() || manner == "a" && !letter.is_ascii() {
                        let ordinal = u32::from(letter);
                        body.push_str(&match ordinal {
                            0..=0xff => format!("\\x{:02x}", ordinal),
                            0x100..=0xffff => format!("\\u{:04x}", ordinal),
                            _ => format!("\\U{:08x}", ordinal),
                        });
                    } else { body.push(letter); }
                }
                result = format!("{delimiter}{body}{delimiter}");
            }
        }
        if manner.is_empty() && matches!(self, Value::Small(_) | Value::Huge(_)) {
            let alternative = pattern.starts_with('#');
            let kind = pattern.strip_prefix('#').unwrap_or(pattern);
            let radix = match kind { "b" => 2, "o" => 8, "x" | "X" => 16, _ => 0 };
            if radix > 0 {
                let rendered = self.as_big().ok()?.to_str_radix(radix);
                let negative = rendered.starts_with('-');
                let magnitude = rendered.trim_start_matches('-');
                let digits = if kind == "X" { magnitude.to_ascii_uppercase() } else { magnitude.to_string() };
                let sign = if negative { "-" } else { "" };
                let header = if alternative { format!("0{kind}") } else { String::new() };
                return Some(format!("{sign}{header}{digits}"));
            }
        }
        if matches!(self, Value::Small(_) | Value::Huge(_)) && manner.is_empty() {
            let decimal_width = pattern.strip_suffix('d').filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()));
            if let Some(field) = decimal_width {
                let size = if field.is_empty() { Ok(0) } else { field.parse::<usize>() };
                if let Ok(size) = size {
                    if size <= 100000 {
                        let extra = size.saturating_sub(result.len());
                        if field.starts_with('0') {
                            let minus = result.starts_with('-');
                            let head = if minus { "-" } else { "" };
                            let body = if minus { &result[1..] } else { &result };
                            return Some(format!("{head}{}{body}", "0".repeat(extra)));
                        }
                        return Some(" ".repeat(extra) + &result);
                    }
                }
            }
        }
        if manner.is_empty() && !matches!(self, Value::Text(_)) && pattern.starts_with('.') && pattern.ends_with('f') {
            let precision = pattern[1..pattern.len() - 1].parse::<usize>();
            if let (Ok(digits), Ok(number)) = (precision, result.parse::<f64>()) {
                if digits <= 1000 { return Some(format!("{:.*}", digits, number)); }
            }
        }
        let mut marks = pattern.chars();
        let Some(first) = marks.next() else { return Some(result); };
        let (padding, direction, rest) = if ['<', '^', '>'].contains(&first) {
            (' ', first, marks.as_str())
        } else {
            match marks.next() {
                Some(second @ ('<' | '^' | '>')) => (first, second, marks.as_str()),
                _ if first != '0' && pattern.chars().all(|c| c.is_ascii_digit()) => {
                    (' ', if manner.is_empty() && !matches!(self, Value::Text(_)) { '>' } else { '<' }, pattern)
                },
                _ => return None,
            }
        };
        let target = if rest.is_empty() { 0 } else { rest.parse::<usize>().ok()? };
        if target > 100000 { return None; }
        let extra = target.saturating_sub(result.chars().count());
        let before = if direction == '<' { 0 } else if direction == '^' { extra / 2 } else { extra };
        let mut padded = padding.to_string().repeat(before);
        padded.push_str(&result);
        padded.push_str(&padding.to_string().repeat(extra - before));
        Some(padded)
    }

    pub fn bare(&self) -> String {
        match self {
            Value::Complex(pair) => crate::complex::written(pair),
            Value::Imaginary { coefficient, .. } => brief_decimal(*coefficient) + "j",
            Value::Mutable(place, _) => within_cell(place, Value::bare),
            Value::Member(..) => String::from("<built-in method>"),
            Value::Window(..) => self.settled().bare(),
            Value::Row(v) => format!("({})", v.iter().map(Value::bare).collect::<Vec<_>>().join(", ")),
            Value::Intrinsic(name) => format!("<built-in function {}>", name),
            Value::Iterator(_) => String::from("<iterator>"),
            Value::SetCursor { .. } => String::from("<set walk>"),
            Value::Set(items) => items.borrow().written(Value::bare),
            Value::Arguments(row) => format!("({}{})", row.iter().map(Value::bare).collect::<Vec<_>>().join(", "), if row.len() == 1 { "," } else { "" }),
            Value::Octets { cell, changeable, lead } => octets_shown(&cell.borrow(), lead, *changeable),
            Value::OctetKind { shown, .. } => shown.to_string(),
            Value::TextCall { name, .. } => format!("<built-in method {}>", name),
            Value::TextRow(words, closed) => crate::text::written_row(words, *closed),
            Value::Channel(port) => format!("<{} stream>", if *port == 2 { "error" } else { "output" }),
            Value::Progression(p) => {
                let tail = if p.stride == BigInt::one() { String::new() } else { format!(", {}", p.stride) };
                format!("{}({}, {}{})", p.word, p.first, p.limit, tail)
            }
            Value::Ellipsis => String::from("Ellipsis"),
            Value::Small(n) => n.to_string(),
            Value::Huge(n) => n.to_string(),
            Value::Frac(e) if e.past_numbers() => e.written().to_string(),
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
            Value::Generator(_) => "<generator>".into(),
            Value::Adorned(_) => String::from("<descriptor>"),
            Value::Backtrace(words) => words.to_string(),
            Value::Keyed(value, _) => value.bare(),
            Value::Attributes(t) => format!("<attributes of {}>", t.of.name),
            Value::Refusal(word) => word.to_string(),
            Value::Traversal(..) | Value::Cursor(_) => "<iterator>".to_owned(),
            Value::Method(p, _) | Value::Routine(p) | Value::Bound(p, _) => format!("<function({})>", p.formals.join(", ")),
            Value::Shared(cell) => cell.borrow().bare(),
            Value::Blueprint(b) => b.presentation.clone().unwrap_or_else(|| format!("<class {}>", b.name)),
            Value::Wrapped(..) => "<member wrapper>".into(),
            Value::Tuple(items) => {
                let body = items.iter().map(|item| if let Value::Text(t) = item { format!("{t:?}") } else { item.bare() }).collect::<Vec<_>>().join(", ");
                format!("({body}{})", if items.len() == 1 { "," } else { "" })
            },
            Value::Thing(t) => format!("<object {}>", t.of.name),
            Value::Span(bounds) => format!("slice({})", bounds.iter().map(|bound| bound.quoted(false)).collect::<Vec<_>>().join(", ")),
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
            Value::Adorned(a) => out.push_str(&format!("d{:p}", Rc::as_ptr(a))),
            Value::Method(p, t) => out.push_str(&format!("m{:p}/{:p}", Rc::as_ptr(p), Rc::as_ptr(t))),
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
/// How far a member of a class is reached from: from anywhere, from the
/// class and those built on it, or from the class alone.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reach {
    Everywhere,
    Within,
    Alone,
}

#[derive(Debug)]
pub struct Blueprint {
    pub ancestry: Vec<Rc<Blueprint>>,
    pub parents: Vec<Rc<Blueprint>>,
    pub presentation: Option<String>,
    pub name: String,
    pub under: Option<Rc<Blueprint>>,
    /// The classes of method names only that this one answers to.
    pub answers: Vec<Rc<Blueprint>>,
    pub fields: Vec<(String, Value)>,
    /// How far each of those is reached from, one for one.
    pub reaches: Vec<Reach>,
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

    /// How far a member of that name is reached from, and the class
    /// saying so: the nearest one declaring it, this class first.
    pub fn reach_of(&self, name: &str) -> Option<(Reach, &str)> {
        match self.fields.iter().position(|(n, _)| n == name) {
            Some(at) => Some((self.reaches.get(at).copied().unwrap_or(Reach::Everywhere), self.name.as_str())),
            None => self.under.as_ref().and_then(|u| u.reach_of(name)),
        }
    }

    /// Every property a thing of this class starts with, what it is
    /// built on first, so this class has the last word. A property a
    /// class holds alone is filed under its own name and the class's
    /// together, so a class built on it may declare one of the same name
    /// without the two becoming one.
    pub fn every_field(&self) -> Vec<(String, Value)> {
        let mut all = self.under.as_ref().map_or_else(Vec::new, |u| u.every_field());
        for (at, (name, value)) in self.fields.iter().enumerate() {
            let alone = self.reaches.get(at) == Some(&Reach::Alone);
            let filed = match alone {
                true => held_alone(name, &self.name),
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

/// A property a class holds alone, filed under its own name and the
/// class's together, so that two classes along one line may each hold a
/// property of that name and neither be the other's.
pub fn held_alone(name: &str, owner: &str) -> String {
    format!("{}\0{}", name, owner)
}

/// The name a property is filed under, taken apart: what it is called,
/// and the class holding it alone where one does.
pub fn holder_of(filed: &str) -> (&str, Option<&str>) {
    match filed.split_once('\0') {
        Some((name, owner)) => (name, Some(owner)),
        None => (filed, None),
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
    // Nought beneath is no ratio but the mark of a worth standing past
    // the numbers, and the width keeps one of each of them.
    if beneath.is_zero() {
        return match above.is_zero() {
            true => f64::NAN,
            false => past(),
        };
    }
    if beneath.is_one() {
        return above.to_f64().unwrap_or_else(past);
    }
    let minus = above.is_negative() != beneath.is_negative();
    let (top, low) = (above.abs(), beneath.abs());
    let (high_bits, low_bits) = (top.bits() as i64, low.bits() as i64);
    let worth = if low.trailing_zeros() == Some(low_bits as u64 - 1) && high_bits <= 53 {
        // Every real of the width is so many halves, so a bottom that
        // is a power of two asks for halvings and nothing besides.
        halved(top.to_f64().unwrap_or(0.0), low_bits - 1)
    } else if high_bits <= 53 && low_bits <= 53 {
        // Both sides held to the last bit by a real of the width: the
        // machine's own division lands on the nearest real to the ratio.
        top.to_f64().unwrap_or(0.0) / low.to_f64().unwrap_or(1.0)
    } else {
        // Bringing each side to the width and dividing after rounds
        // twice, which need not land where rounding once lands, so the
        // division is done on the whole numbers. The top is lifted by
        // as many twos as it takes for what comes of it to keep more
        // bits than the width holds, and they come off again after.
        let lift = low_bits + 128 - high_bits;
        let up = match lift >= 0 {
            true => top << lift as usize,
            false => top >> (-lift) as usize,
        };
        let (mut got, rest) = up.div_rem(&low);
        // The lowest bit set where something was left over keeps the
        // rounding off a halfway that is not truly one.
        if !rest.is_zero() {
            got.set_bit(0, true);
        }
        halved(got.to_f64().unwrap_or(f64::INFINITY), lift)
    };
    match minus {
        true => -worth,
        false => worth,
    }
}

/// A real of the width halved so many times, a few hundred halvings at
/// a go, so that none but the last of them can fall past what the width
/// holds and the answer is rounded once and no more. Halving a negative
/// count of times doubles instead.
fn halved(x: f64, times: i64) -> f64 {
    let mut worth = x;
    let mut still = times;
    while still != 0 && worth != 0.0 && worth.is_finite() {
        let go = still.clamp(-400, 400);
        worth /= (2.0f64).powi(go as i32);
        still -= go;
    }
    worth
}

/// The figures of a number written in groups, run together again. A
/// grouping mark has two figures either side of it; found anywhere else,
/// the text is no number and nothing comes back.
pub fn ungrouped_figures(chars: &str, marks: &[char]) -> Option<String> {
    let mut out = String::with_capacity(chars.len());
    let mut walk = chars.chars().peekable();
    let mut last: Option<char> = None;
    while let Some(c) = walk.next() {
        if marks.contains(&c) {
            let next_is_figure = walk.peek().map_or(false, char::is_ascii_digit);
            if !(last.map_or(false, |l| l.is_ascii_digit()) && next_is_figure) { return None; }
        } else {
            out.push(c);
        }
        last = Some(c);
    }
    Some(out)
}

/// A binary real of the width as a worth: kept as a ratio where it is a
/// number of the width, and as what stands past the numbers where it is
/// not. Every real-valued reckoning comes back this way.
pub fn worth_of_binary(x: f64, figures: usize) -> Value {
    match binary_worth(x) {
        // A nought that came out under nought holds on to its minus.
        Some((above, beneath)) => crate::math::made_number(above, beneath, Some(figures), x.is_sign_negative()),
        None => past_the_numbers(x, figures),
    }
}

/// The worth standing for what nothing is equal to, or for what lies
/// past every number on whichever hand the sign says.
pub fn past_the_numbers(x: f64, figures: usize) -> Value {
    let above = match (x.is_nan(), x.is_sign_negative()) {
        (true, _) => BigInt::zero(),
        (_, true) => -BigInt::one(),
        _ => BigInt::one(),
    };
    Value::Frac(Rc::new(Ratio { float_style: false, above, beneath: BigInt::zero(), places: Some(figures), under: false, pointed: false }))
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
pub fn at_binary_width(v: Value, bits: Option<usize>, figures: usize, brief: bool) -> Value {
    if brief {
        return match &v {
            Value::Frac(r) if r.places.is_some() => {
                let mut worth = rounded_binary(&r.above, &r.beneath);
                if r.under && worth == 0.0 { worth = -0.0; }
                worth_of_binary(worth, figures).keeping_point(r.pointed)
            }
            _ => v,
        };
    }
    if bits.is_none() {
        return v;
    }
    let Value::Frac(e) = &v else { return v };
    let rounded = nearest_binary(&e.above, &e.beneath);
    match binary_worth(rounded) {
        // A nought under nought holds its minus at any width.
        Some((above, beneath)) => crate::math::made_number(above, beneath, Some(figures), e.under || rounded.is_sign_negative()).keeping_point(e.pointed),
        None => worth_of_binary(rounded, figures),
    }
}

thread_local! {
    /// The cells being written out at this moment, the outermost first.
    /// A collection that reaches itself is met here on the way round,
    /// and an ellipsis is written for it in its own stead.
    static UNDERWAY: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    /// Where a language names them, the cells a run keeps its counts of
    /// figures in: how many a real written plainly carries, and how many
    /// one shown with its kind carries. A language that names neither
    /// leaves both standing empty.
    static COUNTS: RefCell<(Option<Rc<RefCell<Value>>>, Option<Rc<RefCell<Value>>>)> = const { RefCell::new((None, None)) };
}

/// Write out a cell's contents by the writer given, or an ellipsis where
/// the same cell is already being written further out.
fn within_cell(cell: &Rc<RefCell<Value>>, writer: impl FnOnce(&Value) -> String) -> String {
    let address = Rc::as_ptr(cell) as usize;
    let met_before = UNDERWAY.with(|stack| {
        let mut stack = stack.borrow_mut();
        let seen = stack.contains(&address);
        if !seen { stack.push(address); }
        seen
    });
    if met_before { return String::from("[...]"); }
    let text = writer(&cell.borrow());
    UNDERWAY.with(|stack| { stack.borrow_mut().pop(); });
    text
}

/// Give the kernel the cells the run keeps its counts of figures in.
/// The run reaches them by the names the definition gives, so whatever
/// it writes there governs every real written out after.
pub fn counts_kept_in(plainly: Option<Rc<RefCell<Value>>>, by_kind: Option<Rc<RefCell<Value>>>) {
    COUNTS.with(|both| *both.borrow_mut() = (plainly, by_kind));
}

/// Where the run's count of figures stands at this moment. Empty where
/// the run keeps none; inside that, a count, or empty once more where
/// the count is under nought, by which the run asks for the fewest
/// figures that read back as the number itself.
fn figures_asked(by_kind: bool) -> Option<Option<usize>> {
    COUNTS.with(|both| {
        let both = both.borrow();
        let cell = if by_kind { both.1.as_ref()? } else { both.0.as_ref()? };
        let worth = cell.borrow();
        let asked = match &*worth {
            Value::Small(n) => *n,
            Value::Huge(n) => n.to_i64().unwrap_or(0),
            Value::Text(s) => s.trim().parse().unwrap_or(0),
            _ => 0,
        };
        // The widest real of the width spells out fewer figures than
        // this, so any count beyond it asks for noughts alone, and
        // those come off again further down.
        if asked < 0 {
            return Some(None);
        }
        Some(Some(asked.min(1100) as usize))
    })
}

/// A binary real written out: the fewest figures that read back as the
/// same number, with a power of ten after them where it stands very
/// high or very low. `figures` caps them, as a language's own setting
/// does where a number is written out rather than shown with its kind.
pub fn spelled_out(x: f64, figures: Option<usize>) -> String {
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
    // Plainly while the power is small, and with the power spelled out
    // past that, which is where such a language changes over: at as
    // many figures as are being shown, that being the count the
    // language sets where it sets one and, where it does not, all that
    // tells such a number from the ones on either side of it.
    if (-4..figures.unwrap_or(17) as i32).contains(&power) {
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

/// A real of the width written as the run shows one with its kind: to
/// the count asked for, else to the count the run keeps for showing
/// one, else to the fewest figures that read back as the number itself.
pub fn figured(x: f64, figures: Option<usize>) -> String {
    spelled_out(x, figures.or_else(|| figures_asked(true).flatten()))
}

/// The figures before an imaginary mark, with a signed two-place
/// exponent beyond the plain range.
pub(crate) fn brief_decimal(number: f64) -> String {
    if number.is_nan() { return "nan".into(); }
    if number.is_infinite() { return if number.is_sign_negative() { "-inf" } else { "inf" }.into(); }
    let written = format!("{number:e}");
    let split = written.find('e').expect("the exponent's letter");
    let scale = written[split + 1..].parse::<i32>().expect("the exponent's figures");
    match scale {
        -4..=15 => format!("{number}"),
        _ => {
            let signed = format!("{scale:+03}");
            format!("{}e{}", &written[..split], signed)
        }
    }
}

/// Write a real to its usual figures, keeping powers of ten expanded.
/// Rounding the display hides the remaining digits of its binary ratio.
fn ordinary_real(worth: f64, figures: usize) -> String {
    let shown = spelled_out(worth, Some(figures));
    let expanded = match shown.split_once('E') {
        None => shown,
        Some((head, power)) => {
            let negative = head.starts_with('-');
            let run = head.trim_start_matches('-').replace('.', "");
            let run = run.trim_end_matches('0');
            let point = power.parse::<i32>().unwrap_or(0) + 1;
            let mut result = if negative { String::from("-") } else { String::new() };
            if point <= 0 {
                result.push_str("0.");
                result.push_str(&"0".repeat(-point as usize));
                result.push_str(run);
            } else {
                for index in 0..run.len().max(point as usize) {
                    if index == point as usize { result.push('.'); }
                    result.push(run.as_bytes().get(index).copied().unwrap_or(b'0') as char);
                }
            }
            result
        }
    };
    // Match the ordinary writer's budget, including the zero before a
    // fraction; figures past that budget are simply left unwritten.
    match expanded.find('.') {
        None => expanded,
        Some(dot) => {
            let whole = expanded[..dot].trim_start_matches('-').len();
            expanded.chars().take(dot + 1 + figures.saturating_sub(whole)).collect()
        }
    }
}

fn octets_shown(content: &[u8], lead: &str, changing: bool) -> String {
    let mark = match (content.contains(&39), content.contains(&34)) { (true, false) => 34, _ => 39 };
    let mut pieces = Vec::new();
    pieces.push(lead.to_owned());
    pieces.push((mark as char).to_string());
    for number in content.iter().copied() {
        pieces.push(if number == mark || number == 92 {
            format!("\\{}", number as char)
        } else if let Some(letter) = match number { 9 => Some('t'), 10 => Some('n'), 13 => Some('r'), _ => None } {
            format!("\\{}", letter)
        } else if (32..127).contains(&number) {
            (number as char).to_string()
        } else { format!("\\x{number:02x}") });
    }
    pieces.push((mark as char).to_string());
    if changing { pieces.push(")".to_owned()); }
    pieces.concat()
}
/// A decimal keeps only the figures needed to name its binary worth.
pub(crate) fn decimal_roundtrip(worth: f64) -> String {
    match (worth.is_nan(), worth.is_infinite(), worth.is_sign_negative()) {
        (true, _, _) => return String::from("nan"),
        (_, true, true) => return String::from("-inf"),
        (_, true, false) => return String::from("inf"),
        _ => (),
    }
    let chosen = (1..=17).map(|count| format!("{:.*e}", count - 1, worth))
        .find(|candidate| candidate.parse::<f64>().map(f64::to_bits).ok() == Some(worth.to_bits()))
        .expect("seventeen figures name every binary real");
    let cut = chosen.find('e').unwrap();
    let order = chosen[cut + 1..].parse::<i32>().unwrap();
    let mut coefficient = chosen[..cut].to_owned();
    if coefficient.contains('.') {
        while coefficient.ends_with('0') { coefficient.pop(); }
        if coefficient.ends_with('.') { coefficient.pop(); }
    }
    if order < -4 || order >= 16 { return format!("{coefficient}e{order:+03}"); }
    let sign = if coefficient.starts_with('-') { "-" } else { "" };
    let mut figures = coefficient.trim_start_matches('-').replace('.', "");
    let split = order + 1;
    if split <= 0 {
        figures = format!("0.{}{figures}", "0".repeat((-split) as usize));
    } else if (split as usize) < figures.len() {
        figures.insert(split as usize, '.');
    } else {
        figures.push_str(&"0".repeat(split as usize - figures.len()));
        figures.push_str(".0");
    }
    format!("{sign}{figures}")
}

/// Keep fifty-three bits, or the fewer bits left near nought, and let
/// the exact remainder choose the last one. No rounded quotient is used.
fn rounded_binary(above: &BigInt, beneath: &BigInt) -> f64 {
    if above.is_zero() || beneath.is_zero() { return nearest_binary(above, beneath); }
    let signed = (above.is_negative() != beneath.is_negative()) as u64 * (1u64 << 63);
    let positive = above.abs();
    let divisor = beneath.abs();
    let guessed = positive.bits() as i64 - divisor.bits() as i64;
    match guessed {
        ..=-1076 => return f64::from_bits(signed),
        1025.. => return f64::from_bits(signed + (2047u64 << 52)),
        _ => (),
    }
    let reaches = if guessed >= 0 { positive >= (&divisor << guessed as usize) }
        else { (&positive << guessed.unsigned_abs() as usize) >= divisor };
    let mut power = guessed - if reaches { 0 } else { 1 };
    let shift = 52 - power.max(-1022);
    let scaled_top = if shift > 0 { &positive << shift as usize } else { positive };
    let scaled_bottom = if shift < 0 { &divisor << shift.unsigned_abs() as usize } else { divisor };
    let (mut quotient, residue) = scaled_top.div_rem(&scaled_bottom);
    let twice = residue << 1usize;
    if twice > scaled_bottom || twice == scaled_bottom && quotient.is_odd() { quotient += 1; }
    let mut significand = quotient.to_u64().unwrap();
    if significand == 0x20000000000000 { power += 1; significand /= 2; }
    let field = if significand < 0x10000000000000 { 0 } else { power.max(-1022) + 1023 };
    if field >= 2047 { return f64::from_bits(signed + (2047u64 << 52)); }
    f64::from_bits(signed + ((field as u64) << 52) + (significand & 0xfffffffffffff))
}
