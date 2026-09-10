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
    pub fn count(&self) -> BigInt {
        let forward = self.stride > BigInt::zero();
        if (forward && self.first >= self.limit) || (!forward && self.first <= self.limit) {
            return BigInt::zero();
        }
        ((&self.limit - &self.first).abs() - BigInt::one()) / self.stride.abs() + BigInt::one()
    }

    pub fn item(&self, position: &BigInt) -> Option<Value> {
        let count = self.count();
        let offset = if position < &BigInt::zero() { position + &count } else { position.clone() };
        if offset < BigInt::zero() || offset >= count { return None; }
        Some(Value::from_big(&self.first + &self.stride * offset))
    }
}

#[derive(Clone)]
pub enum Value {
    Mutable(Rc<RefCell<Value>>, bool),
    Member(Rc<Value>, String),
    Window(Rc<Value>, char),
    Row(Rc<Vec<Value>>),
    Channel(u8),
    Progression(Rc<Progression>),
    Small(i64),
    Huge(Rc<BigInt>),
    Frac(Rc<Ratio>),
    /// The coefficient of an imaginary literal, with its unready words.
    Imaginary { coefficient: f64, unready: Rc<str> },
    Text(Rc<str>),
    Flag(bool),
    Nil,
    Ellipsis,
    Vector(Rc<Vec<Value>>),
    Tuple(Rc<Vec<Value>>),
    Generator(Rc<RefCell<crate::exec::Suspension>>),
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
    /// The words for a member a class shares only with those built on
    /// it, and for one it keeps to itself, as they are written beside
    /// the name where a thing is shown.
    pub within_word: Option<&'a str>,
    pub alone_word: Option<&'a str>,
    /// Whether text is kept as bytes, in which case a character of it
    /// is one byte and the width of a piece of text is how many
    /// characters it has rather than what the letters would take.
    pub kept_as_bytes: bool,
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
            Value::Vector(_) | Value::Dict(_) | Value::Row(_) => Kind::Vector,
            Value::Mutable(place, _) => return place.borrow().kind(),
            Value::Member(..) => return None,
            Value::Window(..) => Kind::Vector,
            Value::Nil | Value::KindOf(_) => Kind::Nothing,
            Value::Shared(cell) => return cell.borrow().kind(),
            Value::Couple(_) | Value::Blueprint(_) | Value::Thing(_) => return None,
            Value::Generator(_) | Value::Tuple(_) | Value::Imaginary { .. } | Value::Ellipsis | Value::Method(..) | Value::Routine(_) | Value::Bound(..) | Value::Unset | Value::Span(_) | Value::Channel(_) | Value::Progression(_) => return None,
        })
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Imaginary { coefficient, .. } => *coefficient != 0.0,
            Value::Mutable(cell, _) => cell.borrow().is_true(),
            Value::Row(items) => !items.is_empty(),
            Value::Window(..) => match self.settled() {Value::Vector(items)=>!items.is_empty(),_=>false},
            Value::Progression(walk) => walk.count() != BigInt::zero(),
            Value::Flag(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Huge(n) => !n.is_zero(),
            // Neither worth standing past the numbers is nought, so
            // both count as true, though the top of the one is nought.
            Value::Frac(e) => e.past_numbers() || !e.above.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::Tuple(parts) => !parts.is_empty(),
            Value::Nil | Value::Unset => false,
            _ => true,
        }
    }

    pub fn as_big(&self) -> Result<BigInt, String> {
        Ok(match self {
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
            Value::Tuple(_) | Value::Vector(_) | Value::Dict(_) | Value::Couple(_) | Value::Row(_) | Value::Window(..) => return Err("Cannot coerce array to number".to_string()),
            Value::Blueprint(_) | Value::Thing(_) => return Err("Cannot coerce object to number".to_string()),
            Value::Shared(cell) => return cell.borrow().as_big(),
            Value::Generator(_) | Value::Method(..) | Value::Routine(_) | Value::Bound(..) => return Err("Cannot coerce function to number".to_string()),
            Value::Mutable(place, _) => return place.borrow().as_big(),
            Value::Member(..) => return Err("Cannot coerce method to number".to_string()),
            Value::Channel(_) | Value::Progression(_) => return Err("Cannot coerce this value to number".into()),
            Value::Ellipsis => return Err("Ellipsis is not a number".to_string()),
            Value::Span(_) => return Err("Cannot coerce slice to number".to_string()),
            Value::KindOf(_) => return Err("Cannot coerce kind meta-value to number".to_string()),
        })
    }

    pub fn equals(&self, other: &Value) -> bool {
        if let Value::Mutable(cell, _) = self { return cell.borrow().equals(&other.settled()); }
        if let Value::Mutable(cell, _) = other { return self.equals(&cell.borrow()); }
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
            (Value::Imaginary { coefficient: x, .. }, Value::Imaginary { coefficient: y, .. }) => x == y,
            (Value::Imaginary { coefficient, .. }, other) | (other, Value::Imaginary { coefficient, .. }) => {
                *coefficient == 0.0 && (matches!(other, Value::Flag(false)) || other.equals(&Value::Small(0)))
            }
            (Value::Channel(left), Value::Channel(right)) => left == right,
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
            (Value::Vector(a), Value::Vector(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Dict(a), Value::Dict(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.equals(k) && x.equals(y))
            }
            (Value::Couple(a), Value::Couple(b)) => a.0.equals(&b.0) && a.1.equals(&b.1),
            // One object is itself and nothing else; two classes are one
            // when they carry the same name.
            (Value::Method(p, a), Value::Method(q, b)) => Rc::ptr_eq(p, q) && Rc::ptr_eq(a, b),
            (Value::Thing(a), Value::Thing(b)) => Rc::ptr_eq(a, b),
            (Value::Blueprint(a), Value::Blueprint(b)) => a.name == b.name,
            (Value::Generator(x), Value::Generator(y)) => Rc::ptr_eq(x, y),
            (Value::Tuple(x), Value::Tuple(y)) => x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| a.equals(b)),
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
            Value::Mutable(cell, true) => cell.borrow().repr(&w),
            Value::Mutable(cell, false) => cell.borrow().render(w),
            Value::Row(_) => self.repr(&w),
            Value::Window(_, portion) => format!("dict_{}({})", match portion { 'k'=>"keys",'v'=>"values",_=>"items" }, self.settled().repr(&w)),
            // A cell that names share is written as what it holds.
            Value::Shared(cell) => cell.borrow().render(w),
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
            Value::Frac(e) if e.past_numbers() => e.written().to_string(),
            // A nought under nought is written so, at any width.
            Value::Frac(e) if e.under && num_traits::Zero::is_zero(&e.above) => "-0".to_string(),
            // A language whose reals are numbers of bits writes one to
            // its own count of figures.
            Value::Frac(e) if w.real_figures.is_some() => spelled_out(nearest_binary(&e.above, &e.beneath), figures_asked(false).unwrap_or(w.real_figures)),
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
            if ratio.places.is_some() {
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
            Value::Imaginary { coefficient, .. } => brief_decimal(*coefficient) + "j",
            Value::Mutable(place, _) => place.borrow().bare(),
            Value::Member(..) => String::from("<built-in method>"),
            Value::Window(..) => self.settled().bare(),
            Value::Row(v) => format!("({})", v.iter().map(Value::bare).collect::<Vec<_>>().join(", ")),
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
            Value::Tuple(parts) => format!("({}{})", parts.iter().map(Value::bare).collect::<Vec<_>>().join(", "), if parts.len() == 1 { "," } else { "" }),
            Value::Method(p, _) | Value::Routine(p) | Value::Bound(p, _) => format!("<function({})>", p.formals.join(", ")),
            Value::Shared(cell) => cell.borrow().bare(),
            Value::Blueprint(b) => format!("<class {}>", b.name),
            Value::Thing(t) => format!("<object {}>", t.of.name),
            Value::Span(bounds) => format!("slice({})", bounds.iter().map(Value::bare).collect::<Vec<_>>().join(", ")),
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
    Value::Frac(Rc::new(Ratio { above, beneath: BigInt::zero(), places: Some(figures), under: false, pointed: false }))
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
        Some((above, beneath)) => crate::math::made_number(above, beneath, Some(figures), e.under).keeping_point(e.pointed),
        None => v,
    }
}

thread_local! {
    /// Where a language names them, the cells a run keeps its counts of
    /// figures in: how many a real written plainly carries, and how many
    /// one shown with its kind carries. A language that names neither
    /// leaves both standing empty.
    static COUNTS: RefCell<(Option<Rc<RefCell<Value>>>, Option<Rc<RefCell<Value>>>)> = const { RefCell::new((None, None)) };
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
fn brief_decimal(number: f64) -> String {
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
