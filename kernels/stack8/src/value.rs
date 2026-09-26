// Values. Small integers are unboxed; everything larger sits behind a
// reference count, so the stack moves pointers. Arrays copy when written
// through a shared reference, which a taking load avoids.

use std::cell::RefCell;
use std::collections::HashSet;
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
    pub floating: bool,
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
    /// The three bounds as machine words where they all fit in one,
    /// and nothing where any of them does not. A stride of nought is
    /// left to the great-number road, which complains of it as before.
    fn narrow(&self) -> Option<(i128, i128, i128)> {
        let step = i128::from(self.step.to_i64()?);
        if step == 0 { return None; }
        Some((i128::from(self.start.to_i64()?), i128::from(self.stop.to_i64()?), step))
    }

    /// How many places a walk of those bounds holds. Wide words are
    /// roomy enough: the two ends lie within one word each, so their
    /// distance lies within two.
    fn places(start: i128, stop: i128, step: i128) -> i128 {
        let distance = if step > 0 { stop - start } else { start - stop };
        if distance <= 0 { 0 } else { (distance - 1) / step.abs() + 1 }
    }

    pub fn length(&self) -> BigInt {
        if let Some((start, stop, step)) = self.narrow() {
            return BigInt::from(Self::places(start, stop, step));
        }
        let distance = if self.step.is_positive() { &self.stop - &self.start } else { &self.start - &self.stop };
        if distance <= BigInt::zero() { BigInt::zero() }
        else { (distance - 1) / self.step.abs() + 1 }
    }

    pub fn at(&self, mut index: BigInt) -> Option<Value> {
        // A walk within the machine's words is counted in words. This
        // is the road a loop over a counted row takes at every step,
        // and the great numbers cost more than the walk itself.
        if let (Some((start, stop, step)), Some(wanted)) = (self.narrow(), index.to_i64()) {
            let length = Self::places(start, stop, step);
            let mut place = i128::from(wanted);
            if place < 0 { place += length; }
            if place < 0 || place >= length { return None; }
            return Some(Value::Small((start + place * step) as i64));
        }
        let length = self.length();
        if index.is_negative() { index += &length; }
        (index >= BigInt::zero() && index < length).then(|| Value::of_big(&self.start + index * &self.step))
    }
}

/// Where a suspended body stood inside a try: which part of the try was
/// running, so that stepping back in sets the same watch up again.
#[derive(Debug, Clone)]
pub enum Phase {
    Body,
    /// The clause at that place, holding the value it took.
    Arm(usize, Value),
    Else,
    /// The last part, with what the try had already come to and how deep
    /// the stack stood before the part began.
    Last(Ending, usize),
}

/// What a try had come to before its last part ran, kept apart from the
/// engine's own faults so that a suspended body may carry it.
#[derive(Debug, Clone)]
pub enum Ending {
    Along(usize),
    Leaves(usize, Option<usize>),
    Thrown(Value),
    Note(String),
    Stopped(String),
    Finished,
}

/// One try a suspended body stands inside: where the try is written
/// among the words, how deep the stack and the held faults stood when it
/// began, and which of its parts was running.
#[derive(Debug, Clone)]
pub struct Step {
    pub at: usize,
    pub floor: usize,
    pub held: usize,
    pub phase: Phase,
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
    /// The cell of a map the walk hands the items of, with the size the
    /// map had when the walk began, so a step may see it has changed.
    pub watched: Option<(Rc<RefCell<Value>>, (usize, u64))>,
    /// The tries the suspension stands inside, innermost first, and
    /// whether the body is on its way back to where it left off.
    pub resume: Vec<Step>,
    pub resuming: bool,
    /// The faults the body itself is handling, kept while it sleeps so
    /// that what is raised next stands behind them.
    pub held: Vec<Value>,
    /// A value to raise where the body left off, rather than hand in.
    pub hurled: Option<Value>,
    /// The word the reference gives a walk of the very thing this one
    /// was made from, where that walk is not a program's own: a map
    /// walked backwards, say. Nothing for a generator the program wrote.
    pub walked: Option<Rc<str>>,
}

impl Generator {
    pub fn new(program: Option<Rc<Routine>>, frame: Vec<Value>, items: Vec<Value>) -> Self {
        Self { program, frame, items, stack: Vec::new(), pc: 0, started: false,
            closed: false, waiting: false, handed: None, returned: Value::Null,
            delegate: None, sent: Value::Null, current: None, watched: None,
            resume: Vec::new(), resuming: false, held: Vec::new(), hurled: None, walked: None }
    }
}

/// A cursor keeps one pending item for a loop's question about its end.
#[derive(Debug, Clone)]
pub struct CursorState {
    pub source: CursorSource,
    pub pending: Option<Value>,
    pub finished: bool,
    pub busy: bool,
    /// The word the reference gives a walk of the very thing this one
    /// was made from. A walk gathered into a row of members has lost
    /// what it was gathered from, and this keeps that much of it.
    pub walked: Option<Rc<str>>,
}

#[derive(Debug, Clone)]
pub enum CursorSource {
    /// A row walked place by place from a copy made whole beforehand,
    /// so that a step forward through it means moving the place the
    /// walk stands at and nothing more: the row itself is one member
    /// of the cursor's own state, shared out to every step of it
    /// rather than copied out and back again at each one.
    Items(Rc<Vec<Value>>, usize),
    /// A counted row walked place by place, never made whole.
    Counted(Rc<Counted>, BigInt),
    /// A list walked through the cell it lives in, read as it stands at
    /// each step rather than as it stood at the first.
    Living(Rc<RefCell<Value>>, usize),
    /// A window upon a map, with the size the map had when the walk
    /// began; the walk stops should that size change.
    Viewed(Value, usize, (usize, u64)),
    /// A thing walked by reading its places from nought upward.
    Indexed(Value, BigInt),
    /// A thing walked by reading its places from its last down to
    /// nought, for one that answers `__getitem__` and `__len__` but
    /// keeps no `__reversed__` of its own.
    IndexedBack(Value, BigInt),
    /// A callable asked again and again until it answers the sentinel.
    Called(Value, Value),
    /// A thing of the program's own, asked for each member the way a
    /// loop asks it.
    Handed(Value),
    Numbered(Value, BigInt),
    /// Walks taken abreast, with the work applied to each row where
    /// there is any, and whether they must all end together.
    Combined(Vec<Value>, Option<Value>, bool),
    Selected(Value, Value),
}

