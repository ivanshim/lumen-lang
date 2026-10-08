// Values. A program value is a closure: the program and the frame it was
// made in, so a nested program sees the bindings around it.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use crate::tuples::Sequence;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::form::{Prim, Routine};

/// A run-time frame: slots, and the frame the program was made in.
pub struct Env {
    pub cells: RefCell<Vec<Value>>,
    pub capture_slots: RefCell<HashSet<usize>>,
    pub outer: Option<Rc<Env>>,
    /// Slots used by closures after this call returns.

    pub weak_callback_frame: Cell<bool>,
}

impl Drop for Env {
    fn drop(&mut self) { crate::ghost::departing_at(self as *const Env as usize); }
}

impl Env {
    pub fn make(size: usize, parent: Option<Rc<Env>>) -> Rc<Env> {
        Rc::new(Env { cells: RefCell::new(vec![Value::Unset; size]), capture_slots: RefCell::new(HashSet::new()), outer: parent, weak_callback_frame: Cell::new(false) })
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
    /// Use wide integers when the bounds, their difference and a
    /// nonzero step fit. Other progressions retain the big integer path.
    fn plain_bounds(&self) -> Option<(i128, i128, i128)> {
        let first=self.first.to_i128()?;
        let limit=self.limit.to_i128()?;
        let stride=self.stride.to_i128()?;
        let magnitude=stride.checked_abs().filter(|step|*step!=0)?;
        let span=if stride<0 {first.checked_sub(limit)?} else {limit.checked_sub(first)?};
        let size=if span<=0 {0} else {(span-1)/magnitude+1};
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
            if let Some(asked)=position.to_i128() {
                let offset=if asked<0 {asked+size} else {asked};
                if !(0..size).contains(&offset) {return None;}
                if let Some(member)=stride.checked_mul(offset).and_then(|jump|first.checked_add(jump)) {
                    return Some(if let Ok(word)=i64::try_from(member) {Value::Small(word)} else {Value::from_big(BigInt::from(member))});
                }
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
    /// The thing of the program's own this walk was asked of, kept so
    /// that it lives as long as the walk, as the reference has it.
    pub from_thing: Option<Value>,
    /// The word the reference gives a walk of the very thing this walk
    /// was made from. Gathering the members loses which kind of thing
    /// they came from, and this keeps that much of it.
    pub walks: Option<Rc<str>>,
}

/// The hidden entry under which a blueprint keeps the routines answering
/// its annotations until they are asked for; no program spells it.
pub const ANNOTATE_WORD: &str = "\0annotate";

#[derive(Clone)]
pub enum IteratorKind {
    Stored { entries: Sequence, next: usize },
    /// A progression stepped through one place at a time.
    Stepping(Rc<Progression>, BigInt),
    /// A list read through its cell at every step, so that members put
    /// in before the end are walked as well.
    Living(Rc<RefCell<Value>>, usize),
    /// A dictionary view with its entry offset and (initial size,
    /// outstanding members); a size mismatch remains sticky.
    Watching { window: Value, at: usize, size: (usize, u64) },
    /// A thing read place by place from nought, until the reading fails.
    Placed(Value, BigInt),
    /// A thing read place by place from its last down to nought, for
    /// one that answers `__getitem__` and `__len__` but keeps no
    /// `__reversed__` of its own.
    PlacedBack(Value, BigInt),
    /// A callable summoned for each member until it answers the sentinel.
    Summoned { work: Value, stop: Value, stop_exception: Option<Value> },
    /// A thing of the program's own, asked for each member the way a
    /// loop asks it.
    Handed(Value),
    Count(Value, BigInt),
    /// Inputs walked abreast, mapped where a mapper is given, and made
    /// to end together where exactness is demanded.
    Parallel { inputs: Vec<Value>, mapper: Option<Value>, exact: bool },
    Select(Value, Value),
    Busy,
}

#[derive(Debug)]
pub struct TraceLink {
    /// Preorder position of the executing expression in the compiled form tree.
    pub instruction: i64,
    pub extent: Option<(u32, u32, u32, u32)>,
    pub location: i64,
    pub activation: Rc<Thing>,
    pub following: RefCell<Value>,
}

#[derive(Debug)]
pub struct MethodMark;
impl Drop for MethodMark {
    fn drop(&mut self) { crate::ghost::departing_at(self as *const MethodMark as usize); }
}

#[derive(Clone)]
pub enum Value {
    Unpaired(Rc<[u32]>),
    Mutable(Rc<RefCell<Value>>, bool),
    Member(Rc<Value>, String),
    Window(Rc<Value>, char),
    Row(Sequence),
    Intrinsic(Prim, Rc<str>),
    Iterator(Rc<RefCell<IteratorState>>),
    Backtrace(Rc<TraceLink>),
    Keyed(Rc<Value>, Rc<Value>),
    Attributes(Rc<Thing>),
    Traversal(Rc<Value>, Rc<RefCell<Option<Value>>>),
    Refusal(Rc<str>),
    Cursor(Rc<RefCell<std::collections::VecDeque<Value>>>),
    Arguments(Sequence),
    Octets { cell: Rc<RefCell<Vec<u8>>>, changeable: bool, lead: Rc<str> },
    Export(Rc<crate::exec::OctetLease>),
    OctetKind { changeable: bool, shown: Rc<str> },
    Wrapped(u8, Sequence),
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
    Vector(Sequence),
    Tuple(Sequence),
    Generator(Rc<RefCell<crate::exec::Suspension>>),
    Set(Rc<RefCell<SetStore>>),
    SetCursor { source: Rc<RefCell<SetStore>>, count: usize },
    /// A span awaiting the length of what it is to read.
    Span(Sequence),
    /// Keys with their values, kept in the order they were written.
    Dict(Rc<MapStore>),
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
    Method(Rc<Routine>, Rc<Thing>, Rc<MethodMark>),
    Adorned(Rc<Adornment>),
    /// A weak hold on a thing behind a pointer: it keeps nothing about
    /// and gives the thing back only while it is still there.
    Dim(Rc<crate::ghost::Dim>),
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
//
// The membership set is scattered by the same plainer, quicker hash
// the label table reads by: an address here is already the outcome of
// `hash_address`, not a program's raw text, and nothing about set
// membership needs the guard against a chosen-key adversary that the
// standard scattering pays for on every insertion.
#[derive(Clone, Debug)]
pub struct SetStore {
    pub entries: Vec<(String, Value)>,
    pub keys: HashSet<String, crate::table::FxBuildHasher>,
    pub spelling: String,
    /// Whether nothing may alter this store. The kind the set is of is
    /// kept here, beside the entries, so that whoever holds the store
    /// knows the kind without going back to the table for the word.
    pub sealed: bool,
    /// A sealed store's fold, once it has been worked out. Nothing may
    /// alter such a store, so the fold cannot go stale and is worked
    /// out once however often it is wanted; a store of stores would
    /// otherwise fold every level beneath it over again each time.
    pub reckoned: std::cell::Cell<Option<i64>>,
}

impl SetStore {
    pub fn new(spelling: &str, sealed: bool) -> SetStore {
        SetStore { entries: vec![], keys: Default::default(), spelling: spelling.into(), sealed, reckoned: std::cell::Cell::new(None) }
    }

    /// The address the whole store takes where a set holds it: the
    /// addresses of its entries, ordered and then gathered into a
    /// single number, so that two stores of the same entries take the
    /// one address however either was gathered. They are gathered
    /// rather than written end to end because a store of stores would
    /// double the writing at every level: the numbers built from sets
    /// alone grow past any length a machine could hold, whereas one
    /// number stays one number however deep the nesting runs.
    pub fn whole_address(&self) -> String {
        let mut places: Vec<&str> = self.entries.iter().map(|(address, _)| address.as_str()).collect();
        places.sort_unstable();
        let mut gathering = std::collections::hash_map::DefaultHasher::new();
        for place in places { std::hash::Hash::hash(place, &mut gathering); }
        format!("sealed:{:x}", std::hash::Hasher::finish(&gathering))
    }

    pub fn put(&mut self, address: String, item: Value) {
        self.reckoned.set(None);
        if self.keys.insert(address.clone()) { self.entries.push((address, item)); }
    }

    pub fn take(&mut self, address: &str) -> Option<Value> {
        self.reckoned.set(None);
        if !self.keys.remove(address) { return None; }
        let place = self.entries.iter().position(|(k, _)| k == address).unwrap();
        Some(self.entries.remove(place).1)
    }

    /// The entries as they are read out. A thing kept beside its hash
    /// comes back as the thing alone: the hash is how the store knows
    /// where the thing lies and is no part of the entry.
    pub fn values(&self) -> Vec<Value> {
        self.entries.iter().map(|(_, v)| match v { Value::Keyed(thing, _) => thing.as_ref().clone(), held => held.clone() }).collect()
    }

    pub fn merge(&self, rhs: &SetStore, rule: u8) -> SetStore {
        let mut answer = SetStore::new(&self.spelling, self.sealed);
        for (key, item) in self.entries.iter().chain(rhs.entries.iter()) {
            let left = self.keys.contains(key);
            let right = rhs.keys.contains(key);
            let keep = match rule { 0 => left || right, 1 => left && right, 2 => left && !right, _ => left != right };
            if keep { answer.put(key.clone(), item.clone()); }
        }
        answer
    }

    /// The store written out. An empty one names its kind with nothing
    /// between its brackets; a sealed one names its kind before the
    /// braces, since no writing in a program stands for such a set and
    /// braces alone would read as the kind that may be altered.
    pub fn written(&self, item_text: impl Fn(&Value) -> String) -> String {
        match self.entries.len() {
            0 => self.spelling.clone() + "()",
            _ => {
                let words = self.entries.iter().map(|(_, item)| item_text(item)).collect::<Vec<_>>();
                let inner = "{".to_owned() + &words.join(", ") + "}";
                if self.sealed { self.spelling.clone() + "(" + &inner + ")" } else { inner }
            }
        }
    }
}

/// Where a key of a given address stands among a map's pairs, answered
/// through the place: `Found` where the place names a position outright;
/// `Absent` where the place accounts for every pair (none of them
/// lacked an address of its own) and none carries this one, so a miss
/// is proof; and `Unknown` where some pair's own key carries no address
/// of its own (a Thing with its own `__hash__`/`__eq__`, kept out of
/// the place entirely) and so a miss proves nothing — such a pair
/// might still be this address's own key by the program's equality,
/// which only the program can answer.
pub enum Found {
    Found(usize),
    Absent,
    Unknown,
}

thread_local! { static DICTIONARY_TURN: std::cell::Cell<u64> = const { std::cell::Cell::new(0) }; }

fn dictionary_turn() -> u64 {
    DICTIONARY_TURN.with(|counter| { counter.set(counter.get().wrapping_add(1)); counter.get() })
}

/// A map's pairs, kept in the order they were written, with a place
/// that answers where a key of a given address stands among them —
/// built the first time one is asked for, from every entry already
/// there, and answered from after that without being walked again.
/// Beside the place sits a plain count of the pairs it could carry no
/// address for, kept exact across every rebuild, so a miss in the
/// place is trusted as absence only when that count is nought.
/// The place belongs to the entries it was built from and to no
/// others: `Deref` reaches the pairs for every road that only reads
/// them, and `DerefMut` empties the place before handing out a way to
/// write them, so a road that grows, shrinks, reorders or overwrites
/// the pairs by hand cannot leave a place standing over pairs it no
/// longer describes. `place_key`, which does not go through
/// `DerefMut`, is the one road that keeps growing the pairs and the
/// place together, key by key, without either emptying it or walking
/// it again.
pub struct MapStore {
    pairs: Vec<(Value, Value)>,
    pub serial: u64,
    /// The entry extent, including deletion holes. Popitem trims its tail;
    /// reallocating the table compacts the surviving positions.
    pub span: usize,
    /// How many times the map has been cleared, so a walk may see that
    /// its places were made anew.
    pub clear_epoch: u64,
    /// Each pair's own place in the entry array, in the order the pairs
    /// stand; a place a key was taken out of is a gap here, so a walk
    /// backwards may skip it.
    pub slots: Vec<usize>,
    /// Table allocation, entries still available, and the specialized string-key kind.
    pub entry_budget: (usize, usize, bool),
    place: RefCell<Option<(std::collections::HashMap<String, usize>, usize)>>,
    spellings: RefCell<Option<Result<std::collections::HashMap<Rc<str>, usize>, ()>>>,
}

impl MapStore {
    /// Namespace spelling uses first text keys, without user hash/equality.
    /// A mutable key cell requires a fresh scan even when the rows are unchanged.
    pub fn namespace_position(&self, name: &str) -> Result<Option<usize>, ()> {
        fn fixed_text(key: &Value) -> Result<Option<Rc<str>>, ()> {
            match key {
                Value::Text(word) => Ok(Some(word.clone())),
                Value::Keyed(inner, _) => fixed_text(inner),
                Value::Shared(_) => Err(()),
                _ => Ok(None),
            }
        }
        let mut cached = self.spellings.borrow_mut();
        if cached.is_none() {
            let mut words = std::collections::HashMap::new();
            let mut immutable = true;
            for (at, (key, _)) in self.pairs.iter().enumerate() {
                match fixed_text(key) {
                    Ok(Some(word)) => { words.entry(word).or_insert(at); }
                    Ok(None) => (),
                    Err(()) => { immutable = false; break; }
                }
            }
            *cached = Some(if immutable { Ok(words) } else { Err(()) });
        }
        match cached.as_ref().expect("namespace spellings indexed") {
            Ok(words) => Ok(words.get(name).copied()),
            Err(()) => Err(()),
        }
    }

    pub fn overwrite_namespace(&mut self, at: usize, value: Value) {
        self.serial = dictionary_turn();
        self.place.get_mut().take();
        self.pairs[at].1 = value;
    }

    /// The place, built from scratch across every pair the first time
    /// one is asked for, and the count of pairs it carries no address
    /// for beside it.
    fn ensure_place(&self) {
        let mut place = self.place.borrow_mut();
        if place.is_none() {
            let mut built = std::collections::HashMap::with_capacity(self.pairs.len());
            let mut unaddressed = 0;
            for (at, (key, _)) in self.pairs.iter().enumerate() {
                match key.hash_address() {
                    Ok(addr) => { built.insert(addr, at); }
                    Err(_) => unaddressed += 1,
                }
            }
            *place = Some((built, unaddressed));
        }
    }

    /// Where a key of this address stands: found outright, proven
    /// absent because the place accounts for every pair, or unknown
    /// because some pair's key carries no address for the place to
    /// have accounted for, and only the program's own equality can
    /// say whether that pair is this address's key after all.
    pub fn locate(&self, address: &str) -> Found {
        self.ensure_place();
        let place = self.place.borrow();
        let (by_address, unaddressed) = place.as_ref().expect("just built");
        match by_address.get(address) {
            Some(at) => Found::Found(*at),
            None if *unaddressed == 0 => Found::Absent,
            None => Found::Unknown,
        }
    }

    /// Write over the pair already at a position the place has
    /// already named, without touching the place: the position it
    /// names does not move for this.
    pub fn overwrite_at(&mut self, at: usize, value: Value) {
        self.pairs[at].1 = value;
    }

    fn string_key(value: &Value) -> bool {
        if let Value::Keyed(original, _) = value { return matches!(original.as_ref(), Value::Text(_)); }
        matches!(value, Value::Text(_))
    }

    /// A new table removes holes; a cursor's saved offset does not change with it.
    pub fn rebuild(positions: &mut [usize], extent: &mut usize, allocation: &mut (usize, usize, bool), wanted: usize) {
        let mut size = 8;
        while size < wanted { size *= 2; }
        for index in 0..positions.len() { positions[index] = index; }
        *extent = positions.len();
        allocation.0 = size;
        allocation.1 = (size * 2 / 3).saturating_sub(*extent);
    }

    pub fn extend_positions(positions: &mut Vec<usize>, extent: &mut usize, allocation: &mut (usize, usize, bool), incoming: &Value) {
        let grown_size = positions.len() * 3;
        if allocation.2 && !Self::string_key(incoming) {
            Self::rebuild(positions, extent, allocation, grown_size);
            allocation.2 = false;
        }
        if allocation.1 == 0 { Self::rebuild(positions, extent, allocation, grown_size); }
        allocation.1 -= 1;
        positions.push(*extent);
        *extent += 1;
    }

    pub fn plan_merge(positions: &mut Vec<usize>, extent: &mut usize, allocation: &mut (usize, usize, bool), offered: &MapStore) {
        let count = offered.len();
        if count == 0 { return; }
        let (size, _, string_only) = offered.entry_budget;
        if positions.is_empty() && offered.span == count && (size == 8 || (size / 2) * 2 / 3 < count) {
            *allocation = (size, offered.entry_budget.1 + count, string_only);
            *extent = 0;
            return;
        }
        if allocation.0 * 2 / 3 < count {
            let expected = positions.len() + count;
            Self::rebuild(positions, extent, allocation, (expected * 3 + 1) / 2);
            allocation.2 = allocation.2 && string_only;
        }
    }

    pub fn copy_dictionary(&self) -> MapStore {
        if self.pairs.len() == 0 { return MapStore::from(Vec::new()); }
        let enough_live = self.pairs.len() >= (self.span * 2) / 3;
        match enough_live {
            true => self.clone(),
            false => {
                let mut compact = MapStore::from(self.pairs.to_vec());
                compact.entry_budget.2 = self.entry_budget.2;
                compact
            },
        }
    }

    pub fn take_last(&mut self) -> Option<(Value, Value)> {
        let result = self.pairs.pop();
        if result.is_some() {
            self.span = self.slots.pop().expect("last map position");
            self.serial = dictionary_turn();
            *self.place.borrow_mut() = None;
        self.spellings.get_mut().take();
        }
        result
    }

    /// Write a key at the next open place, growing the entry array.
    pub fn push_row(&mut self, key: Value, value: Value) {
        self.serial = dictionary_turn();
        self.pairs.push((key, value));
        Self::extend_positions(&mut self.slots, &mut self.span, &mut self.entry_budget, &self.pairs.last().expect("new pair").0);
        *self.place.borrow_mut() = None;
        self.spellings.get_mut().take();
    }

    pub fn push(&mut self, incoming: (Value, Value)) {
        let (key, content) = incoming;
        self.push_row(key, content);
    }

    pub fn remove(&mut self, index: usize) -> (Value, Value) {
        let pair = self.pairs[index].clone();
        self.remove_row(index);
        pair
    }

    pub fn retain(&mut self, mut wanted: impl FnMut(&(Value, Value)) -> bool) {
        let mut index = 0;
        while index < self.pairs.len() {
            if wanted(&self.pairs[index]) { index += 1; }
            else { self.remove_row(index); }
        }
    }

    pub fn extend(&mut self, source: impl IntoIterator<Item = (Value, Value)>) {
        source.into_iter().for_each(|pair| self.push(pair));
    }

    /// Take a row out by its place among the rows, leaving its slot as a
    /// gap the walk skips.
    pub fn remove_row(&mut self, at: usize) {
        self.serial = dictionary_turn();
        self.pairs.remove(at);
        self.slots.remove(at);
        *self.place.borrow_mut() = None;
        self.spellings.get_mut().take();
    }

    /// Rows and their positions have to agree before a cursor can inspect them.
    pub fn slots_synced(&self) -> Vec<usize> {
        assert_eq!(self.pairs.len(), self.slots.len(), "dictionary rows lost their positions");
        self.slots.to_vec()
    }

    /// The pairs themselves, read only: a blueprint that spells its
    /// slots as a mapping reads each name off a key.
    pub fn pairs(&self) -> &[(Value, Value)] {
        &self.pairs
    }

    /// Pairs re-laid after a filtering or a merge, keeping the width and
    /// the clear-history the map already had.
    pub fn kept(pairs: Vec<(Value, Value)>, slots: Vec<usize>, span: usize, clear_epoch: u64, entry_budget: (usize, usize, bool)) -> Self {
        assert_eq!(pairs.len(), slots.len());
        MapStore { pairs, serial: dictionary_turn(), span, clear_epoch, slots, entry_budget, place: RefCell::new(None), spellings: RefCell::new(None) }
    }

    /// Empty the pairs and mark the map's places as begun again: the
    /// entry array a walk reads against is no more.
    pub fn clear(&mut self) {
        self.serial = dictionary_turn();
        self.clear_epoch = self.clear_epoch.wrapping_add(1);
        self.span = 0;
        self.entry_budget = (1, 0, true);
        *self.place.borrow_mut() = None;
        self.spellings.get_mut().take();
        self.pairs.clear();
        self.slots.clear();
    }

    /// Add a key already proven absent and already known by its own
    /// address, growing the pairs and the place together so a map
    /// built up key by key never has its place emptied and walked
    /// afresh for the next key.
    pub fn insert_known_absent(&mut self, key: Value, address: String, value: Value) {
        self.ensure_place();
        let at = self.pairs.len();
        self.serial = dictionary_turn();
        self.pairs.push((key, value));
        self.spellings.get_mut().take();
        Self::extend_positions(&mut self.slots, &mut self.span, &mut self.entry_budget, &self.pairs.last().expect("new pair").0);
        self.place.borrow_mut().as_mut().expect("just built").0.insert(address, at);
    }
}

impl From<Vec<(Value, Value)>> for MapStore {
    fn from(pairs: Vec<(Value, Value)>) -> MapStore {
        let span = pairs.len();
        let slots = (0..span).collect();
        let mut table_size = if span == 0 { 1 } else { 8 };
        while table_size * 2 / 3 < span { table_size *= 2; }
        let entry_budget = (table_size, table_size * 2 / 3 - span, pairs.iter().all(|entry| Self::string_key(&entry.0)));
        MapStore { pairs, serial: dictionary_turn(), span, clear_epoch: 0, slots, entry_budget, place: RefCell::new(None), spellings: RefCell::new(None) }
    }
}

/// Built up pair by pair the plain way (`.collect()`), the place is
/// left to be built on first use rather than kept in step as it is.
impl std::iter::FromIterator<(Value, Value)> for MapStore {
    fn from_iter<I: IntoIterator<Item = (Value, Value)>>(iter: I) -> MapStore {
        MapStore::from(iter.into_iter().collect::<Vec<_>>())
    }
}

/// A copy of the pairs starts fresh: the place is cheap to build
/// again and answers for the copy's own pairs, never the original's.
impl Clone for MapStore {
    fn clone(&self) -> MapStore {
        MapStore { pairs: self.pairs.clone(), serial: self.serial, span: self.span, clear_epoch: self.clear_epoch, slots: self.slots.clone(), entry_budget: self.entry_budget, place: RefCell::new(None), spellings: RefCell::new(None) }
    }
}

impl std::ops::Deref for MapStore {
    type Target = Vec<(Value, Value)>;
    fn deref(&self) -> &Vec<(Value, Value)> { &self.pairs }
}

impl std::ops::DerefMut for MapStore {
    fn deref_mut(&mut self) -> &mut Vec<(Value, Value)> {
        self.serial = dictionary_turn();
        *self.place.borrow_mut() = None;
        self.spellings.get_mut().take();
        &mut self.pairs
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

/// The word CPython gives a window on a map's keys, values or pairs, or
/// the read-only reading of the map itself a window keeps beside it.
pub fn window_kind(portion: char) -> &'static str {
    match portion { 'k' => "dict_keys", 'v' => "dict_values", 'm' => "mappingproxy", _ => "dict_items" }
}

/// The word CPython gives a walk taken backwards over a map's keys,
/// values or pairs.
pub fn reversed_window_kind(portion: char) -> &'static str {
    match portion { 'k' => "dict_reversekeyiterator", 'v' => "dict_reversevalueiterator", _ => "dict_reverseitemiterator" }
}

impl Value {
    pub fn method(code: Rc<Routine>, receiver: Rc<Thing>) -> Value { Value::Method(code, receiver, Rc::new(MethodMark)) }

    pub fn tuple(parts: Vec<Value>) -> Self { Self::Tuple(Sequence::tuple(parts)) }

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

    pub fn proxy_pairs(&self) -> Value {
        if let Value::Shared(cell) | Value::Mutable(cell, _) = self { return cell.borrow().proxy_pairs(); }
        if let Value::Window(owner, 'm') = self { return owner.proxy_pairs(); }
        if let Value::Blueprint(blueprint) = self {
            let fields = blueprint.shared.borrow();
            return Value::Dict(Rc::new(fields.iter().filter_map(|(word, held)| {
                if word.starts_with('\0') || word.starts_with('#') { None }
                else { Some((Value::text(word), held.clone())) }
            }).collect()));
        }
        self.settled()
    }

    pub fn settled(&self) -> Value {
        if let Value::Mutable(place, _) | Value::Shared(place) = self { return place.borrow().settled(); }
        if let Value::Window(owner, portion) = self {
            let mut items=Vec::new();
            if let Value::Dict(entries)=owner.proxy_pairs() {
                // A window upon the pairs shows each of them as a
                // tuple, which is what it is: a pair written between
                // round marks, of the kind a pair may be a key by, and
                // not a row that only reads like one. A key kept beside
                // its hash is handed out as the thing itself: the hash
                // is the dictionary's own reckoning of where the thing
                // lies and no part of the key a window upon it shows.
                for (key,value) in entries.iter() {
                    // A pair whose place is still empty stands for a name
                    // not yet written: a namespace that shows it shows a
                    // name it does not have, so the window passes it by.
                    if matches!(value.settled(), Value::Unset) { continue; }
                    let bare = match key { Value::Keyed(thing, _) => thing.as_ref().clone(), other => other.clone() };
                    // A reading of the map itself walks, and is
                    // measured, the very way its keys are: it is asked
                    // after by key alone.
                    items.push(if *portion=='k' || *portion=='m' {bare} else if *portion=='v' {value.clone()} else {Value::tuple(vec![bare,value.clone()])});
                }
            }
            return Value::Vector(crate::tuples::Sequence::plain(items));
        }
        self.clone()
    }

    pub fn keep(self, quoted: bool) -> Value {
        if let Value::Shared(cell) = self {
            crate::ghost::note_container(&cell);
            return Value::Mutable(cell, quoted);
        }
        if matches!(self, Value::Vector(_) | Value::Dict(_)) {
            {
                let storage = Rc::new(RefCell::new(self));
                crate::ghost::note_container(&storage);
                Value::Mutable(storage, quoted)
            }
        } else { self }
    }

    pub fn repr(&self, names: &Names) -> String {
        if let Value::Mutable(cell, _) | Value::Shared(cell) = self {
            let mark = if matches!(&*cell.borrow(), Value::Dict(_)) { "{...}" } else { "[...]" };
            return within_cell_marked(cell, mark, |inner| inner.repr(names));
        }
        // A window on to a map keeps the name of the part it shows
        // around the members, which are written out within it.
        if matches!(self, Value::Window(..)) { return self.render(*names); }
        let settled = self.settled();
        match &settled {
            Value::Text(_) => settled.in_field(*names, "", "r").unwrap_or_else(|| settled.bare()),
            Value::Vector(v) | Value::Row(v) | Value::Tuple(v) => {
                let among = Among::members(&settled);
                if let Some(marks) = among.instead { return marks.to_string(); }
                let body = v.iter().map(|item| item.repr(names)).collect::<Vec<_>>().join(", ");
                if matches!(settled, Value::Vector(_)) { format!("[{body}]") }
                else { format!("({body}{})", if v.len() == 1 { "," } else { "" }) }
            }
            Value::Dict(entries) => {
                let among = Among::members(&settled);
                if let Some(marks) = among.instead { return marks.to_string(); }
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

    /// Whether a value is a set nothing may alter, read through the
    /// cells that may stand between a name and the set itself. A set
    /// held open whilst its entries are read answers as one that may
    /// be altered, nothing being askable of it at such a moment.
    pub fn set_sealed(&self) -> bool {
        match self {
            Value::Set(store) => store.try_borrow().map_or(false, |held| held.sealed),
            Value::SetCursor { source, .. } => source.try_borrow().map_or(false, |held| held.sealed),
            Value::Shared(cell) | Value::Mutable(cell, _) => cell.borrow().set_sealed(),
            _ => false,
        }
    }

    pub fn hash_address(&self) -> Result<String, &'static str> {
        match self {
            Value::Tuple(items) | Value::Row(items) => {
                let parts = items.iter().map(Value::hash_address).collect::<Result<Vec<_>, _>>()?;
                Ok(format!("row:{parts:?}"))
            }
            Value::Shared(slot) | Value::Mutable(slot, _) => slot.borrow().hash_address(),
            // A span is addressed by its three bounds, precisely as a
            // hashable tuple of them would be; where a bound has no
            // address of its own, the span is named as the one thing
            // unhashable, not the bound within it, as `hash` itself
            // already names it.
            Value::Span(bounds) => {
                let parts = bounds.iter().map(Value::hash_address).collect::<Result<Vec<_>, _>>().map_err(|_| "slice")?;
                Ok(format!("slice:{parts:?}"))
            }
            Value::Vector(_) => Err("list"),
            Value::Dict(_) => Err("dict"),
            // A sealed set is addressed by what it holds; one that may
            // be altered has no address at all.
            Value::Set(store) => match store.try_borrow() {
                Ok(held) if held.sealed => Ok(held.whole_address()),
                _ => Err("set"),
            },
            Value::Unpaired(numbers) => Ok(format!("unpaired:{numbers:?}")),
            Value::Text(word) => Ok(format!("text:{word}")),
            Value::Octets { changeable: true, .. } => Err("bytearray"),
            Value::Octets { cell, .. } => Ok(format!("octets/{:?}", cell.borrow().as_slice())),
            Value::Intrinsic(_, name) => Ok(["intrinsic/", name.as_ref()].concat()),
            Value::OctetKind { changeable, .. } => Ok(format!("octetkind/{changeable}")),
            Value::Blueprint(class) => Ok(format!("blueprint/{:p}", Rc::as_ptr(class))),
            Value::Wrapped(8, names) => Ok(format!("kind/{names:?}")),
            Value::Routine(program) => Ok(format!("code/{:p}", Rc::as_ptr(program))),
            Value::Wrapped(tag, items) if matches!(tag, 1 | 2 | 14 | 19 | 30 | 40..=42 | 60) => {
                let mut address = format!("native/{tag}");
                for item in items.iter() { address.push_str(&format!("/{:?}", item.hash_address()?)); }
                Ok(address)
            }
            Value::Wrapped(4..=7, items) => Ok(format!("wrapper/{:p}", Rc::as_ptr(items))),
            Value::Bound(program, frame) => Ok(format!("closure/{:p}/{:p}", Rc::as_ptr(program), Rc::as_ptr(frame))),
            Value::Method(program, receiver, _) => Ok(format!("bound/{:p}/{:p}", Rc::as_ptr(program), Rc::as_ptr(receiver))),

            // A progression is addressed by the places it names: their
            // count, where they begin and how far apart they stand, so
            // that two naming the same places share one address. One of
            // a single place has no stride, an empty one no start.
            Value::Progression(sequence) => {
                let count = sequence.count();
                if count.is_zero() { return Ok("walk:0".to_owned()); }
                if count.is_one() { return Ok(format!("walk:1:{}", sequence.first)); }
                Ok(format!("walk:{}:{}:{}", count, sequence.first, sequence.stride))
            }
            // A complex is addressed by the two numbers it stands
            // for; one whose imaginary part is nought is the very real
            // number it equals, and takes that number's own address,
            // so the two meet as one. A pair holding a value no number
            // answers to keeps the place it lies in, shared with none.
            Value::Complex(pair) => {
                if pair.0.is_nan() || pair.1.is_nan() { return Ok(format!("apart:{:p}", Rc::as_ptr(pair))); }
                if pair.1 == 0.0 { return crate::complex::decimal_value(pair.0).hash_address(); }
                let limb = |n: f64| if n == 0.0 { "0".to_string() } else { format!("{n:?}") };
                Ok(format!("pair:{}/{}", limb(pair.0), limb(pair.1)))
            }
            // Whole numbers already have a denominator of one, so
            // their address needs no temporary ratio or reduction.
            Value::Small(n) => Ok(format!("number:{n}:1")),
            Value::Huge(n) => Ok(format!("number:{n}:1")),
            Value::Flag(b) => Ok(format!("number:{}:1", u8::from(*b))),
            Value::Nil => Ok("nothing".to_owned()),
            Value::Ellipsis => Ok("ellipsis".to_owned()),
            // A loose member descriptor is addressed by the descriptor it
            // is kept as: two readings of the same member, and of the
            // same member on the same kind, are the one address.
            Value::Wrapped(60, parts) => Ok(format!("descriptor:{:p}", Rc::as_ptr(parts))),
            value => {
                let Some(ratio) = crate::math::ratio_of(value) else { return Err(""); };
                // Nothing under the line marks a worth off the scale.
                // Either endless worth is the one value wherever it is
                // met, since it equals itself; a worth that is no
                // number equals nothing at all, not even itself, so it
                // takes the place it lies in for its address and shares
                // that place with no other.
                if ratio.beneath.is_zero() {
                    if !ratio.above.is_zero() { return Ok(format!("beyond:{}", if ratio.above.is_negative() { '-' } else { '+' })); }
                    if let Value::Frac(parts) = value { return Ok(format!("apart:{:p}", Rc::as_ptr(parts))); }
                    return Err("");
                }
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
            Value::Thing(thing) => {
                if let Some(shown) = Self::gathered_representation(thing, words) { return shown; }
                match self.arguments_held() {
                Some(row) => {
                    let mut shown: Vec<String> = row.iter().map(|x| x.representation(words)).collect();
                    if thing.blueprint().has_public_field("\0import-fault") {
                        for key in ["name", "path", "name_from"] {
                            let value = thing.holds.borrow().iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
                            match value {
                                None | Some(Value::Nil) => {},
                                Some(v) => shown.push(format!("{key}={}", v.representation(words))),
                            }
                        }
                    }
                    format!("{}({})", thing.blueprint().name, shown.join(", "))
                },
                None => self.render(words),
                }
            },
            Value::Vector(row) => {
                let among = Among::members(self);
                match among.instead {
                    Some(marks) => marks.to_string(),
                    None => format!("[{}]", row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", ")),
                }
            }
            _ => self.render(words),
        }
    }

    fn gathered_representation(thing: &Rc<Thing>, words: Names) -> Option<String> {
        let holds = thing.holds.borrow();
        let style = match holds.iter().find(|(k, _)| k == "\0source-kind") { Some((_, Value::Small(style))) => *style, _ => return None };
        let named = thing.blueprint().name.clone();
        let message = holds.iter().find(|(k, _)| k == "\0heading").map(|(_, v)| v.representation(words)).unwrap_or_default();
        let parts: Vec<Value> = match holds.iter().find(|(k, _)| k == "\0gathered") {
            Some((_, Value::Tuple(items))) => items.to_vec(),
            _ => Vec::new(),
        };
        let list_like = style == 1 && matches!(holds.iter().find(|(k, _)| k == "\0raised-values"), Some((_, Value::Arguments(row))) if row.len() == 2 && matches!(row[1].settled(), Value::Vector(_)));
        let body = if style == 2 {
            holds.iter().find(|(k, _)| k == "\0source-repr").map(|(_, v)| v.bare()).unwrap_or_default()
        } else if list_like {
            format!("[{}]", parts.iter().map(|v| v.representation(words)).collect::<Vec<_>>().join(", "))
        } else { Self::argument_text(&parts, words) };
        Some(format!("{named}({message}, {body})"))
    }

    fn argument_text(row: &[Value], words: Names) -> String {
        let contents = row.iter().map(|x| x.representation(words)).collect::<Vec<_>>().join(", ");
        if row.len() == 1 { format!("({contents},)") } else { format!("({contents})") }
    }

    fn arguments_held(&self) -> Option<Vec<Value>> {
        if let Value::Thing(thing) = self {
            if thing.blueprint().has_public_field("\0fault-kind") {
                let holds = thing.holds.borrow();
                return Some(match holds.iter().find(|(key, _)| key == "\0raised-values") {
                    Some((_, Value::Arguments(row))) => row.to_vec(),
                    _ => holds.iter().filter(|(key, value)| key == "message" && !matches!(value, Value::Text(s) if s.is_empty())).map(|(_, value)| value.clone()).collect(),
                });
            }
        }
        None
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
    /// members it carries rather than from the row it was made with,
    /// so that changing one afterward changes what it is shown by.
    /// Nothing here is a language's own word: these three kinds and
    /// their wording belong to Python alone, and are reached only
    /// through the markers Python's own roster puts on its kinds.
    fn unicode_fault_text(thing: &Rc<Thing>, words: Names) -> Option<String> {
        let marker = thing.blueprint().every_field().iter().find_map(|(k, _)| match k.as_str() {
            "\0unicode-encode" => Some(0u8), "\0unicode-decode" => Some(1u8),
            "\0unicode-translate" => Some(2u8), _ => None,
        })?;
        let holds = thing.holds.borrow();
        let get = |name: &str| holds.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
        let as_i64 = |v: Option<Value>| match v { Some(Value::Small(n)) => Some(n), _ => None };
        let start = as_i64(get("start"))?;
        let end = as_i64(get("end"))?;
        let reason = get("reason").unwrap_or(Value::Nil).render(words);
        let object = get("object");
        let single = end == start + 1 && start >= 0;
        let one = if !single { None } else { match &object {
            Some(Value::Unpaired(numbers)) if marker != 1 => numbers.get(start as usize).map(|&n| Self::unicode_escaped(n)),
            Some(Value::Text(s)) if marker != 1 => usize::try_from(start).ok().and_then(|i| s.chars().nth(i)).map(|c| Self::unicode_escaped(c as u32)),
            Some(Value::Octets { cell, .. }) if marker == 1 => usize::try_from(start).ok().and_then(|i| cell.borrow().get(i).copied()).map(|b| format!("{:02x}", b)),
            _ => None,
        }};
        Some(match (marker, &one) {
            (0, Some(escaped)) => format!("'{}' codec can't encode character '{escaped}' in position {start}: {reason}", get("encoding").unwrap_or(Value::Nil).render(words)),
            (0, None) => format!("'{}' codec can't encode characters in position {start}-{}: {reason}", get("encoding").unwrap_or(Value::Nil).render(words), end - 1),
            (1, Some(hexed)) => format!("'{}' codec can't decode byte 0x{hexed} in position {start}: {reason}", get("encoding").unwrap_or(Value::Nil).render(words)),
            (1, None) => format!("'{}' codec can't decode bytes in position {start}-{}: {reason}", get("encoding").unwrap_or(Value::Nil).render(words), end - 1),
            (_, Some(escaped)) => format!("can't translate character '{escaped}' in position {start}: {reason}"),
            (_, None) => format!("can't translate characters in position {start}-{}: {reason}", end - 1),
        })
    }

    pub fn raised_words(&self, words: Names) -> Option<String> {
        let row = self.arguments_held()?;
        let Value::Thing(thing) = self else { return None };
        if let Some(told) = Self::unicode_fault_text(thing, words) { return Some(told); }
        let syntax = {
            let members = thing.holds.borrow();
            members.iter().find_map(|(key, value)| if key == "\0syntax-layout" { Some(value.clone()) } else { None })
        };
        if let Some(Value::Tuple(keys)) = syntax {
            let members = thing.holds.borrow();
            let read = |index: usize| -> Value {
                if let Some(Value::Text(key)) = keys.get(index) {
                    if let Some((_, value)) = members.iter().find(|(name, _)| name == key.as_ref()) { return value.clone(); }
                }
                Value::Nil
            };
            let message = read(0).render(words);
            let filename = match read(1) { Value::Text(path) => Some(path.rsplit('/').next().unwrap_or("").to_owned()), _ => None };
            let lineno = if let Value::Small(n) = read(2) { Some(n) } else { None };
            return Some(if let Some(file) = filename {
                if let Some(n) = lineno { format!("{message} ({file}, line {n})") } else { format!("{message} ({file})") }
            } else if let Some(n) = lineno { format!("{message} (line {n})") } else { message });
        }
        // A gatherer, or a system fault with its number, was given the
        // words to show itself with when it was made.
        if let Some((_, Value::Text(told))) = thing.holds.borrow().iter().find(|(key, _)| key == "\0told-as") { return Some(told.to_string()); }
        Some(if row.is_empty() { String::new() }
            else if row.len() > 1 { Self::argument_text(&row, words) }
            else if thing.blueprint().has_public_field("\0key-fault") { row[0].representation(words) }
            else { row[0].render(words) })
    }

    pub fn from_big(n: BigInt) -> Value {
        match n.to_i64() {
            Some(i) => Value::Small(i),
            None => Value::Huge(Rc::new(n)),
        }
    }

    pub fn character_numbers(&self) -> Option<Vec<u32>> {
        match self {
            Value::Unpaired(numbers) => Some(numbers.to_vec()),
            Value::Text(word) => Some(word.chars().map(|c| c as u32).collect()),
            _ => None,
        }
    }

    /// A stand-in text for a row of numbers a category question walks
    /// one at a time: a stow that reading left behind answers none of
    /// them, exactly as the half of a surrogate pair it stands for
    /// answers none of Python's own, so the stand-in below is a real
    /// character no category claims, standing in for one no `char` can
    /// hold at all.
    pub fn category_text(numbers: &[u32]) -> String {
        numbers.iter().map(|&n| char::from_u32(n).unwrap_or('\u{FFFE}')).collect()
    }

    pub fn characters(numbers: Vec<u32>) -> Value {
        let mut word = String::new();
        for &number in &numbers {
            let Some(letter) = char::from_u32(number) else { return Value::Unpaired(Rc::from(numbers)); };
            word.push(letter);
        }
        Value::text(&word)
    }

    pub fn unpaired_quoted(numbers: &[u32]) -> String {
        let delimiter = if numbers.contains(&39) && !numbers.contains(&34) { '"' } else { '\'' };
        let pieces = numbers.iter().map(|&n| match n {
            0xd800..=0xdfff => format!("\\u{:04x}", n),
            10 => String::from("\\n"), 13 => String::from("\\r"), 9 => String::from("\\t"),
            92 => String::from("\\\\"),
            n if n == delimiter as u32 => format!("\\{delimiter}"),
            n => match char::from_u32(n) {
                Some(c) if crate::unicode::property(c, 1) => c.to_string(),
                _ => Self::unicode_escaped(n),
            },
        }).collect::<String>();
        format!("{delimiter}{pieces}{delimiter}")
    }

    pub fn text(s: &str) -> Value {
        Value::Text(Rc::from(s))
    }

    pub fn kind(&self) -> Option<Kind> {
        Some(match self {
            Value::Small(_) | Value::Huge(_) => Kind::Whole,
            Value::Frac(e) => if e.places.is_some() { Kind::Decimal } else { Kind::Fraction },
            Value::Text(_) | Value::Unpaired(_) => Kind::Chars,
            Value::Flag(_) => Kind::Truth,
            Value::TextRow(..) | Value::Vector(_) | Value::Dict(_) | Value::Row(_) => Kind::Vector,
            Value::Mutable(place, _) => return place.borrow().kind(),
            Value::Member(..) | Value::Complex(_) => return None,
            Value::Window(..) => Kind::Vector,
            Value::Nil | Value::KindOf(_) => Kind::Nothing,
            Value::Shared(cell) => return cell.borrow().kind(),
            Value::Wrapped(..) | Value::Octets { .. } | Value::Export(_) | Value::OctetKind { .. } | Value::Arguments(_) | Value::Backtrace(_) | Value::Keyed(..) | Value::Attributes(_) | Value::Traversal(..) | Value::Refusal(_) | Value::Cursor(_) | Value::SetCursor { .. } | Value::Couple(_) | Value::Blueprint(_) | Value::Thing(_) => return None,
            Value::TextCall { .. } | Value::Intrinsic(..) | Value::Iterator(_) | Value::Adorned(_) | Value::Dim(_) | Value::Generator(_) | Value::Tuple(_) | Value::Imaginary { .. } | Value::Ellipsis | Value::Method(..) | Value::Routine(_) | Value::Bound(..) | Value::Unset | Value::Span(_) | Value::Channel(_) | Value::Progression(_) => return None,
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
            Value::Unpaired(numbers) => !numbers.is_empty(),
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
            Value::Unpaired(_) => return Err("ValueError: invalid literal for int()".to_string()),
            Value::Text(s) => s.parse().map_err(|_| format!("Cannot coerce '{}' to number", s))?,
            Value::TextRow(..) | Value::Arguments(_) | Value::Set(_) | Value::Tuple(_) | Value::Vector(_) | Value::Dict(_) | Value::Couple(_) | Value::Row(_) | Value::Window(..) => return Err("Cannot coerce array to number".to_string()),
            Value::Wrapped(..) | Value::Blueprint(_) | Value::Thing(_) => return Err("Cannot coerce object to number".to_string()),
            Value::Shared(cell) => return cell.borrow().as_big(),
            Value::TextCall { .. } | Value::Adorned(_) | Value::Dim(_) | Value::Generator(_) | Value::Method(..) | Value::Routine(_) | Value::Bound(..) => return Err("Cannot coerce function to number".to_string()),
            Value::Mutable(place, _) => return place.borrow().as_big(),
            Value::Member(..) => return Err("Cannot coerce method to number".to_string()),
            Value::Octets { .. } | Value::Export(_) | Value::OctetKind { .. } | Value::Backtrace(_) | Value::Keyed(..) | Value::Attributes(_) | Value::Traversal(..) | Value::Refusal(_) | Value::Cursor(_) | Value::SetCursor { .. } | Value::Intrinsic(..) | Value::Iterator(_) | Value::Channel(_) | Value::Progression(_) => return Err("Cannot coerce this value to number".into()),
            Value::Ellipsis => return Err("Ellipsis is not a number".to_string()),
            Value::Span(_) => return Err("Cannot coerce slice to number".to_string()),
            Value::KindOf(_) => return Err("Cannot coerce kind meta-value to number".to_string()),
        })
    }

    /// Keep only the semantic attributes of a source code object.
    /// Its origin file and constant-replacement markers are operational data.
    pub(crate) fn source_code_attribute(name: &str) -> bool {
        ["source", "mode", "co_flags", "co_firstlineno", "co_consts"].contains(&name)
    }

    /// Build a routine's comparison key from its compiled body address,
    /// calling layout, names and recursively replaceable literals. The token
    /// origin plus body boundary describes the original instruction tree.
    fn program_worth(program: &crate::form::Routine) -> Value {
        let names = |words: &[String]| Value::tuple(words.iter().map(|word| Value::text(word)).collect());
        let kinds = program.taking.as_deref().unwrap_or(&[]).iter().map(|kind| Value::Small(*kind as i64)).collect();
        let mut attributes = vec![Value::text(&program.lexical_origin), Value::Small(program.body_boundary as i64)];
        attributes.extend([
            Value::text(&program.ident), Value::Small(program.declared_on.into()),
            Value::Small(program.flags), Value::Small(program.future_bits),
            Value::Flag(program.lineless), Value::Flag(program.generator),
            names(&program.formals), Value::tuple(kinds),
            program.gather_from.map_or(Value::Nil, |index| Value::Small(index as i64)),
            names(&program.referenced), names(&program.locals), names(&program.idents),
            Value::tuple(program.literals.clone()),
        ]);
        Value::tuple(attributes)
    }

    /// Weigh nested code by its own body and current constants at every level.
    fn code_worth_eq(one: &Value, two: &Value) -> bool {
        if one.one_place(two) { return true; }
        match (one, two) {
            (Value::Wrapped(7, p), Value::Wrapped(7, q)) => match (p.first(), q.first()) {
                (Some(Value::Routine(a) | Value::Bound(a, _)), Some(Value::Routine(b) | Value::Bound(b, _))) =>
                    Self::code_worth_eq(&Self::program_worth(a), &Self::program_worth(b)),
                _ => Rc::ptr_eq(p, q),
            },
            (Value::Routine(a), Value::Routine(b)) => Self::code_worth_eq(&Self::program_worth(a), &Self::program_worth(b)),
            (Value::Tuple(p), Value::Tuple(q)) => p.len() == q.len() && p.iter().zip(q.iter()).all(|(x, y)| Self::code_worth_eq(x, y)),
            (Value::Small(_) | Value::Huge(_), Value::Small(_) | Value::Huge(_)) => one.equals(two),
            (Value::Frac(p), Value::Frac(q)) => p.places.is_some() == q.places.is_some()
                && one.equals(two) && (p.above != BigInt::from(0) || p.under == q.under),
            // Numerically equal constants of different types remain distinct.
            _ => one.kind_word() == two.kind_word() && one.equals(two),
        }
    }

    /// Fold exactly the attributes that recursive code equality weighs.
    pub(crate) fn code_worth_hash(value: &Value) -> Option<i64> {
        match value {
            Value::Wrapped(7, parts) => match parts.first() {
                Some(Value::Routine(body) | Value::Bound(body, _)) => Self::code_worth_hash(&Self::program_worth(body)),
                _ => Some((Rc::as_ptr(parts) as usize / 16) as i64),
            },
            Value::Routine(program) => Self::code_worth_hash(&Self::program_worth(program)),
            Value::Thing(item) if item.blueprint().constants.iter().any(|(name, value)| name == "\0native" && matches!(value, Value::Text(kind) if kind.as_ref() == "code")) => {
                let attributes = item.holds.borrow().iter().filter(|(name, _)| Self::source_code_attribute(name)).map(|(_, worth)| worth.settled()).collect();
                Self::code_worth_hash(&Value::tuple(attributes))
            }
            Value::Tuple(parts) => {
                let mut accum: u64 = 2_870_177_450_012_600_261;
                for part in parts.iter() {
                    let lane = Self::code_worth_hash(part)? as u64;
                    accum = accum.wrapping_add(lane.wrapping_mul(14_029_467_366_897_019_727)).rotate_left(31);
                    accum = accum.wrapping_mul(11_400_714_785_074_694_791);
                }
                accum = accum.wrapping_add(parts.len() as u64 ^ (2_870_177_450_012_600_261 ^ 3_527_539));
                Some(if accum == u64::MAX { 1_546_275_796 } else { accum as i64 })
            }
            _ => value.hash_number(),
        }
    }

    pub fn equals(&self, other: &Value) -> bool {
        for (candidate, text) in [(self, other), (other, self)] {
            if let (Value::Thing(object), Value::Text(word)) = (candidate, text) {
                let slots = object.holds.borrow();
                if let Some((_, Value::Text(under))) = slots.iter().find(|entry| entry.0 == "\0underlying") { return under == word; }
            }
        }
        // Most askings are of two small numbers, two great ones, or two
        // pieces of text, and all three can be settled here and now.
        // Left to the ratios below, a pair of small numbers would have
        // two great numbers made of it and then be cross-multiplied.
        if let (Value::Small(here), Value::Small(there)) = (self, other) { return here == there; }
        if let (Value::Huge(here), Value::Huge(there)) = (self, other) { return here.as_ref() == there.as_ref(); }
        if let (Value::Text(here), Value::Text(there)) = (self, other) { return here.as_ref() == there.as_ref(); }
        if let Value::Mutable(cell, _) = self { return cell.borrow().equals(&other.settled()); }
        if let Value::Mutable(cell, _) = other { return self.equals(&cell.borrow()); }
        // A view of the entries a routine keeps is equal to whatever
        // the dictionary of those entries is equal to, whichever side
        // of the asking it stands on.
        if let Value::Attributes(held) = self { return held.entries_shown().equals(other); }
        if let Value::Attributes(held) = other { return self.equals(&held.entries_shown()); }
        match (self, other) {
            (Value::TextRow(a, fixed), Value::TextRow(b, closed)) => return fixed == closed && a == b,
            (Value::Span(a), Value::Span(b)) => return a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::TextRow(words, false), Value::Vector(values)) | (Value::Vector(values), Value::TextRow(words, false)) => {
                return words.len() == values.len() && words.iter().zip(values.iter()).all(|(word, value)| Value::text(word.as_str()).equals(value));
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
            (Value::Intrinsic(_, left), Value::Intrinsic(_, right)) => left == right,
            (Value::Iterator(left), Value::Iterator(right)) => Rc::ptr_eq(left,right),
            (Value::Set(left), Value::Set(right)) => left.borrow().keys == right.borrow().keys,
            // The arguments a fault was made with are a tuple in their
            // own right, and weigh the same as one written out.
            (Value::Arguments(one) | Value::Tuple(one), Value::Arguments(two) | Value::Tuple(two)) => one.len() == two.len() && one.iter().zip(two.iter()).all(|(a, b)| a.equals(b)),
            (Value::Octets { cell: x, .. }, Value::Octets { cell: y, .. }) => x.borrow().as_slice() == y.borrow().as_slice(),
            (Value::OctetKind { changeable: x, .. }, Value::OctetKind { changeable: y, .. }) => x == y,
            (Value::Intrinsic(crate::form::Prim::Octets(code), _), Value::OctetKind { changeable, .. })
            | (Value::OctetKind { changeable, .. }, Value::Intrinsic(crate::form::Prim::Octets(code), _)) => (*code == 1) == *changeable && *code < 2,
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
            (Value::Unpaired(one), Value::Unpaired(two)) => one == two,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Flag(a), Value::Flag(b)) => a == b,
            (Value::Nil, Value::Nil) | (Value::Ellipsis, Value::Ellipsis) => true,
            (Value::Refusal(a), Value::Refusal(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y)),
            (Value::Dict(a), Value::Dict(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|((j, x), (k, y))| j.equals(k) && x.equals(y))
            }
            (Value::Couple(a), Value::Couple(b)) => a.0.equals(&b.0) && a.1.equals(&b.1),
            // One object is itself and nothing else; two classes are one
            // when they carry the same name.
            (Value::Adorned(x), Value::Adorned(y)) => Rc::ptr_eq(x, y),
            (Value::Method(p, a, _), Value::Method(q, b, _)) => Rc::ptr_eq(p, q) && Rc::ptr_eq(a, b),
            (Value::Backtrace(a), Value::Backtrace(b)) => Rc::ptr_eq(a, b),
            (Value::Thing(a), Value::Thing(b)) => {
                let is_union = |thing: &Thing| thing.blueprint().constants.iter().any(|(word, held)| word == "\0native" && matches!(held, Value::Text(text) if text.as_ref() == "Union"));
                if is_union(a) && is_union(b) {
                    let args = |thing: &Thing| thing.holds.borrow().iter().find_map(|(name, item)| {
                        if name != "__args__" { return None; }
                        if let Value::Tuple(row) = item { Some(row.to_vec()) } else { None }
                    });
                    if let (Some(left), Some(right)) = (args(a), args(b)) {
                        return left.len() == right.len() && right.iter().all(|item| left.iter().any(|candidate| candidate.equals(item)));
                    }
                    return false;
                }
                // Two code values read from the same text in the same
                // manner weigh the same, the way the reference compares
                // its code objects' bytecode; the file each names is no
                // part of it.
                let is_code = |thing: &Thing| thing.blueprint().constants.iter().any(|(word, held)| word == "\0native" && matches!(held, Value::Text(text) if text.as_ref() == "code"));
                if is_code(a) && is_code(b) {
                    let held = |thing: &Thing| thing.holds.borrow().iter().filter(|(name, _)| Self::source_code_attribute(name)).map(|(_, item)| item.settled()).collect::<Vec<_>>();
                    let (left, right) = (held(a), held(b));
                    return left.len() == right.len() && left.iter().zip(right.iter()).all(|(x, y)| Self::code_worth_eq(x, y));
                }
                Rc::ptr_eq(a, b)
            },
            (Value::Blueprint(a), Value::Blueprint(b)) => if a.presentation.is_none() { a.name == b.name } else { Rc::ptr_eq(a,b) },
            (Value::Generator(x), Value::Generator(y)) => Rc::ptr_eq(x, y),
            // Independently built code handles weigh their semantic content.
            (Value::Wrapped(7, _), Value::Wrapped(7, _)) => Self::code_worth_eq(self, other),
            // A routine bound to a value is the one bound method where it
            // binds the one routine to the very same value.
            (Value::Wrapped(3,x), Value::Wrapped(3,y)) => {
                if let (Some(a @ Value::Intrinsic(..)), Some(b @ Value::Intrinsic(..)), Some(one), Some(two)) = (x.first(), y.first(), x.get(1), y.get(1)) { return a.equals(b) && one.one_and_same(two); }
                if let (Some(Value::Wrapped(60,p)), Some(Value::Wrapped(60,q)), Some(a), Some(b)) = (x.first(), y.first(), x.get(1), y.get(1)) {
                    p.len() == q.len() && p.iter().zip(q.iter()).all(|(u,v)| u.equals(v)) && (a.one_and_same(b) || a.one_place(b))
                } else { Rc::ptr_eq(x,y) || x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| p.equals(q)) }
            },
            (Value::Wrapped(132, one), Value::Wrapped(132, two)) => one[0].equals(&two[0]) && one[1].one_place(&two[1]),
            // A cell asked after twice is the one cell where both
            // wrappers name the one room they look into.
            (Value::Wrapped(35, x), Value::Wrapped(35, y)) => match (x.as_slice(), y.as_slice()) {
                ([Value::Shared(a)], [Value::Shared(b)]) => Rc::ptr_eq(a, b),
                ([Value::Bound(p, r), Value::Small(i), ..], [Value::Bound(q, s), Value::Small(j), ..]) => Rc::ptr_eq(p, q) && Rc::ptr_eq(r, s) && i == j,
                _ => Rc::ptr_eq(x, y),
            },
            (Value::Wrapped(k,x), Value::Wrapped(l,y)) => k == l && Rc::ptr_eq(x,y),
            // A routine bound to a frame is one value with itself alone:
            // the same code bound in another frame is another closure,
            // with names and a namespace of its own, as CPython has it.
            (Value::Bound(a, here), Value::Bound(b, there)) => Rc::ptr_eq(a, b) && Rc::ptr_eq(here, there),
            // A routine not yet bound is itself alone.
            (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a, b),
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
            (Value::Backtrace(x), Value::Backtrace(y)) => Rc::ptr_eq(x, y),
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
            Value::Window(owner, 'm') => format!("mappingproxy({})", owner.proxy_pairs().repr(&w)),
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
                let among = Among::members(self);
                if let Some(marks) = among.instead { return marks.to_string(); }
                let inside = parts.iter().map(|part| part.in_field(w, "", "r").unwrap_or_else(|| part.bare())).collect::<Vec<_>>().join(", ");
                format!("({}{})", inside, if parts.len() == 1 { "," } else { "" })
            }
            Value::Vector(items) => {
                let among = Among::members(self);
                match among.instead {
                    Some(marks) => marks.to_string(),
                    None => format!("[{}]", items.iter().map(|v| v.render(w)).collect::<Vec<_>>().join(", ")),
                }
            }
            Value::Dict(entries) => {
                let among = Among::members(self);
                if let Some(marks) = among.instead { return marks.to_string(); }
                format!("[{}]", entries.iter().map(|(k, v)| format!("{} => {}", k.render(w), v.render(w))).collect::<Vec<_>>().join(", "))
            }
            Value::Couple(e) => format!("{} => {}", e.0.render(w), e.1.render(w)),
            // A worth past the numbers is written by its name at any
            // width, there being no figures in it to write.
            // A real a working gave is written as any real is where the
            // table asks the shortest spelling, and in the library's own
            // spelling elsewhere.
            Value::Frac(e) if e.float_style && !w.brief_reals => {
                let number = if e.under && e.above.is_zero() && !e.past_numbers() { -0.0 } else { nearest_binary(&e.above, &e.beneath) };
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

    /// Internal storage is a string only when its blueprint descends from str.
    pub(super) fn type_text(&self) -> Value {
        let value = self.settled();
        if let Value::Thing(thing) = &value {
            let class = thing.blueprint();
            let string = std::iter::once(class.as_ref()).chain(class.ancestry.borrow().iter().map(Rc::as_ref))
                .any(|base| base.constants.iter().any(|(key, held)| key == "\0native" && matches!(held, Value::Text(word) if word.as_ref() == "str")));
            if string {
                if let Some(entry) = thing.holds.borrow().iter().find(|entry| entry.0 == "\0underlying") { return entry.1.settled(); }
            }
        }
        value
    }
    pub fn bare(&self) -> String {
        match self {
            Value::Complex(pair) => crate::complex::written(pair),
            Value::Imaginary { coefficient, .. } => brief_decimal(*coefficient) + "j",
            Value::Mutable(place, _) => within_cell(place, Value::bare),
            // A method of a builtin's own, handed over bound to what
            // it was read from, is written by its name, the kind of the
            // thing it was read from and where that thing is kept. One
            // read from the kind itself is bound to no thing at all and
            // is named with the kind it belongs to instead.
            Value::Member(held, word) => method_written(held, word, Rc::as_ptr(held) as *const u8 as usize),
            Value::Window(..) => self.settled().bare(),
            Value::Row(v) => format!("({})", v.iter().map(Value::bare).collect::<Vec<_>>().join(", ")),
            // A word naming a kind stands for the kind itself, and is
            // written as the reference writes a class; every other
            // intrinsic word is written as work waiting to be done.
            Value::Intrinsic(op, name) if op.names_a_kind() => format!("<class '{}'>", name),
            Value::Intrinsic(_, name) => format!("<built-in function {}>", name),
            Value::Iterator(_) => String::from("<iterator>"),
            Value::SetCursor { .. } => String::from("<set walk>"),
            Value::Set(items) => items.borrow().written(Value::bare),
            Value::Arguments(row) => format!("({}{})", row.iter().map(Value::bare).collect::<Vec<_>>().join(", "), if row.len() == 1 { "," } else { "" }),
            Value::Octets { cell, changeable, lead } => octets_shown(&cell.borrow(), lead, *changeable),
            Value::Export(_) => "<buffer export>".to_owned(),
            Value::OctetKind { shown, .. } => shown.to_string(),
            Value::TextCall { subject, name, .. } => method_written(&Value::Text(subject.clone()), name, subject.as_ptr() as usize),
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
            Value::Unpaired(numbers) => Self::unpaired_quoted(numbers),
            Value::Text(s) => s.to_string(),
            Value::Flag(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Nil | Value::Unset => "null".to_string(),
            Value::Vector(items) => {
                let among = Among::members(self);
                match among.instead {
                    Some(marks) => marks.to_string(),
                    None => format!("[{}]", items.iter().map(Value::bare).collect::<Vec<_>>().join(", ")),
                }
            }
            Value::Dict(entries) => {
                let among = Among::members(self);
                if let Some(marks) = among.instead { return marks.to_string(); }
                format!("[{}]", entries.iter().map(|(k, v)| format!("{} => {}", k.bare(), v.bare())).collect::<Vec<_>>().join(", "))
            }
            Value::Couple(e) => format!("{} => {}", e.0.bare(), e.1.bare()),
            Value::Generator(generator) => {
                if let Ok(state) = generator.try_borrow() {
                    if !state.titles[1].is_empty() {
                        return format!("<generator object {} at 0x1>", state.titles[1]);
                    }
                }
                "<generator>".into()
            },
            Value::Adorned(_) => String::from("<descriptor>"),
            Value::Dim(_) => String::from("<weak hold>"),
            Value::Backtrace(_) => String::from("<traceback object>"),
            Value::Keyed(value, _) => value.bare(),
            Value::Attributes(t) => format!("<attributes of {}>", t.blueprint().name),
            Value::Refusal(word) => word.to_string(),
            Value::Traversal(..) | Value::Cursor(_) => "<iterator>".to_owned(),
            Value::Routine(p) | Value::Bound(p, _) => {
                let title = if p.qualification.is_empty() { p.ident.clone() } else { p.qualification.clone() };
                format!("<function {title} at 0x1>")
            }
            Value::Method(p, _, _) => format!("<function({})>", p.formals.join(", ")),
            Value::Shared(cell) => cell.borrow().bare(),
            Value::Blueprint(b) => {
                if let Some(title) = b.python_title() { return format!("<class '{title}'>"); }
                b.presentation.clone().unwrap_or_else(|| format!("<class {}>", b.name))
            },
            // A method or a data member carried by a native kind and
            // read off the kind's own word stands loose, and is named
            // with that kind, under CPython's own word for the
            // descriptor that carries it.
            Value::Wrapped(32, fields) if matches!(fields.get(2), Some(Value::Small(-2))) => match &fields[1] {
                Value::Blueprint(owner) => format!("<attribute '{}' of '{}' objects>", fields[0].bare(), owner.name),
                _ => "<member wrapper>".into(),
            },
            Value::Wrapped(60, parts) => match parts.as_slice() {
                [Value::Text(kind), Value::Text(word)] => match Self::loose_member_descriptor(kind, word) {
                    Some((label, _)) => format!("<{label} '{word}' of '{kind}' objects>"),
                    None => format!("<method '{word}' of '{kind}' objects>"),
                },
                _ => "<member wrapper>".into(),
            },
            // Both wrapper kinds are written as the wrapping builtin
            // around the routine they hold, as CPython writes them.
            Value::Wrapped(4, parts) => format!("<staticmethod({})>", parts[0].bare()),
            Value::Wrapped(5, parts) => format!("<classmethod({})>", parts[0].bare()),
            Value::Wrapped(..) => "<member wrapper>".into(),
            Value::Tuple(items) => {
                let among = Among::members(self);
                if let Some(marks) = among.instead { return marks.to_string(); }
                let body = items.iter().map(|item| if let Value::Text(t) = item { format!("{t:?}") } else { item.bare() }).collect::<Vec<_>>().join(", ");
                format!("({body}{})", if items.len() == 1 { "," } else { "" })
            },
            Value::Thing(t) => match t.holds.borrow().iter().find(|entry| entry.0 == "\0type-display") {
                Some(entry) => entry.1.bare(),
                None => format!("<object {}>", t.blueprint().name),
            },
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
            Value::Method(p, t, _) => out.push_str(&format!("m{:p}/{:p}", Rc::as_ptr(p), Rc::as_ptr(t))),
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
/// The word a worth goes by as a kind, where it stands for one and not
/// merely for something of one: the reference's own name for that
/// kind. Nothing for a worth that is one of a kind.
impl Value {
    /// Whether an entry read off a native kind's own word is a data
    /// member rather than a method, for the small set of kinds that
    /// carry one: what CPython calls the descriptor in its repr, and
    /// the name `type()` gives it. `int`, `bool` and `float` show an
    /// attribute; `complex`, `range` and `slice` show a member, as
    /// CPython 3.11 has it.
    pub(crate) fn loose_member_descriptor(kind: &str, name: &str) -> Option<(&'static str, &'static str)> {
        match (kind, name) {
            ("getset_descriptor" | "member_descriptor" | "method_descriptor" | "wrapper_descriptor" | "classmethod_descriptor", "__get__") => return Some(("slot wrapper", "wrapper_descriptor")),
            _ => (),
        }
        if (kind, name) == ("dict", "fromkeys") { return Some(("method", "classmethod_descriptor")); }
        match (name, kind) {
            ("__getitem__", "dict" | "list") | ("__contains__", "frozenset" | "set" | "dict") => return Some(("method", "method_descriptor")),
            _ => (),
        }
        let native_slot = match name.len() {
            6 => matches!(name, "__ge__" | "__eq__" | "__gt__" | "__ne__" | "__lt__" | "__le__" | "__or__"),
            7 => matches!(name, "__and__" | "__add__" | "__abs__" | "__get__" | "__del__" | "__ior__" | "__int__" | "__mul__" | "__mod__" | "__len__" | "__pow__" | "__pos__" | "__neg__" | "__ror__" | "__xor__" | "__sub__" | "__str__" | "__set__"),
            8 => matches!(name, "__call__" | "__bool__" | "__imod__" | "__iand__" | "__iadd__" | "__hash__" | "__iter__" | "__isub__" | "__ipow__" | "__init__" | "__imul__" | "__ixor__" | "__repr__" | "__rand__" | "__radd__" | "__next__" | "__rsub__" | "__rpow__" | "__rmul__" | "__rmod__" | "__rxor__"),
            9 => matches!(name, "__await__" | "__anext__" | "__aiter__" | "__float__" | "__index__"),
            10 => matches!(name, "__buffer__" | "__divmod__" | "__delete__" | "__invert__" | "__matmul__" | "__lshift__" | "__rshift__"),
            11 => matches!(name, "__delitem__" | "__delattr__" | "__imatmul__" | "__ilshift__" | "__getitem__" | "__irshift__" | "__rdivmod__" | "__rrshift__" | "__rmatmul__" | "__rlshift__" | "__truediv__" | "__setitem__" | "__setattr__"),
            12 => matches!(name, "__contains__" | "__floordiv__" | "__itruediv__" | "__rtruediv__"),
            13 => matches!(name, "__ifloordiv__" | "__rfloordiv__"),
            16 => matches!(name, "__getattribute__"),
            18 => matches!(name, "__release_buffer__"),
            _ => false,
        };
        if native_slot { return Some(("slot wrapper", "wrapper_descriptor")); }
        match kind {
            "type" if matches!(name, "__dict__" | "__mro__" | "__name__") => Some(("attribute", "getset_descriptor")),
            "function" if name == "__code__" => Some(("attribute", "getset_descriptor")),
            "function" if name == "__globals__" => Some(("member", "member_descriptor")),
            "dict" if name == "fromkeys" => Some(("method", "classmethod_descriptor")),
            "int" | "bool" | "float" if matches!(name, "real" | "imag" | "numerator" | "denominator") => Some(("attribute", "getset_descriptor")),
            "complex" if matches!(name, "real" | "imag") => Some(("member", "member_descriptor")),
            "range" | "slice" if matches!(name, "start" | "stop" | "step") => Some(("member", "member_descriptor")),
            _ => None,
        }
    }

    pub fn kind_it_names(&self) -> Option<String> {
        match self {
            Value::Blueprint(b) => Some(b.name.clone()),
            Value::KindOf(kind) => Some(Value::word_for_kind(*kind).to_owned()),
            Value::OctetKind { changeable, .. } => Some(String::from(if *changeable { "bytearray" } else { "bytes" })),
            Value::Intrinsic(op, word) if op.names_a_kind() => Some(word.to_string()),
            Value::Shared(cell) | Value::Mutable(cell, _) => cell.borrow().kind_it_names(),
            _ => None,
        }
    }

    /// Where the thing behind a worth is kept: the cell a collection
    /// lives in, else the place its own members or letters stand at.
    /// This is what the reference writes after a bound method's kind.
    /// Nothing for a worth kept in no place of its own.
    pub fn standing(&self) -> Option<usize> {
        Some(match self {
            Value::Shared(cell) | Value::Mutable(cell, _) => Rc::as_ptr(cell) as *const u8 as usize,
            Value::Vector(items) | Value::Tuple(items) | Value::Row(items) => Rc::as_ptr(items) as *const u8 as usize,
            Value::Dict(entries) => Rc::as_ptr(entries) as *const u8 as usize,
            Value::Set(members) => Rc::as_ptr(members) as *const u8 as usize,
            Value::Octets { cell, .. } => Rc::as_ptr(cell) as *const u8 as usize,
            Value::Text(letters) => letters.as_ptr() as usize,
            Value::Thing(thing) => Rc::as_ptr(thing) as *const u8 as usize,
            Value::Iterator(state) => Rc::as_ptr(state) as *const u8 as usize,
            _ => return None,
        })
    }
}

/// How a method of a builtin's own is written: the name it answers to,
/// and either the kind of the thing it was read from with where that
/// thing is kept, or, where it was read from the kind itself and so is
/// bound to nothing, the name of that kind. Where the thing is kept in
/// no place of its own, the method's own place stands for it.
fn method_written(subject: &Value, word: &str, elsewhere: usize) -> String {
    let held = subject.settled();
    if let Some(kind) = held.kind_it_names() { return format!("<method '{word}' of '{kind}' objects>"); }
    format!("<built-in method {word} of {} object at 0x{:x}>", held.kind_word(), subject.standing().unwrap_or(elsewhere))
}

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
pub struct TypeNames {
    pub short: Value,
    pub full: Value,
    pub module_key: String,
    /// Original qualification for the inherited string-based super lookup.
    pub declared: Value,
}

#[derive(Debug)]
pub struct Blueprint {
    /// The mutable names of a Python class, outside its dictionary.
    pub type_names: RefCell<Option<TypeNames>>,
    pub ancestry: RefCell<Vec<Rc<Blueprint>>>,
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
    /// Set from the declared layout before class-creation hooks run;
    /// subsequent edits to members do not add or remove weak storage.
    pub weak_slot: Cell<Option<bool>>,
    /// The seal builtin's mark, kept where no member write of the
    /// program's can reach it: the class takes no write to a member of
    /// it and stands as no class's base.
    pub sealed: Cell<bool>,
    /// Instance slot storage established when the class was constructed.
    pub has_slot_storage: bool,
    /// Whether a builder's own mro supplied the order the class keeps:
    /// the order is then complete, and no allocation link may add a
    /// forebear it left out.
    pub order_supplied: Cell<bool>,
    // A custom order is separate from layout parents. Other entries remain
    // alive through ancestry, while this blueprint refers to itself weakly.
    pub supplied_order: RefCell<Vec<std::rc::Weak<Blueprint>>>,
}

impl Blueprint {
    pub(super) fn python_title(&self) -> Option<String> {
        let kept = self.type_names.borrow();
        let names = kept.as_ref()?;
        let module = self.shared.borrow().iter().find(|entry| entry.0 == names.module_key).and_then(|entry| match entry.1.type_text() { text @ (Value::Text(_) | Value::Unpaired(_)) => Some(text.bare()), _ => None });
        Some(match module { Some(word) if word != "builtins" => format!("{word}.{}", names.full.type_text().bare()), _ => names.short.type_text().bare() })
    }
    /// A character-format complaint uses qualification, unlike repr's
    /// short-name fallback for builtin or non-string modules.
    pub(super) fn python_qualified_title(&self) -> Option<String> {
        let kept = self.type_names.borrow();
        let names = kept.as_ref()?;
        let module = self.shared.borrow().iter().find(|entry| entry.0 == names.module_key).and_then(|entry| match entry.1.type_text() { text @ (Value::Text(_) | Value::Unpaired(_)) => Some(text.bare()), _ => None });
        let full = names.full.type_text().bare();
        Some(match module { Some(word) if word != "builtins" => format!("{word}.{full}"), _ => full })
    }

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
        it
            || self.under.as_ref().map_or(false, |u| u.goes_by(name, either_way))
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
    /// Ask whether an inherited public field exists without constructing
    /// the complete instance layout or cloning its names and values.
    pub fn has_public_field(&self, name: &str) -> bool {
        let mut current = Some(self);
        while let Some(class) = current {
            if class.fields.iter().enumerate().any(|(at, (key, _))|
                key == name && class.reaches.get(at) != Some(&Reach::Alone)) { return true; }
            current = class.under.as_deref();
        }
        false
    }

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
impl Thing {
    pub fn blueprint(&self) -> Rc<Blueprint> {
        self.reclassified.borrow().clone().unwrap_or_else(|| self.of.clone())
    }
}

#[derive(Debug)]
pub struct Thing {
    pub reclassified: RefCell<Option<Rc<Blueprint>>>,
    pub of: Rc<Blueprint>,
    pub holds: RefCell<Vec<(String, Value)>>,
    /// Which thing this is by the turn it was made in, counting from
    /// one, for a language that names them when showing them.
    pub turn: usize,
}

impl Thing {
    /// The entries a program can see, as the dictionary of them: none
    /// that is unset, and none under a name no program can spell.
    pub fn entries_shown(&self) -> Value {
        let holds = self.holds.borrow();
        let mut pairs: Vec<(Value, Value)> = holds.iter()
            .filter(|(name, v)| !matches!(v, Value::Unset) && !name.starts_with('\0'))
            .map(|(name, v)| (Value::text(name), v.clone())).collect();
        // Keys of any other kind are kept beside the names under the
        // hidden name `\0keys` and shown with them.
        if let Some((_, Value::Dict(extra))) = holds.iter().find(|(name, _)| name == "\0keys") {
            pairs.extend(extra.iter().cloned());
        }
        Value::Dict(Rc::new(pairs.into()))
    }
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
        Some((above, beneath)) => Value::Frac(Rc::new(Ratio {
            float_style: false, above, beneath, places: Some(figures),
            under: x == 0.0 && x.is_sign_negative(), pointed: false,
        })),
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
    Value::Frac(Rc::new(Ratio { float_style: false, above, beneath: BigInt::zero(), places: Some(figures), under: x.is_nan() && x.is_sign_negative(), pointed: false }))
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
    // Return lowest terms directly, including subnormal mantissas.
    let cancelled = run.trailing_zeros();
    let (run, halvings) = (run >> cancelled, halvings + cancelled as i64);
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
                if r.past_numbers() { return v; }
                let mut worth = rounded_binary(&r.above, &r.beneath);
                if r.under && (worth == 0.0 || worth.is_nan()) { worth = -worth; }
                worth_of_binary(worth, figures).keeping_point(r.pointed)
            }
            _ => v,
        };
    }
    if bits.is_none() {
        return v;
    }
    let Value::Frac(e) = &v else { return v };
    if e.past_numbers() { return v; }
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
    /// The collections whose members are being written out at this
    /// moment, each by the place its members stand in. A fixed row
    /// carries no cell to know it by, and the walk that lets a thing
    /// say how it is written is given a collection's members and not
    /// the cell about them, so this note serves for both. The places
    /// are gathered in a set and not a list, so that asking whether one
    /// is already among them costs the same whether the writing stands
    /// one deep or a hundred thousand deep.
    static AMONG: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    /// Where a language names them, the cells a run keeps its counts of
    /// figures in: how many a real written plainly carries, and how many
    /// one shown with its kind carries. A language that names neither
    /// leaves both standing empty.
    static COUNTS: RefCell<(Option<Rc<RefCell<Value>>>, Option<Rc<RefCell<Value>>>)> = const { RefCell::new((None, None)) };
}

/// A note left on a collection's members for as long as they are being
/// written out. Where a note already lies on the very same members, the
/// collection has been reached from somewhere within itself, and
/// `instead` holds the marks to write in its place. The note is lifted
/// when it goes out of use, so a collection reached twice by two roads
/// is written whole on each of them.
pub struct Among {
    left: Option<usize>,
    pub instead: Option<&'static str>,
}

impl Among {
    /// A map is marked by its braces, a fixed row by the brackets a
    /// fixed row stands between, and any other row by its own. What is
    /// no collection takes no note, nothing else being able to hold
    /// itself.
    pub fn members(value: &Value) -> Among {
        let (address, marks) = match value {
            Value::Dict(pairs) => (Rc::as_ptr(pairs) as usize, "{...}"),
            Value::Tuple(parts) | Value::Row(parts) | Value::Arguments(parts) => (Rc::as_ptr(parts) as usize, "(...)"),
            Value::Vector(items) => (Rc::as_ptr(items) as usize, "[...]"),
            Value::Set(store) => (Rc::as_ptr(store) as usize, "(...)"),
            _ => return Among { left: None, instead: None },
        };
        AMONG.with(|notes| {
            if !notes.borrow_mut().insert(address) {
                return Among { left: None, instead: Some(marks) };
            }
            Among { left: Some(address), instead: None }
        })
    }
}

impl Drop for Among {
    fn drop(&mut self) {
        if let Some(address) = self.left {
            AMONG.with(|notes| { notes.borrow_mut().remove(&address); });
        }
    }
}

/// Write out a cell's contents by the writer given, or a row's ellipsis
/// where the same cell is already being written further out.
fn within_cell(cell: &Rc<RefCell<Value>>, writer: impl FnOnce(&Value) -> String) -> String {
    within_cell_marked(cell, "[...]", writer)
}

/// The same, where the caller says what the cell come round to again is
/// marked with: a map is marked by the braces it would have been written
/// in, and anything else by a row's brackets.
fn within_cell_marked(cell: &Rc<RefCell<Value>>, mark: &str, writer: impl FnOnce(&Value) -> String) -> String {
    let address = Rc::as_ptr(cell) as usize;
    let met_before = UNDERWAY.with(|stack| {
        let mut stack = stack.borrow_mut();
        let seen = stack.contains(&address);
        if !seen { stack.push(address); }
        seen
    });
    if met_before { return String::from(mark); }
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
        pieces.push(if (number == 39 && changing) || number == mark || number == 92 {
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
    let exact_dyadic = above.bits() < 54 && beneath.bits() < 1076
        && beneath.trailing_zeros().is_some_and(|shift| shift + 1 == beneath.bits());
    // No quotient or remainder is needed when all bits already fit the
    // binary format; whole numbers use its integer rounding directly.
    if exact_dyadic || beneath.is_one() { return nearest_binary(above, beneath); }
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

/// A thing on its way out tells the listeners, and one whose class bids
/// farewell is put back together so that the machine may bid it.
impl Drop for Thing {
    fn drop(&mut self) {
        crate::ghost::thing_departing(self);
    }
}

impl Drop for Blueprint {
    fn drop(&mut self) {
        crate::ghost::departing_at(self as *const Blueprint as usize);
    }
}

impl Drop for SetStore {
    fn drop(&mut self) {
        crate::ghost::departing_generic();
    }
}

#[cfg(test)]
mod namespace_tests {
    use super::*;

    #[test]
    fn namespace_names_follow_structural_writes_and_copy_on_write() {
        let mut original = MapStore::from(vec![(Value::text("first"), Value::Small(1)), (Value::text("second"), Value::Small(2))]);
        assert_eq!(original.namespace_position("second"), Ok(Some(1)));
        let mut copy = original.clone();
        original.remove(0);
        assert_eq!(original.namespace_position("second"), Ok(Some(0)));
        assert_eq!(copy.namespace_position("second"), Ok(Some(1)));
        copy.overwrite_namespace(1, Value::Small(9));
        assert!(matches!(copy[1].1, Value::Small(9)));
        assert!(matches!(original[0].1, Value::Small(2)));
        copy.insert_known_absent(Value::text("third"), "text:third".to_owned(), Value::Nil);
        assert_eq!(copy.namespace_position("third"), Ok(Some(2)));
    }

    #[test]
    fn mutable_namespace_keys_do_not_retain_a_spelling_index() {
        let key = Rc::new(RefCell::new(Value::text("before")));
        let pairs = MapStore::from(vec![(Value::Shared(key.clone()), Value::Small(4))]);
        assert_eq!(pairs.namespace_position("before"), Err(()));
        *key.borrow_mut() = Value::text("after");
        assert_eq!(pairs.namespace_position("before"), Err(()));
        assert_eq!(pairs.namespace_position("after"), Err(()));
    }
}