#[derive(Debug, Clone)]
pub enum Value {
    Collection(Rc<RefCell<Value>>, bool),
    ValueMethod(Rc<(Value, String)>),
    View(Rc<(Value, String)>),
    Native(crate::code::Builtin, Rc<str>),
    Cursor(Rc<RefCell<CursorState>>),
    Trace(Rc<str>),
    Hashed(Rc<(Value, Value)>),
    Fields(Rc<Instance>),
    Walking(Rc<RefCell<(Value, Option<Value>)>>),
    Declined(Rc<str>),
    Walk(Rc<RefCell<(Vec<Value>, usize)>>),
    Bytes(Rc<RefCell<Vec<u8>>>, bool, Rc<str>),
    ByteKind(bool, Rc<str>),
    Adapter(Rc<(u8, Vec<Value>)>),
    Stream(bool),
    Counted(Rc<Counted>),
    Small(i64),
    Huge(Rc<BigInt>),
    Frac(Rc<Frac>),
    Real(Rc<Real>),
    /// An imaginary literal and the words for working with it too soon.
    Imaginary(f64, Rc<str>),
    Complex(Rc<crate::complex::Complex>),
    Text(Rc<str>),
    Words(Rc<Vec<String>>, bool),
    TextMethod(Rc<str>, crate::strings::TextOp, Rc<str>),
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
    Map(Rc<KeyedPairs>),
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

/// A stand-in for the standard library's own scattering, which guards
/// against an adversary choosing keys on purpose. A set's own members
/// are kept under a key already reckoned by `set_key`, never a
/// program's raw text, so that guard buys nothing here and every
/// member read or written pays for it regardless. This is the
/// textbook Fowler-Noll-Vo pass, one byte at a time: no table, no
/// lookahead, just a running product folded against each byte in turn.
pub struct QuickHash(u64);

impl QuickHash {
    const START: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
}

impl Default for QuickHash {
    fn default() -> Self { QuickHash(Self::START) }
}

impl std::hash::Hasher for QuickHash {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut running = self.0;
        for &byte in bytes {
            running ^= byte as u64;
            running = running.wrapping_mul(Self::PRIME);
        }
        self.0 = running;
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

pub type FxBuildHasher = std::hash::BuildHasherDefault<QuickHash>;

/// The hash finds a member; the row remembers when it first came.
#[derive(Debug, Clone)]
pub struct Members {
    pub row: Vec<String>,
    pub held: std::collections::HashMap<String, Value, FxBuildHasher>,
    pub word: String,
    /// Whether these are the members of a set that cannot be changed.
    /// The kind a set is of stands here, beside the members themselves,
    /// so that whoever holds the members knows which kind they are of
    /// without asking the definition for the word again.
    pub fixed: bool,
    /// The fold of a fixed set's members, once it has been reckoned.
    /// Nothing may alter such a set, so the fold never goes stale and
    /// is reckoned but once however often it is asked for. A set of
    /// sets would otherwise fold its members afresh at every level and
    /// cost what the whole nesting beneath it costs.
    pub folded: std::cell::Cell<Option<i64>>,
}

impl Members {
    pub fn empty(word: String, fixed: bool) -> Self {
        Self { row: Vec::new(), held: std::collections::HashMap::default(), word, fixed, folded: std::cell::Cell::new(None) }
    }

    pub fn insert(&mut self, key: String, value: Value) {
        self.folded.set(None);
        if !self.held.contains_key(&key) {
            self.row.push(key.clone());
            self.held.insert(key, value);
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.folded.set(None);
        let found = self.held.remove(key);
        if found.is_some() { self.row.retain(|k| k != key); }
        found
    }

    /// The members as they are read out. A thing kept beside its hash
    /// is handed over as the thing itself: the hash is the set's own
    /// reckoning of where the thing lies and no part of the member.
    pub fn items(&self) -> Vec<Value> {
        self.row.iter().map(|k| match &self.held[k] { Value::Hashed(pair) => pair.0.clone(), held => held.clone() }).collect()
    }

    pub fn beneath(&self, other: &Self) -> bool {
        self.held.keys().all(|k| other.held.contains_key(k))
    }

    pub fn combine(&self, other: &Self, how: u8) -> Self {
        let mut result = Self::empty(self.word.clone(), self.fixed);
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

    /// The members written out. An empty set names its kind and shows
    /// nothing between braces at all; a set that cannot be changed
    /// names its kind before the braces, since nothing written in the
    /// program stands for one and the braces alone would read as the
    /// changeable kind.
    pub fn show(&self, shown: impl Fn(&Value) -> String) -> String {
        if self.row.is_empty() { return format!("{}()", self.word); }
        let apart = self.row.iter().map(|k| shown(&self.held[k])).collect::<Vec<_>>().join(", ");
        if self.fixed { return format!("{}({{{}}})", self.word, apart); }
        format!("{{{apart}}}")
    }

    /// The address the whole of these members takes where a set holds
    /// them: the addresses of the members themselves, put in order and
    /// then folded into one number, so that two sets of the same
    /// members share the one address however either was gathered. The
    /// addresses are folded rather than written one after another
    /// because a set of sets would double the writing at every level:
    /// the numbers built out of sets alone reach a length no machine
    /// could hold, whilst the fold stays one number wide however deep
    /// the nesting goes.
    pub fn address(&self) -> String {
        let mut places: Vec<&str> = self.row.iter().map(String::as_str).collect();
        places.sort_unstable();
        let mut state = std::collections::hash_map::DefaultHasher::new();
        for place in places { std::hash::Hash::hash(place, &mut state); }
        format!("frozen:{:x}", std::hash::Hasher::finish(&state))
    }
}

/// Where a key's own text stands among a map's rows: `AtRow` where the
/// lookup names a row outright; `NotThere` where the lookup accounts
/// for every row (none of them went without text of its own) and none
/// carries this text, so a miss is proof; and `Uncertain` where some
/// row's own key went without text of its own (a class with its own
/// `__hash__`/`__eq__`, left out of the lookup entirely) and so a miss
/// proves nothing — that row might still hold this key by the
/// program's own equality, which only the program can settle.
pub enum Placement {
    AtRow(usize),
    NotThere,
    Uncertain,
}

thread_local! { static MAP_REVISION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

fn next_map_revision() -> u64 {
    MAP_REVISION.with(|stamp| { let next = stamp.get().wrapping_add(1); stamp.set(next); next })
}

/// A map's rows, in the order a program wrote them, paired with a
/// lookup from a key's own text (`member_key`) to the row it sits at.
/// The lookup is worked out the first time something asks for it, from
/// every row already present, and answered without doing that walk
/// again afterwards. Beside the lookup sits a plain count of the rows
/// whose key went without text of its own, kept exact across every
/// rebuild, so a miss in the lookup is trusted as "no such row" only
/// when that count is nought. It belongs only to the rows it was drawn
/// from: `Deref` reaches those rows for reading, unchanged, while
/// `DerefMut` throws the lookup away before handing out a way to
/// change the rows, so nothing that adds, drops, reorders or overwrites
/// a row by hand can leave the lookup pointing at rows that moved out
/// from under it. `set_by_key`, the one road that does not pass through
/// `DerefMut`, keeps the rows and the lookup growing side by side
/// instead, so a map built one key at a time never has its lookup
/// thrown away and walked afresh for the next key.
#[derive(Debug)]
pub struct KeyedPairs {
    rows: Vec<(Value, Value)>,
    pub revision: u64,
    lookup: RefCell<Option<(std::collections::HashMap<String, usize>, usize)>>,
}

impl KeyedPairs {
    /// The lookup, worked out from scratch across every row the first
    /// time one is needed, with the count of rows it could find no
    /// text for beside it.
    fn settle_lookup(&self) {
        let mut lookup = self.lookup.borrow_mut();
        if lookup.is_none() {
            let mut fresh = std::collections::HashMap::with_capacity(self.rows.len());
            let mut untexted = 0;
            for (at, (key, _)) in self.rows.iter().enumerate() {
                match key.member_key() {
                    Ok(text) => { fresh.insert(text, at); }
                    Err(_) => untexted += 1,
                }
            }
            *lookup = Some((fresh, untexted));
        }
    }

    /// Where a row of this key's text stands: named outright, proven
    /// absent because the lookup accounts for every row, or uncertain
    /// because some row's key went without text for the lookup to have
    /// accounted for.
    pub fn locate(&self, keytext: &str) -> Placement {
        self.settle_lookup();
        let lookup = self.lookup.borrow();
        let (by_text, untexted) = lookup.as_ref().expect("just settled");
        match by_text.get(keytext) {
            Some(at) => Placement::AtRow(*at),
            None if *untexted == 0 => Placement::NotThere,
            None => Placement::Uncertain,
        }
    }

    /// Write over the row already at a place the lookup has already
    /// named, without touching the lookup: the place it names does not
    /// move for this.
    pub fn overwrite_row(&mut self, at: usize, value: Value) {
        self.rows[at].1 = value;
    }

    /// Add a key already proven absent and already known by its own
    /// text, growing the rows and the lookup together so a map built
    /// one key at a time never has its lookup thrown away and walked
    /// afresh for the next key.
    pub fn insert_proven_absent(&mut self, key: Value, keytext: String, value: Value) {
        self.settle_lookup();
        let at = self.rows.len();
        self.revision = next_map_revision();
        self.rows.push((key, value));
        self.lookup.borrow_mut().as_mut().expect("just settled").0.insert(keytext, at);
    }
}

impl From<Vec<(Value, Value)>> for KeyedPairs {
    fn from(rows: Vec<(Value, Value)>) -> KeyedPairs {
        KeyedPairs { rows, revision: next_map_revision(), lookup: RefCell::new(None) }
    }
}

/// A copy carries only the rows onward; its lookup is left for
/// whatever next asks for it to work out again, over the copy's own
/// rows and never the rows it was copied from.
impl Clone for KeyedPairs {
    fn clone(&self) -> KeyedPairs {
        KeyedPairs { rows: self.rows.clone(), revision: self.revision, lookup: RefCell::new(None) }
    }
}

impl std::iter::FromIterator<(Value, Value)> for KeyedPairs {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> KeyedPairs {
        KeyedPairs::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl std::ops::Deref for KeyedPairs {
    type Target = Vec<(Value, Value)>;
    fn deref(&self) -> &Vec<(Value, Value)> { &self.rows }
}

impl std::ops::DerefMut for KeyedPairs {
    fn deref_mut(&mut self) -> &mut Vec<(Value, Value)> {
        self.revision = next_map_revision();
        *self.lookup.borrow_mut() = None;
        &mut self.rows
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
    pub binary_reals: bool,
    pub shortest_reals: bool,
    /// The words for a member the class shares only with those standing
    /// on it, and for one it keeps to itself, as they are marked beside
    /// the name where a thing is shown.
    /// Whether text is held as the bytes it was written in, so that the
    /// width of a piece of text is the count of its characters and not
    /// the count of bytes the letters they spell would take.
    pub text_is_bytes: bool,
    pub guarded_word: Option<&'a str>,
    pub hidden_word: Option<&'a str>,
    /// Whether a map's keys stand for their worth, a flag being the
    /// number it counts as and a whole number one key with its real.
    pub value_keys: bool,
}

/// The word CPython gives a view of a map's keys, values or pairs, or
/// the read-only reading of the map itself a view keeps beside it.
pub fn view_kind(tag: &str) -> &'static str {
    match tag { "keys" => "dict_keys", "values" => "dict_values", "mapping" => "mappingproxy", _ => "dict_items" }
}

/// The word CPython gives a walk taken backwards over a map's keys,
/// values or pairs.
pub fn reversed_view_kind(tag: &str) -> &'static str {
    match tag { "keys" => "dict_reversekeyiterator", "values" => "dict_reversevalueiterator", _ => "dict_reverseitemiterator" }
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
                // A key kept beside its hash is handed out as the thing
                // itself: the hash is the map's own reckoning of where
                // the thing lies and no part of the key a viewer of the
                // keys, or of the pairs, should ever see.
                Value::array(pairs.iter().map(|(k,v)| {
                    let bare = match k { Value::Hashed(pair) => pair.0.clone(), other => other.clone() };
                    match view.1.as_str() {
                        // A reading of the map itself walks, and is
                        // measured, the very way its keys are: the map
                        // read only is asked after by key alone.
                        "keys" | "mapping" => bare, "values" => v.clone(), _ => Value::Tuple(Rc::new(vec![bare,v.clone()])),
                    }
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
            // A collection is written from its cell however the cell is
            // held, so that a collection standing inside another is
            // written out and not merely named.
            Value::Collection(cell, _) | Value::Bond(cell) | Value::Binding(cell) => {
                // A map come round to again is marked by the braces it
                // would have been written in, a row by its brackets.
                let round = if matches!(&*cell.borrow(), Value::Map(_)) { "{...}" } else { "[...]" };
                shown_once(cell, round, |held| held.representation(words))
            }
            Value::Text(_) => self.string_field(words, "", "r").unwrap_or_else(|| self.plain()),
            Value::Array(row) => members_written(self, || format!("[{}]", row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", "))),
            Value::Tuple(row) => members_written(self, || format!("({}{})", row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", "), if row.len() == 1 { "," } else { "" })),
            Value::Map(row) => members_written(self, || format!("{{{}}}", row.iter().map(|(k,v)| format!("{}: {}", k.representation(words), v.representation(words))).collect::<Vec<_>>().join(", "))),
            _ => self.display(words),
        }
    }

    fn tuple_text(items: &[Value], sp: &Wording) -> String {
        format!("({}{})", items.iter().map(|v| v.repr(sp)).collect::<Vec<_>>().join(", "), if items.len() == 1 { "," } else { "" })
    }

    pub fn repr(&self, sp: &Wording) -> String {
        match self {
            Value::Text(s) => {
                let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
                let mut out = String::from(quote);
                for c in s.chars() {
                    match c {
                        '\\' => out.push_str("\\\\"), '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"),
                        c if c == quote => { out.push('\\'); out.push(c); }
                        c if c.is_control() => { let _ = write!(out, "\\x{:02x}", c as u32); }
                        c => out.push(c),
                    }
                }
                out.push(quote);
                out
            }
            Value::Object(o) => {
                if let Some(args) = self.raised_arguments() {
                    return format!("{}({})", o.class.name, args.iter().map(|v| v.repr(sp)).collect::<Vec<_>>().join(", "));
                }
                self.display(sp)
            }
            Value::Tuple(items) => members_written(self, || Self::tuple_text(items, sp)),
            Value::Array(items) => members_written(self, || format!("[{}]", items.iter().map(|v| v.repr(sp)).collect::<Vec<_>>().join(", "))),
            _ => self.display(sp),
        }
    }

    fn raised_arguments(&self) -> Option<Vec<Value>> {
        let Value::Object(object) = self else { return None };
        if !object.class.all_fields().iter().any(|(n, _)| n == "\0exception") { return None; }
        let fields = object.fields.borrow();
        if let Some((_, Value::Tuple(args))) = fields.iter().find(|(n, _)| n == "\0arguments") { return Some(args.as_ref().clone()); }
        Some(fields.iter().filter(|(n, _)| n == "message").map(|(_, v)| v.clone()).collect())
    }

    /// The one character or byte a Unicode fault's own account picks
    /// out shows as CPython escapes it: two hex digits under 0x100,
    /// four under 0x10000, eight beyond.
    fn unicode_escaped(codepoint: u32) -> String {
        if codepoint <= 0xff { format!("\\x{:02x}", codepoint) }
        else if codepoint <= 0xffff { format!("\\u{:04x}", codepoint) }
        else { format!("\\U{:08x}", codepoint) }
    }

    /// A Unicode codec fault's own account of itself, read from the
    /// members it carries rather than from the tuple it was made with,
    /// so that changing one afterward changes what it is shown by.
    /// Nothing here is a language's own word: these three classes and
    /// their wording belong to Python alone, and are reached only
    /// through the markers Python's own roster puts on its classes.
    fn unicode_error_text(o: &Rc<Instance>, sp: &Wording) -> Option<String> {
        let kind = o.class.all_fields().iter().find_map(|(n, _)| match n.as_str() {
            "\0unicode-encode" => Some(0u8), "\0unicode-decode" => Some(1u8),
            "\0unicode-translate" => Some(2u8), _ => None,
        })?;
        let fields = o.fields.borrow();
        let get = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v.clone());
        let as_i64 = |v: Option<Value>| match v { Some(Value::Small(n)) => Some(n), _ => None };
        let start = as_i64(get("start"))?;
        let end = as_i64(get("end"))?;
        let reason = get("reason").unwrap_or(Value::Null).display(sp);
        let object = get("object");
        // Whether the range names exactly one place worth naming by
        // its own character or byte, which needs the object to still
        // be there and the place to still be within it.
        let single = end == start + 1 && start >= 0;
        let one = if !single { None } else { match &object {
            Some(Value::Text(s)) if kind != 1 => usize::try_from(start).ok().and_then(|i| s.chars().nth(i)).map(|c| Self::unicode_escaped(c as u32)),
            Some(Value::Bytes(cell, ..)) if kind == 1 => usize::try_from(start).ok().and_then(|i| cell.borrow().get(i).copied()).map(|b| format!("{:02x}", b)),
            _ => None,
        }};
        Some(match (kind, &one) {
            (0, Some(escaped)) => format!("'{}' codec can't encode character '{escaped}' in position {start}: {reason}", get("encoding").unwrap_or(Value::Null).display(sp)),
            (0, None) => format!("'{}' codec can't encode characters in position {start}-{}: {reason}", get("encoding").unwrap_or(Value::Null).display(sp), end - 1),
            (1, Some(hexed)) => format!("'{}' codec can't decode byte 0x{hexed} in position {start}: {reason}", get("encoding").unwrap_or(Value::Null).display(sp)),
            (1, None) => format!("'{}' codec can't decode bytes in position {start}-{}: {reason}", get("encoding").unwrap_or(Value::Null).display(sp), end - 1),
            (_, Some(escaped)) => format!("can't translate character '{escaped}' in position {start}: {reason}"),
            (_, None) => format!("can't translate characters in position {start}-{}: {reason}", end - 1),
        })
    }

    pub fn exception_message(&self, sp: &Wording) -> Option<String> {
        let args = self.raised_arguments()?;
        let Value::Object(o) = self else { return None };
        if let Some(told) = Self::unicode_error_text(o, sp) { return Some(told); }
        // A group and an operating-system fault carry the words they
        // are shown with, made when they were.
        if let Some((_, Value::Text(shown))) = o.fields.borrow().iter().find(|(n, _)| n == "\0shown") { return Some(shown.to_string()); }
        Some(match args.as_slice() {
            [] => String::new(),
            [one] if o.class.all_fields().iter().any(|(n, _)| n == "\0quoted") => one.repr(sp),
            [one] => one.display(sp),
            many => Self::tuple_text(many, sp),
        })
    }
    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn member_text(&self, words: &Wording) -> String {
        let mut text = self.string_field(words, "", "r").unwrap_or_else(|| self.display(words));
        if matches!(self, Value::Real(r) if !r.outside()) && !text.contains(['.', 'e', 'E']) { text.push_str(".0"); }
        text
    }

    /// Whether a value is a set that cannot be changed, read through
    /// whatever cells stand between a name and the set itself. A set
    /// held whilst its members are being read answers as a changeable
    /// one, since nothing may ask it anything at such a moment.
    pub fn set_fixed(&self) -> bool {
        match self {
            Value::Set(members) | Value::SetWalk(members, _) => members.try_borrow().map_or(false, |held| held.fixed),
            Value::Collection(cell, _) | Value::Bond(cell) | Value::Binding(cell) => cell.borrow().set_fixed(),
            _ => false,
        }
    }

    pub fn member_key(&self) -> Result<String, &'static str> {
        if let Value::Tuple(items) = self {
            let keys = items.iter().map(Value::member_key).collect::<Result<Vec<_>, _>>()?;
            return Ok(format!("tuple:{keys:?}"));
        }
        // A slice is addressed by its three bounds, precisely as a
        // hashable tuple of them would be; where a bound has no address
        // of its own, the slice is named as the one thing unhashable,
        // not the bound within it, as `hash` itself already names it.
        if let Value::Slice(bounds) = self {
            let keys = bounds.iter().map(Value::member_key).collect::<Result<Vec<_>, _>>().map_err(|_| "slice")?;
            return Ok(format!("slice:{keys:?}"));
        }
        if let Value::Collection(cell, _) | Value::Bond(cell) | Value::Binding(cell) = self { return cell.borrow().member_key(); }
        // A real outside the numbers proper. Either endless number is
        // the one value wherever it is met, since it equals itself; a
        // real that is no number equals nothing at all, not even
        // itself, so it takes the place it lies in for its key and
        // shares that key with nothing else.
        if let Value::Real(number) = self {
            if number.outside() {
                if number.p.is_zero() { return Ok(format!("apart{:p}", Rc::as_ptr(number))); }
                return Ok(format!("beyond{}", if number.p.is_negative() { "-" } else { "+" }));
            }
        }
        // A span of numbers is keyed by the places it names: how many
        // there are, where they begin and how far apart they stand, so
        // that two spans naming the same places are one key. A span of
        // one place has no stride to it, and an empty one no start.
        if let Value::Counted(span) = self {
            let length = span.length();
            if length.is_zero() { return Ok("span0".into()); }
            if length.is_one() { return Ok(format!("span1/{}", span.start)); }
            return Ok(format!("span{}/{}/{}", length, span.start, span.step));
        }
        if let Some((p, q)) = crate::arith::parts(self) {
            if q.is_zero() { return Err(""); }
            let common = p.gcd(&q);
            return Ok(format!("n{}/{}", p / &common, q / common));
        }
        match self {
            Value::Flag(b) => Ok(format!("n{}/1", u8::from(*b))),
            Value::Text(s) => Ok(format!("s{}", s)),
            Value::Bytes(bytes, false, _) => Ok(format!("bytes:{:?}", bytes.borrow())),
            Value::Bytes(_, true, _) => Err("bytearray"),
            Value::Routine(code) => Ok(format!("function:{:p}", Rc::as_ptr(code))),
            Value::Method(owner, code) => Ok(format!("method:{:p}:{:p}", Rc::as_ptr(owner), Rc::as_ptr(code))),

            Value::Null => Ok("nil".into()),
            Value::Ellipsis => Ok("dots".into()),
            Value::Array(_) => Err("list"),
            Value::Map(_) => Err("dict"),
            // A set that cannot be changed is addressed by what it
            // holds; a changeable one has no address at all.
            Value::Set(members) => match members.try_borrow() {
                Ok(held) if held.fixed => Ok(held.address()),
                _ => Err("set"),
            },
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
            Value::Words(..) | Value::Array(_) | Value::Map(_) | Value::Tuple(_) => Sort::Array,
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
            Value::Complex(z) => z.real != 0.0 || z.imag != 0.0,
            Value::Imaginary(n, _) => *n != 0.0,
            Value::Collection(cell, _) => cell.borrow().is_true(),
            Value::ValueMethod(_) => true,
            Value::View(_) => if let Value::Array(row) = self.contents() { !row.is_empty() } else { false },
            Value::Native(..) | Value::Cursor(_) => true,
            Value::Set(s) => !s.borrow().held.is_empty(),
            Value::Trace(_) | Value::Hashed(_) | Value::Fields(_) | Value::Walking(_) | Value::Declined(_) | Value::Walk(_) | Value::SetWalk(..) | Value::Stream(_) => true,
            Value::Bytes(row, ..) => !row.borrow().is_empty(),
            Value::ByteKind(..) => true,
            Value::Counted(r) => !r.length().is_zero(),
            Value::Flag(b) => *b,
            Value::Small(n) => *n != 0,
            Value::Huge(n) => !n.is_zero(),
            // Neither what stands outside the numbers is nought, so
            // both count as true, though the top of the one is nought.
            Value::Real(r) => r.outside() || !r.p.is_zero(),
            Value::Text(s) => !s.is_empty(),
            Value::Words(row, _) => !row.is_empty(),
            Value::Tuple(items) => !items.is_empty(),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => false,
            Value::TextMethod(..) | Value::Descriptor(_) | Value::Generator(_) | Value::Frac(_) | Value::Array(_) | Value::Map(_) | Value::Tie(_) | Value::Routine(_) | Value::Method(..) | Value::SortOf(_) => true,
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().is_true(),
            Value::Adapter(_) | Value::Class(_) | Value::Object(_) | Value::Ellipsis | Value::Slice(_) => true,
        }
    }

    /// The integer a non-number stands in for: booleans and null count,
    /// text is parsed, the rest refuse.
    pub fn as_big(&self) -> Result<BigInt, String> {
        match self {
            Value::Complex(z) => Err(z.integer_fault.to_string()),
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
            Value::Words(..) | Value::Set(_) | Value::Tuple(_) | Value::Array(_) | Value::Map(_) | Value::Tie(_) | Value::View(_) => Err("Cannot coerce array to number".to_string()),
            Value::Class(_) | Value::Object(_) => Err("Cannot coerce object to number".to_string()),
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().as_big(),
            Value::TextMethod(..) | Value::Descriptor(_) | Value::Generator(_) | Value::Method(..) | Value::Routine(_) => Err("Cannot coerce function to number".to_string()),
            Value::Collection(cell, _) => cell.borrow().as_big(),
            Value::ValueMethod(_) => Err("Cannot coerce method to number".to_string()),
            Value::Bytes(..) | Value::ByteKind(..) | Value::Trace(_) | Value::Hashed(_) | Value::Fields(_) | Value::Walking(_) | Value::Declined(_) | Value::Walk(_) | Value::SetWalk(..) | Value::Native(..) | Value::Cursor(_) | Value::Stream(_) | Value::Counted(_) => Err("Cannot coerce this value to number".to_string()),
            Value::Ellipsis => Err("Ellipsis is not a number".to_string()),
            Value::Adapter(_) => Err("Cannot coerce this value to number".to_string()),
            Value::Slice(_) => Err("Cannot coerce slice to number".to_string()),
            Value::SortOf(_) => Err("Cannot coerce kind meta-value to number".to_string()),
        }
    }

    /// Equal: numbers by value across kinds, arrays elementwise, programs
    /// by identity, the rest by content.
    /// Whether these are one value and not two alike: a small number
    /// by its worth, and whatever is kept behind a pointer by that
    /// pointer.
    pub fn same_value(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Counted(x), Value::Counted(y)) => Rc::ptr_eq(x, y),
            (Value::Collection(x, _), Value::Collection(y, _)) => Rc::ptr_eq(x, y),
            (Value::Bond(x), Value::Bond(y)) => Rc::ptr_eq(x, y),
            (Value::Binding(x), Value::Binding(y)) => Rc::ptr_eq(x, y),
            (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
            (Value::Tuple(x), Value::Tuple(y)) => Rc::ptr_eq(x, y),
            (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y),
            (Value::Set(x), Value::Set(y)) => Rc::ptr_eq(x, y),
            (Value::Text(x), Value::Text(y)) => Rc::ptr_eq(x, y),
            (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
            (Value::Small(x), Value::Small(y)) => x == y,
            (Value::Flag(x), Value::Flag(y)) => x == y,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

    pub fn equals(&self, other: &Value) -> bool {
        // Two of a kind, and that kind a plain one: answered outright,
        // before the number tower is entered or a cell is looked into.
        // These three are the great bulk of all the asking.
        match (self, other) {
            (Value::Small(here), Value::Small(there)) => return here == there,
            (Value::Huge(here), Value::Huge(there)) => return here == there,
            (Value::Text(here), Value::Text(there)) => return here == there,
            _ => {}
        }
        if let Value::Collection(cell, _) = self { return cell.borrow().equals(&other.contents()); }
        if let Value::Collection(cell, _) = other { return self.equals(&cell.borrow()); }
        if let Some(order) = crate::arith::order_values(self, other) {
            return order == std::cmp::Ordering::Equal;
        }
        match (self, other) {
            (Value::Complex(z), Value::Complex(w)) => z.real == w.real && z.imag == w.imag,
            (Value::Complex(z), other) | (other, Value::Complex(z)) => z.imag == 0.0 && crate::complex::real(z.real).equals(&if let Value::Flag(b) = other { Value::Small(i64::from(*b)) } else { other.clone() }),
            (Value::Imaginary(a, _), Value::Imaginary(b, _)) => a == b,
            (Value::Imaginary(a, _), b) | (b, Value::Imaginary(a, _)) => *a == 0.0 && (matches!(b, Value::Flag(false)) || b.equals(&Value::Small(0))),
            // Two builtin words are one value only where the word is
            // the same word: a definition may spell two kinds with one
            // piece of work beneath them, and they are not each other.
            (Value::Native(a,x), Value::Native(b,y)) => a == b && x == y,
            (Value::Slice(a), Value::Slice(b)) => a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Cursor(a), Value::Cursor(b)) => Rc::ptr_eq(a,b),
            (Value::Set(a), Value::Set(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.held.len() == b.held.len() && a.beneath(&b)
            }
            (Value::Bytes(a, ..), Value::Bytes(b, ..)) => *a.borrow() == *b.borrow(),
            (Value::ByteKind(a, _), Value::ByteKind(b, _)) => a == b,
            (Value::Stream(a), Value::Stream(b)) => a == b,
            (Value::Counted(a), Value::Counted(b)) => {
                let length = a.length();
                length == b.length() && (length.is_zero() || a.start == b.start && (length.is_one() || a.step == b.step))
            }
            (Value::Words(a, x), Value::Words(b, y)) => x == y && a == b,
            (Value::Words(a, false), Value::Array(b)) | (Value::Array(b), Value::Words(a, false)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(s,v)| matches!(v,Value::Text(t) if s.as_str()==t.as_ref())),
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Flag(a), Value::Flag(b)) => a == b,
            (Value::Null, Value::Null) | (Value::Ellipsis, Value::Ellipsis) => true,
            (Value::SortOf(a), Value::SortOf(b)) => a == b,
            (Value::Generator(a), Value::Generator(b)) => Rc::ptr_eq(a, b),
            (Value::Tuple(a), Value::Tuple(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a, b),
            (Value::Descriptor(a), Value::Descriptor(b)) => Rc::ptr_eq(a, b),
            (Value::Method(a, p), Value::Method(b, q)) => Rc::ptr_eq(a, b) && Rc::ptr_eq(p, q),
            // Two readings of a value's own method come to the same
            // method where the word is the word and the value read is
            // the very value, not merely one equal to it.
            (Value::ValueMethod(a), Value::ValueMethod(b)) => Rc::ptr_eq(a, b) || a.1 == b.1 && a.0.same_value(&b.0),
            (Value::Array(a), Value::Array(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Map(a), Value::Map(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.equals(k) && x.equals(y))
            }
            (Value::Tie(a), Value::Tie(b)) => a.0.equals(&b.0) && a.1.equals(&b.1),
            // Two names for one object are the same object; two objects
            // of one class are not.
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => if a.outline.is_some() { Rc::ptr_eq(a, b) } else { a.name == b.name },
            (Value::Adapter(a), Value::Adapter(b)) => Rc::ptr_eq(a,b),
            _ => false,
        }
    }

    /// Whether two values are the very same. Equal is not enough: they
    /// must be of one kind, so a whole number and a real that stand for
    /// the same amount are equal but not the same. An array is the same
    /// as another when it holds the same keys in the same order, each
    /// with a value that is itself the same.
    /// Whether two values are one and the same place: the same cell,
    /// the same allocation, or the same worth where a value is held by
    /// worth alone. Nothing for kinds that have no place of their own.
    pub fn same_place(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Bond(x), _) => x.borrow().same_place(other),
            (_, Value::Bond(y)) => self.same_place(&y.borrow()),
            (Value::Collection(x, _), _) => x.borrow().same_place(other),
            (_, Value::Collection(y, _)) => self.same_place(&y.borrow()),
            (Value::Real(x), Value::Real(y)) => Rc::ptr_eq(x, y),
            (Value::Frac(x), Value::Frac(y)) => Rc::ptr_eq(x, y),
            (Value::Huge(x), Value::Huge(y)) => Rc::ptr_eq(x, y),
            (Value::Array(x), Value::Array(y)) | (Value::Tuple(x), Value::Tuple(y)) => Rc::ptr_eq(x, y),
            (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y),
            (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
            (Value::Small(x), Value::Small(y)) => x == y,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }

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
        if let Some(told) = self.exception_message(sp) { return told; }
        match self {
            Value::Collection(cell, quote) => shown_once(cell, "[...]", |held| if *quote { held.representation(sp) } else { held.display(sp) }),
            Value::View(view) if view.1 == "mapping" => format!("mappingproxy({})", view.0.contents().representation(sp)),
            Value::View(view) => format!("dict_{}({})", view.1, self.contents().representation(sp)),
            // A cell two names share is written as what it holds: the
            // sharing is between the names and not in the value.
            Value::Bond(shared) | Value::Binding(shared) => shown_once(shared, "[...]", |held| held.display(sp)),
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
            Value::Tuple(items) => members_written(self, || {
                let shown = items.iter().map(|v| v.string_field(sp, "", "r").unwrap_or_else(|| v.plain())).collect::<Vec<_>>().join(", ");
                format!("({}{})", shown, if items.len() == 1 { "," } else { "" })
            }),
            Value::Array(items) => members_written(self, || {
                let shown: Vec<String> = items.iter().map(|v| v.display(sp)).collect();
                format!("[{}]", shown.join(", "))
            }),
            Value::Map(pairs) => members_written(self, || {
                let shown: Vec<String> = pairs.iter().map(|(k, v)| format!("{} => {}", k.display(sp), v.display(sp))).collect();
                format!("[{}]", shown.join(", "))
            }),
            Value::Tie(pair) => format!("{} => {}", pair.0.display(sp), pair.1.display(sp)),
            // What stands outside the numbers is written by its name at
            // any width, since there are no figures to write.
            Value::Real(r) if sp.shortest_reals => real_roundtrip(if r.below && r.p.is_zero() && !r.outside() { -0.0 } else { as_binary(&r.p, &r.q) }),
            Value::Real(r) if r.floating => format!("{:?}", if r.below && r.p.is_zero() { -0.0 } else { as_binary(&r.p, &r.q) }).to_lowercase(),
            Value::Real(r) if r.outside() => r.spelled().to_string(),
            // A language whose reals are binary numbers writes one out
            // to its own count of significant figures.
            Value::Real(r) if r.below && r.p.is_zero() => "-0".to_string(),
            Value::Real(r) if sp.real_digits.is_some() => written_out(as_binary(&r.p, &r.q), figures_now(false).unwrap_or(sp.real_digits)),
            Value::Real(r) if sp.binary_reals => expanded_real(as_binary(&r.p, &r.q), r.places),
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
            if !words.shortest_reals {
                let number = if real.below && real.p.is_zero() { -0.0 } else { as_binary(&real.p, &real.q) };
                shown = format!("{number:?}").to_ascii_lowercase();
                if let Some((mantissa, exponent)) = shown.split_once('e') {
                    let power = exponent.parse::<i32>().ok()?;
                    shown = format!("{mantissa}e{power:+03}");
                }
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
            Value::Complex(z) => crate::complex::shown(z),
            Value::Imaginary(n, _) => format!("{}j", shortest_real(*n)),
            Value::Collection(cell, _) => shown_once(cell, "[...]", |held| held.plain()),
            // A method of a builtin's own, handed over bound to what
            // it was read from, is written by its name, the kind of the
            // thing it was read from and where that thing is kept. One
            // read from the kind itself is bound to no thing at all and
            // is named with the kind it belongs to instead.
            Value::ValueMethod(pair) => method_written(&pair.0, &pair.1, Rc::as_ptr(pair) as *const u8 as usize),
            Value::View(view) if view.1 == "mapping" => format!("mappingproxy({})", view.0.contents().plain()),
            Value::View(view) => format!("dict_{}({})", view.1, self.contents().plain()),
            // A builtin word naming a kind stands for the kind itself,
            // and is written as the reference writes a class; every
            // other builtin word is written as work to be done.
            Value::Native(op, word) if op.names_kind() => format!("<class '{}'>", word),
            Value::Native(_, word) => format!("<built-in function {}>", word),
            Value::Cursor(_) => "<iterator>".to_string(),
            Value::SetWalk(..) => "<set walk>".into(),
            Value::Set(s) => s.borrow().show(Value::plain),
            Value::Bytes(row, mutable, opening) => byte_repr(&row.borrow(), *mutable, opening),
            Value::ByteKind(_, text) => text.to_string(),
            Value::TextMethod(text, _, name) => method_written(&Value::Text(text.clone()), name, text.as_ptr() as usize),
            Value::Words(row, fixed) => crate::strings::row(row, *fixed),
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
            Value::Array(items) => members_written(self, || {
                let shown: Vec<String> = items.iter().map(Value::plain).collect();
                format!("[{}]", shown.join(", "))
            }),
            Value::Map(pairs) => members_written(self, || {
                let shown: Vec<String> = pairs.iter().map(|(k, v)| format!("{} => {}", k.plain(), v.plain())).collect();
                format!("[{}]", shown.join(", "))
            }),
            Value::Tie(pair) => format!("{} => {}", pair.0.plain(), pair.1.plain()),
            Value::Generator(_) => "<generator>".to_string(),
            Value::Tuple(items) => members_written(self, || format!("({}{})", items.iter().map(Value::plain).collect::<Vec<_>>().join(", "), if items.len() == 1 { "," } else { "" })),
            Value::Descriptor(_) => "<descriptor>".to_string(),
            Value::Trace(words) => words.to_string(),
            Value::Hashed(pair) => pair.0.plain(),
            Value::Fields(o) => format!("<attributes of {}>", o.class.name),
            Value::Declined(word) => word.to_string(),
            Value::Walking(_) | Value::Walk(_) => "<iterator>".to_string(),
            Value::Routine(p) => {
                let named = if p.qualified.is_empty() { p.ident.as_str() } else { p.qualified.as_str() };
                format!("<function {named} at 0x1>")
            }
            Value::Method(_, p) => format!("<function({})>", p.formals.join(", ")),
            Value::Bond(shared) | Value::Binding(shared) => shared.borrow().plain(),
            Value::Class(c) => c.outline.clone().unwrap_or_else(|| format!("<class {}>", c.name)),
            // A kind's own method read from the kind itself is bound to
            // nothing and is written with the kind it belongs to; a data
            // member reads the same way, but under CPython's own word
            // for the descriptor that carries it.
            Value::Adapter(w) if w.0 == 29 => match w.1.as_slice() {
                [Value::Text(kind), Value::Text(word)] => match Self::loose_member_descriptor(kind, word) {
                    Some((label, _)) => format!("<{label} '{word}' of '{kind}' objects>"),
                    None => format!("<method '{word}' of '{kind}' objects>"),
                },
                _ => "<member wrapper>".to_string(),
            },
            Value::Adapter(_) => "<member wrapper>".to_string(),
            Value::Object(o) => format!("<object {}>", o.class.name),
            Value::SortOf(k) => k.tag().to_string(),
            Value::Slice(parts) => format!("slice({}, {}, {})", parts[0].core_repr(false), parts[1].core_repr(false), parts[2].core_repr(false)),
        }
    }

    /// Whether a loose member, read off a builtin kind's own word, is a
    /// data member rather than a method, for the small set of kinds
    /// that carry one: what CPython calls the descriptor in its repr,
    /// and the name `type()` gives it. `int`, `bool` and `float` show
    /// an attribute; `complex`, `range` and `slice` show a member, as
    /// CPython 3.11 has it.
    pub(crate) fn loose_member_descriptor(kind: &str, name: &str) -> Option<(&'static str, &'static str)> {
        match kind {
            "int" | "bool" | "float" if matches!(name, "real" | "imag" | "numerator" | "denominator") => Some(("attribute", "getset_descriptor")),
            "complex" if matches!(name, "real" | "imag") => Some(("member", "member_descriptor")),
            "range" | "slice" if matches!(name, "start" | "stop" | "step") => Some(("member", "member_descriptor")),
            _ => None,
        }
    }

    /// The word a value goes by as a kind, where it stands for one and
    /// not merely for something of one: the reference's own name for
    /// that kind. Nothing for a value that is one of a kind.
    pub fn kind_it_names(&self) -> Option<String> {
        match self {
            Value::Class(c) => Some(c.name.clone()),
            Value::SortOf(sort) => Some(Value::sort_called(*sort).to_string()),
            Value::ByteKind(mutable, _) => Some(if *mutable { "bytearray" } else { "bytes" }.to_string()),
            Value::Native(b, word) if b.names_kind() => Some(word.to_string()),
            Value::Bond(cell) | Value::Binding(cell) | Value::Collection(cell, _) => cell.borrow().kind_it_names(),
            _ => None,
        }
    }

    /// Where the thing behind a value is kept: the cell a collection
    /// lives in, else the place its own members or letters stand at.
    /// This is what the reference writes after a bound method's kind.
    /// Nothing for a value that is kept nowhere of its own.
    pub fn standing(&self) -> Option<usize> {
        Some(match self {
            Value::Bond(cell) | Value::Binding(cell) | Value::Collection(cell, _) => Rc::as_ptr(cell) as *const u8 as usize,
            Value::Array(items) | Value::Tuple(items) => Rc::as_ptr(items) as *const u8 as usize,
            Value::Map(pairs) => Rc::as_ptr(pairs) as *const u8 as usize,
            Value::Set(members) => Rc::as_ptr(members) as *const u8 as usize,
            Value::Bytes(row, ..) => Rc::as_ptr(row) as *const u8 as usize,
            Value::Text(letters) => letters.as_ptr() as usize,
            Value::Object(thing) => Rc::as_ptr(thing) as *const u8 as usize,
            Value::Cursor(state) => Rc::as_ptr(state) as *const u8 as usize,
            _ => return None,
        })
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
/// How a method of a builtin's own is written: the name it answers to,
/// and either the kind of the thing it was read from with where that
/// thing is kept, or, where it was read from the kind itself and so is
/// bound to nothing, the name of that kind. Where the thing is kept in
/// no place of its own, the method's own place stands for it.
fn method_written(subject: &Value, word: &str, elsewhere: usize) -> String {
    let held = subject.contents();
    if let Some(kind) = held.kind_it_names() { return format!("<method '{word}' of '{kind}' objects>"); }
    format!("<built-in method {word} of {} object at 0x{:x}>", held.core_kind(), subject.standing().unwrap_or(elsewhere))
}

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

/// The name a class keeps its metaclass under, among its constants,
/// where its header named one or a class it stands on was made by one.
/// No program can spell it.
pub const MAKER_MEMBER: &str = "\0metaclass";

/// A class: what it is called, what it stands on, the properties an
/// object of it begins with, the programs it answers to, its constants
/// and the values it keeps for itself.
#[derive(Debug)]
pub struct Class {
    pub lineage: Vec<Rc<Class>>,
    pub direct: Vec<Rc<Class>>,
    pub outline: Option<String>,
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
    Value::Real(Rc::new(Real { floating: false, p, q: BigInt::zero(), places, below: false, point: false }))
}

/// A real brought to the nearest one of a width of bits, held exactly.
/// Where the language holds no width, or the number is past every one
/// of that width, it is left as it stands.
pub fn to_binary_width(v: Value, bits: Option<usize>, places: usize, shortest: bool) -> Value {
    if shortest {
        if let Value::Real(r) = &v {
            let number = nearest_real(&r.p, &r.q);
            return real_of(if number == 0.0 && r.below { -0.0 } else { number }, places).with_point(r.point);
        }
        return v;
    }
    if bits.is_none() {
        return v;
    }
    let (p, q, below) = match &v {
        Value::Real(r) => (r.p.clone(), r.q.clone(), r.below),
        Value::Frac(r) => (r.p.clone(), r.q.clone(), false),
        _ => return v,
    };
    let binary = as_binary(&p, &q);
    match from_binary(binary) {
        // A nought below nought keeps its minus at any width.
        Some((p, q)) => crate::arith::shape_signed(p, q, Some(places), below || binary.is_sign_negative()).with_point(v.keeps_point()),
        None => real_of(binary, places),
    }
}

thread_local! {
    /// The cells whose contents are being written out at this moment,
    /// outermost first, so that a collection holding itself, however
    /// far down, is written as an ellipsis where it comes round again.
    static SHOWING: RefCell<Vec<*const RefCell<Value>>> = const { RefCell::new(Vec::new()) };
    /// Where the members lie of the collections being written out at
    /// this moment. A fixed row keeps no cell of its own, and the walk
    /// that asks a thing how it is written is handed a collection's
    /// members rather than the cell holding them, so both are known
    /// here by the place their members stand in. They are gathered in a
    /// set and not a list, so that asking whether a place is already
    /// among them costs the same whether the writing stands one deep or
    /// a hundred thousand deep.
    static MEMBERS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    /// The cells a run keeps its counts of figures in, where the
    /// language gives those counts a name of their own: how many
    /// figures a real written plainly carries, and how many one shown
    /// with its kind carries. Both stand empty for a language that
    /// keeps no such count.
    static FIGURES: RefCell<(Option<Rc<RefCell<Value>>>, Option<Rc<RefCell<Value>>>)> = const { RefCell::new((None, None)) };
}

/// Where a collection's members lie, and the marks the collection is
/// put down as where it is met again within its own writing: a map
/// stands as its braces, a fixed row as the brackets a fixed row is
/// written between, and any other row as its own. Nothing at all for
/// what is no collection, since nothing else can hold itself.
fn members_of(value: &Value) -> Option<(usize, &'static str)> {
    match value {
        Value::Map(pairs) => Some((Rc::as_ptr(pairs) as usize, "{...}")),
        Value::Tuple(items) => Some((Rc::as_ptr(items) as usize, "(...)")),
        Value::Array(items) => Some((Rc::as_ptr(items) as usize, "[...]")),
        _ => None,
    }
}

/// Note that a collection's members are being written out. Answers the
/// marks it stands as where a note already lies on those same members,
/// and otherwise the note to take away again when the writing is done.
fn note_members(value: &Value) -> Result<Option<usize>, &'static str> {
    let Some((place, marks)) = members_of(value) else { return Ok(None) };
    MEMBERS.with(|held| {
        let mut held = held.borrow_mut();
        if !held.insert(place) { return Err(marks); }
        Ok(Some(place))
    })
}

fn forget_members(note: Option<usize>) {
    if let Some(place) = note {
        MEMBERS.with(|held| { held.borrow_mut().remove(&place); });
    }
}

/// Write out a collection's members, unless those very members are
/// already being written out further up, where the marks the collection
/// would have stood between are put down in its stead. A collection met
/// twice by two roads is written in full both times, the note being
/// taken away as soon as its own writing is done, so only a collection
/// standing within itself is ever cut short. This is for a walk that
/// may be stopped by a complaint the program raised.
pub fn members_once<E>(value: &Value, write: impl FnOnce() -> Result<String, E>) -> Result<String, E> {
    let note = match note_members(value) {
        Ok(note) => note,
        Err(marks) => return Ok(marks.to_string()),
    };
    let written = write();
    forget_members(note);
    written
}

/// The same for a walk that always has an answer.
pub(crate) fn members_written(value: &Value, write: impl FnOnce() -> String) -> String {
    let note = match note_members(value) {
        Ok(note) => note,
        Err(marks) => return marks.to_string(),
    };
    let written = write();
    forget_members(note);
    written
}

/// Write out what a cell holds, unless that cell is already being
/// written out further up, where the ellipsis given stands in its stead.
fn shown_once(cell: &Rc<RefCell<Value>>, come_round_as: &str, write: impl FnOnce(&Value) -> String) -> String {
    let place = Rc::as_ptr(cell);
    let come_round = SHOWING.with(|held| {
        let mut held = held.borrow_mut();
        if held.contains(&place) { return true; }
        held.push(place);
        false
    });
    if come_round { return come_round_as.to_string(); }
    let written = write(&cell.borrow());
    SHOWING.with(|held| { held.borrow_mut().pop(); });
    written
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
pub(crate) fn shortest_real(x: f64) -> String {
    if !x.is_finite() { return x.to_string().to_ascii_lowercase(); }
    let scientific = format!("{:e}", x);
    let (mantissa, exponent) = scientific.split_once('e').expect("a power follows");
    let power: i32 = exponent.parse().expect("a whole power");
    if x != 0.0 && !(-4..16).contains(&power) {
        return format!("{}e{}{:02}", mantissa, if power < 0 { "-" } else { "+" }, power.unsigned_abs());
    }
    x.to_string()
}

/// Keep the ordinary decimal spelling after arithmetic rounds to a binary
/// width, without exposing the tail of the stored binary ratio.
fn expanded_real(number: f64, places: usize) -> String {
    let text = written_out(number, Some(places));
    let mut text = match text.split_once('E') {
        None => text,
        Some((front, power)) => {
            let sign = if front.starts_with('-') { "-" } else { "" };
            let digits = front.trim_start_matches('-').replace('.', "");
            let digits = digits.trim_end_matches('0');
            format!("{}{}", sign, laid_flat(digits, power.parse().unwrap_or(0)))
        }
    };
    // The ordinary writer counts the leading zero among the places and
    // stops the fraction when those places are used, without rounding it.
    if let Some(point) = text.find('.') {
        let whole = point - usize::from(text.starts_with('-'));
        text.truncate(text.len().min(point + 1 + places.saturating_sub(whole)));
    }
    text
}

fn byte_repr(row: &[u8], mutable: bool, opening: &str) -> String {
    let quote = if row.contains(&b'\'') && !row.contains(&b'"') { '"' } else { '\'' };
    let mut out = format!("{}{}", opening, quote);
    for &byte in row {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            b if b == quote as u8 => { out.push('\\'); out.push(quote); }
            32..=126 => out.push(byte as char),
            _ => { let _ = write!(out, "\\x{:02x}", byte); }
        }
    }
    out.push(quote);
    if mutable { out.push(')'); }
    out
}
/// Try each count of figures until the nearest decimal returns to this
/// binary worth. Rounding the last figure takes the even one at a tie.
pub(crate) fn real_roundtrip(x: f64) -> String {
    if x.is_nan() { return "nan".into(); }
    if x.is_infinite() { return if x.is_sign_negative() { "-inf" } else { "inf" }.into(); }
    let mut written = String::new();
    for places in 0..17 {
        written = format!("{x:.places$e}");
        if written.parse::<f64>().is_ok_and(|back| back.to_bits() == x.to_bits()) { break; }
    }
    let (head, tail) = written.split_once('e').expect("a decimal exponent");
    let power: i32 = tail.parse().expect("an exponent is whole");
    let head = if head.contains('.') { head.trim_end_matches('0').trim_end_matches('.') } else { head };
    if !(-4..16).contains(&power) {
        return format!("{head}e{power:+03}");
    }
    let minus = head.starts_with('-');
    let digits = head.trim_start_matches('-').replace('.', "");
    let mut body = laid_flat(&digits, power);
    if !body.contains('.') { body.push_str(".0"); }
    if minus { body.insert(0, '-'); }
    body
}

/// Round a ratio once, including at the smallest binary places. The
/// remainder decides a tie before any bits are handed to the host real.
fn nearest_real(p: &BigInt, q: &BigInt) -> f64 {
    let minus = p.is_negative() != q.is_negative();
    let sign = u64::from(minus) << 63;
    if q.is_zero() || p.is_zero() { return as_binary(p, q); }
    let n = p.abs();
    let d = q.abs();
    let mut order = n.bits() as i64 - d.bits() as i64;
    if order > 1024 { return f64::from_bits(sign | 0x7ff0000000000000); }
    if order < -1075 { return f64::from_bits(sign); }
    if if order < 0 { (&n << -order as usize) < d } else { n < (&d << order as usize) } { order -= 1; }
    let place = (order - 52).max(-1074);
    let (n, d) = match place {
        0.. => (n, d << place as usize),
        _ => (n << -place as usize, d),
    };
    let (integral, remainder) = n.div_rem(&d);
    let halfway = (remainder * 2u8).cmp(&d);
    let mut bits = integral.to_u64().expect("the significand fits");
    bits += u64::from(halfway.is_gt() || (halfway.is_eq() && bits & 1 != 0));
    if bits >= 1 << 53 { bits >>= 1; order += 1; }
    if order > 1023 { return f64::from_bits(sign | 0x7ff0000000000000); }
    let magnitude = if bits < 1 << 52 { bits }
        else { ((order.max(-1022) + 1023) as u64) << 52 | (bits - (1 << 52)) };
    f64::from_bits(sign | magnitude)
}
