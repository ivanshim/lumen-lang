// The seven forms, run.
//
// A constant is itself, a routine constant becoming a bound routine over
// the current environment. A read walks up the environments. A write
// walks up and stores. A call of a primitive applies the kernel's
// mechanics; a call of a routine value makes an environment and runs the
// body, or runs it in the caller's when the routine holds no names. A
// cycle loops in place; a dyad reads its two operands without a visit
// to a form and does machine integers in place; a bump steps a binding.
// A routine in tail position of another replaces it in the same native
// frame. Yield, leave and resume travel up as escapes until a routine
// that traps them stops them.

use std::collections::HashMap;
use std::cell::RefCell;
use num_bigint::BigInt;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::math::{self, Calc};
use crate::table::Table;
use crate::form::{Input, Traps, Form, Prim, Routine, Address, Callee};
use crate::data::{Blueprint, Env, Kind, Names, Reach, Thing, Value};

/// Which shell the host keeps, and the switch by which it is handed a
/// command spelled out instead of a file holding one.
const THE_HOSTS_SHELL: &str = "/bin/sh";
const A_COMMAND_FOLLOWS: &str = "-c";

/// The label under which a language spells each kind of complaint.
pub const COMPLAINT_LABELS: [(&str, &str); 4] = [
    ("warning", "ext.system.complaint.warning"),
    ("notice", "ext.system.complaint.notice"),
    ("deprecated", "ext.system.complaint.deprecated"),
    ("fatal", "ext.system.complaint.fatal"),
];

/// The label under which a language spells each kind of value.
pub const KIND_LABELS: [(&str, Kind); 7] = [
    ("system.kind.integer", Kind::Whole), ("system.kind.rational", Kind::Fraction), ("system.kind.real", Kind::Decimal),
    ("system.kind.string", Kind::Chars), ("system.kind.boolean", Kind::Truth), ("system.kind.array", Kind::Vector),
    ("system.kind.null", Kind::Nothing),
];

pub enum Escape {
    Error(String),
    /// The run is over and no clause may take it back: a limit the
    /// language set on the run itself was passed.
    Stopped(String),
    /// The program said the run was over. Nothing went amiss and nothing
    /// is told; whatever was to be written was written already.
    Done,
    Yield(Value),
    /// A value raised for a clause to take.
    Thrown(Value),
    /// break, out of so many loops.
    Leave(usize),
    /// continue, the next pass of the loop so many levels out.
    Resume(usize),
}

impl From<String> for Escape {
    fn from(s: String) -> Escape {
        Escape::Error(s)
    }
}

/// A call under way: what it called, where the call itself was
/// written, and where among the arguments kept the ones it was handed
/// stand.
struct Called {
    named: Rc<str>,
    within: Option<Rc<str>>,
    from: Rc<str>,
    on: u32,
    handed_at: Option<usize>,
    /// Whether what was called belongs to the language's own library,
    /// standing before the program's own text, and whether the call was
    /// made from within it. A call the library made has no line of the
    /// program's to name, and one it made of its own is no business of
    /// the program's at all.
    of_library: bool,
    from_library: bool,
}

/// How a place is being read: plainly, while a value is being taken
/// apart, or so that what comes of it may be written back there. Only
/// the last turns down a value with no places at all, there being no
/// place there to write.
#[derive(Clone, Copy, PartialEq)]
enum Reading {
    Plain,
    Apart,
    Toward,
}

type Res<T = Value> = Result<T, Escape>;

enum Next {
    Value(Value),
    /// The program to run next, its frame already built.
    Jump(Rc<Routine>, Rc<Env>),
}

pub struct Machine<'a> {
    pub library_sources: HashMap<String, String>,
    imported: HashMap<String, Value>,
    in_output_method: bool,
    table: &'a Table,
    pub outermost: Rc<Env>,
    idents: Vec<String>,
    memo: HashMap<String, Value>,
    args_cell: Option<usize>,
    memo_cell: Option<usize>,
    /// Whether the language can read what a call was handed. Where it
    /// can, a call may hand over more than a routine gives names to.
    reads_handed: bool,
    /// Whether every key of an array is a whole number or text, and
    /// whether being equal is the looser question.
    plain_keys: bool,
    loose_equals: bool,
    /// How many things have been made, so each carries its own turn.
    made: usize,
    /// The line of the source now running and the file it is written
    /// in, which a complaint names, and the line the last value raised
    /// was raised on.
    row: u32,
    holding_fault: Vec<Value>,
    fault_roots: HashMap<String, Value>,
    raised_on: u32,
    written_in: Rc<str>,
    /// Where the language keeps its own pages, as the run was
    /// started. Nothing where the run was started naming nowhere,
    /// and then a complaint about a word points at no page of it.
    pages_at: Option<String>,
    /// The calls under way, innermost last: what each called, where the
    /// call itself was written, and where among the arguments kept the
    /// ones it was handed stand.
    calls: Vec<Called>,
    /// The calls a fault was raised under, written out as they stood
    /// then, since by the time it is told they are all left behind. The
    /// first fault to leave a call writes this; a trap taking one
    /// wipes it again.
    under: Option<String>,
    /// Where the program a fault was raised on the way into is written.
    /// Such a fault belongs there and not where the call stood, which
    /// is worth saying only where nothing takes it.
    entering: Option<(Rc<str>, u32, bool)>,
    /// The words this language has for the kinds of complaint.
    complaint_words: Vec<(&'static str, String)>,
    /// How many seconds the run may take and when the count began;
    /// nought is no limit at all.
    allowed: usize,
    /// Every run raised alongside this one, filed under the
    /// number it was raised with so it may be laid to rest.
    alongside: HashMap<i64, std::process::Child>,
    started: Option<std::time::Instant>,
    /// How many bytes of room the run may take; nought is no mark at
    /// all. What it has taken is not written down here — that tally is
    /// the allocator's, kept for the whole process, and only read.
    ceiling: usize,
    /// How many pieces now being found asked to be quiet. Counted, not
    /// flagged, because a quiet piece may hold another.
    quieted: usize,
    /// The same for a piece the program silenced outright: nothing at
    /// all is said of it, not even a word about how it is written.
    silenced: usize,
    /// The class each call runs inside, innermost last: what a class
    /// holds alone is reached from there and from nowhere else.
    inside: Vec<Option<Rc<str>>>,
    /// What the run has written out while it was being kept rather than
    /// let go, innermost last. A keeping within a keeping writes into
    /// the one around it when it is given up.
    holding: RefCell<Vec<String>>,
    /// The routines to run once the program's last statement is done,
    /// each with what it is to be handed, in the order they were named.
    afterward: RefCell<Vec<(Value, Vec<Value>)>>,
    /// Every thing made, held loosely and in the order they were made,
    /// so that any still standing at the end can be let go.
    things: RefCell<Vec<std::rc::Weak<Thing>>>,
    /// The files already read where the program asked that they be read
    /// only once, under the whole name each stands by.
    read_before: RefCell<std::collections::HashSet<String>>,
    /// What a source read in got away with: the reading answers with a
    /// value or a note, so a throw or an ending is kept here and let
    /// go again where the reading stood.
    got_away: Option<Escape>,
    /// Text read in that would not be read: the words said of it, the
    /// place it counts as standing in, and the line of its own that the
    /// reading stopped on. The reading answers with a plain note, so
    /// this is set aside for the end of the run to name, and is trusted
    /// only for the very words it was set aside with.
    would_not_read: Option<(String, Rc<str>, u32)>,
    /// What the program already read declared about cells: which
    /// parameters take one and which routines hand one back. Text read
    /// while the run goes is a piece of the same program and is built
    /// knowing it.
    pub knows_cells: (HashMap<String, Vec<bool>>, HashMap<String, Vec<String>>, std::collections::HashSet<String>),
    /// The names of the frame each call is running in, innermost last,
    /// so that text read while the run goes can be built knowing them.
    frames_named: Vec<Rc<Routine>>,
    /// The routine every complaint is handed to, where the program has
    /// put one in the way of them; the complaints still to be handed
    /// over, since one may be raised where the run is only reading; and
    /// whether any wait, which each step asks before it runs.
    hearer: RefCell<Option<Value>>,
    /// The routine a value nobody took is handed to, where the program
    /// put one in its way.
    untaken: RefCell<Option<Value>>,
    /// Whether anything has gone out of the run yet. What is held back
    /// in a piece of output kept aside has not gone out.
    written_out: std::cell::Cell<bool>,
    unheard: RefCell<Vec<(String, String, u32)>>,
    any_unheard: std::cell::Cell<bool>,
    /// Whether writing into a place makes what is needed to hold it: an
    /// array where a name holds nothing, and one at each place along the
    /// way that is not there yet.
    builds_places: bool,
    /// Whether text is a row of places holding one letter each. Where it
    /// is, such a place takes a letter as well as giving one, and a place
    /// named by text is the number that text opens with.
    letter_places: bool,
    /// Whether a piece of text spelling the name of a routine or a class
    /// may stand where the routine or class itself would.
    spelled_stands: bool,
    /// Whether a class goes by its name however the name is written.
    classes_either_way: bool,
    /// Pieces of text this language holds untrue past text with nothing
    /// in it, and whether an array with nothing in it is untrue.
    false_words: Vec<String>,
    hollow_is_false: bool,
    /// What each call still running was handed, the innermost last, and
    /// what the call about to start is to be handed.
    handed: Vec<Vec<Value>>,
    pending: Vec<Value>,
}

fn ascend(frame: &Rc<Env>, depth: usize) -> &Rc<Env> {
    let mut f = frame;
    for _ in 0..depth {
        f = f.outer.as_ref().expect("a frame above");
    }
    f
}

impl<'a> Machine<'a> {
    pub fn new(table: &'a Table, idents: Vec<String>) -> Machine<'a> {
        let find = |key: &str| table.single(key).and_then(|n| idents.iter().position(|x| x == n));
        let mut fault_roots = HashMap::new();
        let words = table.strings("ext.system.fault.bases");
        let mut at = 0;
        while at + 1 < words.len() {
            let under = if let Some(Value::Blueprint(parent)) = fault_roots.get(&words[at + 1]) { Some(Rc::clone(parent)) } else { None };
            let name = words[at].clone();
            let shape = Blueprint {
                shared: RefCell::new(vec![("__name__".to_string(), Value::text(&name))]),
                constants: vec![], methods: vec![], reaches: vec![], fields: vec![], answers: vec![],
                under, name: name.clone(),
            };
            fault_roots.insert(name, Value::Blueprint(Rc::new(shape)));
            at += 2;
        }
        let outermost = Env::make(idents.len(), None);
        for (position, word) in idents.iter().enumerate() {
            if let Some(value) = fault_roots.get(word.trim_end_matches(crate::form::OF_A_CLASS)) {
                outermost.cells.borrow_mut()[position] = value.clone();
            }
        }
        Machine {
            library_sources: HashMap::new(),
            imported: HashMap::new(),
            in_output_method: false,
            table,
            outermost,
            fault_roots,
            args_cell: find("system.args"),
            memo_cell: find("system.memoization"),
            idents,
            memo: HashMap::new(),
            reads_handed: ["ext.builtin.args.all", "ext.builtin.args.count", "ext.builtin.args.at"]
                .iter()
                .any(|label| table.single(label).is_some()),
            handed: Vec::new(),
            pending: Vec::new(),
            made: 0,
            row: 0,
            holding_fault: Vec::new(),
            raised_on: 0,
            allowed: 0,
            alongside: HashMap::new(),
            started: None,
            ceiling: 0,
            written_in: Rc::from(""),
            pages_at: None,
            calls: Vec::new(),
            under: None,
            entering: None,
            quieted: 0,
            silenced: 0,
            inside: Vec::new(),
            holding: RefCell::new(Vec::new()),
            afterward: RefCell::new(Vec::new()),
            things: RefCell::new(Vec::new()),
            read_before: RefCell::new(std::collections::HashSet::new()),
            got_away: None,
            would_not_read: None,
            knows_cells: (HashMap::new(), HashMap::new(), std::collections::HashSet::new()),
            frames_named: Vec::new(),
            hearer: RefCell::new(None),
            untaken: RefCell::new(None),
            written_out: std::cell::Cell::new(false),
            unheard: RefCell::new(Vec::new()),
            any_unheard: std::cell::Cell::new(false),
            builds_places: table.flag("ext.op.index.makes"),
            letter_places: table.flag("ext.op.index.text"),
            spelled_stands: table.flag("ext.op.spelled"),
            classes_either_way: table.flag("ext.system.class.folded"),
            false_words: table.strings("ext.system.untrue.text").to_vec(),
            hollow_is_false: table.flag("ext.system.untrue.empty_array"),
            complaint_words: COMPLAINT_LABELS
                .iter()
                .filter_map(|(kind, key)| table.single(key).map(|word| (*kind, word.to_string())))
                .collect(),
            plain_keys: table.flag("ext.op.index.plain_keys"),
            // A language with a word for being the very same means
            // something looser by being equal.
            loose_equals: table.has_any("ext.op.identical") && !table.has_any("ext.op.identical.negated"),
        }
    }

    /// Where the program is written, which a complaint names.
    pub fn found_in(&mut self, place: &str) {
        self.written_in = Rc::from(place);
    }

    /// Where the language keeps its own pages, as the run was started.
    pub fn pages_lie_at(&mut self, held: Option<String>) {
        self.pages_at = held;
    }

    /// Say a complaint of this kind in the language's own word for it
    /// and carry on. A language with no word for the kind says nothing.
    /// The places of an array with their keys, a place's number standing
    /// for its key where the places carry none.
    fn places_with_keys(v: &Value) -> Vec<(Value, Value)> {
        match v {
            Value::Vector(items) => items.iter().enumerate().map(|(at, x)| (Value::Small(at as i64), x.clone())).collect(),
            Value::Dict(pairs) => pairs.as_ref().clone(),
            _ => Vec::new(),
        }
    }

    /// Whether the first of two values comes before the second, asked
    /// the loose way a language with a word for being the very same
    /// means it. A flag or nothing on either hand makes the question one
    /// of which is true; an array sits above whatever is not an array,
    /// and two arrays are set against each other by how much they hold
    /// and then place by place; text spelling a number stands for that
    /// number, and a number met by text spelling none is read as text.
    fn comes_first(&mut self, a: &Value, b: &Value) -> Result<bool, String> {
        let w = self.wording();
        let counts = |x: &Value| matches!(x.kind(), Some(Kind::Whole | Kind::Fraction | Kind::Decimal));
        let empty = |x: &Value| matches!(x, Value::Nil | Value::Unset);
        let gathered = |x: &Value| matches!(x, Value::Vector(_) | Value::Dict(_));
        // Two numbers are set against each other at the width the
        // language holds them in, as they are worked at it.
        let plainly = |x: &Value, y: &Value| -> Result<bool, String> {
            // The worth nothing is equal to comes before nothing and
            // after nothing, so it is out of the question before the
            // two are brought to the width at all.
            if math::no_order(x, y) {
                return Ok(false);
            }
            let (x, y) = match self.holds_reals_to_width() && (self.a_real(x) || self.a_real(y)) {
                true => (self.at_width(self.as_wide_real(x)), self.at_width(self.as_wide_real(y))),
                false => (x.clone(), y.clone()),
            };
            match math::below(&x, &y) {
                Some(r) => Ok(r),
                None => Ok(x.as_big()? < y.as_big()?),
            }
        };
        Ok(match (a, b) {
            (Value::Flag(_), _) | (_, Value::Flag(_)) => !self.stands_true(a) && self.stands_true(b),
            // Nothing set against text is text with nothing in it, and
            // nothing at all comes before that.
            (one, Value::Text(s)) if empty(one) => !s.is_empty(),
            (Value::Text(_), other) if empty(other) => false,
            (one, other) if empty(one) => self.stands_true(other),
            (_, other) if empty(other) => false,
            (x, y) if gathered(x) && gathered(y) => {
                let (mine, theirs) = (Self::places_with_keys(x), Self::places_with_keys(y));
                if mine.len() != theirs.len() {
                    return Ok(mine.len() < theirs.len());
                }
                for (key, held) in mine {
                    let Some((_, yours)) = theirs.iter().find(|(k, _)| k.equals(&key)) else {
                        // A place the other never names leaves the two
                        // past setting against each other, and neither
                        // comes before what it cannot be set against.
                        return Ok(false);
                    };
                    let yours = yours.clone();
                    if !self.prim(Prim::Eq, "equals", &[held.clone(), yours.clone()])?.is_true() {
                        return self.comes_first(&held, &yours);
                    }
                }
                false
            }
            (x, _) if gathered(x) => false,
            (_, y) if gathered(y) => true,
            (Value::Text(_), Value::Text(_)) => match (self.number_said(a), self.number_said(b)) {
                (Some(x), Some(y)) => plainly(&x, &y)?,
                _ => a.bare() < b.bare(),
            },
            (Value::Text(s), other) if counts(other) => match self.number_said(a) {
                Some(x) => plainly(&x, other)?,
                None => **s < *other.render(w),
            },
            (other, Value::Text(s)) if counts(other) => match self.number_said(b) {
                Some(y) => plainly(other, &y)?,
                None => *other.render(w) < **s,
            },
            _ => plainly(a, b)?,
        })
    }

    /// Whether a value stands as true in this language. Past nought and
    /// nothing, a language may hold an array with nothing in it untrue,
    /// and may name pieces of text it holds untrue.
    fn stands_true(&self, v: &Value) -> bool {
        match v {
            Value::Shared(cell) => self.stands_true(&cell.borrow()),
            Value::Vector(items) if self.hollow_is_false => !items.is_empty(),
            Value::Dict(pairs) if self.hollow_is_false => !pairs.is_empty(),
            Value::Text(s) => match self.false_words.iter().any(|w| w == s.as_ref()) {
                true => false,
                false => v.is_true(),
            },
            other => other.is_true(),
        }
    }

    /// The binding a piece of text calls. Where a language marks its
    /// variables, the mark belongs to the name and not to the text
    /// spelling it, so it goes back on.
    /// What a value stands for where a routine or a class is wanted.
    /// Where a language lets a name be worked out as the run goes, a
    /// piece of text spells one of the outermost bindings, and that
    /// binding is what stands there.
    /// Where a name written out stands for the class it names, text
    /// that names no class says so by name: the class wanted is the one
    /// there is none of, not a piece of text put where a class belongs.
    /// The steps of a walk that a thing may answer for itself. Each is
    /// asked of the thing where it is one, and counted through as an
    /// array is where it is not.
    fn walking(&mut self, op: &Prim, name: &str, v: &[Value]) -> Result<Value, Escape> {
        let n = |want: usize| match v.len() == want {
            true => Ok(()),
            false => Err(Escape::Error(format!("{}() expects {} argument(s), got {}", name, want, v.len()))),
        };
        Ok(match op {
            // A thing may be its own walk, or may hand another over to
            // be walked for it. Either way a walk begins here.
            Prim::Walked => {
                n(1)?;
                let mut walking = self.protocol_value(&v[0], 4, &[])?.unwrap_or_else(|| v[0].clone());
                // Asking a thing what it hands over runs a piece of the
                // program standing elsewhere. The walk is written where
                // it is written, and is spoken of as standing there, so
                // the row is put back after the asking.
                let row = self.row;
                let mut handed_by = None;
                while let Some((giver, further)) = self.walk_handed(&walking)? {
                    handed_by = Some(giver);
                    walking = further;
                }
                self.row = row;
                match self.walks_itself(&walking) {
                    Some(_) => {
                        self.walk_asked(&walking, self.table.single("ext.op.walk.rewind").map(str::to_string))?;
                    }
                    // What one thing hands over for another to walk has
                    // to be a walk in its own right.
                    None => match handed_by.as_deref().and_then(|giver| self.handed_no_walk(giver)) {
                        Some(said) => return Err(said.into()),
                        None => self.can_be_walked(&walking)?,
                    },
                }
                walking
            }
            Prim::AloneWalk => {
                n(1)?;
                let said = self.table.single("ext.op.walk.no_cell").map(str::to_string);
                match (self.walks_itself(&v[0]), said) {
                    (Some(_), Some(words)) => return Err(words.into()),
                    (Some(_), None) => {}
                    (None, _) => self.can_be_walked(&v[0])?,
                }
                Value::Nil
            }
            Prim::MoreYet => {
                n(2)?;
                match self.walk_asked(&v[0], self.table.single("ext.op.walk.more").map(str::to_string))? {
                    Some(answer) => Value::Flag(self.stands_true(&answer)),
                    None if matches!(&v[0], Value::Progression(_)) => {
                        let Value::Progression(walk) = &v[0] else { unreachable!() };
                        let count = walk.count();
                        Value::Flag(v[1].as_big().map_or(false, |place| place < count && place >= BigInt::from(0)))
                    }
                    None => {
                        let far = match &v[0] {
                            Value::Vector(items) => items.len(),
                            Value::Dict(pairs) => pairs.len(),
                            Value::Thing(thing) => thing.holds.borrow().len(),
                            _ => 0,
                        };
                        Value::Flag(as_index(&v[1]).map_or(false, |at| at < far))
                    }
                }
            }
            Prim::AtHand | Prim::NamedHere => {
                let names = matches!(op, Prim::NamedHere);
                let asked = match names {
                    true => "ext.op.walk.key",
                    false => "ext.op.walk.this",
                };
                match self.walk_asked(&v[0], self.table.single(asked).map(str::to_string))? {
                    Some(answer) => answer,
                    None => self.prim(if names { Prim::KeyAt } else { Prim::ItemAt }, name, v).map_err(Escape::Error)?,
                }
            }
            Prim::StepOn => {
                n(1)?;
                self.walk_asked(&v[0], self.table.single("ext.op.walk.onward").map(str::to_string))?;
                Value::Nil
            }
            // Where the item handed out has got to, and one place past
            // it. A thing keeps its members where they are while it is
            // walked, so a walk over one goes on counting.
            Prim::PastHeld => {
                n(3)?;
                let stood = as_index(&v[1])?;
                let gone_on = match (&v[0], &v[2]) {
                    (Value::Thing(_), _) => stood + 1,
                    (walked, Value::Shared(cell)) => match found_at(walked, cell, stood) {
                        Some(now) => now + 1,
                        // The item is out of the array, so what came
                        // after it stands where it stood, and that is
                        // where the walk takes up.
                        None => stood,
                    },
                    _ => stood + 1,
                };
                Value::Small(gone_on as i64)
            }
            _ => return Err(Escape::Error(format!("{}() is no step of a walk", name))),
        })
    }

    /// A value with no places at all cannot be walked. A language with
    /// a word for a warning is told so and walks it no times, instead of
    /// having the run stopped over it.
    fn can_be_walked(&mut self, x: &Value) -> Result<(), Escape> {
        if matches!(x, Value::Vector(_) | Value::Dict(_) | Value::Thing(_) | Value::Progression(_)) {
            return Ok(());
        }
        if !self.complaint_words.iter().any(|(k, _)| *k == "warning") {
            return Err(Escape::Error("Cannot walk a value that is not an array".to_string()));
        }
        let told = format!("foreach() argument must be of type array|object, {} given", self.kind_called(x));
        self.grumble("warning", &told);
        Ok(())
    }

    /// The bits of a value, with a word said where a real is too wide
    /// for the whole numbers this language holds, so that working on its
    /// bits means working on something else. A language with no word for
    /// a warning says nothing and works on it just the same.
    fn bits_told(&self, x: &Value) -> Result<i64, String> {
        let bits = sixty_four(x)?;
        if !self.complaint_words.iter().any(|(k, _)| *k == "warning") {
            return Ok(bits);
        }
        if let Value::Frac(_) = x {
            let Some(exact) = math::ratio_of(x) else { return Ok(bits) };
            // The nearest real of the width is what such a language
            // holds, so whether that can be held as a whole number is
            // the question, not whether the exact ratio can.
            let near = crate::data::nearest_binary(&exact.above, &exact.beneath);
            let widest = 9223372036854775808.0f64;
            if !(near >= -widest && near < widest) {
                let told = format!(
                    "The float {} is not representable as an int, cast occurred",
                    crate::data::figured(near, None)
                );
                self.grumble("warning", &told);
            }
        }
        Ok(bits)
    }

    /// The class the run stands inside, where it stands inside one.
    fn standing_in(&self) -> Option<&str> {
        self.inside.last().and_then(|named| named.as_deref())
    }

    /// Where a thing holds a property of that name, reached from where
    /// the run stands: a class holding one alone files it under its own
    /// name and the class's together, so a program written in that class
    /// finds it there and every other finds the open one.
    fn member_place(&self, holds: &[(String, Value)], called: &str) -> Option<usize> {
        if let Some(here) = self.standing_in() {
            let alone = crate::data::held_alone(called, here);
            if let Some(at) = holds.iter().position(|(k, x)| *k == alone && kept(x)) {
                return Some(at);
            }
        }
        holds.iter().position(|(k, x)| k == called && kept(x))
    }

    /// Whether a class shares what it holds for itself and those along
    /// its line with the class the run stands in: the two lie along one
    /// line when either is built on the other.
    fn along_with(&self, holder: &str, here: Option<&str>) -> bool {
        let Some(here) = here else { return false };
        if here == holder {
            return true;
        }
        let built_on = |below: &str, above: &str| match self.class_bound(below) {
            Some(Value::Blueprint(c)) => c.goes_by(above, self.classes_either_way),
            _ => false,
        };
        built_on(here, holder) || built_on(holder, here)
    }

    /// The thing that is its own walk, where this value is one.
    fn walks_itself(&self, x: &Value) -> Option<Rc<Thing>> {
        let class = self.table.single("ext.op.walk.class")?;
        match x {
            Value::Thing(thing) if thing.of.goes_by(class, self.classes_either_way) => Some(thing.clone()),
            _ => None,
        }
    }

    /// What a thing hands over to be walked for it, where it is a thing
    /// that hands one over and what comes back is not the thing itself.
    fn walk_handed(&mut self, x: &Value) -> Result<Option<(String, Value)>, Escape> {
        let (Some(class), Some(gives)) = (self.table.single("ext.op.walk.giver.class"), self.table.single("ext.op.walk.giver")) else {
            return Ok(None);
        };
        let Value::Thing(thing) = x else { return Ok(None) };
        if !thing.of.goes_by(class, self.classes_either_way) {
            return Ok(None);
        }
        let Some(program) = thing.of.program(gives).cloned() else { return Ok(None) };
        let handed = self.invoke(program, self.outermost.clone(), vec![x.clone()])?;
        let itself = matches!((&handed, x), (Value::Thing(a), Value::Thing(b)) if Rc::ptr_eq(a, b));
        Ok(match itself {
            true => None,
            false => Some((thing.of.name.clone(), handed)),
        })
    }

    /// What is said where a thing hands over, to be walked for it,
    /// something that is no walk. Nothing where the definition has no
    /// words for it, and the value is walked as best it may be.
    fn handed_no_walk(&self, giver: &str) -> Option<String> {
        let said = self.table.strings("ext.op.walk.giver.unwalkable");
        let gives = self.table.single("ext.op.walk.giver")?;
        match said {
            [before, after] => Some(format!("{} {}::{}() {}", before, giver, gives, after)),
            _ => None,
        }
    }

    /// Ask a thing that is its own walk one of the walk's questions.
    /// Nothing where it is no such thing, so that the caller counts it
    /// through instead.
    fn walk_asked(&mut self, x: &Value, named: Option<String>) -> Result<Option<Value>, Escape> {
        let (Some(thing), Some(named)) = (self.walks_itself(x), named) else { return Ok(None) };
        let Some(program) = thing.of.program(&named).cloned() else { return Ok(None) };
        Ok(Some(self.invoke(program, self.outermost.clone(), vec![x.clone()])?))
    }

    fn class_lacking(&self, x: &Value) -> Option<String> {
        if !self.spelled_stands {
            return None;
        }
        match x {
            Value::Text(written) => Some(format!("Class \"{}\" not found", written)),
            _ => None,
        }
    }

    /// How a language names using a value with no places at all as
    /// though it had them.
    fn no_places(&self) -> String {
        match self.table.single("ext.op.index.scalar") {
            Some(said) => said.to_string(),
            None => "Cannot index non-array value".to_string(),
        }
    }

    fn what_it_spells(&self, v: Value) -> Value {
        let Value::Text(name) = &v else { return v };
        if !self.spelled_stands {
            return v;
        }
        // A routine and a class may go by one name. Standing where
        // either would do, the routine is meant: a class named by a
        // value stands where only a class will do, and is sought there.
        match self.lookup(name) {
            Some(found @ (Value::Routine(_) | Value::Bound(..))) => found,
            _ => self.class_bound(name).unwrap_or(v),
        }
    }

    /// The same, where only a class will do: `new $c`, `$c::m()` and the
    /// rest, which a routine of that name is no answer to.
    fn class_it_spells(&self, v: Value) -> Value {
        let Value::Text(name) = &v else { return v };
        if !self.spelled_stands {
            return v;
        }
        self.class_bound(name).unwrap_or(v)
    }

    /// The class of that name. Where classes go by their names however
    /// the names are written, a name nothing stands under is asked for
    /// again with its letters written small.
    fn class_bound(&self, name: &str) -> Option<Value> {
        if self.fault_roots.contains_key(name) { return self.fault_roots.get(name).cloned(); }
        let filed = format!("{}{}", name, crate::form::OF_A_CLASS);
        if let found @ Some(_) = self.lookup(&filed) {
            return found;
        }
        match self.classes_either_way {
            true => self.lookup(&format!("{}{}", name.to_lowercase(), crate::form::OF_A_CLASS)),
            false => None,
        }
    }

    fn name_it_spells(&self, spelled: &Value) -> String {
        let w = self.wording();
        let said = spelled.render(w).to_string();
        match self.table.letter("identifier.variable_prefix") {
            Some(mark) if !said.starts_with(mark) => format!("{}{}", mark, said),
            _ => said,
        }
    }

    /// Where among the outermost bindings that name stands, making room
    /// for it if it is a name nothing has stood under yet.
    fn place_called(&mut self, name: &str) -> usize {
        match self.idents.iter().position(|n| n == name) {
            Some(at) => at,
            None => {
                self.idents.push(name.to_string());
                self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
                self.idents.len() - 1
            }
        }
    }

    /// What a value is worth as a number: text for the number it opens
    /// with, a flag for one or nought, nothing for nought, and an array
    /// for whether it holds anything at all.
    fn worth_of(&self, v: &Value) -> Value {
        match v {
            Value::Text(_) => number_opening_in(v).0.unwrap_or(Value::Small(0)),
            Value::Flag(yes) => Value::Small(i64::from(*yes)),
            Value::Nil | Value::Unset => Value::Small(0),
            Value::Vector(_) | Value::Dict(_) => Value::Small(i64::from(self.stands_true(v))),
            held => held.clone(),
        }
    }

    /// Everything the run writes out passes through here, so a language
    /// able to keep its own output has one place that keeps it.
    fn utter(&self, text: &str) {
        let mut holding = self.holding.borrow_mut();
        match holding.last_mut() {
            Some(kept) => kept.push_str(text),
            None => {
                drop(holding);
                if !text.is_empty() {
                    self.written_out.set(true);
                }
                self.put_out(text);
            }
        }
    }

    /// Text sent where the run puts what it writes. A language holding
    /// text as bytes means every character of it stands for a byte, and
    /// what goes out is those bytes and no encoding of them; otherwise
    /// the letters go out spelled as letters are.
    fn put_out(&self, text: &str) {
        match self.table.flag("ext.system.text.bytes") {
            false => print!("{}", text),
            true => {
                use std::io::Write as _;
                let raw = text.chars().fold(Vec::with_capacity(text.len()), |mut so_far, c| {
                    so_far.push(c as u32 as u8);
                    so_far
                });
                let _ = std::io::stdout().lock().write_all(&raw);
            }
        }
    }

    /// The routines named to run once the program is done, in the order
    /// they were named. One that raises something stops the rest, as a
    /// fault anywhere else does.
    pub fn run_afterward(&mut self) -> Result<(), String> {
        // The clock the run was timed against starts afresh: what a
        // program named to run afterward runs even where the run was
        // stopped for taking too long, and is given the whole of the
        // time the run was allowed rather than what was left of it.
        if self.allowed > 0 {
            self.started = Some(std::time::Instant::now());
        }
        loop {
            let next = {
                let mut waiting = self.afterward.borrow_mut();
                if waiting.is_empty() {
                    return Ok(());
                }
                waiting.remove(0)
            };
            let (work, given) = next;
            if let Value::Bound(p, env) = self.what_it_spells(work) {
                if let Err(over) = self.invoke(p, env, given) {
                    // What a program named to run afterward may itself
                    // be stopped, and that is told as the ending of the
                    // run it was named by is told.
                    return Err(match over {
                        Escape::Error(m) => m,
                        Escape::Stopped(told) => {
                            if let Some((_, word)) = self.complaint_words.iter().find(|(k, _)| *k == "fatal") {
                                let head = complaint_head(&self.table, word, &told);
                                self.utter(&format!("{head}{}", complaint_tail(&self.table, &self.written_in, self.row)));
                            }
                            told
                        }
                        _ => String::new(),
                    });
                }
            }
        }
    }

    /// Each thing still standing at the end is let go by the method its
    /// class names for that, in the order the things were made. One made
    /// while another is being let go is let go in its own turn.
    pub fn let_things_go(&mut self) {
        let Some(named) = self.table.single("ext.stmt.class.destructor").map(str::to_string) else { return };
        let mut reached = 0;
        while let Some(loosely) = { let all = self.things.borrow(); all.get(reached).cloned() } {
            reached += 1;
            let Some(thing) = loosely.upgrade() else { continue };
            let Some(program) = thing.of.program(&named).cloned() else { continue };
            let _ = self.invoke(program, self.outermost.clone(), vec![Value::Thing(thing)]);
        }
        self.things.borrow_mut().clear();
    }

    /// Whatever is still being kept when the run ends is let go,
    /// outermost last, as a language that keeps its output does.
    pub fn let_go_all(&self) {
        loop {
            let held = self.holding.borrow_mut().pop();
            match held {
                Some(text) => self.utter(&text),
                None => break,
            }
        }
    }

    fn grumble(&self, kind: &str, about: &str) {
        if self.quieted > 0 || self.silenced > 0 {
            return;
        }
        self.said_regardless(kind, about);
    }

    /// The same, said even where the run keeps quiet about what is not
    /// there: a word about how a program is written is no word about
    /// what the run found, and only a piece silenced outright holds it
    /// back.
    fn said_regardless(&self, kind: &str, about: &str) {
        if self.silenced > 0 {
            return;
        }
        // A program may put a routine in the way of every complaint. One
        // is often raised where the run is only reading and cannot reach
        // back into the program, so it waits here and is handed over
        // before the next step runs.
        if self.hearer.borrow().is_some() {
            self.unheard.borrow_mut().push((kind.to_string(), about.to_string(), self.row));
            self.any_unheard.set(true);
            return;
        }
        self.said_plainly(kind, about, self.row);
    }

    fn said_plainly(&self, kind: &str, about: &str, row: u32) {
        let Some((_, word)) = self.complaint_words.iter().find(|(k, _)| *k == kind) else { return };
        let head = complaint_head(&self.table, word, about);
        self.utter(&format!("{head}{}", complaint_tail(&self.table, &self.written_in, row)));
    }

    /// Where a word of the language has its page, written into a
    /// complaint about that word. Nothing at all where the run knows
    /// nowhere the pages are kept, or where it is not dressing its
    /// complaints for a reader with any way of following an address.
    fn word_page(&self, word: &str) -> String {
        let Some(kept) = self.pages_at.as_deref() else { return String::new() };
        let [opens, joins, closes, ..] = self.table.strings("ext.system.complaint.markup.reference") else { return String::new() };
        let Some((ahead, behind)) = self.table.around("ext.system.complaint.reference.page") else { return String::new() };
        let spelled = match self.table.around("ext.system.complaint.reference.mark") {
            Some((mark, instead)) => word.replace(mark, instead),
            None => word.to_string(),
        };
        let page = format!("{ahead}{spelled}{behind}");
        format!("{opens}{kept}{page}{joins}{page}{closes}")
    }

    /// The complaints still waiting go to the routine the program put in
    /// their way, oldest first. One that answers false is left to be
    /// written out as it would have been.
    fn hand_over_unheard(&mut self) -> Res<()> {
        self.any_unheard.set(false);
        loop {
            let next = {
                let mut unheard = self.unheard.borrow_mut();
                if unheard.is_empty() {
                    return Ok(());
                }
                unheard.remove(0)
            };
            let (kind, about, row) = next;
            let hook = self.hearer.borrow().clone();
            let Some(Value::Bound(p, env)) = hook.map(|v| self.what_it_spells(v)) else {
                self.said_plainly(&kind, &about, row);
                continue;
            };
            let word = self.complaint_words.iter().find(|(k, _)| *k == kind).map(|(_, w)| w.clone());
            let told = vec![
                Value::text(&word.unwrap_or_default()),
                Value::text(&about),
                Value::text(&self.written_in),
                Value::Small(row as i64),
            ];
            if !self.invoke(p, env, told)?.is_true() {
                self.said_plainly(&kind, &about, row);
            }
        }
    }

    pub fn define(&mut self, name: &str, value: Value) {
        if let Some(i) = self.idents.iter().position(|n| n == name) {
            self.outermost.cells.borrow_mut()[i] = value;
        }
    }

    pub fn lookup(&self, name: &str) -> Option<Value> {
        let i = self.idents.iter().position(|n| n == name)?;
        match &self.outermost.cells.borrow()[i] {
            Value::Unset => None,
            v => Some(v.clone()),
        }
    }

    fn a_real(&self, v: &Value) -> bool {
        matches!(v, Value::Frac(_))
    }

    fn holds_reals_to_width(&self) -> bool {
        self.table.count("ext.system.real.bits").is_some()
    }

    fn real_figures(&self) -> usize {
        self.table.count("ext.system.real.digits").unwrap_or(math::DEFAULT_PLACES)
    }

    /// Whether two numbers are the one number at the width the language
    /// holds its reals in. Where it holds none, or neither is a real,
    /// they are one only where they are exactly one.
    fn alike_at_width(&self, a: &Value, b: &Value) -> bool {
        if !self.holds_reals_to_width() || !(self.a_real(a) || self.a_real(b)) {
            return a.equals(b);
        }
        self.at_width(self.as_wide_real(a)).equals(&self.at_width(self.as_wide_real(b)))
    }

    /// A number as a real of the language's own width.
    fn as_wide_real(&self, v: &Value) -> Value {
        match math::ratio_of(v) {
            Some(r) => math::make_number(r.above, r.beneath, Some(self.real_figures())),
            None => v.clone(),
        }
    }

    /// A number brought within the widths the language holds numbers
    /// in: a whole number too wide to be one becomes a real, and a real
    /// is brought to the nearest one of its width.
    fn at_width(&self, v: Value) -> Value {
        if let (Some(bits), Value::Huge(n)) = (self.table.count("ext.system.integer.bits"), &v) {
            if n.bits() >= bits as u64 {
                return math::make_number((**n).clone(), BigInt::from(1), Some(self.real_figures()));
            }
        }
        crate::data::at_binary_width(v, self.table.count("ext.system.real.bits"), self.real_figures())
    }

    /// The number a piece of text says, brought to the width the
    /// language holds its numbers in. Without that, a number written
    /// out and the same number said in text would be told apart, one
    /// having been brought to the width and the other not.
    fn number_said(&self, v: &Value) -> Option<Value> {
        number_spelled_in(v).map(|n| self.at_width(n))
    }

    fn quoted_remainder(&self, item: &Value) -> Result<String, String> {
        match item {
            Value::Vector(elements) => {
                let mut shown = Vec::new();
                for element in elements.iter() { shown.push(self.quoted_remainder(element)?); }
                return Ok(format!("[{}]", shown.join(", ")));
            }
            Value::Dict(entries) => {
                let mut shown = Vec::new();
                for (key, value) in entries.iter() {
                    let key = self.quoted_remainder(key)?;
                    let value = self.quoted_remainder(value)?;
                    shown.push(format!("{}: {}", key, value));
                }
                return Ok(format!("{{{}}}", shown.join(", ")));
            }
            Value::Frac(n) if n.places.is_some() => {
                let mut shown = item.render(self.wording());
                if !n.past_numbers() && !shown.chars().any(|c| matches!(c, '.' | 'e' | 'E')) { shown += ".0"; }
                return Ok(shown);
            }
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Nil | Value::Ellipsis => return Ok(item.render(self.wording())),
            Value::Text(_) => {}
            _ => return Err(self.table.single("ext.op.rem.format.unsupported").unwrap_or_default().to_string()),
        }
        let Value::Text(text) = item else { unreachable!() };
        let delimiter = if !text.contains('"') && text.contains('\'') { '"' } else { '\'' };
        let middle: String = text.chars().map(|letter| match letter {
            '\\' => "\\\\".to_string(),
            '\t' => "\\t".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            c if c == delimiter => format!("\\{}", c),
            c if c.is_control() => format!("\\x{:02x}", c as u32),
            c => c.to_string(),
        }).collect();
        Ok(format!("{}{}{}", delimiter, middle, delimiter))
    }

    fn text_remainder(&self, pattern: &str, rhs: &Value) -> Result<String, String> {
        let unsupported = self.table.single("ext.op.rem.format.unsupported").unwrap_or_default();
        let mismatch = self.table.single("ext.op.rem.format.arguments").unwrap_or_default();
        let supplied = match rhs { Value::Vector(list) => list.as_slice(), _ => std::slice::from_ref(rhs) };
        let mut arguments = supplied.iter();
        let letters: Vec<char> = pattern.chars().collect();
        let mut at = 0;
        let mut result = String::new();
        while at < letters.len() {
            let letter = letters[at];
            at += 1;
            if letter != '%' { result.push(letter); continue; }
            if letters.get(at) == Some(&'%') { result.push('%'); at += 1; continue; }
            let mut places = 6usize;
            let specified = letters.get(at) == Some(&'.');
            if specified {
                at += 1;
                let start = at;
                while letters.get(at).map_or(false, char::is_ascii_digit) { at += 1; }
                places = letters[start..at].iter().collect::<String>().parse().map_err(|_| unsupported.to_string())?;
                if places > 10000 { return Err(unsupported.to_string()); }
            }
            let code = *letters.get(at).ok_or_else(|| unsupported.to_string())?;
            at += 1;
            if specified && code != 'f' { return Err(unsupported.to_string()); }
            let worth = arguments.next().ok_or_else(|| mismatch.to_string())?;
            match code {
                'r' => result.push_str(&self.quoted_remainder(worth)?),
                's' => {
                    let text = match worth { Value::Text(s) => s.to_string(), other => self.quoted_remainder(other)? };
                    result.push_str(&text);
                }
                'x' | 'd' => {
                    let permitted = matches!(worth, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) || (code == 'd' && matches!(worth, Value::Frac(n) if n.places.is_some()));
                    if !permitted { return Err(mismatch.to_string()); }
                    let n = worth.as_big().map_err(|_| mismatch.to_string())?;
                    result.push_str(&n.to_str_radix(if code == 'x' { 16 } else { 10 }));
                }
                'f' => {
                    if !matches!(worth, Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Flag(_)) { return Err(mismatch.to_string()); }
                    let binary = match worth {
                        Value::Flag(truth) => if *truth { 1.0 } else { 0.0 },
                        other => {
                            let ratio = math::ratio_of(other).ok_or_else(|| mismatch.to_string())?;
                            crate::data::nearest_binary(&ratio.above, &ratio.beneath)
                        }
                    };
                    result.push_str(&format!("{:.1$}", binary, places));
                }
                _ => return Err(unsupported.to_string()),
            }
        }
        if arguments.next().is_some() { return Err(mismatch.to_string()); }
        Ok(result)
    }

    fn wording(&self) -> Names<'a> {
        Names {
            truth: self.table.single("literal.true").unwrap_or("true"),
            falsity: self.table.single("literal.false").unwrap_or("false"),
            flag_counted: self.table.flag("system.flag.counts"),
            // A language may show nothing as no text at all, as PHP does,
            // rather than as the word a program writes for it.
            real_figures: self.table.count("ext.system.real.bits").and(self.table.count("ext.system.real.digits")),
            nil: match self.table.flag("literal.null.silent") {
                true => "",
                false => self.table.single("literal.null").unwrap_or("null"),
            },
            within_word: self.table.single("ext.stmt.class.guarded"),
            alone_word: self.table.single("ext.stmt.class.hidden"),
            kept_as_bytes: self.table.flag("ext.system.text.bytes"),
        }
    }

    /// A fault of the kernel's own as a value of the class the language
    /// names for one. Nothing where it names none, or where the class
    /// itself is nowhere to be found.
    /// The class a fault of the kernel's own goes under. A language may
    /// name one for a fault of a kind, and this kernel knows which of
    /// its own faults are of which kind, being the one that words them.
    /// Where the language names none for the kind, the plain class does.
    fn class_of_fault(&self, told: &str) -> Option<String> {
        for name in self.fault_roots.keys() {
            if told.starts_with(&format!("{name}: ")) { return Some(name.clone()); }
        }
        let told_of = |label: &str| self.table.single(label) == Some(told);
        let by_kind = match told {
            _ if told.starts_with("Undefined array key ") => Some("ext.system.fault.class.key"),
            // Words the definition itself gave for a place outside the
            // range a value may take are known by being those very words.
            _ if told_of("ext.builtin.args.at.below") || told_of("ext.builtin.args.at.beyond") => Some("ext.system.fault.class.value"),
            // So too the words for a name spelling one of a class's own
            // values where the name of a constant was wanted.
            _ if told_of("ext.builtin.define.class_constant") => Some("ext.system.fault.class.value"),
            // Words a definition gave for the remainder by nought and
            // for a shift below nought are known by being those very
            // words, each a fault of the kind the kernel words itself.
            _ if told_of("ext.system.fault.modulo") => Some("ext.system.fault.class.division"),
            _ if told_of("ext.system.fault.shift") => Some("ext.system.fault.class.arithmetic"),
            _ if told.starts_with("Division by zero") => Some("ext.system.fault.class.division"),
            _ if told.starts_with("Bit shift by") || told_of("ext.system.fault.shift") => Some("ext.system.fault.class.arithmetic"),
            _ if told.starts_with("Cannot coerce") => Some("ext.system.fault.class.kind"),
            // An argument that is not of the class its parameter takes.
            _ if told.contains(" must be of type ") => Some("ext.system.fault.class.kind"),
            // Words the definition gave for an operand that can take no
            // part are known by the message opening with them.
            _ if self.table.single("ext.system.fault.operands").map_or(false, |w| told.starts_with(w)) => {
                Some("ext.system.fault.class.kind")
            }
            // So too the words for what was handed over to be walked.
            _ if self.table.strings("ext.op.walk.giver.unwalkable").first().map_or(false, |w| told.starts_with(w.as_str())) => {
                Some("ext.system.fault.class.walk")
            }
            // A program that would not be read is known the same way:
            // by the reading opening with words the definition gave.
            _ if self.would_not_read_words(told) => Some("ext.system.fault.class.reading"),
            _ => None,
        };
        by_kind
            .and_then(|label| self.table.single(label))
            .or_else(|| self.table.single("ext.system.fault.class"))
            .map(str::to_string)
    }

    /// Whether these words are a reading that stopped: they open with
    /// one of the three openings the definition gives for one.
    fn would_not_read_words(&self, told: &str) -> bool {
        let opening = |key: &str| self.table.around(key).map(|(head, _)| head);
        [self.table.single("ext.system.reading.unexpected"), opening("ext.system.reading.unclosed"), opening("ext.system.reading.unmatched")]
            .into_iter()
            .flatten()
            .any(|head| told.starts_with(head))
    }

    fn as_raised(&mut self, told: &str) -> Option<Value> {
        let named = self.class_of_fault(told)?;
        let class = self.class_bound(&named).or_else(|| {
            if self.table.flag("ext.stmt.class.this.explicit") { self.lookup(&named) } else { None }
        });
        let Some(Value::Blueprint(of)) = class else { return None };
        self.made += 1;
        let mut holds = of.every_field();
        // A fault of the kernel's own carries the words said and the
        // place in the program they were said of.
        let carried = [
            ("message", Value::text(match told { "Division by zero" => self.table.single("ext.system.fault.division").unwrap_or(told), _ => told })),
            ("file", Value::text(&self.written_in)),
            ("line", Value::Small(self.row as i64)),
        ];
        for (key, value) in carried {
            match holds.iter_mut().find(|(k, _)| k == key) {
                Some(place) => place.1 = value,
                None => holds.push((key.to_string(), value)),
            }
        }
        Some(Value::Thing(Rc::new(Thing { of, holds: RefCell::new(holds), turn: self.made })))
    }

    /// What a complaint calls a value where it names its kind. A flag
    /// is called by the word a program writes for it, since that is
    /// what was written; everything else by its kind, in the shorter
    /// form where the language gives one, a lone dash saying it gives
    /// none. Nothing is called by its kind too, since a language may
    /// write it as no word at all.
    /// How an operation is written in this language. Where more than
    /// one spelling stands for the same, a complaint names the shortest,
    /// and the first in order among those.
    fn written_as(&self, op: &Prim) -> String {
        let alike = |one: &Prim| std::mem::discriminant(one) == std::mem::discriminant(op);
        let mut ways: Vec<&str> = self
            .table
            .dyadic
            .iter()
            .filter(|(_, infix)| alike(&infix.prim))
            .map(|(lex, _)| lex.as_str())
            .collect();
        ways.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        ways.first().map_or_else(|| "?".to_string(), |lex| (*lex).to_string())
    }

    fn kind_called(&self, v: &Value) -> String {
        if let Value::Flag(_) = v {
            // The word, whatever a flag becomes where text is wanted: a
            // complaint names what was written, not what it counts as.
            let mut w = self.wording();
            w.flag_counted = false;
            return v.render(w);
        }
        let Some(kind) = v.kind() else { return "value".to_string() };
        let order = [Kind::Whole, Kind::Fraction, Kind::Decimal, Kind::Chars, Kind::Truth, Kind::Vector, Kind::Nothing];
        let brief = order
            .iter()
            .position(|k| *k == kind)
            .and_then(|at| self.table.strings("ext.system.kind.brief").get(at))
            .filter(|word| *word != "-");
        match brief {
            Some(word) => word.clone(),
            None => KIND_LABELS
                .iter()
                .find(|(_, k)| *k == kind)
                .and_then(|(label, _)| self.table.single(label))
                .unwrap_or("value")
                .to_string(),
        }
    }

    /// Whether the run has taken longer than the language allowed it.
    fn past_its_time(&self) -> Option<Escape> {
        let started = self.started?;
        if self.allowed == 0 || started.elapsed().as_secs() < self.allowed as u64 {
            return None;
        }
        let ending = if self.allowed == 1 { "second" } else { "seconds" };
        Some(Escape::Stopped(format!("Maximum execution time of {} {} exceeded", self.allowed, ending)))
    }

    /// Whether the run has taken more of the host's room than the
    /// language allowed it. The allocator's tally is right to the byte
    /// at every moment, but it is looked at where the clock is looked
    /// at, since looking costs more than the work between two looks.
    /// A run may therefore stand a little past the mark before it is
    /// stopped, and one value swollen past the mark in a single step
    /// is not caught until that step has finished.
    fn past_its_room(&mut self) -> Option<Escape> {
        if self.ceiling == 0 {
            return None;
        }
        let held = lumen_room::used();
        if held <= self.ceiling {
            return None;
        }
        let mark = self.ceiling;
        // Nothing may go on being kept back: there is no room to keep
        // it in, nor to write it out with, and the words ending the run
        // would fall into the keeping and come out behind all of it.
        self.holding.borrow_mut().clear();
        // The mark goes now that it has been passed, so that the ending
        // can be told and whatever was named for the end can run.
        self.ceiling = 0;
        Some(Escape::Stopped(format!("Allowed memory size of {} bytes exhausted ({} bytes were taken)", mark, held)))
    }

    /// How a run ended, told the way a language with a word for the end
    /// of one tells it: what stopped it, where, and how the run stood.
    /// Nothing is written where a language has no word for it.
    fn end_of_run(&self, said: &str) {
        let Some((_, word)) = self.complaint_words.iter().find(|(k, _)| *k == "fatal") else { return };
        // A fault raised on the way into a program belongs where that
        // program is written, and says as much.
        let (said, place, at) = match &self.entering {
            // Where the words named where the call stood, the place
            // that follows is where the program itself is written, and
            // is said to be.
            Some((place, on, defined)) => match defined {
                true => (format!("{} and defined", said), place.clone(), *on),
                false => (said.to_string(), place.clone(), *on),
            },
            None => (said.to_string(), self.written_in.clone(), self.raised_on),
        };
        self.utter(&format!("{} in {}:{}\n", complaint_head(&self.table, word, &said), place, at));
        self.utter(&format!("{}  thrown{}", self.calls_under(), complaint_tail(&self.table, &place, at)));
    }

    /// Build source against the globals this run already has and run it
    /// where it stands, giving back what it answered with.
    /// Text read as the run goes, built and run where it stands. Where
    /// it came out of a file of its own, that file is where the run is
    /// written while it lasts: a complaint names it, and a file it asks
    /// for in turn is sought beside it.
    /// Text read while the run goes, read as standing where the call to
    /// read it stands: the names of the routine around it are its own,
    /// and what it writes to one of them the routine sees afterwards.
    /// Names it makes itself go on the end of that routine's frame.
    fn run_text_within(&mut self, source: &str, frame: &Rc<Env>) -> Result<Value, Escape> {
        let tokens = self.text_scanned(source).map_err(Escape::Error)?;
        let held: Vec<String> = self.frames_named.last().map_or_else(Vec::new, |p| p.idents.clone());
        let knows = (&self.knows_cells.0, &self.knows_cells.1, &self.knows_cells.2);
        // Text read inside a method is read as standing in that
        // method's class: what the class keeps to itself is reached
        // from there, and a call written through a class is a call
        // from within it.
        let within = self.standing_in().map(str::to_string).map(|named| {
            let under = match self.class_bound(&named) {
                Some(Value::Blueprint(c)) => c.under.as_ref().map(|b| b.name.clone()),
                _ => None,
            };
            (named, under)
        });
        let built = match crate::build::build_within_at(&tokens, self.table, &self.idents, &held, knows, 0, within) {
            Ok(built) => built,
            Err((said, row)) => return Err(Escape::Error(self.text_would_not_read(said, row))),
        };
        self.idents = built.globals;
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        frame.cells.borrow_mut().resize(built.program.idents.len().max(held.len()), Value::Unset);
        let answer = self.value_of(&built.program.body, frame)?;
        Ok(match answer {
            Value::Nil | Value::Unset => Value::Small(1),
            other => other,
        })
    }

    /// How many lines a text carries ahead of the program written in
    /// it. Only the mark that opens code puts any there, and it one.
    fn lines_ahead(&self) -> u32 {
        u32::from(self.table.single("lexical.prologue").is_some())
    }

    /// Where text read while the run goes counts as standing: the file
    /// the reading was asked for in and the line that asking is written
    /// on, set about by the words the language has for marking it text
    /// read in rather than a file of its own.
    fn place_of_text(&self) -> Rc<str> {
        match self.table.around("ext.builtin.eval.place") {
            Some((head, tail)) => Rc::from(format!("{}{head}{}{tail}", self.written_in, self.row).as_str()),
            None => self.written_in.clone(),
        }
    }

    /// Text that would not be read, its place set aside so that the end
    /// of the run may name where it stood.
    fn text_would_not_read(&mut self, said: String, row: u32) -> String {
        let place = self.place_of_text();
        self.would_not_read = Some((said.clone(), place, row.saturating_sub(self.lines_ahead()).max(1)));
        said
    }

    /// The tokens of text read while the run goes, or the words for why
    /// there are none, counting its lines from the program's own first.
    fn text_scanned(&mut self, source: &str) -> Result<Vec<crate::scan::Token>, String> {
        let ahead = self.lines_ahead();
        match crate::scan::scan_at(source, self.table).and_then(|read| crate::indent::indent(read, self.table, ahead)) {
            Ok(tokens) => Ok(tokens),
            Err((said, row)) => Err(self.text_would_not_read(said, row)),
        }
    }

    fn run_source(&mut self, source: &str, came_out_of: Option<String>) -> Result<Value, String> {
        // Text handed over outright counts as standing where the call
        // to read it stands; a file stands as itself and is read from
        // its own first line.
        let reading_text = came_out_of.is_none();
        let tokens = match reading_text {
            true => self.text_scanned(source)?,
            false => crate::indent::indent(crate::scan::scan(source, self.table)?, self.table, 0).map_err(|(said, _)| said)?,
        };
        let built = match reading_text {
            true => match crate::build::build_from_at(&tokens, self.table, &self.idents, HashMap::new(), true, 0) {
                Ok(built) => built,
                Err((said, row)) => return Err(self.text_would_not_read(said, row)),
            },
            false => crate::build::build_from(&tokens, self.table, &self.idents, HashMap::new(), true, 0, came_out_of.as_deref().map(Rc::from))?,
        };
        self.idents = built.globals;
        // Names the new source brought with it want room to stand in.
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        let (was_in, was_on) = (self.written_in.clone(), self.row);
        if let Some(place) = came_out_of {
            self.written_in = Rc::from(place.as_str());
        }
        let top = self.outermost.clone();
        let ran = self.value_of(&built.program.body, &top);
        self.written_in = was_in;
        self.row = was_on;
        match ran {
            Ok(answer) => Ok(match answer {
                // Nothing answered is a plain yes, as such a language says.
                Value::Nil | Value::Unset => Value::Small(1),
                other => other,
            }),
            Err(Escape::Error(told)) => Err(told),
            Err(Escape::Yield(answer)) => Ok(answer),
            // A throw, or an ending, is whatever stands around the
            // reading to answer for and not the reading itself, so it
            // is kept and let go again there.
            Err(other) => {
                self.got_away = Some(other);
                Err("the source read in did not finish".to_string())
            }
        }
    }

    /// A value nobody took, handed to the routine the program put in
    /// its way. Answers whether it was taken up, since a run whose
    /// fault was handed over says nothing more of it in its own words.
    fn taken_up(&mut self, over: &Escape) -> bool {
        let put = self.untaken.borrow().clone();
        let Some(v) = put else { return false };
        let raised = match over {
            Escape::Thrown(raised) => raised.clone(),
            Escape::Error(told) => match self.as_raised(told) {
                Some(made) => made,
                None => return false,
            },
            _ => return false,
        };
        let Value::Bound(p, env) = self.what_it_spells(v) else { return false };
        let _ = self.invoke(p, env, vec![raised]);
        true
    }

    pub fn run_main(&mut self, body: &Form) -> Result<(), String> {
        let top = self.outermost.clone();
        let ran = self.value_of(body, &top);
        // A program may put a routine in the way of a value nobody took;
        // the run says nothing of its own where one took it up.
        if matches!(ran, Err(Escape::Thrown(_)) | Err(Escape::Error(_))) && self.taken_up(ran.as_ref().unwrap_err()) {
            return match ran {
                Err(Escape::Thrown(v)) => Err(format!("Uncaught {}", v.bare())),
                Err(Escape::Error(told)) => Err(told),
                _ => Ok(()),
            };
        }
        match ran {
            // A run the program itself said was over came out right.
            Ok(_) | Err(Escape::Done) | Err(Escape::Yield(_)) | Err(Escape::Leave(_)) | Err(Escape::Resume(_)) => Ok(()),
            // A value nobody took is a fault, told the way PHP tells it.
            Err(Escape::Thrown(Value::Thing(thing))) => {
                let told = thing.holds.borrow().iter().find(|(k, _)| k == "message").map(|(_, x)| x.bare());
                let said = match told.filter(|m| !m.is_empty()) {
                    Some(told) => format!("Uncaught {}: {}", thing.of.name, told),
                    None => format!("Uncaught {}", thing.of.name),
                };
                self.end_of_run(&said);
                Err(said)
            }
            Err(Escape::Thrown(v)) => {
                let said = format!("Uncaught {}", v.bare());
                self.end_of_run(&said);
                Err(said)
            }
            // A limit passed is told plainly, since nothing was raised.
            Err(Escape::Stopped(told)) => {
                if let Some((_, word)) = self.complaint_words.iter().find(|(k, _)| *k == "fatal") {
                    let head = complaint_head(&self.table, word, &told);
                    self.utter(&format!("{head}{}", complaint_tail(&self.table, &self.written_in, self.row)));
                }
                Err(told)
            }
            // A fault of the kernel's own is told under the class the
            // language names for one, where it names any.
            // A program that would not be read is told as a reading
            // that stopped rather than as a fault nobody took: nothing
            // was ever running there for a fault to leave.
            Err(Escape::Error(e)) if self.would_not_read_words(&e) && self.table.single("ext.system.complaint.reading").is_some() => {
                let word = self.table.single("ext.system.complaint.reading").expect("a word for a reading that stopped");
                let (place, at) = match &self.would_not_read {
                    Some((said, place, on)) if *said == e => (place.clone(), *on),
                    _ => (self.written_in.clone(), self.row),
                };
                let head = complaint_head(&self.table, word, &e);
                self.utter(&format!("{head}{}", complaint_tail(&self.table, &place, at)));
                Err(e)
            }
            Err(Escape::Error(e)) => {
                if let Some(named) = self.class_of_fault(&e) {
                    if self.complaint_words.iter().any(|(k, _)| *k == "fatal") {
                        self.raised_on = self.row;
                        self.end_of_run(&format!("Uncaught {}: {}", named, e));
                    }
                }
                Err(e)
            }
        }
    }

    // ---------- bindings

    fn fetch(&self, slot: &Address, frame: &Rc<Env>) -> Result<Value, String> {
        let f = ascend(frame, slot.up);
        let v = f.cells.borrow()[slot.at].clone();
        if let Value::Shared(cell) = v {
            return Ok(cell.borrow().clone());
        }
        if !matches!(v, Value::Unset) {
            return Ok(v);
        }
        if let Some(g) = slot.fallback {
            let v = self.outermost.cells.borrow()[g].clone();
            if !matches!(v, Value::Unset) {
                return Ok(v);
            }
        }
        // A language with a word for a warning does not stop where a
        // binding was never written: it says so and reads nothing. Only
        // a variable counts — where variables carry a mark, a name
        // without it names a constant or a class, and reaching for one
        // that is not there is a fault. The cells the builder makes for
        // itself wear no mark either, and are none of the program's.
        let marked = self.table.letter("identifier.variable_prefix").map_or(true, |mark| slot.ident.starts_with(mark));
        if marked && self.complaint_words.iter().any(|(k, _)| *k == "warning") {
            self.grumble("warning", &format!("Undefined variable {}", slot.ident));
            return Ok(Value::Nil);
        }
        Err(format!("Undefined variable: {}", slot.ident))
    }

    fn store(&self, slot: &Address, frame: &Rc<Env>, value: Value) -> Result<(), String> {
        // A cell becomes a name's own only by being tied to it. A plain
        // write of one writes what it holds, so a routine giving back a
        // cell, called without the mark that shares one, hands over a
        // copy like any other.
        let value = match value {
            Value::Shared(cell) => cell.borrow().clone(),
            held => held,
        };
        let f = ascend(frame, slot.up);
        if Rc::ptr_eq(f, &self.outermost) && Some(slot.at) == self.args_cell {
            return Err(format!("Cannot reassign {} (system-provided immutable value)", slot.ident));
        }
        // A name standing for a shared cell writes inside it.
        let shared = match &f.cells.borrow()[slot.at] {
            Value::Shared(cell) => Some(cell.clone()),
            _ => None,
        };
        match shared {
            Some(cell) => *cell.borrow_mut() = value,
            None => f.cells.borrow_mut()[slot.at] = value,
        }
        Ok(())
    }

    /// The shared cell a name stands for, made from what it holds when
    /// it does not stand for one yet.
    fn shared_cell(&self, slot: &Address, frame: &Rc<Env>) -> Rc<RefCell<Value>> {
        let f = ascend(frame, slot.up);
        let held = f.cells.borrow()[slot.at].clone();
        if let Value::Shared(cell) = held {
            return cell;
        }
        let cell = Rc::new(RefCell::new(match held {
            Value::Unset => Value::Nil,
            other => other,
        }));
        f.cells.borrow_mut()[slot.at] = Value::Shared(cell.clone());
        cell
    }

    /// The frame and index an array lives in, for writing it in place.
    fn locate(&self, slot: &Address, frame: &Rc<Env>) -> Result<(Rc<Env>, usize), String> {
        let f = ascend(frame, slot.up);
        if !matches!(f.cells.borrow()[slot.at], Value::Unset) {
            return Ok((f.clone(), slot.at));
        }
        match slot.fallback {
            Some(g) if !matches!(self.outermost.cells.borrow()[g], Value::Unset) => Ok((self.outermost.clone(), g)),
            // Where a language makes a place on writing into it, a name
            // holding nothing is where the write goes, and the array it
            // needs is made there.
            _ if self.builds_places => Ok((f.clone(), slot.at)),
            _ => Err(format!("Undefined variable '{}'", slot.ident)),
        }
    }

    // ---------- evaluation

    /// An operand: a binding or a constant is read without visiting a form.
    fn input(&mut self, o: &Input, frame: &Rc<Env>) -> Res {
        match o {
            Input::Address(slot) => Ok(self.fetch(slot, frame)?),
            Input::Const(v) => Ok(v.clone()),
            Input::Form(n) => self.value_of(n, frame),
        }
    }

    fn value_of(&mut self, node: &Form, frame: &Rc<Env>) -> Res {
        // A complaint raised where the run was only reading waits to be
        // handed over; here, before the next step, is where the run can
        // reach back into the program to hand it on.
        if self.any_unheard.get() {
            self.hand_over_unheard()?;
        }
        match node {
            Form::Const(Value::Routine(p)) => Ok(Value::Bound(p.clone(), frame.clone())),
            Form::Const(v) => Ok(v.clone()),
            Form::Read(slot) => Ok(self.fetch(slot, frame)?),
            Form::Glance(slot) => {
                let f = ascend(frame, slot.up);
                let held = f.cells.borrow()[slot.at].clone();
                let held = match (&held, slot.fallback) {
                    (Value::Unset, Some(g)) => self.outermost.cells.borrow()[g].clone(),
                    _ => held,
                };
                Ok(match held {
                    Value::Shared(cell) => cell.borrow().clone(),
                    other => other,
                })
            }
            Form::Bump { slot, by } => {
                let v = self.fetch(slot, frame)?;
                // A step down subtracts, so a string of digits counts
                // rather than being joined to.
                let (op, size) = if *by < 0 { (Prim::Minus, -*by) } else { (Prim::Plus, *by) };
                let r = match v {
                    Value::Small(x) => match x.checked_add(*by) {
                        Some(y) => Value::Small(y),
                        None => self.prim(op, "", &[v, Value::Small(size)])?,
                    },
                    other => self.prim(op, "", &[other, Value::Small(size)])?,
                };
                self.store(slot, frame, r.clone())?;
                Ok(r)
            }
            Form::Dyad { op, name, a, b } => {
                let av = self.input(a, frame)?;
                let bv = self.input(b, frame)?;
                let fast = match (&av, &bv) {
                    (Value::Small(x), Value::Small(y)) => match op {
                        Prim::Plus => x.checked_add(*y).map(Value::Small),
                        Prim::Minus => x.checked_sub(*y).map(Value::Small),
                        Prim::Times => x.checked_mul(*y).map(Value::Small),
                        Prim::Lt => Some(Value::Flag(x < y)),
                        Prim::Le => Some(Value::Flag(x <= y)),
                        Prim::Gt => Some(Value::Flag(x > y)),
                        Prim::Ge => Some(Value::Flag(x >= y)),
                        Prim::Eq => Some(Value::Flag(x == y)),
                        Prim::Ne => Some(Value::Flag(x != y)),
                        Prim::Mod if *y != 0 => x.checked_rem(*y).map(Value::Small),
                        Prim::IntDiv if *y != 0 => x.checked_div(*y).map(Value::Small),
                        _ => None,
                    },
                    _ => None,
                };
                match fast {
                    Some(v) => Ok(v),
                    None => Ok(self.prim(*op, name, &[av, bv])?),
                }
            }
            Form::Share(slot) => Ok(Value::Shared(self.shared_cell(slot, frame))),
            Form::Tie(slot, source) => {
                let cell = self.value_of(source, frame)?;
                let f = ascend(frame, slot.up);
                f.cells.borrow_mut()[slot.at] = cell;
                Ok(Value::Nil)
            }
            // A name worked out as the run goes stands for the binding
            // of that name among the outermost ones, those being the
            // only ones whose names the run can still see.
            Form::Called(spells) => {
                let spelled = self.value_of(spells, frame)?;
                let name = self.name_it_spells(&spelled);
                let at = self.place_called(&name);
                let held = self.outermost.cells.borrow()[at].clone();
                return Ok(match held {
                    Value::Shared(cell) => cell.borrow().clone(),
                    Value::Unset if self.complaint_words.iter().any(|(k, _)| *k == "warning") => {
                        self.grumble("warning", &format!("Undefined variable {}", name));
                        Value::Nil
                    }
                    Value::Unset => return Err(format!("Undefined variable '{}'", name).into()),
                    other => other,
                });
            }
            // Words a definition holds ready for a shape it still allows.
            // The line is set to where the shape stands, so that a call
            // worked out on the way does not leave its own line behind.
            Form::CellOrSaid(kind, said, row, inner) => {
                let worth = self.value_of(inner, frame)?;
                if let Value::Shared(_) = worth {
                    return Ok(worth);
                }
                if *row > 0 {
                    self.row = *row;
                }
                match kind {
                    Some(word) => {
                        self.grumble(word, said);
                        return Ok(worth);
                    }
                    None => return Err(Escape::Error(said.to_string())),
                }
            }
            Form::HeldEither(kind, said, row, inner) => {
                let worth = self.value_of(inner, frame)?;
                if let Value::Shared(_) = worth {
                    return Ok(worth);
                }
                if *row > 0 {
                    self.row = *row;
                }
                self.grumble(kind, said);
                return Ok(Value::Shared(Rc::new(RefCell::new(worth))));
            }
            Form::Remark(kind, said, row) => {
                if *row > 0 {
                    self.row = *row;
                }
                self.grumble(kind, said);
                return Ok(Value::Nil);
            }
            Form::CallWrite(spells, worth) => {
                let spelled = self.value_of(spells, frame)?;
                let name = self.name_it_spells(&spelled);
                let value = self.value_of(worth, frame)?;
                let at = self.place_called(&name);
                let shared = match &self.outermost.cells.borrow()[at] {
                    Value::Shared(cell) => Some(cell.clone()),
                    _ => None,
                };
                match shared {
                    Some(cell) => *cell.borrow_mut() = value.clone(),
                    None => self.outermost.cells.borrow_mut()[at] = value.clone(),
                }
                return Ok(value);
            }
            Form::Muted(inner) => {
                self.quieted += 1;
                let found = self.value_of(inner, frame);
                self.quieted -= 1;
                return found;
            }
            Form::Silenced(inner) => {
                self.silenced += 1;
                let found = self.value_of(inner, frame);
                self.silenced -= 1;
                return found;
            }
            // The property becomes a cell the thing and the name taking
            // it both stand for, so a write through either is seen by
            // both.
            Form::ShareField(of, called) => {
                let thing = self.value_of(of, frame)?;
                let Value::Thing(thing) = thing else {
                    return Err(format!("Cannot share property '{}' of {}", called, thing.bare()).into());
                };
                let mut holds = thing.holds.borrow_mut();
                let at = match self.member_place(&holds, called) {
                    Some(at) => at,
                    None => {
                        holds.push((called.to_string(), Value::Nil));
                        holds.len() - 1
                    }
                };
                if let Value::Shared(cell) = &holds[at].1 {
                    let cell = cell.clone();
                    drop(holds);
                    return Ok(Value::Shared(cell));
                }
                let was = std::mem::replace(&mut holds[at].1, Value::Nil);
                let cell = Rc::new(RefCell::new(was));
                holds[at].1 = Value::Shared(cell.clone());
                drop(holds);
                return Ok(Value::Shared(cell));
            }
            // A class's own value, and a binding named as the run goes,
            // each made a cell that names may share.
            Form::ShareOwn(of, called) => {
                let class = self.value_of(of, frame)?;
                let Value::Blueprint(class) = class else {
                    return Err(format!("Cannot share '{}' in {}", called, class.bare()).into());
                };
                let keeper = class.keeper(called).unwrap_or(&class);
                let mut shared = keeper.shared.borrow_mut();
                let at = match shared.iter().position(|(k, _)| k.as_str() == called.as_ref()) {
                    Some(at) => at,
                    None => {
                        shared.push((called.to_string(), Value::Nil));
                        shared.len() - 1
                    }
                };
                if let Value::Shared(cell) = &shared[at].1 {
                    let cell = cell.clone();
                    drop(shared);
                    return Ok(Value::Shared(cell));
                }
                let was = std::mem::replace(&mut shared[at].1, Value::Nil);
                let cell = Rc::new(RefCell::new(was));
                shared[at].1 = Value::Shared(cell.clone());
                drop(shared);
                return Ok(Value::Shared(cell));
            }
            Form::ShareCalled(spells) => {
                let spelled = self.value_of(spells, frame)?;
                let name = self.name_it_spells(&spelled);
                let at = self.place_called(&name);
                let mut cells = self.outermost.cells.borrow_mut();
                if let Value::Shared(cell) = &cells[at] {
                    let cell = cell.clone();
                    drop(cells);
                    return Ok(Value::Shared(cell));
                }
                let was = std::mem::replace(&mut cells[at], Value::Nil);
                let cell = Rc::new(RefCell::new(match was {
                    Value::Unset => Value::Nil,
                    held => held,
                }));
                cells[at] = Value::Shared(cell.clone());
                drop(cells);
                return Ok(Value::Shared(cell));
            }
            Form::ShareItem(slot, place) => {
                let at = self.value_of(place, frame)?;
                let f = ascend(frame, slot.up);
                let mut cells = f.cells.borrow_mut();
                let held = &mut cells[slot.at];
                // Through the shared cell when the name stands for one.
                if let Value::Shared(cell) = held {
                    let cell = cell.clone();
                    drop(cells);
                    let mut inside = cell.borrow_mut();
                    return Ok(Value::Shared(shared_item(&mut inside, &at)?));
                }
                Ok(Value::Shared(shared_item(held, &at)?))
            }
            Form::SharePlace(slot, places) => {
                let mut keys = Vec::with_capacity(places.len());
                for place in places {
                    let named = self.value_of(place, frame)?;
                    keys.push(self.as_key(&named));
                }
                let makes = self.builds_places;
                let f = ascend(frame, slot.up);
                let mut cells = f.cells.borrow_mut();
                let held = &mut cells[slot.at];
                // Through the shared cell where the name stands for one.
                if let Value::Shared(cell) = held {
                    let cell = cell.clone();
                    drop(cells);
                    let mut inside = cell.borrow_mut();
                    return Ok(Value::Shared(shared_deep(&mut inside, &keys, makes)?));
                }
                Ok(Value::Shared(shared_deep(held, &keys, makes)?))
            }
            Form::Ready(slot) => {
                let f = ascend(frame, slot.up);
                let mut cells = f.cells.borrow_mut();
                if matches!(cells[slot.at], Value::Unset) {
                    cells[slot.at] = Value::Nil;
                }
                Ok(Value::Nil)
            }
            Form::Forget(slot) => {
                let f = ascend(frame, slot.up);
                f.cells.borrow_mut()[slot.at] = Value::Unset;
                Ok(Value::Nil)
            }
            Form::ShareWithin(under, places) => {
                let holder = self.value_of(under, frame)?;
                let mut keys = Vec::with_capacity(places.len());
                for place in places {
                    let named = self.value_of(place, frame)?;
                    keys.push(self.as_key(&named));
                }
                let Value::Shared(cell) = holder else {
                    return Err("Cannot take a cell from a place in something that is not an array".to_string().into());
                };
                let makes = self.builds_places;
                let mut inside = cell.borrow_mut();
                Ok(Value::Shared(shared_deep(&mut inside, &keys, makes)?))
            }
            Form::ForgetWithin(under, place) => {
                let holder = self.value_of(under, frame)?;
                let named = self.value_of(place, frame)?;
                let at = self.as_key_spoken(&named);
                let Value::Shared(cell) = holder else {
                    return Err("Cannot take a place out of something that is not an array".to_string().into());
                };
                let mut inside = cell.borrow_mut();
                let left = match &*inside {
                    Value::Vector(items) if self.table.has_any("ext.stmt.del") => {
                        let offset = (match &at { Value::Flag(b) => Some(if *b { 1 } else { 0 }), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None }).ok_or_else(|| self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string())?;
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string().into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, v)| v.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) => {
                        let i = as_index(&at)?;
                        Value::Dict(Rc::new(
                            items
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .map(|(j, v)| (Value::Small(j as i64), v.clone()))
                                .collect(),
                        ))
                    }
                    Value::Dict(pairs) => {
                        let present = pairs.iter().any(|entry| entry.0.equals(&at));
                        if self.table.has_any("ext.stmt.del") && !present {
                            return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string().into());
                        }
                        Value::Dict(Rc::new(pairs.iter().filter(|(k, _)| !k.equals(&at)).cloned().collect()))
                    },
                    held => return Err(format!("Cannot take a place out of {}", held.bare()).into()),
                };
                *inside = left;
                Ok(Value::Nil)
            }
            Form::ForgetCalled(spells) => {
                let spelled = self.value_of(spells, frame)?;
                let name = self.name_it_spells(&spelled);
                let at = self.place_called(&name);
                self.outermost.cells.borrow_mut()[at] = Value::Unset;
                Ok(Value::Nil)
            }
            Form::TieCalled(spells, cell) => {
                let spelled = self.value_of(spells, frame)?;
                let held = self.value_of(cell, frame)?;
                let name = self.name_it_spells(&spelled);
                let at = self.place_called(&name);
                // What has no cell to share is simply written, as a
                // language asking to share one from something without
                // one does rather than stopping.
                self.outermost.cells.borrow_mut()[at] = held;
                Ok(Value::Nil)
            }
            Form::ReadyCalled(spells) => {
                let spelled = self.value_of(spells, frame)?;
                let name = self.name_it_spells(&spelled);
                let at = self.place_called(&name);
                let mut cells = self.outermost.cells.borrow_mut();
                if matches!(cells[at], Value::Unset) {
                    cells[at] = Value::Nil;
                }
                Ok(Value::Nil)
            }
            Form::OnLine(row, inner) => {
                self.row = *row;
                // A statement reached is a fault gone by: whatever calls
                // an earlier one was raised under are none of its
                // business.
                self.under = None;
                self.entering = None;
                // A statement is a fair place to look at the clock:
                // often enough to stop a run that runs away, seldom
                // enough that asking costs little.
                if let Some(over) = self.past_its_time() {
                    return Err(over);
                }
                if let Some(over) = self.past_its_room() {
                    return Err(over);
                }
                self.value_of(inner, frame)
            }
            Form::Missing(slot) => {
                let f = ascend(frame, slot.up);
                let empty = matches!(f.cells.borrow()[slot.at], Value::Unset);
                Ok(Value::Flag(empty))
            }
            Form::Again => {
                match self.holding_fault.last() {
                    Some(value) => Err(Escape::Thrown(value.clone())),
                    None => Err(self.table.single("ext.stmt.throw.empty").unwrap_or_default().to_string().into()),
                }
            }
            Form::Assert { condition, message } => {
                let tested = self.value_of(condition, frame)?;
                if self.stands_true(&tested) { return Ok(Value::Nil); }
                let held = self.value_of(message, frame)?;
                let kind = Blueprint {
                    name: self.table.single("ext.stmt.assert.kind").unwrap_or_default().to_string(),
                    fields: Vec::new(), methods: Vec::new(), constants: Vec::new(),
                    shared: RefCell::new(Vec::new()), reaches: Vec::new(), under: None, answers: Vec::new(),
                };
                self.raised_on = self.row;
                Err(Escape::Thrown(Value::Thing(Rc::new(Thing {
                    of: Rc::new(kind), turn: 0,
                    holds: RefCell::new(vec![("message".to_string(), held)]),
                }))))
            }
            Form::Context { manager, entered, body } => {
                let held = self.value_of(manager, frame)?;
                let mut entry = held.clone();
                let mut leave = None;
                if let Value::Thing(object) = &held {
                    if let Some(method) = self.table.single("ext.stmt.with.enter").and_then(|word| object.of.program(word)).cloned() {
                        entry = self.invoke(method, self.outermost.clone(), vec![held.clone()])?;
                        leave = self.table.single("ext.stmt.with.leave").and_then(|word| object.of.program(word)).cloned();
                    }
                }
                self.store(entered, frame, entry)?;
                let outcome = self.value_of(body, frame);
                if let Some(method) = leave {
                    let fault = match &outcome {
                        Err(Escape::Thrown(value)) => Some(value.clone()),
                        Err(Escape::Error(words)) => self.as_raised(words).or_else(|| Some(Value::text(words))),
                        _ => None,
                    };
                    let kind = match &fault {
                        Some(Value::Thing(thing)) => Value::Blueprint(thing.of.clone()),
                        Some(_) => Value::text("exception"),
                        None => Value::Nil,
                    };
                    let failed = fault.is_some();
                    let answer = self.invoke(method, self.outermost.clone(), vec![held, kind, fault.unwrap_or(Value::Nil), Value::Nil])?;
                    if failed && answer.is_true() { return Ok(Value::Nil); }
                }
                outcome
            }
            Form::Attempt { body, clauses, last, otherwise } => {
                if clauses.iter().any(|part| part.grouped) {
                    return Err(self.table.single("ext.stmt.catch.group.unsupported").unwrap_or_default().to_string().into());
                }
                let preceding = self.holding_fault.len();
                let body_result = match self.value_of(body, frame) {
                    Err(Escape::Error(told)) => match self.as_raised(&told) {
                        Some(value) => Err(Escape::Thrown(value)),
                        None => Err(Escape::Error(told)),
                    },
                    result => result,
                };
                let ending = match body_result {
                    Ok(value) => match otherwise {
                        Some(limb) => self.value_of(limb, frame),
                        None => Ok(value),
                    },
                    Err(Escape::Thrown(raised)) => {
                        self.holding_fault.push(raised.clone());
                        let chosen = (|| {
                            for clause in clauses {
                                let accepts = match &clause.choices {
                                    None => match &raised {
                                        Value::Thing(value) => clause.classes.iter().any(|name| value.of.goes_by(name, self.classes_either_way)),
                                        _ => false,
                                    },
                                    Some(choices) => {
                                        let mut fits = clause.takes_all;
                                        for choice in choices {
                                            let class = self.value_of(choice, frame)?;
                                            if !matches!(class, Value::Unset | Value::Blueprint(_)) {
                                                return Err(self.table.single("ext.stmt.catch.invalid").unwrap_or("A catch needs a class").to_string().into());
                                            }
                                            if let Value::Blueprint(kind) = class {
                                                if let Value::Thing(value) = &raised {
                                                    fits |= value.of.goes_by(&kind.name, self.classes_either_way);
                                                }
                                            }
                                            if fits { break; }
                                        }
                                        fits
                                    }
                                };
                                if accepts {
                                    self.under = None;
                                    self.entering = None;
                                    if let Some(place) = &clause.held { self.store(place, frame, raised.clone())?; }
                                    let answer = self.value_of(&clause.body, frame);
                                    if clause.choices.is_some() {
                                        if let Some(place) = &clause.held { self.store(place, frame, Value::Unset)?; }
                                    }
                                    return answer;
                                }
                            }
                            Err(Escape::Thrown(raised))
                        })();
                        self.holding_fault.truncate(preceding);
                        chosen
                    }
                    escape => escape,
                };
                if let Some(limb) = last {
                    if let Err(Escape::Thrown(value)) = &ending { self.holding_fault.push(value.clone()); }
                    let final_result = self.value_of(limb, frame);
                    self.holding_fault.truncate(preceding);
                    final_result?;
                }
                ending
            }
            Form::Class { plan, values } => {
                // What was written: the class it is built on, then a value
                // for every property, kept value and constant, in the
                // order the plan names them.
                let mut given = self.value_list(values, frame)?.into_iter();
                let under = match plan.extends {
                    false => None,
                    true => match given.next() {
                        Some(Value::Blueprint(b)) => Some(b),
                        _ => return Err(format!("Class {} cannot be built on that", plan.name).into()),
                    },
                };
                let mut answers = Vec::with_capacity(plan.answers);
                for _ in 0..plan.answers {
                    match given.next() {
                        Some(Value::Blueprint(b)) => answers.push(b),
                        _ => return Err(format!("Class {} cannot answer to that", plan.name).into()),
                    }
                }
                let mut named = |names: &[String]| -> Vec<(String, Value)> {
                    names.iter().map(|n| (n.clone(), given.next().unwrap_or(Value::Nil))).collect()
                };
                // Taken in the order they were written, not the order the
                // class holds them in.
                let fields = named(&plan.field_names);
                let shared = named(&plan.shared_names);
                let constants = named(&plan.constant_names);
                let built = Value::Blueprint(Rc::new(Blueprint {
                    name: plan.name.clone(),
                    under,
                    answers,
                    fields,
                    reaches: plan.field_reach.clone(),
                    methods: plan.methods.clone(),
                    constants,
                    shared: RefCell::new(shared),
                }));
                if let Value::Blueprint(blueprint) = &built {
                    if let Some(word) = self.table.strings("ext.stmt.class.annotations").get(1) {
                        blueprint.shared.borrow_mut().push((word.clone(), Value::text(&plan.name)));
                    }
                    if blueprint.under.is_some() {
                        let entries = blueprint.shared.borrow().iter().map(|(name, value)| (Value::text(name), value.clone())).collect();
                        let hook = self.table.strings("ext.op.object.protocol").get(5)
                            .and_then(|name| blueprint.under.as_ref().and_then(|parent| parent.program(name))).cloned();
                        if let Some(hook) = hook {
                            let outer = self.outermost.clone();
                            self.invoke(hook, outer, vec![built.clone(), Value::Dict(Rc::new(entries))])?;
                        }
                    }
                }
                Ok(built)
            }
            Form::Cycle { test, body, step, after, otherwise } => {
                let mut broken = false;
                loop {
                    // A pass of a loop is a fair place to look at the
                    // clock as well as a statement is, since a loop whose
                    // body holds no statement at all — `for (;;) {}` —
                    // would otherwise never be looked at again.
                    if let Some(over) = self.past_its_time() {
                        return Err(over);
                    }
                    if let Some(over) = self.past_its_room() {
                        return Err(over);
                    }
                    // The test is worked out where it is looked at and
                    // nowhere else: a test that changes something as it
                    // is read must change it once a pass, not twice.
                    if !after {
                        let told = self.value_of(test, frame)?;
                        if !self.stands_true(&told) {
                            break;
                        }
                    }
                    match self.value_of(body, frame) {
                        Ok(_) | Err(Escape::Resume(1)) => {}
                        Err(Escape::Leave(1)) => { broken = true; break; },
                        Err(Escape::Leave(n)) => return Err(Escape::Leave(n - 1)),
                        Err(Escape::Resume(n)) => return Err(Escape::Resume(n - 1)),
                        Err(other) => return Err(other),
                    }
                    if let Some(step) = step {
                        self.value_of(step, frame)?;
                    }
                    if *after {
                        let told = self.value_of(test, frame)?;
                        if self.stands_true(&told) {
                            break;
                        }
                    }
                }
                if !broken {
                    if let Some(arm) = otherwise { self.value_of(arm, frame)?; }
                }
                Ok(Value::Nil)
            }
            Form::Write(slot, value) => {
                let v = self.value_of(value, frame)?;
                self.store(slot, frame, v.clone())?;
                Ok(v)
            }
            Form::Apply(Callee::Code(target), args) => {
                let found = self.value_of(target, frame)?;
                let stands = self.what_it_spells(found);
                if let Value::Method(body, object) = &stands {
                    let mut given = vec![Value::Thing(object.clone())];
                    given.extend(self.value_list(args, frame)?);
                    return self.invoke(body.clone(), self.outermost.clone(), given).map_err(Escape::from);
                }
                if let Some(done) = self.paired_call(&stands, args, frame) {
                    return done;
                }
                if let Some(done) = self.word_it_spells(&stands, args, frame) {
                    return done;
                }
                if self.table.flag("ext.stmt.class.this.explicit") {
                    if let Value::Blueprint(class) = &stands {
                        let values = self.value_list(args, frame)?;
                        return self.make_instance(class.clone(), values);
                    }
                }
                let (p, env) = self.routine_of(stands, target)?;
                let callee = self.env_for(&p, env, args, frame)?;
                self.drive(p, callee)
            }
            Form::Apply(Callee::Prim(op, name), args) => match op {
                Prim::Seq => {
                    let mut last = Value::Nil;
                    for a in args {
                        // What the statement before came to is let go
                        // before the next is worked out and not after
                        // it: a program asking how much room it holds
                        // must not be told of a value it has already
                        // finished with.
                        drop(std::mem::replace(&mut last, Value::Nil));
                        last = self.value_of(a, frame)?;
                    }
                    Ok(last)
                }
                Prim::Choose => {
                    let (p, env) = self.pick(args, frame)?;
                    self.invoke(p, env, Vec::new())
                }
                Prim::Walked | Prim::AloneWalk | Prim::MoreYet | Prim::AtHand | Prim::NamedHere | Prim::StepOn | Prim::PastHeld => {
                    let v = self.value_list(args, frame)?;
                    self.walking(op, name, &v)
                }
                // Text read while the run goes is read where it stands:
                // inside a routine it knows that routine's names, as the
                // reference has it, and only the outermost body has none
                // but the globals.
                Prim::Weigh if !Rc::ptr_eq(frame, &self.outermost) => {
                    let v = self.value_list(args, frame)?;
                    if v.len() != 1 {
                        return Err(Escape::Error(format!("{}() expects 1 argument, got {}", name, v.len())));
                    }
                    let w = self.wording();
                    let given = v[0].render(w);
                    let source = match self.table.single("lexical.prologue") {
                        Some(open) => format!("{}\n{}", open, given),
                        None => given,
                    };
                    self.run_text_within(&source, frame)
                }

                Prim::Both | Prim::Either => {
                    let seen = self.value_of(&args[0], frame)?;
                    let left = self.stands_true(&seen);
                    if (*op == Prim::Both && !left) || (*op == Prim::Either && left) {
                        return Ok(Value::Flag(left));
                    }
                    // The right side is read in place and evaluated only here.
                    let right = match self.value_of(&args[1], frame)? {
                        Value::Bound(p, env) => self.invoke(p, env, Vec::new())?,
                        v => v,
                    };
                    Ok(Value::Flag(self.stands_true(&right)))
                }
                Prim::Yield => {
                    let v = match args.first() {
                        Some(a) => self.value_of(a, frame)?,
                        None => Value::Nil,
                    };
                    Err(Escape::Yield(v))
                }
                Prim::Leave | Prim::Resume => {
                    let levels = match args.first() {
                        Some(a) => match self.value_of(a, frame)? {
                            Value::Small(n) if n >= 1 => n as usize,
                            _ => return Err("The number of loops to leave must be a positive integer".to_string().into()),
                        },
                        None => 1,
                    };
                    Err(if *op == Prim::Leave { Escape::Leave(levels) } else { Escape::Resume(levels) })
                }
                Prim::Otherwise => {
                    // The second is worked out only when the first is nothing.
                    let first = self.value_of(&args[0], frame)?;
                    if !matches!(first, Value::Nil | Value::Unset) {
                        return Ok(first);
                    }
                    // The second is read in place and run only here.
                    match self.value_of(&args[1], frame)? {
                        Value::Bound(p, env) => Ok(self.invoke(p, env, Vec::new())?),
                        other => Ok(other),
                    }
                }
                Prim::Hurl => {
                    let values = self.value_list(args, frame)?;
                    let raised = values.into_iter().next().ok_or_else(|| format!("{}() needs a value to raise", name))?;
                    self.raised_on = self.row;
                    Err(Escape::Thrown(raised))
                }
                // A routine written where a value stands takes names
                // from around it away with it: it is bound to a frame of
                // its own holding them, and the frame of a call is
                // filled from that one.
                Prim::Carry => {
                    let mut values = self.value_list(args, frame)?;
                    if values.is_empty() {
                        return Err("Nothing was given to take away".to_string().into());
                    }
                    let Value::Bound(program, env) = values.remove(0) else {
                        return Err("Only a routine can take names away with it".to_string().into());
                    };
                    let took = Env::make(program.idents.len(), Some(env));
                    {
                        let mut cells = took.cells.borrow_mut();
                        for (slot, held) in program.carried.iter().zip(values) {
                            if program.taking.is_some() && program.formal_slots.contains(slot)
                                && matches!(held, Value::Vector(_) | Value::Dict(_) | Value::Thing(_)) {
                                return Err(self.argument_fault("ext.stmt.function.defaults.amiss", None).into());
                            }
                            cells[*slot] = held;
                        }
                    }
                    Ok(Value::Bound(program, took))
                }
                Prim::Spawn => {
                    let mut values = self.value_list(args, frame)?;
                    if values.is_empty() {
                        return Err("Nothing was given to make".to_string().into());
                    }
                    let stands = self.class_it_spells(values.remove(0));
                    let Value::Blueprint(class) = stands else {
                        return Err("Only a class can be made into a thing".to_string().into());
                    };
                    self.made += 1;
                    let thing = Rc::new(Thing { of: class.clone(), holds: RefCell::new(class.every_field()), turn: self.made });
                    if self.table.single("ext.stmt.class.destructor").is_some() {
                        self.things.borrow_mut().push(Rc::downgrade(&thing));
                    }
                    let maker = self.table.single("ext.stmt.class.constructor").and_then(|m| class.program(m)).cloned();
                    match maker {
                        Some(maker) => {
                            let mut all = vec![Value::Thing(thing.clone())];
                            all.extend(values);
                            self.invoke(maker, self.outermost.clone(), all)?;
                        }
                        None if !values.is_empty() => {
                            return Err(format!("Class {} takes nothing when it is made", class.name).into());
                        }
                        None => {}
                    }
                    Ok(Value::Thing(thing))
                }
                Prim::Ask => {
                    let mut values = self.value_list(args, frame)?;
                    if values.len() < 2 {
                        return Err(format!("{}() needs a thing and a method name", name).into());
                    }
                    let subject = values.remove(0);
                    let called = values.remove(0).bare();
                    if self.table.flag("ext.op.member.pipes") {
                        let target = self.attribute(&subject, &called);
                        let target = match target { Some(held) => Some(held), None => self.ask_namespace(&subject, &called)? };
                        if let Some(target) = target {
                            let expressions: Vec<Form> = values.into_iter().map(Form::Const).collect();
                            let call = Form::Apply(Callee::Code(Box::new(Form::Const(target))), expressions);
                            return self.value_of(&call, frame);
                        }
                    }
                    let Value::Thing(thing) = subject else {
                        return Err(format!("Cannot call '{}' on something that is not an object", called).into());
                    };
                    let program = thing.of.program(&called).cloned();
                    // A class may answer for a call it does not have:
                    // where the language names such a method and the
                    // class is written with it, it stands in, given the
                    // name asked for and the arguments as an array.
                    let Some(program) = program else {
                        let stands = self.stands_for_calls(&thing.of);
                        let Some(stands) = stands else {
                            return Err(format!("Call to undefined method {}::{}()", thing.of.name, called).into());
                        };
                        let handed = vec![Value::Thing(thing), Value::text(&called), Value::Vector(Rc::new(values))];
                        return Ok(self.invoke(stands, self.outermost.clone(), handed)?);
                    };
                    let mut all = vec![Value::Thing(thing)];
                    all.extend(values);
                    Ok(self.invoke(program, self.outermost.clone(), all)?)
                }
                Prim::Bid => {
                    let mut values = self.value_list(args, frame)?;
                    if values.len() < 3 {
                        return Err(format!("{}() needs a class and a method name", name).into());
                    }
                    let subject = values.remove(0);
                    let holder = self.class_it_spells(values.remove(0));
                    let called = values.remove(0).bare();
                    let Value::Blueprint(class) = holder else {
                        let said = self.class_lacking(&holder).unwrap_or_else(|| format!("Cannot call '{}' on something that is not a class", called));
                        return Err(said.into());
                    };
                    let program = class.program(&called).cloned();
                    let program = program.ok_or_else(|| format!("Call to undefined method {}::{}()", class.name, called))?;
                    let mut all = vec![subject];
                    all.extend(values);
                    Ok(self.invoke(program, self.outermost.clone(), all)?)
                }
                Prim::Append | Prim::Replace => {
                    let Some(Form::Read(slot)) = args.first() else {
                        return Err(format!("First argument to {}() must be an array variable name", name).into());
                    };
                    let mut values = self.value_list(&args[1..], frame)?;
                    if self.table.flag("ext.syntax.call.bind_names") {
                        let (plain, named) = self.open_arguments(values)?;
                        if !named.is_empty() { return Err(self.builtin_keyword_fault(name).into()); }
                        values = plain;
                    }
                    let want = if *op == Prim::Append { 1 } else { 2 };
                    if values.len() != want {
                        return Err(format!("{}() expects {} arguments, got {}", name, want + 1, values.len() + 1).into());
                    }
                    let (f, i) = self.locate(slot, frame)?;
                    let mut values = values;
                    let value = values.pop().unwrap();
                    let key = values.pop().map(|k| self.as_key_spoken(&k));
                    if let Some(Value::Span(bounds)) = &key {
                        let old = f.cells.borrow()[i].clone();
                        match old {
                            Value::Shared(cell) => {
                                let mut row = cell.borrow_mut();
                                self.span_written(&mut row, bounds, &value)?;
                            }
                            _ => self.span_written(&mut f.cells.borrow_mut()[i], bounds, &value)?,
                        }
                        return Ok(Value::Nil);
                    }
                    // Worked out before the place is reached, since
                    // reaching it holds the frame the name lives in.
                    let letter = self.letter_places.then(|| value.render(self.wording()));
                    // Through the shared cell when the name stands for one.
                    let shared = match &f.cells.borrow()[i] {
                        Value::Shared(cell) => Some(cell.clone()),
                        _ => None,
                    };
                    let over = match shared {
                        Some(cell) => {
                            let mut held = cell.borrow_mut();
                            written_into(&mut held, key, value, &self.no_places(), self.builds_places, letter)?
                        }
                        None => {
                            let mut slots = f.cells.borrow_mut();
                            written_into(&mut slots[i], key, value, &self.no_places(), self.builds_places, letter)?
                        }
                    };
                    // More letters handed to a place in text than it has
                    // room for: the first went in and the language says so.
                    if over {
                        if let Some(said) = self.table.single("ext.op.index.text.first") {
                            self.grumble("warning", said);
                        }
                    }
                    Ok(Value::Nil)
                }
                // Values put before everything the named array holds.
                // The array is worked on where it lives, so a place
                // handed out to a walk is still the place it was.
                Prim::Front => {
                    let Some(first) = args.first() else {
                        return Err(format!("First argument to {}() must be an array variable name", name).into());
                    };
                    // A name handed over as a cell is worked on through
                    // that cell, wherever the array behind it lives.
                    let through = match first {
                        Form::Read(_) => None,
                        other => match self.value_list(std::slice::from_ref(other), frame)?.pop() {
                            Some(Value::Shared(cell)) => Some(cell),
                            _ => return Err(format!("First argument to {}() must be an array variable name", name).into()),
                        },
                    };
                    let coming = self.value_list(&args[1..], frame)?;
                    if coming.is_empty() {
                        return Err(format!("{}() expects an array and a value at least", name).into());
                    }
                    if let Some(cell) = through {
                        let mut inside = cell.borrow_mut();
                        return Ok(Value::Small(put_before(&mut inside, coming, name)? as i64));
                    }
                    let Form::Read(slot) = first else { unreachable!("a name read in place") };
                    let (f, i) = self.locate(slot, frame)?;
                    let shared = match &f.cells.borrow()[i] {
                        Value::Shared(cell) => Some(cell.clone()),
                        _ => None,
                    };
                    let many = match shared {
                        Some(cell) => {
                            let mut held = cell.borrow_mut();
                            put_before(&mut held, coming, name)?
                        }
                        None => {
                            let mut slots = f.cells.borrow_mut();
                            put_before(&mut slots[i], coming, name)?
                        }
                    };
                    Ok(Value::Small(many as i64))
                }
                // A language may say the run is over where it stands.
                // Text given is written out first; a number is not.
                Prim::Quit => {
                    let values = self.value_list(args, frame)?;
                    if let Some(Value::Text(said)) = values.first() {
                        self.utter(said);
                    }
                    Err(Escape::Done)
                }
                op => {
                    let mut values = self.value_list(args, frame)?;
                    if self.table.flag("ext.syntax.call.bind_names") && self.table.prims.contains_key(name.as_ref()) {
                        let (mut positions, keywords) = self.open_arguments(values)?;
                        if let Some(answer) = self.builtin_names(*op, name, &mut positions, keywords)? {
                            return Ok(answer);
                        }
                        values = positions;
                    }
                    // What a call was handed is read back as it stands
                    // now: a parameter written to since holds what was
                    // written, and one handed a cell reads as what the
                    // cell holds.
                    if matches!(op, Prim::Handed | Prim::HowMany | Prim::HandedAt) {
                        self.as_they_stand(frame);
                    }
                    // A class may answer for a property the thing does
                    // not hold, and take the write of one: where the
                    // language names such methods and the class is
                    // written with them, they stand in its place.
                    if let Some(done) = self.stands_for_property(*op, &values)? {
                        return Ok(done);
                    }
                    let made = self.prim(*op, name, &values);
                    if let Some(away) = self.got_away.take() {
                        return Err(away);
                    }
                    let made = made?;
                    // A complaint the operation itself raised is handed
                    // over here, before whatever holds it goes on, so
                    // that it is said where it happened.
                    if self.any_unheard.get() {
                        self.hand_over_unheard()?;
                    }
                    Ok(made)
                }
            },
        }
    }

    /// The method a class answers a property it does not hold with, or
    /// takes the write of one with, run in the property's stead.
    /// Nothing where the thing holds the property already, or where the
    /// language names no such method and the class is written without.
    fn stands_for_property(&mut self, op: Prim, values: &[Value]) -> Res<Option<Value>> {
        let key = match op {
            Prim::Of if values.len() == 2 => "ext.stmt.class.reader",
            Prim::Onto if values.len() == 3 => "ext.stmt.class.writer",
            _ => return Ok(None),
        };
        let Some(named) = self.table.single(key).map(str::to_string) else {
            return Ok(None);
        };
        let Value::Thing(thing) = &values[0] else {
            return Ok(None);
        };
        let thing = thing.clone();
        let called = values[1].bare();
        let held = {
            let holds = thing.holds.borrow();
            self.member_place(&holds, &called).is_some()
        };
        if held {
            return Ok(None);
        }
        let Some(program) = thing.of.program(&named).cloned() else {
            return Ok(None);
        };
        let mut all = vec![values[0].clone(), Value::text(&called)];
        if let Some(written) = values.get(2) {
            all.push(written.clone());
        }
        self.invoke(program, self.outermost.clone(), all).map(Some)
    }

    /// What a call was handed, brought level with what its parameters
    /// hold now. The reference reads back the values, not the cells, so
    /// a parameter handed one reads as what that cell holds.
    fn as_they_stand(&mut self, frame: &Rc<Env>) {
        let Some(program) = self.frames_named.last().cloned() else { return };
        let cells = frame.cells.borrow();
        let mut held = Vec::with_capacity(program.formal_slots.len());
        for at in &program.formal_slots {
            match cells.get(*at) {
                Some(Value::Shared(cell)) => held.push(cell.borrow().clone()),
                Some(other) => held.push(other.clone()),
                None => break,
            }
        }
        drop(cells);
        let Some(handed) = self.handed.last_mut() else { return };
        for (at, worth) in held.into_iter().enumerate() {
            let Some(place) = handed.get_mut(at) else { break };
            *place = worth;
        }
    }

    fn value_list(&mut self, args: &[Form], frame: &Rc<Env>) -> Res<Vec<Value>> {
        let mut out = Vec::with_capacity(args.len());
        for a in args {
            out.push(self.value_of(a, frame)?);
        }
        Ok(out)
    }

    fn bound(&mut self, node: &Form, frame: &Rc<Env>) -> Res<(Rc<Routine>, Rc<Env>)> {
        let found = self.value_of(node, frame)?;
        let stands = self.what_it_spells(found);
        self.routine_of(stands, node)
    }

    /// A word of the language's own reached through a value that spells
    /// it, run as though the word itself had been written there. Only
    /// the words that work on the values handed to them can be reached
    /// this way: the ones that hold on to the pieces they are written
    /// with have nothing to work on when there are none.
    fn word_it_spells(&mut self, stands: &Value, args: &[Form], frame: &Rc<Env>) -> Option<Res<Value>> {
        let Value::Text(word) = stands else { return None };
        let op = self.table.prims.get(word.as_ref()).copied()?;
        let name = word.to_string();
        Some((|| {
            let values = self.value_list(args, frame)?;
            let made = self.prim(op, &name, &values);
            if let Some(away) = self.got_away.take() {
                return Err(away);
            }
            let made = made?;
            if self.any_unheard.get() {
                self.hand_over_unheard()?;
            }
            Ok(made)
        })())
    }

    fn routine_of(&mut self, stands: Value, node: &Form) -> Res<(Rc<Routine>, Rc<Env>)> {
        match stands {
            Value::Bound(p, env) => Ok((p, env)),
            Value::Unset => Err("Unknown function".to_string().into()),
            _ => match node {
                Form::Read(slot) => Err(format!("'{}' is not a function", slot.ident).into()),
                _ => Err("eval needs a program".to_string().into()),
            },
        }
    }

    /// The method a class answers a call it does not have with, where
    /// the language names one and the class is written with it.
    fn stands_for_calls(&self, class: &Rc<Blueprint>) -> Option<Rc<Routine>> {
        let named = self.table.single("ext.stmt.class.caller")?;
        class.program(named).cloned()
    }

    /// A pair of a thing and a method's name, standing where a routine
    /// would: that method of that thing, the thing handed over first.
    /// A class in the first place names a method of the class itself.
    fn attribute(&self, value: &Value, name: &str) -> Option<Value> {
        let class = match value {
            Value::Thing(thing) => {
                let fields = thing.holds.borrow();
                if let Some(at) = self.member_place(&fields, name) {
                    return Some(match &fields[at].1 { Value::Shared(cell) => cell.borrow().clone(), value => value.clone() });
                }
                &thing.of
            }
            Value::Blueprint(class) => class,
            _ => return None,
        };
        if let Some(keeper) = class.keeper(name) {
            return keeper.shared.borrow().iter().find(|(n, _)| n == name).map(|(_, x)| {
                match (value, x) {
                    (Value::Thing(object), Value::Routine(body) | Value::Bound(body, _)) => Value::Method(body.clone(), object.clone()),
                    _ => x.clone(),
                }
            });
        }
        if let Some(value) = class.constant(name) { return Some(value.clone()); }
        class.program(name).map(|body| match value {
            Value::Thing(o) => Value::Method(body.clone(), o.clone()),
            _ => Value::Bound(body.clone(), self.outermost.clone()),
        })
    }

    /// Ask the method named for an operation, if the owner supplies it.
    fn protocol_value(&mut self, receiver: &Value, index: usize, supplied: &[Value]) -> Result<Option<Value>, String> {
        let blueprint = match receiver {
            Value::Thing(thing) => &thing.of,
            Value::Blueprint(blueprint) => blueprint,
            _ => return Ok(None),
        };
        let names = self.table.strings("ext.op.object.protocol");
        let body = names.get(index).and_then(|name| blueprint.program(name)).cloned();
        let Some(body) = body else { return Ok(None) };
        let mut arguments = Vec::with_capacity(supplied.len() + 1);
        arguments.push(receiver.clone());
        arguments.extend_from_slice(supplied);
        let env = self.outermost.clone();
        match self.invoke(body, env, arguments) {
            Ok(value) => Ok(Some(value)),
            Err(Escape::Error(message)) => Err(message),
            Err(away) => { self.got_away = Some(away); Err("the protocol call did not return".to_string()) }
        }
    }

    fn protocol_text(&mut self, item: &Value, quoted: bool) -> Result<String, String> {
        if !self.table.strings("ext.op.object.protocol").is_empty() {
            match item {
                Value::Vector(items) => {
                    let mut text = String::from("[");
                    for (n, value) in items.iter().enumerate() {
                        if n > 0 { text.push_str(", "); }
                        text.push_str(&self.protocol_text(value, true)?);
                    }
                    text.push(']');
                    return Ok(text);
                }
                Value::Thing(_) => {
                    if !quoted {
                        if let Some(value) = self.protocol_value(item, 6, &[])? { return Ok(value.render(self.wording())); }
                    }
                    if let Some(value) = self.protocol_value(item, 0, &[])? { return Ok(value.render(self.wording())); }
                }
                _ => (),
            }
        }
        Ok(item.render(self.wording()))
    }

    fn make_instance(&mut self, class: Rc<Blueprint>, args: Vec<Value>) -> Res<Value> {
        if let Some(answer) = self.protocol_value(&Value::Blueprint(class.clone()), 3, &args)? { return Ok(answer); }
        self.made += 1;
        let fields = class.every_field();
        let object = Rc::new(Thing { of: class.clone(), holds: RefCell::new(fields), turn: self.made });
        if let Some(body) = self.table.single("ext.stmt.class.constructor").and_then(|word| class.program(word)).cloned() {
            let mut given = Vec::with_capacity(args.len() + 1);
            given.push(Value::Thing(object.clone()));
            given.extend(args);
            self.invoke(body, self.outermost.clone(), given)?;
        } else if !args.is_empty() {
            return Err(format!("Class {} takes no arguments when it is made", class.name).into());
        }
        Ok(Value::Thing(object))
    }

    fn paired_call(&mut self, stands: &Value, args: &[Form], frame: &Rc<Env>) -> Option<Res<Value>> {
        if !self.spelled_stands {
            return None;
        }
        let Value::Vector(pair) = stands else { return None };
        if pair.len() != 2 {
            return None;
        }
        let called = pair[1].bare();
        // The thing may stand in the pair through a cell it shares with
        // a name, and reads there as what the cell holds, exactly as
        // reading that name would.
        let first_of = match &pair[0] {
            Value::Shared(cell) => cell.borrow().clone(),
            held => held.clone(),
        };
        let subject = self.class_it_spells(first_of);
        let given = match self.value_list(args, frame) {
            Ok(given) => given,
            Err(e) => return Some(Err(e)),
        };
        let (class, first) = match subject {
            Value::Thing(thing) => (thing.of.clone(), Value::Thing(thing)),
            Value::Blueprint(class) => (class, Value::Nil),
            _ => return Some(Err(format!("Cannot call '{}' on something that holds no method", called).into())),
        };
        let Some(program) = class.program(&called).cloned() else {
            // A class may answer for a call it does not have, given the
            // name asked for and the arguments as an array.
            let stands = self.stands_for_calls(&class);
            let Some(stands) = stands else {
                return Some(Err(format!("Call to undefined method {}::{}()", class.name, called).into()));
            };
            let handed = vec![first, Value::text(&called), Value::Vector(Rc::new(given))];
            return Some(self.invoke(stands, self.outermost.clone(), handed).map_err(Escape::from));
        };
        let mut all = vec![first];
        all.extend(given);
        Some(self.invoke(program, self.outermost.clone(), all).map_err(Escape::from))
    }

    /// One step in tail position: a value, or the program to run next.
    fn advance(&mut self, node: &Form, frame: &Rc<Env>) -> Res<Next> {
        match node {
            Form::Apply(Callee::Code(target), args) => {
                let found = self.value_of(target, frame)?;
                let stands = self.what_it_spells(found);
                if let Value::Method(body, object) = &stands {
                    let mut given = self.value_list(args, frame)?;
                    given.insert(0, Value::Thing(object.clone()));
                    let value = self.invoke(body.clone(), self.outermost.clone(), given)?;
                    return Ok(Next::Value(value));
                }
                if let Some(done) = self.paired_call(&stands, args, frame) {
                    return Ok(Next::Value(done?));
                }
                if let Some(done) = self.word_it_spells(&stands, args, frame) {
                    return Ok(Next::Value(done?));
                }
                if let (true, Value::Blueprint(class)) = (self.table.flag("ext.stmt.class.this.explicit"), &stands) {
                    let given = self.value_list(args, frame)?;
                    return Ok(Next::Value(self.make_instance(class.clone(), given)?));
                }
                let (p, env) = self.routine_of(stands, target)?;
                let callee = self.env_for(&p, env, args, frame)?;
                Ok(Next::Jump(p, callee))
            }
            Form::Apply(Callee::Prim(Prim::Seq, _), args) if !args.is_empty() => {
                for a in &args[..args.len() - 1] {
                    self.value_of(a, frame)?;
                }
                self.advance(&args[args.len() - 1], frame)
            }
            Form::Apply(Callee::Prim(Prim::Choose, _), args) => {
                let (p, env) = self.pick(args, frame)?;
                let callee = self.env_for(&p, env, &[], frame)?;
                Ok(Next::Jump(p, callee))
            }
            other => Ok(Next::Value(self.value_of(other, frame)?)),
        }
    }

    /// Run a program: a frame under the closure's, the parameters bound,
    /// the body stepped. A tail call replaces the program; what the
    /// replaced programs caught is still caught.
    /// The callee's environment, its arguments evaluated straight into
    /// their slots, with no vector between.
    fn pick(&mut self, args: &[Form], frame: &Rc<Env>) -> Res<(Rc<Routine>, Rc<Env>)> {
        let asked = self.value_of(&args[0], frame)?;
        let test = self.stands_true(&asked);
        self.bound(&args[if test { 1 } else { 2 }], frame)
    }

    /// The frame a call runs in. A routine that took names away with it
    /// is bound to a frame of its own holding them: the call runs in a
    /// fresh frame filled from that one and standing where it stands, so
    /// that a name reached further out is reached from the same place
    /// whether the routine took anything away or not.
    fn frame_for(&self, program: &Rc<Routine>, env: &Rc<Env>) -> Rc<Env> {
        if program.carried.is_empty() {
            return Env::make(program.idents.len(), Some(env.clone()));
        }
        let frame = Env::make(program.idents.len(), env.outer.clone());
        let took = env.cells.borrow();
        let mut cells = frame.cells.borrow_mut();
        for slot in &program.carried {
            if let Some(held) = took.get(*slot) {
                cells[*slot] = held.clone();
            }
        }
        drop(cells);
        frame
    }

    fn builtin_keyword_fault(&self, written: &str) -> String {
        let name = written.rsplit('.').next().unwrap_or(written);
        let has_tail = self.table.strings("ext.syntax.call.amiss.builtin").len() == 2;
        self.argument_fault("ext.syntax.call.amiss.builtin", has_tail.then_some(name))
    }

    /// Fit only the names the builtin owns. The print writer answers
    /// here; the other calls go on with their places filled.
    fn builtin_names(&mut self, op: Prim, name: &str, positional: &mut Vec<Value>, keywords: Vec<(String, Value)>) -> Res<Option<Value>> {
        let table = self.table;
        let mut seen = std::collections::HashSet::new();
        for (key, _) in &keywords {
            if !seen.insert(key) { return Err(self.argument_fault("ext.syntax.call.amiss.duplicate", Some(key)).into()); }
        }
        if op == Prim::Say && table.single("ext.builtin.print.sep").is_some() {
            let mut join = String::from(" ");
            let mut tail = String::from("\n");
            let mut channel = 1;
            for (key, value) in keywords {
                let joining = table.spells("ext.builtin.print.sep", &key);
                if joining || table.spells("ext.builtin.print.end", &key) {
                    match value {
                        Value::Text(text) => if joining { join = text.to_string(); } else { tail = text.to_string(); },
                        Value::Nil => (),
                        _ => {
                            let label = if joining { "ext.builtin.print.sep.amiss" } else { "ext.builtin.print.end.amiss" };
                            return Err(self.argument_fault(label, None).into());
                        }
                    }
                } else if table.spells("ext.builtin.print.file", &key) {
                    match value {
                        Value::Channel(port) => channel = port,
                        Value::Nil => channel = 1,
                        _ => return Err(self.argument_fault("ext.builtin.print.file.unready", None).into()),
                    }
                } else if !table.spells("ext.builtin.print.flush", &key) {
                    return Err(self.argument_fault("ext.syntax.call.amiss.unknown", Some(&key)).into());
                }
            }
            let mut written = String::new();
            for (at, item) in positional.iter().enumerate() {
                if at != 0 { written.push_str(&join); }
                written.push_str(&self.protocol_text(item, false)?);
            }
            written.push_str(&tail);
            let route = table.strings("ext.builtin.print.redirect");
            if channel == 1 && !self.in_output_method && route.len() == 3 {
                let target = self.imported.get(&route[0]).and_then(|module| self.attribute(module, &route[1]));
                if let Some(Value::Thing(thing)) = target {
                    let routine = thing.of.program(&route[2]).cloned();
                    if let Some(routine) = routine {
                        self.in_output_method = true;
                        let outer = self.outermost.clone();
                        let done = self.invoke(routine, outer, vec![Value::Thing(thing), Value::text(&written)]);
                        self.in_output_method = false;
                        done?;
                        return Ok(Some(Value::Nil));
                    }
                }
            }
            match channel {
                2 => eprint!("{}", written),
                _ => self.utter(&written),
            }
            return Ok(Some(Value::Nil));
        }
        for (key, value) in keywords {
            let index = match op {
                Prim::AsInt if table.spells("ext.builtin.to_int.base", &key) => 1,
                Prim::AsText if table.spells("ext.builtin.to_string.object", &key) => 0,
                Prim::AsText if table.spells("ext.builtin.to_string.encoding", &key) || table.spells("ext.builtin.to_string.errors", &key) => {
                    return Err(self.argument_fault("ext.builtin.to_string.unready", None).into());
                }
                _ => return Err(self.builtin_keyword_fault(name).into()),
            };
            match positional.len().cmp(&index) {
                std::cmp::Ordering::Equal => positional.push(value),
                std::cmp::Ordering::Greater => return Err(self.argument_fault("ext.syntax.call.amiss.duplicate", Some(&key)).into()),
                std::cmp::Ordering::Less => return Err(self.argument_fault("ext.syntax.call.amiss", None).into()),
            }
        }
        Ok(None)
    }

    fn whole_from_call(&self, values: &[Value]) -> Result<Value, String> {
        let complaint = |ending: &str| self.argument_fault(&format!("ext.builtin.to_int.{}", ending), None);
        if values.len() > 2 { return Err(self.argument_fault("ext.syntax.call.amiss", None)); }
        if values.is_empty() { return Ok(Value::Small(0)); }
        let radix = match values.get(1) {
            Some(Value::Small(base)) if *base == 0 || (2..37).contains(base) => *base as u32,
            Some(Value::Small(_) | Value::Huge(_)) => return Err(complaint("base.amiss")),
            Some(_) => return Err(self.argument_fault("ext.syntax.call.amiss", None)),
            None => 10,
        };
        if let Value::Text(text) = &values[0] {
            let mut source = text.trim();
            let negative = source.starts_with('-');
            if source.starts_with(['+', '-']) { source = &source[1..]; }
            let prefix = match source.get(..2).map(str::to_ascii_lowercase).as_deref() {
                Some("0b") => 2, Some("0o") => 8, Some("0x") => 16, _ => 0,
            };
            let base = match (radix, prefix) { (0, 0) => 10, (0, p) => p, (r, _) => r };
            let has_prefix = prefix > 0 && base == prefix;
            if has_prefix {
                source = &source[2..];
                if source.starts_with('_') { source = &source[1..]; }
            }
            let mut digits = String::new();
            let mut was_digit = false;
            for c in source.chars() {
                if c == '_' {
                    if !was_digit { return Err(complaint("text.amiss")); }
                    was_digit = false;
                } else {
                    if !c.is_ascii() || c.to_digit(base).is_none() { return Err(complaint("text.amiss")); }
                    digits.push(c);
                    was_digit = true;
                }
            }
            if !was_digit || (radix == 0 && !has_prefix && digits.starts_with('0') && digits.bytes().any(|b| b != b'0')) {
                return Err(complaint("text.amiss"));
            }
            let mut number = BigInt::parse_bytes(digits.as_bytes(), base).ok_or_else(|| complaint("text.amiss"))?;
            if negative { number = -number; }
            return Ok(Value::from_big(number));
        }
        if values.len() > 1 { return Err(complaint("text.required")); }
        match &values[0] {
            Value::Flag(truth) => Ok(Value::Small(if *truth { 1 } else { 0 })),
            other => math::whole_part(other).map(Value::from_big).ok_or_else(|| self.argument_fault("ext.syntax.call.amiss", None)),
        }
    }

    fn argument_fault(&self, key: &str, name: Option<&str>) -> String {
        let words = self.table.strings(key);
        let mut said = words.first().cloned().unwrap_or_default();
        if let Some(name) = name {
            said.push_str(name);
            if let Some(end) = words.get(1) { said.push_str(end); }
        }
        said
    }

    /// Gather the positional things apart from the named ones, retaining
    /// every keyword until the call has checked for repeated names.
    fn open_arguments(&self, values: Vec<Value>) -> Res<(Vec<Value>, Vec<(String, Value)>)> {
        let mut positions = Vec::new();
        let mut names = Vec::new();
        for worth in values {
            let Value::Couple(pair) = worth else { positions.push(worth); continue };
            match &pair.0 {
                Value::Text(key) => names.push((key.to_string(), pair.1.clone())),
                Value::Flag(true) => {
                    let fault = || self.argument_fault("ext.syntax.call.spread.pairs.amiss", None);
                    if let Value::Dict(entries) = &pair.1 {
                        for (k, v) in entries.iter() {
                            match k {
                                Value::Text(text) => names.push((text.to_string(), v.clone())),
                                _ => return Err(fault().into()),
                            }
                        }
                    } else { return Err(fault().into()); }
                }
                Value::Flag(false) => {
                    match &pair.1 {
                        Value::Progression(walk) => {
                            let mut place = BigInt::from(0);
                            while place < walk.count() {
                                if let Some(item) = walk.item(&place) { positions.push(item); }
                                place += 1;
                            }
                        }
                        Value::Vector(values) => positions.extend(values.iter().cloned()),
                        Value::Dict(entries) => positions.extend(entries.iter().map(|entry| entry.0.clone())),
                        Value::Text(text) => {
                            for letter in text.chars() { positions.push(Value::text(&letter.to_string())); }
                        }
                        _ => return Err(self.argument_fault("ext.syntax.call.spread.amiss", None).into()),
                    }
                }
                _ => return Err(self.argument_fault("ext.syntax.call.amiss", None).into()),
            }
        }
        Ok((positions, names))
    }

    fn fit_arguments(&self, program: &Routine, manners: &[char], values: Vec<Value>) -> Res<Vec<Value>> {
        let (positional, named) = self.open_arguments(values)?;
        let mut fitted = vec![Value::Unset; manners.len()];
        let ordinary: Vec<usize> = manners.iter().enumerate()
            .filter_map(|(slot, how)| matches!(how, 'b' | 'p').then_some(slot)).collect();
        let gather = manners.iter().position(|how| *how == 'v');
        let gather_names = manners.iter().position(|how| *how == 'k');
        if positional.len() > ordinary.len() && gather.is_none() {
            return Err(self.argument_fault("ext.syntax.call.amiss", None).into());
        }
        let mut remaining = Vec::new();
        for (n, held) in positional.into_iter().enumerate() {
            match ordinary.get(n) {
                Some(slot) => fitted[*slot] = held,
                None => remaining.push(held),
            }
        }
        if let Some(slot) = gather { fitted[slot] = Value::Vector(Rc::new(remaining)); }
        let mut spare_names = Vec::new();
        let mut already = std::collections::HashSet::new();
        for (key, worth) in named {
            let duplicate = || self.argument_fault("ext.syntax.call.amiss.duplicate", Some(&key));
            if !already.insert(key.clone()) { return Err(duplicate().into()); }
            let found = program.formals.iter().enumerate()
                .find(|(at, name)| **name == key && matches!(manners[*at], 'b' | 'n'));
            if let Some((at, _)) = found {
                if !matches!(fitted[at], Value::Unset) { return Err(duplicate().into()); }
                fitted[at] = worth;
            } else if gather_names.is_some() {
                spare_names.push((Value::text(&key), worth));
            } else {
                return Err(self.argument_fault("ext.syntax.call.amiss.unknown", Some(&key)).into());
            }
        }
        if let Some(slot) = gather_names { fitted[slot] = Value::Dict(Rc::new(spare_names)); }
        for at in 0..fitted.len() {
            if matches!(fitted[at], Value::Unset) && !program.carried.contains(&program.formal_slots[at])
                && !program.local_defaults.contains(&program.formal_slots[at]) {
                return Err(self.argument_fault("ext.syntax.call.amiss.missing", Some(&program.formals[at])).into());
            }
        }
        Ok(fitted)
    }

    fn env_for(&mut self, program: &Rc<Routine>, env: Rc<Env>, args: &[Form], caller: &Rc<Env>) -> Res<Rc<Env>> {
        if let Some(manners) = &program.taking {
            let values = self.value_list(args, caller)?;
            let fitted = self.fit_arguments(program, manners, values)?;
            let frame = self.frame_for(program, &env);
            for (slot, worth) in program.formal_slots.iter().zip(fitted) {
                if !matches!(worth, Value::Unset) { frame.cells.borrow_mut()[*slot] = worth; }
            }
            return Ok(frame);
        }
        let most = if self.reads_handed || program.gather_from.is_some() { usize::MAX } else { program.formals.len() };
        if args.len() > most || args.len() < program.least {
            self.value_list(args, caller)?;
            let wanted = program.formals.len();
            return Err(format!("Function {} expects {} arguments, got {}", program.ident, wanted, args.len()).into());
        }
        if let Some(start) = program.gather_from {
            let mut values = self.value_list(args, caller)?;
            let tail = values.split_off(start.min(values.len()));
            values.resize(start, Value::Unset);
            values.push(Value::Vector(Rc::new(tail)));
            let frame = self.frame_for(program, &env);
            for (slot, value) in program.formal_slots.iter().zip(values) {
                frame.cells.borrow_mut()[*slot] = value;
            }
            return Ok(frame);
        }
        // Where what a call hands over can be read back, every argument
        // is worked out, even one the routine gives no name to.
        if self.reads_handed {
            let all = self.value_list(args, caller)?;
            for at in 0..all.len() {
                self.of_the_class_written(program, at, &all)?;
            }
            let frame = if program.frameless { env.clone() } else { self.frame_for(program, &env) };
            if !program.frameless {
                let mut cells = frame.cells.borrow_mut();
                for (i, v) in program.formal_slots.iter().zip(all.iter()) {
                    cells[*i] = v.clone();
                }
            }
            self.pending = all;
            return Ok(frame);
        }
        if program.frameless {
            return Ok(env);
        }
        let frame = self.frame_for(program, &env);
        let mut so_far = Vec::with_capacity(args.len());
        for (at, (i, a)) in program.formal_slots.iter().zip(args).enumerate() {
            let v = self.value_of(a, caller)?;
            so_far.push(v.clone());
            self.of_the_class_written(program, at, &so_far)?;
            frame.cells.borrow_mut()[*i] = v;
        }
        Ok(frame)
    }

    /// An argument is of the class its parameter was written to take. A
    /// parameter written with a kind naming no class is let be, since a
    /// language may bring such a value to the kind rather than refusing
    /// it, and nothing at all is let through, since a parameter with no
    /// value of its own may be handed nothing.
    fn of_the_class_written(&mut self, program: &Rc<Routine>, at: usize, given: &[Value]) -> Result<(), Escape> {
        let Some(Some(written)) = program.formal_kinds.get(at) else { return Ok(()) };
        let Some(x) = given.get(at) else { return Ok(()) };
        // A parameter handed a cell holds that cell; what was given is
        // what the cell holds, and that is what has a class.
        let held;
        let x = match x {
            Value::Shared(cell) => {
                held = cell.borrow().clone();
                &held
            }
            other => other,
        };
        if matches!(x, Value::Nil | Value::Unset) {
            return Ok(());
        }
        let Some(Value::Blueprint(_)) = self.class_bound(written) else { return Ok(()) };
        if matches!(x, Value::Thing(t) if t.of.goes_by(written, self.classes_either_way)) {
            return Ok(());
        }
        let handed = match x {
            Value::Thing(t) => t.of.name.clone(),
            other => self.kind_called(other),
        };
        // A fault of this kind names where the call stood. Where the
        // program is written is kept aside: a trap taking it wants the
        // words alone, and only a run ended by it says as much. The
        // call being entered stands in the trace all the same.
        let from_library = self.calls.last().map_or(false, |c| c.of_library && !self.stands_for_the_run(&c.named));
        let head = format!(
            "{}(): Argument #{} ({}) must be of type {}, {} given",
            program.ident,
            at + 1,
            program.formals.get(at).map_or("", String::as_str),
            written,
            handed
        );
        // Where the call was made from inside the language itself there
        // is no line of the program's to name, so the words say only
        // what was given and where the routine stands.
        let told = match from_library {
            true => head,
            false => format!("{}, called in {} on line {}", head, self.written_in, self.row),
        };
        if self.under.is_none() {
            self.under = Some(self.calls_told_entering(program, given, from_library));
        }
        let place = program.written_in.clone().unwrap_or_else(|| self.written_in.clone());
        self.entering = Some((place, program.declared_on, !from_library));
        Err(Escape::Error(told))
    }

    /// The same, with one more call about to be entered: a fault raised
    /// on the way in stands inside it, though it never began to run.
    fn calls_told_entering(&self, program: &Rc<Routine>, given: &[Value], entered: bool) -> String {
        let named = match &program.within {
            Some(class) => format!("{}::{}", class, program.ident),
            None => program.ident.clone(),
        };
        let handed = given.iter().map(|v| self.handed_told(v)).collect::<Vec<_>>().join(", ");
        let stood = match entered {
            true => "[internal function]".to_string(),
            false => format!("{}({})", self.written_in, self.row),
        };
        let mut out = format!("Stack trace:\n#0 {}: {}({})\n", stood, named, handed);
        let mut at = 1;
        for call in self.calls.iter().rev() {
            if self.stands_for_the_run(&call.named) || self.library_alone(call) {
                continue;
            }
            out.push_str(&format!("#{} {}: {}({})\n", at, self.call_stood(call), self.call_named(call), self.call_handed(call)));
            at += 1;
        }
        out.push_str(&format!("#{} {{main}}\n", at));
        out
    }

    /// Whether a call stands for the run and not for the program: the
    /// routine the run hands its complaints to is the run's own doing,
    /// so a trace looks past it to whatever raised the complaint.
    /// Whether a call is the library's own doing from end to end: what
    /// the language's own library does among itself is no part of the
    /// program's calls and stands in no trace of them.
    fn library_alone(&self, call: &Called) -> bool {
        call.of_library && call.from_library
    }

    /// Where a call stands, as a trace tells it: a call the library made
    /// has no line of the program's to name.
    fn call_stood(&self, call: &Called) -> String {
        match call.from_library {
            true => "[internal function]".to_string(),
            false => format!("{}({})", call.from, call.on),
        }
    }

    fn stands_for_the_run(&self, named: &str) -> bool {
        match self.hearer.borrow().as_ref() {
            Some(Value::Bound(p, _)) => p.ident == named,
            Some(Value::Routine(p)) => p.ident == named,
            Some(v) => matches!(v, Value::Text(t) if t.as_ref() == named),
            None => false,
        }
    }

    /// The calls under way, innermost first, each named with where it
    /// was written and what it was handed, and the outermost body last.
    fn calls_told(&self) -> String {
        let mut out = String::from("Stack trace:\n");
        let mut at = 0;
        for call in self.calls.iter().rev() {
            if self.stands_for_the_run(&call.named) || self.library_alone(call) {
                continue;
            }
            out.push_str(&format!("#{} {}: {}({})\n", at, self.call_stood(call), self.call_named(call), self.call_handed(call)));
            at += 1;
        }
        out.push_str(&format!("#{} {{main}}\n", at));
        out
    }

    fn call_named(&self, call: &Called) -> String {
        match &call.within {
            Some(class) => format!("{}::{}", class, call.named),
            None => call.named.to_string(),
        }
    }

    fn call_handed(&self, call: &Called) -> String {
        match self.handed_to(call) {
            Some(values) => values.iter().map(|v| self.handed_told(v)).collect::<Vec<_>>().join(", "),
            None => String::new(),
        }
    }

    /// What a call was handed, as a trace tells it: a method is handed
    /// the thing it is for before all else, and that is no argument of
    /// the call's.
    fn handed_to(&self, call: &Called) -> Option<&[Value]> {
        let given = call.handed_at.and_then(|i| self.handed.get(i))?;
        match call.within.is_some() && !given.is_empty() {
            true => Some(&given[1..]),
            false => Some(given),
        }
    }

    /// An argument as a trace writes it: enough of it to know it by and
    /// never the whole.
    fn handed_told(&self, v: &Value) -> String {
        const MOST: usize = 15;
        match v {
            Value::Shared(cell) => self.handed_told(&cell.borrow()),
            Value::Text(t) => {
                let mut kept: String = t.chars().take(MOST).collect();
                if t.chars().nth(MOST).is_some() {
                    kept.push_str("...");
                }
                format!("'{}'", kept)
            }
            Value::Thing(thing) => format!("Object({})", thing.of.name),
            Value::Vector(_) | Value::Dict(_) => "Array".to_string(),
            Value::Nil | Value::Unset => "NULL".to_string(),
            Value::Flag(true) => "true".to_string(),
            Value::Flag(false) => "false".to_string(),
            other => other.bare(),
        }
    }

    /// The calls a fault was raised under, where any were written down,
    /// and the outermost body alone where none were.
    fn calls_under(&self) -> String {
        match &self.under {
            Some(told) => told.clone(),
            None => "Stack trace:\n#0 {main}\n".to_string(),
        }
    }

    pub fn invoke(&mut self, program: Rc<Routine>, env: Rc<Env>, args: Vec<Value>) -> Res {
        if let Some(manners) = &program.taking {
            let fitted = self.fit_arguments(&program, manners, args)?;
            let frame = self.frame_for(&program, &env);
            {
                let mut cells = frame.cells.borrow_mut();
                for (slot, value) in program.formal_slots.iter().zip(fitted) {
                    if !matches!(value, Value::Unset) { cells[*slot] = value; }
                }
            }
            return self.drive(program, frame);
        }
        let most = if self.reads_handed || program.gather_from.is_some() { usize::MAX } else { program.formals.len() };
        if args.len() > most || args.len() < program.least {
            let wanted = program.formals.len();
            return Err(format!("Function {} expects {} arguments, got {}", program.ident, wanted, args.len()).into());
        }
        if self.reads_handed {
            self.pending = args.clone();
        }
        let mut args = args;
        if let Some(first) = program.gather_from {
            let rest = args.split_off(first.min(args.len()));
            args.resize(first, Value::Unset);
            args.push(Value::Vector(Rc::new(rest)));
        }
        let frame = if program.frameless {
            env
        } else {
            let frame = self.frame_for(&program, &env);
            {
                let mut slots = frame.cells.borrow_mut();
                for (i, a) in program.formal_slots.iter().zip(args) {
                    slots[*i] = a;
                }
            }
            frame
        };
        self.drive(program, frame)
    }

    /// Run a program in a frame already built. A tail call replaces the
    /// program and the frame; what the replaced programs caught is still caught.
    fn drive(&mut self, program: Rc<Routine>, frame: Rc<Env>) -> Res {
        let memo = program.traps == Traps::Yields && self.memo_cell.map_or(false, |i| matches!(self.outermost.cells.borrow()[i], Value::Flag(true)));
        let key = memo.then(|| {
            let mut k = format!("{}(", program.ident);
            let slots = frame.cells.borrow();
            program.formal_slots.iter().for_each(|i| slots[*i].memo_key(&mut k));
            k
        });
        if let Some(hit) = key.as_ref().and_then(|k| self.memo.get(k)) {
            return Ok(hit.clone());
        }
        // What this call was handed is kept while it runs. A call in
        // tail position takes the place of this one, so what it was
        // handed takes the place of this call's too. A piece with no
        // names of its own -- an arm of a branch -- is no call of
        // anybody's and was handed nothing, so nothing is kept for it
        // and what the call around it was handed still stands.
        let mut watching = self.reads_handed && !program.frameless;
        if watching {
            let mine = std::mem::take(&mut self.pending);
            self.handed.push(mine);
        }
        let (mut program, mut frame) = (program, frame);
        // Where the call itself stands, kept before running the program
        // moves the run into whatever file the program was written in.
        let (was_written_in, was_on_row) = (self.written_in.clone(), self.row);
        // A program written in a file of its own runs as being in it: a
        // complaint names that file, and a file it asks for is sought
        // beside it, wherever the call was made.
        let elsewhere = program.written_in.as_ref().map(|place| {
            let was = std::mem::replace(&mut self.written_in, place.clone());
            (was, self.row)
        });
        let mut caught: u8 = 0;
        // The names of the frame being run in, kept while it runs, so
        // that text read while the run goes can be built knowing them.
        // A program holding no names of its own runs in the frame around
        // it, whose names are already kept.
        // A program holding no names of its own runs in the frame around
        // it, and inside the class around it: an arm of a branch is such
        // a one, and the class it stands in is the class it stands in.
        let mut mine = !program.frameless;
        if mine {
            self.frames_named.push(program.clone());
            self.inside.push(program.within.clone());
        }
        // The call is written down while it runs, so that a fault
        // raised inside it can name it.
        // A program holding no names of its own is a piece of the one
        // around it — an arm of a branch — and no call of anybody's, so
        // it is not written down as one.
        let mut noted = !program.frameless;
        if noted {
            let from_library = self.calls.last().map_or(false, |c| c.of_library && !self.stands_for_the_run(&c.named));
            self.calls.push(Called {
                named: Rc::from(program.ident.as_str()),
                within: program.within.clone(),
                from: was_written_in.clone(),
                on: was_on_row,
                handed_at: watching.then(|| self.handed.len() - 1),
                of_library: program.declared_on == 0,
                from_library,
            });
        }
        let outcome: Res = loop {
            caught |= match program.traps {
                Traps::Naught => 0,
                Traps::Yields => 1,
                Traps::Leaves => 2,
                Traps::Resumes => 4,
            };
            match self.advance(&program.body, &frame) {
                Ok(Next::Value(v)) => {
                    // A language whose functions yield what they assigned to their own name.
                    if program.traps == Traps::Yields && self.table.flag("stmt.function.result_by_name") {
                        if let Some(i) = program.idents.iter().position(|n| *n == program.ident) {
                            let own = frame.cells.borrow()[i].clone();
                            if !matches!(own, Value::Unset | Value::Bound(..)) {
                                break Ok(own);
                            }
                        }
                    }
                    break Ok(v);
                }
                Ok(Next::Jump(p, f)) => {
                    program = p;
                    frame = f;
                    // A program holding no names of its own is a piece
                    // of the one it was reached from and runs in its
                    // frame and its class: stepping into one does not
                    // take the place of what is running, however it was
                    // reached.
                    let stands_alone = !program.frameless;
                    // A piece with no names of its own may step into a
                    // program that has them: the run stands inside that
                    // one from here, so its names and its call are put
                    // down now rather than taking the place of any.
                    let just_entered = stands_alone && !mine;
                    if just_entered {
                        self.frames_named.push(program.clone());
                        self.inside.push(program.within.clone());
                        mine = true;
                        if self.reads_handed {
                            let taken = std::mem::take(&mut self.pending);
                            self.handed.push(taken);
                            watching = true;
                        }
                    } else if stands_alone {
                        if let Some(top) = self.frames_named.last_mut() {
                            *top = program.clone();
                        }
                        if let Some(here) = self.inside.last_mut() {
                            *here = program.within.clone();
                        }
                    }
                    if stands_alone && !noted {
                        let from_library = self.calls.last().map_or(false, |c| c.of_library && !self.stands_for_the_run(&c.named));
                        self.calls.push(Called {
                            named: Rc::from(program.ident.as_str()),
                            within: program.within.clone(),
                            from: was_written_in.clone(),
                            on: was_on_row,
                            handed_at: watching.then(|| self.handed.len() - 1),
                            of_library: program.declared_on == 0,
                            from_library,
                        });
                        noted = true;
                    } else if stands_alone {
                        if let Some(top) = self.calls.last_mut() {
                            top.named = Rc::from(program.ident.as_str());
                            top.within = program.within.clone();
                        }
                    }
                    // Where the call this one takes the place of was
                    // written down, what it was handed is written over
                    // too. Where it was just now put down, it holds
                    // what this call was handed already.
                    if watching && !just_entered {
                        let next = std::mem::take(&mut self.pending);
                        if let Some(top) = self.handed.last_mut() {
                            *top = next;
                        }
                    }
                }
                Err(Escape::Yield(v)) if caught & 1 != 0 => break Ok(v),
                Err(Escape::Leave(1)) if caught & 2 != 0 => break Ok(Value::Nil),
                Err(Escape::Resume(1)) if caught & 4 != 0 => break Ok(Value::Nil),
                Err(Escape::Leave(n)) if caught & 2 != 0 => break Err(Escape::Leave(n - 1)),
                Err(Escape::Resume(n)) if caught & 4 != 0 => break Err(Escape::Resume(n - 1)),
                Err(e) => break Err(e),
            }
        };
        if mine {
            self.frames_named.pop();
            self.inside.pop();
        }
        if let Some((was, on)) = elsewhere {
            self.written_in = was;
            self.row = on;
        }
        // Where a fault leaves the call and none has been written down
        // yet, the calls go down as they stand: the moment after this
        // they are gone. Leaving a loop or going round it again leaves
        // a call the same way and is no fault at all, so nothing of the
        // sort is written down for it.
        let a_fault = matches!(outcome, Err(Escape::Error(_) | Escape::Thrown(_) | Escape::Stopped(_)));
        if a_fault && self.under.is_none() {
            self.under = Some(self.calls_told());
        }
        if watching {
            self.handed.pop();
        }
        if noted {
            self.calls.pop();
        }
        let result = outcome?;
        if let Some(k) = key {
            self.memo.insert(k, result.clone());
        }
        Ok(result)
    }

    // ---------- operations

    fn prim(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        if v.len() == 2 {
            let index = match op { Prim::Eq | Prim::Ne => Some(1), Prim::Fetch | Prim::At => Some(2), _ => None };
            if let Some(index) = index {
                if let Some(answer) = self.protocol_value(&v[0], index, &v[1..])? {
                    return Ok(if op == Prim::Ne { Value::Flag(!answer.is_true()) } else { answer });
                }
            }
        }
        if matches!(op, Prim::Eq | Prim::Ne) && v.len() == 2 {
            if let Some(value) = self.protocol_value(&v[1], 1, &v[..1])? {
                return Ok(Value::Flag(value.is_true() != (op == Prim::Ne)));
            }
        }
        if op == Prim::AsText && v.len() == 1 && matches!(v[0], Value::Thing(_)) {
            return Ok(Value::text(&self.protocol_text(&v[0], false)?));
        }
        if matches!(op, Prim::Listed | Prim::Iterated) && v.len() == 1 {
            if let Some(answer) = self.protocol_value(&v[0], 4, &[])? { return Ok(answer); }
        }
        let w = self.wording();
        let n = |k: usize| -> Result<(), String> {
            if v.len() == k { Ok(()) } else { Err(format!("{}() expects {} argument{}, got {}", name, k, if k == 1 { "" } else { "s" }, v.len())) }
        };
        Ok(match op {
            Prim::SliceRefused => return Err(self.span_complaint("unsupported")),
            Prim::SliceBounds => Value::Span(Rc::new(v.to_vec())),
            // A step onward or back adds or takes away one, save on text
            // spelling no number: a language may walk such text along
            // its letters instead, or leave it standing, and says so.
            Prim::Onward(forward) => {
                n(2)?;
                let label = match forward {
                    true => "ext.op.increment.text",
                    false => "ext.op.decrement.text",
                };
                let said = self.table.single(label).map(str::to_string);
                if let (Value::Text(letters), Some(words)) = (&v[0], said) {
                    if number_spelled_in(&v[0]).is_none() {
                        let stepped = match forward {
                            true => Value::text(&letters_walked(letters)),
                            false => v[0].clone(),
                        };
                        self.grumble("deprecated", &words);
                        return Ok(stepped);
                    }
                }
                let plain = match forward {
                    true => Prim::Plus,
                    false => Prim::Minus,
                };
                return self.prim(plain, name, v);
            }
            Prim::TupleJoined => match (&v[0], &v[1]) {
                (Value::Vector(left), Value::Vector(right)) => {
                    let joined = left.iter().chain(right.iter()).cloned().collect();
                    Value::Vector(Rc::new(joined))
                }
                _ => return Err("Tuple portion is not an array".to_string()),
            },
            Prim::Partition(wanted, star) => {
                let mut values: Vec<Value> = match &v[0] {
                    Value::Progression(_) => self.gathered_members(&v[0])?,
                    Value::Text(s) => s.chars().map(|letter| Value::text(&letter.to_string())).collect(),
                    Value::Dict(entries) => entries.iter().map(|entry| entry.0.clone()).collect(),
                    Value::Vector(v) => v.to_vec(),
                    _ => return Err(self.table.single("ext.stmt.unpack.unwalkable").unwrap_or("Value cannot be taken apart").to_string()),
                };
                let minimum = if star.is_some() { wanted - 1 } else { wanted };
                let fault = if values.len() < minimum { Some("ext.stmt.unpack.short") }
                    else if star.is_none() && values.len() != wanted { Some("ext.stmt.unpack.long") }
                    else { None };
                if let Some(label) = fault {
                    return Err(self.table.single(label).unwrap_or("Wrong number of values").to_string());
                }
                if let Some(middle) = star {
                    let tail = wanted - middle - 1;
                    let after = values.split_off(values.len() - tail);
                    let gathered = values.split_off(middle);
                    values.push(Value::Vector(Rc::new(gathered)));
                    values.extend(after);
                }
                Value::Vector(Rc::new(values))
            }
            Prim::Iterated => Value::Vector(Rc::new(self.gathered_members(&v[0])?)),
            Prim::CheckUnpack(count) => {
                let values = self.gathered_members(&v[0])?;
                if values.len() != count {
                    return Err(self.table.single("ext.op.comprehension.unpack.amiss").unwrap_or("Comprehension target and item have different lengths").into());
                }
                Value::Vector(Rc::new(values))
            }
            Prim::ExtendLiteral(dictionary, expanded) => {
                if !dictionary {
                    let Value::Vector(prior) = &v[0] else { unreachable!() };
                    let mut next = prior.to_vec();
                    next.extend(if expanded { self.gathered_members(&v[1])? } else { vec![v[1].clone()] });
                    Value::Vector(Rc::new(next))
                } else {
                    let Value::Dict(prior) = &v[0] else { unreachable!() };
                    let incoming = match &v[1] {
                        Value::Couple(pair) if !expanded => vec![pair.as_ref().clone()],
                        Value::Dict(entries) if expanded => entries.to_vec(),
                        _ => return Err(self.table.single("ext.syntax.map.spread.unmapped").unwrap_or("A map spread needs a map").into()),
                    };
                    let mut combined = prior.to_vec();
                    for entry in incoming { set_key(&mut combined, entry.0, entry.1); }
                    Value::Dict(Rc::new(combined))
                }
            }
            Prim::MakeArray => assembled(v.to_vec(), false, self.plain_keys),
            Prim::MakeMap => assembled(v.to_vec(), true, self.plain_keys),
            Prim::Couple => {
                n(2)?;
                Value::Couple(Rc::new((v[0].clone(), v[1].clone())))
            }
            Prim::KeyAt | Prim::ItemAt => {
                n(2)?;
                let at = as_index(&v[1])?;
                let wants_key = op == Prim::KeyAt;
                match &v[0] {
                    Value::Progression(walk) => {
                        if wants_key { Value::Small(at as i64) }
                        else { walk.item(&BigInt::from(at)).ok_or_else(|| self.argument_fault("ext.builtin.range.index", None))? }
                    }
                    Value::Vector(items) => match items.get(at) {
                        Some(_) if wants_key => Value::Small(at as i64),
                        Some(x) => x.clone(),
                        None => return Err(format!("Array index {} out of bounds (length: {})", at, items.len())),
                    },
                    Value::Dict(entries) => match entries.get(at) {
                        Some((k, x)) => if wants_key { k.clone() } else { x.clone() },
                        None => return Err(format!("Array index {} out of bounds (length: {})", at, entries.len())),
                    },
                    // A thing keeps named values too, and walking it
                    // walks those, in the order they were written.
                    Value::Thing(thing) => {
                        let holds = thing.holds.borrow();
                        match holds.get(at) {
                            // What a member is called, not the name it
                            // is filed under: a class holding one alone
                            // files it under its own name as well.
                            Some((k, x)) => if wants_key { Value::text(crate::data::holder_of(k).0) } else { x.clone() },
                            None => return Err(format!("Array index {} out of bounds (length: {})", at, holds.len())),
                        }
                    }
                    _ => return Err("Cannot walk a value that is not an array".to_string()),
                }
            }
            Prim::Extent => {
                n(1)?;
                match &v[0] {
                    Value::Progression(walk) => Value::from_big(walk.count()),
                    Value::Vector(items) => Value::Small(items.len() as i64),
                    Value::Dict(entries) => Value::Small(entries.len() as i64),
                    Value::Thing(thing) => Value::Small(thing.holds.borrow().len() as i64),
                    // A language with a word for a warning hears that a
                    // value cannot be walked and walks it no times,
                    // rather than having the run stopped over it.
                    other if self.complaint_words.iter().any(|(k, _)| *k == "warning") => {
                        let told = format!("foreach() argument must be of type array|object, {} given", self.kind_called(other));
                        self.grumble("warning", &told);
                        Value::Small(0)
                    }
                    _ => return Err("Cannot walk a value that is not an array".to_string()),
                }
            }
            Prim::AtEnd => return Err("An empty index belongs on the left of an assignment".to_string()),
            // A place in text has room for one letter, so a write there
            // is worth that letter and not the whole of what was handed
            // over. Where more was handed over the language says so, and
            // where nothing was there is nothing to put.
            Prim::Letter => {
                n(2)?;
                match &v[1] {
                    Value::Text(_) if self.letter_places => {
                        let handed = v[0].render(w);
                        let mut spelling = handed.chars();
                        let Some(first) = spelling.next() else {
                            return Err("Cannot write nothing into a place in text".to_string());
                        };
                        if spelling.next().is_some() {
                            if let Some(said) = self.table.single("ext.op.index.text.first") {
                                self.grumble("warning", said);
                            }
                        }
                        Value::text(&first.to_string())
                    }
                    _ => v[0].clone(),
                }
            }
            // The steps of a walk that a thing may answer for itself are
            // worked out where a call can be made, not here.
            Prim::Walked | Prim::AloneWalk | Prim::MoreYet | Prim::AtHand | Prim::NamedHere | Prim::StepOn | Prim::PastHeld => {
                return Err(format!("{}() is worked out where a call can be made", name))
            }
            Prim::Kept => {
                n(3)?;
                match (&v[0], as_index(&v[1])) {
                    // A thing that is its own walk hands out what it
                    // pleases: what it holds is none of the walk's work.
                    (Value::Thing(_), _) if self.walks_itself(&v[0]).is_some() => Value::Flag(true),
                    (Value::Thing(thing), Ok(at)) => {
                        let here = match &v[2] {
                            Value::Text(named) => Some(named.to_string()),
                            _ => None,
                        };
                        let holds = thing.holds.borrow();
                        let reaches = |filed: &str| match crate::data::holder_of(filed) {
                            // What a class holds alone is that class's
                            // business and no other's.
                            (_, Some(owner)) => here.as_deref() == Some(owner),
                            (called, None) => match thing.of.reach_of(called) {
                                Some((Reach::Within, holder)) => self.along_with(holder, here.as_deref()),
                                _ => true,
                            },
                        };
                        Value::Flag(holds.get(at).map_or(true, |(k, x)| kept(x) && reaches(k)))
                    }
                    _ => Value::Flag(true),
                }
            }
            Prim::BringModule => {
                let path = v[0].bare();
                let namespace = self.load_namespace(&path)?;
                match &v[1] {
                    Value::Text(wanted) => self.namespace_item(&namespace, &path, wanted)?,
                    _ if matches!(v[2], Value::Flag(true)) => self.load_namespace(path.split('.').next().unwrap_or(&path))?,
                    _ => namespace,
                }
            }
            Prim::SpreadModule => {
                if let Value::Thing(namespace) = &v[0] {
                    let names = namespace.holds.borrow().clone();
                    for (name, entry) in names {
                        if name.starts_with('_') || !self.table.name_like(&name) { continue; }
                        let worth = if let Value::Shared(cell) = entry { cell.borrow().clone() } else { entry };
                        if matches!(worth, Value::Unset) { continue; }
                        let slot = match self.idents.iter().position(|word| word == &name) {
                            Some(at) => at,
                            None => { self.idents.push(name); self.idents.len() - 1 }
                        };
                        let mut cells = self.outermost.cells.borrow_mut();
                        cells.resize(self.idents.len(), Value::Unset);
                        cells[slot] = worth;
                    }
                }
                Value::Nil
            }
            Prim::FaultHeld => {
                if v.len() > 1 { return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()); }
                let pair = if let Some(value) = v.first().or_else(|| self.holding_fault.last()) {
                    if let Value::Thing(thing) = value {
                        let holds = thing.holds.borrow();
                        let said = holds.iter().find_map(|(key, held)| (key == "message").then(|| held.clone())).unwrap_or(Value::Nil);
                        vec![Value::text(&thing.of.name), said]
                    } else { vec![Value::Nil, value.clone()] }
                } else { vec![Value::Nil; 2] };
                Value::Vector(Rc::new(pair))
            }
            Prim::HostFacts => {
                n(0)?;
                let mut facts = vec![];
                facts.push(match std::env::current_dir() {
                    Err(_) => Value::Nil,
                    Ok(directory) => Value::text(&directory.to_string_lossy()),
                });
                facts.extend([Value::text(std::env::consts::OS), Value::text(std::env::consts::ARCH)]);
                let mut variables = Vec::new();
                for (key, value) in std::env::vars_os() {
                    variables.push((Value::text(&key.to_string_lossy()), Value::text(&value.to_string_lossy())));
                }
                facts.push(Value::Dict(Rc::new(variables)));
                Value::Vector(Rc::new(facts))
            }
            Prim::FileSort => {
                n(1)?;
                let metadata = std::fs::metadata(v[0].render(w));
                Value::Small(match metadata {
                    Ok(details) if details.is_dir() => 2,
                    Ok(details) if details.is_file() => 1,
                    _ => 0,
                })
            }
            Prim::LoadModule => { n(1)?; self.load_namespace(&v[0].bare())? }
            Prim::MakeHeir => {
                n(3)?;
                let title = match &v[0] { Value::Text(word) => word.to_string(), _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()) };
                let ancestor = match &v[1] { Value::Blueprint(old) => old.clone(), _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()) };
                let members = match &v[2] { Value::Dict(pairs) => pairs, _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()) };
                let mut holdings = Vec::with_capacity(members.len());
                for pair in members.iter() {
                    match &pair.0 {
                        Value::Text(key) => holdings.push((key.to_string(), pair.1.clone())),
                        _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()),
                    }
                }
                let heir = Blueprint {
                    under: Some(ancestor), name: title, shared: RefCell::new(holdings),
                    methods: vec![], constants: vec![], reaches: vec![], answers: vec![], fields: vec![],
                };
                Value::Blueprint(Rc::new(heir))
            }
            Prim::KeepWorth => {
                n(1)?;
                let tree = self.keep_worth(&v[0], &mut Vec::new(), 0)?;
                let envelope = serde_json::json!(["LMP1", tree]);
                Value::text(&envelope.to_string())
            }
            Prim::RestoreWorth => {
                n(1)?;
                let source = match &v[0] { Value::Text(s) => s, _ => return Err(self.keeping_fault(1)) };
                let envelope = serde_json::from_str::<serde_json::Value>(source).map_err(|_| self.keeping_fault(1))?;
                if envelope.as_array().map_or(true, |a| a.len() != 2) || envelope[0] != "LMP1" { return Err(self.keeping_fault(1)); }
                self.restore_worth(&envelope[1], &mut vec![], 0)?
            }
            Prim::CopyWorth => {
                n(2)?;
                let deep = matches!(v[1], Value::Flag(true));
                let hooks = self.table.strings("ext.builtin.copy.hooks");
                if let Value::Thing(thing) = &v[0] {
                    if let Some(word) = hooks.get(if deep { 1 } else { 0 }) {
                        let handed = if deep { vec![Value::Dict(Rc::new(vec![]))] } else { Vec::new() };
                        if let Some(answer) = self.state_answer(thing, word, handed)? { return Ok(answer); }
                    }
                }
                let mut known = Vec::new();
                self.copy_worth(&v[0], deep, &mut known)
            }
            Prim::CallResult => {
                n(1)?;
                let call = Form::Apply(Callee::Code(Box::new(Form::Const(v[0].clone()))), Vec::new());
                let outer = self.outermost.clone();
                let mut items = Vec::with_capacity(3);
                match self.value_of(&call, &outer) {
                    Ok(value) => items.extend([Value::Flag(true), value, Value::text("")]),
                    Err(Escape::Error(told)) => items.extend([Value::Flag(false), Value::text(&told), Value::text(&told)]),
                    Err(Escape::Thrown(value)) => {
                        let message = value.render(self.wording());
                        items.extend([Value::Flag(false), value, Value::text(&message)]);
                    }
                    Err(escape) => { self.got_away = Some(escape); return Err("the call ended the run".into()); }
                }
                Value::Vector(Rc::new(items))
            }
            Prim::ProgramNames => {
                n(0)?;
                let cells = self.outermost.cells.borrow();
                let mut bindings = Vec::new();
                for (word, value) in self.idents.iter().zip(cells.iter()) {
                    if !word.starts_with('\0') && !word.contains('\0') && !matches!(value, Value::Unset) {
                        bindings.push((Value::text(word), value.clone()));
                    }
                }
                Value::Dict(Rc::new(bindings))
            }
            Prim::ReadMember => {
                if v.len() < 2 || v.len() > 3 { return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()); }
                let word = v[1].bare();
                let found = self.attribute(&v[0], &word);
                let found = if found.is_none() { self.ask_namespace(&v[0], &word)? } else { found };
                found.or_else(|| v.get(2).cloned()).ok_or_else(|| {
                    let (opening, ending) = self.table.around("ext.builtin.member.absent").unwrap_or(("", ""));
                    format!("{opening}{word}{ending}")
                })?
            }
            Prim::WriteMember => {
                n(3)?;
                let key = v[1].bare();
                let places = match &v[0] {
                    Value::Thing(object) => &object.holds,
                    Value::Blueprint(class) => &class.shared,
                    _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()),
                };
                let mut held = places.borrow_mut();
                match held.iter_mut().find(|(word, _)| word == &key) {
                    None => held.push((key, v[2].clone())),
                    Some((_, Value::Shared(cell))) => *cell.borrow_mut() = v[2].clone(),
                    Some((_, old)) => *old = v[2].clone(),
                }
                Value::Nil
            }
            Prim::IsInstance => { n(2)?; Value::Flag(belongs_to(&v[0], &v[1])) }
            Prim::LinesOfText => {
                if v.len() != 1 && v.len() != 2 { return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().into()); }
                let text = match &v[0] { Value::Text(chars) => chars, _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().into()) };
                let with_endings = v.get(1).map_or(false, |flag| flag.is_true());
                let chars: Vec<char> = text.chars().collect();
                let mut result = Vec::new();
                let (mut begin, mut cursor) = (0, 0);
                while cursor < chars.len() {
                    let stops = [10, 13, 11, 12, 28, 29, 30, 133, 8232, 8233].contains(&u32::from(chars[cursor]));
                    if stops {
                        let end = cursor;
                        cursor += 1;
                        if chars[end] == '\r' && chars.get(cursor) == Some(&'\n') { cursor += 1; }
                        let bound = if with_endings { cursor } else { end };
                        result.push(Value::text(&chars[begin..bound].iter().collect::<String>()));
                        begin = cursor;
                    } else { cursor += 1; }
                }
                if begin != chars.len() { result.push(Value::text(&chars[begin..].iter().collect::<String>())); }
                Value::Vector(Rc::new(result))
            }
            Prim::IsDictionary => { n(1)?; Value::Flag(if let Value::Dict(_) = &v[0] { true } else { false }) }
            Prim::HasMember | Prim::ResolvesMember => {
                n(2)?;
                let word = v[1].bare();
                let (class, own) = match &v[0] {
                    Value::Thing(o) => (Some(&o.of), self.member_place(&o.holds.borrow(), &word).is_some()),
                    Value::Blueprint(c) => (Some(c), false),
                    _ => (None, false),
                };
                Value::Flag(own || (matches!(op, Prim::ResolvesMember) && self.namespace_answers(&v[0])) || class.map_or(false, |c| c.keeper(&word).is_some() || c.program(&word).is_some() || c.constant(&word).is_some()))
            }
            Prim::Of => {
                n(2)?;
                let called = v[1].bare();
                if self.table.flag("ext.op.member.pipes") {
                    if let Some(found) = self.attribute(&v[0], &called) { return Ok(found); }
                    if let Some(answer) = self.ask_namespace(&v[0], &called)? { return Ok(answer); }
                }
                match &v[0] {
                    Value::Thing(thing) => {
                        let found = {
                            let holds = thing.holds.borrow();
                            self.member_place(&holds, &called).map(|at| holds[at].1.clone())
                        };
                        match found {
                            // A property kept in a shared cell reads as
                            // what the cell holds; the sharing lies
                            // between the names, not in the value.
                            Some(Value::Shared(cell)) => cell.borrow().clone(),
                            Some(x) => x,
                            // A language with a word for a warning says
                            // a property is not there, and reads nothing.
                            None if self.complaint_words.iter().any(|(k, _)| *k == "warning") => {
                                self.grumble("warning", &format!("Undefined property: {}::${}", thing.of.name, called));
                                Value::Nil
                            }
                            None => return Err(format!("Undefined property: {}::${}", thing.of.name, called)),
                        }
                    }
                    other => return Err(format!("Cannot read property '{}' of {}", called, other.bare())),
                }
            }
            Prim::Pluck => {
                n(2)?;
                let called = v[1].bare();
                match &v[0] {
                    Value::Thing(thing) => {
                        // The place itself stays, emptied. A walk under
                        // way counts places, and closing one up would
                        // draw every later member back a step beneath it.
                        let mut holds = thing.holds.borrow_mut();
                        let place = self.member_place(&holds, &called);
                        if self.table.has_any("ext.stmt.del") && place.map_or(true, |at| matches!(holds[at].1, Value::Unset)) {
                            return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string());
                        }
                        if let Some(at) = place { holds[at].1 = Value::Unset; }
                        Value::Nil
                    }
                    other => return Err(format!("Cannot take property '{}' off {}", called, other.bare())),
                }
            }
            Prim::Onto => {
                n(3)?;
                let called = v[1].bare();
                if self.table.flag("ext.op.member.pipes") {
                    if let Value::Blueprint(class) = &v[0] {
                        let mut own = class.shared.borrow_mut();
                        match own.iter_mut().find(|(n, _)| n == &called) {
                            Some((_, value)) => *value = v[2].clone(),
                            None => own.push((called, v[2].clone())),
                        }
                        return Ok(Value::Nil);
                    }
                }
                match &v[0] {
                    Value::Thing(thing) => {
                        let mut holds = thing.holds.borrow_mut();
                        match self.member_place(&holds, &called) {
                            Some(at) => match &holds[at].1 {
                                Value::Shared(cell) => *cell.borrow_mut() = v[2].clone(),
                                _ => holds[at].1 = v[2].clone(),
                            },
                            None => holds.push((called, v[2].clone())),
                        }
                        Value::Nil
                    }
                    other => return Err(format!("Cannot write property '{}' of {}", called, other.bare())),
                }
            }
            Prim::Within => {
                n(2)?;
                let called = v[1].bare();
                let stands = self.class_it_spells(v[0].clone());
                match &stands {
                    Value::Blueprint(class) => match class.constant(&called) {
                        Some(x) => x.clone(),
                        None => match class.keeper(&called) {
                            Some(keeper) => {
                                let held = keeper.shared.borrow().iter().find(|(k, _)| *k == called).map(|(_, x)| x.clone());
                                // Kept in a shared cell, it reads as
                                // what the cell holds: the sharing is
                                // between the names, not in the value.
                                match held.expect("the keeper holds it") {
                                    Value::Shared(cell) => cell.borrow().clone(),
                                    held => held,
                                }
                            }
                            None => return Err(format!("Undefined constant {}::{}", class.name, called)),
                        },
                    },
                    other => return Err(self.class_lacking(other).unwrap_or_else(|| format!("Cannot reach '{}' in {}", called, other.bare()))),
                }
            }
            Prim::Into => {
                n(3)?;
                let called = v[1].bare();
                let stands = self.class_it_spells(v[0].clone());
                match &stands {
                    Value::Blueprint(class) => {
                        let keeper = class.keeper(&called).unwrap_or(class);
                        let mut shared = keeper.shared.borrow_mut();
                        match shared.iter_mut().find(|(k, _)| *k == called) {
                            Some(place) => place.1 = v[2].clone(),
                            None => shared.push((called, v[2].clone())),
                        }
                        Value::Nil
                    }
                    other => return Err(self.class_lacking(other).unwrap_or_else(|| format!("Cannot write '{}' in {}", called, other.bare()))),
                }
            }
            Prim::Akin => {
                n(2)?;
                let against = match &v[1] {
                    Value::Thing(thing) => thing.of.name.clone(),
                    Value::Blueprint(class) => class.name.clone(),
                    other => other.bare(),
                };
                match &v[0] {
                    Value::Thing(thing) => Value::Flag(thing.of.goes_by(&against, self.classes_either_way)),
                    _ => Value::Flag(false),
                }
            }
            Prim::Named => {
                n(1)?;
                match &v[0] {
                    Value::Thing(thing) => Value::text(&thing.of.name),
                    Value::Blueprint(class) => Value::text(&class.name),
                    other => return Err(format!("{} has no class name", other.bare())),
                }
            }
            // A language that makes a place on writing into it finds an
            // array where nothing at all was there.
            Prim::Added | Prim::Placed
                if self.builds_places && matches!(v.first(), Some(Value::Nil) | Some(Value::Unset)) =>
            {
                let mut made = v.to_vec();
                made[0] = Value::Vector(Rc::new(Vec::new()));
                self.prim(op, name, &made)?
            }
            Prim::Added => {
                n(2)?;
                match &v[0] {
                    Value::Vector(items) => {
                        let mut all = items.as_ref().clone();
                        all.push(v[1].clone());
                        Value::Vector(Rc::new(all))
                    }
                    Value::Dict(entries) => {
                        let mut all = entries.as_ref().clone();
                        let key = Value::Small(after_keys(&all));
                        all.push((key, v[1].clone()));
                        Value::Dict(Rc::new(all))
                    }
                    _ => return Err(self.no_places()),
                }
            }
            Prim::Placed => {
                n(3)?;
                match &v[0] {
                    Value::Text(had) if self.letter_places => {
                        let (made, over) = letter_put(had, &v[1], &v[2].render(w))?;
                        if over {
                            if let Some(said) = self.table.single("ext.op.index.text.first") {
                                self.grumble("warning", said);
                            }
                        }
                        made
                    }
                    Value::Vector(items) if as_index(&v[1]).map_or(false, |at| at < items.len()) => {
                        let mut all = items.as_ref().clone();
                        all[as_index(&v[1])?] = v[2].clone();
                        Value::Vector(Rc::new(all))
                    }
                    Value::Vector(items) => {
                        let mut all: Vec<(Value, Value)> =
                            items.iter().enumerate().map(|(at, x)| (Value::Small(at as i64), x.clone())).collect();
                        set_key(&mut all, v[1].clone(), v[2].clone());
                        Value::Dict(Rc::new(all))
                    }
                    Value::Dict(entries) => {
                        let mut all = entries.as_ref().clone();
                        set_key(&mut all, v[1].clone(), v[2].clone());
                        Value::Dict(Rc::new(all))
                    }
                    _ => return Err(self.no_places()),
                }
            }
            Prim::Gather => return Err(format!("{}() is a literal, not a call", name)),
            // Source read while the program runs, built against the
            // globals it already has and run where it stands. A file
            // that cannot be read answers false, as such a language says.
            Prim::Weigh | Prim::Bring | Prim::BringOnce => {
                // A source read in stands as a call of its own, so that
                // a fault raised in the reading, or by what it reads,
                // names the reading among the calls it stood under.
                let from_library = self.calls.last().map_or(false, |c| c.of_library && !self.stands_for_the_run(&c.named));
                self.calls.push(Called {
                    named: Rc::from(name),
                    within: None,
                    from: self.written_in.clone(),
                    on: self.row,
                    handed_at: None,
                    of_library: false,
                    from_library,
                });
                let done = self.read_in(op, name, v);
                // A complaint the reading raised is handed over while
                // the reading still stands, so whatever takes it sees
                // the reading and not what came after.
                if self.any_unheard.get() {
                    if let Err(over) = self.hand_over_unheard() {
                        self.got_away = Some(over);
                    }
                }
                self.calls.pop();
                return done;
            }
            // Reaching outside the run, which only a language that
            // spells these labels does at all. What cannot be done
            // answers false rather than stopping the run.
            Prim::Slurp => {
                n(1)?;
                let w = self.wording();
                match std::fs::read(v[0].render(w)) {
                    Ok(bytes) => Value::text(&String::from_utf8_lossy(&bytes)),
                    Err(_) => Value::Flag(false),
                }
            }
            Prim::Spill => {
                n(2)?;
                let w = self.wording();
                let (place, what) = (v[0].render(w), v[1].render(w));
                match std::fs::write(place, self.table.raw_of(&what)) {
                    Ok(()) => Value::Small(what.len() as i64),
                    Err(_) => Value::Flag(false),
                }
            }
            Prim::There => {
                n(1)?;
                let w = self.wording();
                Value::Flag(std::path::Path::new(&v[0].render(w)).exists())
            }
            Prim::Gone => {
                n(1)?;
                let w = self.wording();
                Value::Flag(std::fs::remove_file(v[0].render(w)).is_ok())
            }
            // The host's shell, handed a command and asked afterwards
            // for all it put where a run puts what it writes. Both ways
            // the words travel as the bytes their text stands for: the
            // command as it goes out, the answer as it is read back in,
            // with whatever break of line ended it left where it fell.
            // Anything the shell said in complaint goes out beside this
            // run's own complaints and is not gathered. Where the shell
            // put out nothing whatever the answer is nothing; where no
            // shell could be started the answer is false.
            Prim::Shelled => {
                n(1)?;
                let w = self.wording();
                let order = self.table.raw_of(&v[0].render(w));
                // Whatever this run has written already is let go first,
                // so the shell's words fall after it and not before.
                {
                    use std::io::Write as _;
                    let _ = std::io::stdout().flush();
                }
                let spelled = <std::ffi::OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(&order);
                let mut shell = std::process::Command::new(THE_HOSTS_SHELL);
                shell.args([std::ffi::OsStr::new(A_COMMAND_FOLLOWS), spelled]);
                shell.stderr(std::process::Stdio::inherit());
                match shell.output() {
                    Ok(said) if !said.stdout.is_empty() => Value::text(&self.table.said_of(&said.stdout)),
                    Ok(_) => Value::Nil,
                    Err(_) => Value::Flag(false),
                }
            }
            // Reaching another host: the connection is made, what was
            // handed over goes on it whole, and what comes back is read
            // until the far end shuts it. A request that asks for the
            // connection to be shut wants no more than that.
            // Biding a while, counted in millionths of a second. Without
            // it a run watching for something to come ready spins as
            // fast as it can and takes the first no for the last one.
            Prim::Bided => {
                n(1)?;
                let tiny = as_index(&v[0])?;
                std::thread::sleep(std::time::Duration::from_micros(tiny as u64));
                Value::Nil
            }
            Prim::Reached => {
                n(4)?;
                use std::io::{Read as _, Write as _};
                let w = self.wording();
                let (host, port) = (v[0].render(w), as_index(&v[1])?);
                let carried = self.table.raw_of(&v[2].render(w));
                let patience = match as_index(&v[3])? {
                    0 => None,
                    wait => Some(std::time::Duration::from_secs(wait as u64)),
                };
                let door = format!("{}:{}", host, port);
                let opened = match patience {
                    None => std::net::TcpStream::connect(&door),
                    Some(waiting) => match std::net::ToSocketAddrs::to_socket_addrs(&door) {
                        Err(e) => Err(e),
                        Ok(mut every) => match every.next() {
                            None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, "nowhere")),
                            Some(one) => std::net::TcpStream::connect_timeout(&one, waiting),
                        },
                    },
                };
                let spoken = |mut line: std::net::TcpStream| -> std::io::Result<Vec<u8>> {
                    line.set_write_timeout(patience)?;
                    line.set_read_timeout(patience)?;
                    line.write_all(&carried)?;
                    line.flush()?;
                    let mut back = Vec::new();
                    line.read_to_end(&mut back)?;
                    Ok(back)
                };
                match opened.and_then(spoken) {
                    Err(_) => Value::Flag(false),
                    Ok(back) => Value::text(&self.table.said_of(&back)),
                }
            }
            // A program raised alongside this one. Nothing it writes is
            // kept: it is raised to answer on a connection, and what it
            // would otherwise say is let fall.
            Prim::Raised => {
                n(3)?;
                use std::os::unix::ffi::OsStrExt as _;
                let w = self.wording();
                let named = self.table.raw_of(&v[0].render(w));
                let mut raising = std::process::Command::new(std::ffi::OsStr::from_bytes(&named));
                if let Value::Vector(words) = &v[1] {
                    for word in words.iter() {
                        let word = self.table.raw_of(&word.render(w));
                        raising.arg(std::ffi::OsStr::from_bytes(&word));
                    }
                }
                if let Value::Dict(pairs) = &v[2] {
                    for (called, worth) in pairs.iter() {
                        let called = self.table.raw_of(&called.render(w));
                        let worth = self.table.raw_of(&worth.render(w));
                        raising.env(std::ffi::OsStr::from_bytes(&called), std::ffi::OsStr::from_bytes(&worth));
                    }
                }
                raising.stdin(std::process::Stdio::null());
                raising.stdout(std::process::Stdio::null());
                raising.stderr(std::process::Stdio::null());
                match raising.spawn() {
                    Err(_) => Value::Flag(false),
                    Ok(begun) => {
                        let mark = begun.id() as i64;
                        self.alongside.insert(mark, begun);
                        Value::Small(mark)
                    }
                }
            }
            Prim::Laid => {
                n(1)?;
                let mark = as_index(&v[0])? as i64;
                match self.alongside.remove(&mark) {
                    None => Value::Flag(false),
                    Some(mut begun) => {
                        let _ = begun.kill();
                        let _ = begun.wait();
                        Value::Flag(true)
                    }
                }
            }
            Prim::Clock => {
                n(1)?;
                self.allowed = as_index(&v[0])?;
                self.started = Some(std::time::Instant::now());
                Value::Flag(true)
            }
            // What the run has taken of the host's room, as the tally
            // beside the allocator has it: what stands taken now, the
            // highest it ever stood at, and that highest reading thrown
            // away so it is gathered again from this moment on.
            Prim::RoomHeld => {
                n(0)?;
                Value::Small(lumen_room::used() as i64)
            }
            Prim::RoomHighest => {
                n(0)?;
                Value::Small(lumen_room::most() as i64)
            }
            Prim::RoomAnew => {
                n(0)?;
                lumen_room::forget_most();
                Value::Nil
            }
            // How much room the run may take from here on.
            Prim::RoomMark => {
                n(1)?;
                self.ceiling = as_index(&v[0])?;
                Value::Flag(true)
            }
            // Words said as a complaint of a kind the language names,
            // where the run stands.
            // The calls under way, as the program may read them: each
            // what it called, the class it stands in, where the call
            // itself is written, and what it was handed.
            Prim::Under => {
                n(0)?;
                let mut told = Vec::new();
                for call in self.calls.iter().rev() {
                    if self.stands_for_the_run(&call.named) || self.library_alone(call) {
                        continue;
                    }
                    // A call the library made stands nowhere the program
                    // was written, so it is told without a place.
                    let mut pairs = Vec::new();
                    if !call.from_library {
                        pairs.push((Value::text("file"), Value::text(&call.from)));
                        pairs.push((Value::text("line"), Value::Small(call.on as i64)));
                    }
                    pairs.push((Value::text("function"), Value::text(&call.named)));
                    if let Some(class) = &call.within {
                        pairs.push((Value::text("class"), Value::text(class)));
                    }
                    let handed = self.handed_to(call).unwrap_or_default().to_vec();
                    pairs.push((Value::text("args"), Value::Vector(Rc::new(handed))));
                    told.push(Value::Dict(Rc::new(pairs)));
                }
                Value::Vector(Rc::new(told))
            }
            Prim::Complain => {
                n(2)?;
                let w = self.wording();
                let named = v[0].render(w).to_string();
                let said = v[1].render(w).to_string();
                match self.complaint_words.iter().find(|(_, word)| *word == named).map(|(k, _)| k.to_string()) {
                    Some(kind) => {
                        self.grumble(&kind, &said);
                        Value::Flag(true)
                    }
                    None => Value::Flag(false),
                }
            }
            // The routine every complaint is to be handed to, or none.
            // What the run has bound under a name, by name: the
            // classes, and the routines.
            Prim::ClassesBound | Prim::RoutinesBound => {
                n(0)?;
                let cells = self.outermost.cells.borrow();
                let mut named = Vec::new();
                for (at, held) in cells.iter().enumerate() {
                    let wanted = match op {
                        Prim::ClassesBound => matches!(held, Value::Blueprint(_)),
                        _ => matches!(held, Value::Bound(..) | Value::Routine(_)),
                    };
                    if wanted {
                        // A class is filed under more than its name, so
                        // what it is filed under is taken off again.
                        if let Some(name) = self.idents.get(at) {
                            named.push(Value::text(name.trim_end_matches(crate::form::OF_A_CLASS)));
                        }
                    }
                }
                Value::Vector(Rc::new(named))
            }
            // The words the language has of its own, by name: what a
            // program may call though nobody wrote them.
            Prim::Carry => return Err("A routine takes names away where it is written and nowhere else".to_string()),
            Prim::WordsSpelled => {
                n(0)?;
                let mut words: Vec<String> = self.table.prims.keys().cloned().collect();
                words.sort();
                Value::Vector(Rc::new(words.iter().map(|w| Value::text(w)).collect()))
            }
            // What a class answers to, by name: the methods written in
            // it, or the properties its things carry, its own coming
            // before those of the class it is built on. A thing is asked
            // of the class that made it.
            Prim::ClassMethods | Prim::ClassProperties => {
                n(1)?;
                let of = match self.class_it_spells(v[0].clone()) {
                    Value::Thing(thing) => Some(thing.of.clone()),
                    Value::Blueprint(class) => Some(class),
                    _ => None,
                };
                let mut gathered: Vec<String> = Vec::new();
                let mut here = of;
                while let Some(class) = here {
                    let names: Vec<String> = match op {
                        Prim::ClassMethods => class.methods.iter().map(|(called, _)| called.clone()).collect(),
                        _ => class.fields.iter().map(|(called, _)| crate::data::holder_of(called).0.to_string()).collect(),
                    };
                    for called in names {
                        if !gathered.iter().any(|held| *held == called) {
                            gathered.push(called);
                        }
                    }
                    here = class.under.clone();
                }
                Value::Vector(Rc::new(gathered.iter().map(|called| Value::text(called)).collect()))
            }
            // The class a class stands on, by name: a thing is asked of
            // the class it is of. Nothing where it stands on none.
            Prim::ClassBeneath => {
                n(1)?;
                let of = match self.class_it_spells(v[0].clone()) {
                    Value::Thing(thing) => Some(thing.of.clone()),
                    Value::Blueprint(class) => Some(class),
                    _ => None,
                };
                match of.and_then(|c| c.under.clone()) {
                    Some(under) => Value::text(&under.name),
                    None => Value::Nil,
                }
            }
            // How far the clock the system keeps has come since the
            // year it counts from. A clock that will not answer counts
            // as standing at the start of it.
            Prim::SinceEpoch => {
                if v.len() == 1 && self.table.flag("ext.builtin.clock.parts") {
                    let steady = match v.first() {
                        Some(Value::Flag(choice)) => *choice,
                        _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()),
                    };
                    let elapsed = if steady {
                        static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                        let origin = ORIGIN.get_or_init(std::time::Instant::now);
                        origin.elapsed()
                    } else {
                        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default()
                    };
                    let digits = self.table.count("ext.system.real.digits").unwrap_or(15);
                    let mut answer = crate::data::worth_of_binary(elapsed.as_secs_f64(), digits);
                    if let Value::Frac(ratio) = &mut answer { Rc::make_mut(ratio).float_style = true; }
                    return Ok(answer);
                }
                n(0)?;
                let gone = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
                Value::Small(gone.map_or(0, |since| since.as_secs() as i64))
            }
            // The roots, the curves and the angles, worked over the
            // reals of the width and named by the first worth handed
            // over. They are worked at the width and nowhere else: a
            // root is hardly ever a ratio, and holding one exactly
            // would mean choosing beforehand how many figures to keep.
            Prim::Reckon => {
                let Some(working) = v.first().map(|x| x.render(w)) else {
                    return Err(format!("{}() wants the name of a working first of all", name));
                };
                let takes = math::worked_takes(&working);
                if v.len() != takes + 1 {
                    return Err(format!("{}('{}') expects {} argument(s) after the name, got {}", name, working, takes, v.len() - 1));
                }
                let width = |at: usize| -> f64 {
                    let worth = self.worth_of(&v[at]);
                    // A nought under nought is a nought of its own at
                    // the width, and some of these workings answer
                    // differently for it, so the minus is put back.
                    if let Value::Frac(e) = &worth {
                        if e.under && num_traits::Zero::is_zero(&e.above) {
                            return -0.0;
                        }
                    }
                    match math::ratio_of(&worth) {
                        Some(r) => crate::data::nearest_binary(&r.above, &r.beneath),
                        None => f64::NAN,
                    }
                };
                let two = match takes {
                    2 => width(2),
                    _ => 0.0,
                };
                match math::worked(&working, width(1), two) {
                    Some(got) => {
                        let mut result = crate::data::worth_of_binary(got, self.real_figures());
                        if let Value::Frac(number) = &mut result { Rc::make_mut(number).float_style = self.table.flag("ext.builtin.math.floating"); }
                        result
                    },
                    None => return Err(format!("{}(): there is no working called '{}'", name, working)),
                }
            }
            Prim::OutBegun => {
                n(0)?;
                Value::Flag(self.written_out.get())
            }
            Prim::Untaken => {
                let put = v.first().cloned().unwrap_or(Value::Nil);
                let none = matches!(put, Value::Nil | Value::Unset);
                *self.untaken.borrow_mut() = if none { None } else { Some(put) };
                Value::Flag(true)
            }
            Prim::Hearer => {
                let put = v.first().cloned().unwrap_or(Value::Nil);
                let none = matches!(put, Value::Nil | Value::Unset);
                *self.hearer.borrow_mut() = if none { None } else { Some(put) };
                Value::Flag(true)
            }
            // A routine to run once the run is over, with whatever else
            // was given standing as its arguments.
            Prim::Afterward => {
                if v.is_empty() {
                    return Err(format!("{}() needs a program to run", name));
                }
                let mut given = v.to_vec();
                let work = given.remove(0);
                self.afterward.borrow_mut().push((work, given));
                Value::Nil
            }
            // Keeping what the run writes out, and giving it up again.
            Prim::KeepOut => {
                self.holding.borrow_mut().push(String::new());
                Value::Flag(true)
            }
            Prim::KeptOut => match self.holding.borrow().last() {
                Some(kept) => Value::text(kept),
                None => Value::Flag(false),
            },
            Prim::LooseOut => {
                let held = self.holding.borrow_mut().pop();
                Value::Flag(held.is_some())
            }
            Prim::DeepOut => Value::Small(self.holding.borrow().len() as i64),
            Prim::Handed | Prim::HowMany | Prim::HandedAt => {
                let Some(handed) = self.handed.last().cloned() else {
                    // What a language says when one of these is reached
                    // where no call is running, in its own words where it
                    // has them.
                    let label = match op {
                        Prim::Handed => "ext.builtin.args.all.outside",
                        Prim::HowMany => "ext.builtin.args.count.outside",
                        _ => "ext.builtin.args.at.outside",
                    };
                    let said = self.table.single(label).map(str::to_string);
                    return Err(said.unwrap_or_else(|| format!("{}() belongs inside a function", name)));
                };
                match op {
                    Prim::Handed => {
                        n(0)?;
                        Value::Vector(Rc::new(handed.clone()))
                    }
                    Prim::HowMany => {
                        n(0)?;
                        Value::Small(handed.len() as i64)
                    }
                    _ => {
                        n(1)?;
                        // A place before the first and one past the last
                        // are each told in the language's own words where
                        // it has them.
                        if matches!(&v[0], Value::Small(k) if *k < 0) {
                            let said = self.table.single("ext.builtin.args.at.below").map(str::to_string);
                            return Err(said.unwrap_or_else(|| format!("{}(): the place asked for comes before the first", name)));
                        }
                        let at = as_index(&v[0])?;
                        match handed.get(at) {
                            Some(x) => x.clone(),
                            None => {
                                let said = self.table.single("ext.builtin.args.at.beyond").map(str::to_string);
                                return Err(said.unwrap_or_else(|| format!("{}(): nothing was handed over at {}", name, at)))
                            }
                        }
                    }
                }
            }
            // A language that asks equality loosely ranks loosely too,
            // so that ranking two values and asking which comes first
            // give answers that agree.
            Prim::Rank if self.loose_equals => {
                n(2)?;
                let alike = self.prim(Prim::Eq, name, v)?.is_true();
                Value::Small(if alike {
                    0
                } else if self.comes_first(&v[0], &v[1])? {
                    -1
                } else {
                    1
                })
            }
            Prim::Rank => {
                n(2)?;
                // Numbers by their order, anything else by its text.
                match (math::below(&v[0], &v[1]), math::below(&v[1], &v[0])) {
                    (Some(true), _) => Value::Small(-1),
                    (_, Some(true)) => Value::Small(1),
                    (Some(false), Some(false)) => Value::Small(0),
                    _ => {
                        let (x, y) = (v[0].bare(), v[1].bare());
                        Value::Small(if x < y { -1 } else if x > y { 1 } else { 0 })
                    }
                }
            }
            Prim::Erase => {
                n(2)?;
                match &v[0] {
                    Value::Vector(items) if self.table.has_any("ext.stmt.del") => {
                        let offset = (match &v[1] { Value::Flag(b) => Some(if *b { 1 } else { 0 }), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None }).ok_or_else(|| self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string())?;
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string().into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, v)| v.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) => {
                        let i = as_index(&v[1])?;
                        let kept: Vec<(Value, Value)> = items
                            .iter()
                            .enumerate()
                            .filter(|(at, _)| *at != i)
                            .map(|(at, x)| (Value::Small(at as i64), x.clone()))
                            .collect();
                        Value::Dict(Rc::new(kept))
                    }
                    Value::Dict(entries) => {
                        let at = self.as_key(&v[1]);
                        if self.table.has_any("ext.stmt.del") && entries.iter().all(|entry| !entry.0.equals(&at)) {
                            return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string());
                        }
                        let kept: Vec<(Value, Value)> = entries.iter().filter(|(k, _)| !k.equals(&at)).cloned().collect();
                        Value::Dict(Rc::new(kept))
                    }
                    other => return Err(format!("{}() cannot take a place out of {}", name, other.bare())),
                }
            }
            Prim::Portray => {
                n(1)?;
                self.utter(&over_lines(&v[0], 0, self.wording()));
                Value::Flag(true)
            }
            Prim::Invert => Value::Flag(!self.stands_true(&v[0])),
            // Turning text over works letter by letter; anything else is
            // read as a whole number of sixty-four bits first.
            Prim::BitsOver => match &v[0] {
                Value::Text(s) => {
                    let over: Vec<u8> = self.table.raw_of(s).iter().map(|b| !b).collect();
                    Value::text(&self.table.said_of(&over))
                }
                other => Value::Small(!self.bits_told(other)?),
            },
            // Two pieces of text meet letter by letter. The shorter one
            // says how far it goes, save where either bit will do, and
            // there the longer one carries on alone.
            Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne
                if matches!(v[0], Value::Text(_)) && matches!(v[1], Value::Text(_)) =>
            {
                let (left, right) = (v[0].bare(), v[1].bare());
                let (left, right) = (self.table.raw_of(&left), self.table.raw_of(&right));
                let (left, right) = (&left[..], &right[..]);
                let far = if op == Prim::BitsEither { left.len().max(right.len()) } else { left.len().min(right.len()) };
                let mut letters: Vec<u8> = Vec::with_capacity(far);
                for at in 0..far {
                    let one = *left.get(at).unwrap_or(&0);
                    let other = *right.get(at).unwrap_or(&0);
                    letters.push(match op {
                        Prim::BitsBoth => one & other,
                        Prim::BitsEither => one | other,
                        _ => one ^ other,
                    });
                }
                Value::text(&String::from_utf8_lossy(&letters))
            }
            Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne => {
                let (left, right) = (self.bits_told(&v[0])?, self.bits_told(&v[1])?);
                Value::Small(match op {
                    Prim::BitsBoth => left & right,
                    Prim::BitsEither => left | right,
                    _ => left ^ right,
                })
            }
            // Text turned about is text taken times minus one, the way a
            // language reading a number out of text does it: the number
            // the text opens with is turned about, and text opening with
            // none is a pair no such step can take.
            Prim::Negate
                if matches!(v[0], Value::Text(_)) && self.complaint_words.iter().any(|(k, _)| *k == "warning") =>
            {
                let pair = [v[0].clone(), Value::Small(-1)];
                return self.prim(Prim::Times, name, &pair);
            }
            Prim::Negate => {
                let turned = match math::compute(Calc::Minus, &Value::Small(0), &v[0]) {
                    Some(r) => r?,
                    None => return Err("Cannot negate non-numeric value".to_string()),
                };
                // A nought turned about is the other nought.
                match (&v[0], &turned) {
                    (Value::Frac(was), Value::Frac(now)) if num_traits::Zero::is_zero(&now.above) => {
                        math::made_number(now.above.clone(), now.beneath.clone(), now.places, !was.under)
                    }
                    // The lowest whole number turned about lies one past
                    // the width, so it comes back a real, as a sum that
                    // runs over does.
                    _ => self.at_width(turned),
                }
            }
            // Nothing stands in any order beside the worth nothing is
            // equal to, so each of the four is answered no, whichever
            // way round it is put: the wider three are asked here and
            // not deeper down, being the narrow one turned about.
            Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge if v.len() == 2 && math::no_order(&v[0], &v[1]) => Value::Flag(false),
            // Which of two comes first is asked just as loosely, so an
            // array, a flag, nothing and text spelling no number are
            // each set against the other the way such a language sets
            // them.
            Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge if self.loose_equals => {
                n(2)?;
                let (left, right) = (v[0].clone(), v[1].clone());
                Value::Flag(match op {
                    Prim::Lt => self.comes_first(&left, &right)?,
                    Prim::Gt => self.comes_first(&right, &left)?,
                    Prim::Le => !self.comes_first(&right, &left)?,
                    _ => !self.comes_first(&left, &right)?,
                })
            }
            // Where a language has a word for being the very same,
            // being equal is the looser question: text spelling a
            // number stands for that number, a flag turns the question
            // into whether the other side is true, and nothing counts
            // as untrue and as text with nothing in it.
            Prim::Eq | Prim::Ne if self.loose_equals => {
                n(2)?;
                let counts = |x: &Value| matches!(x.kind(), Some(Kind::Whole | Kind::Fraction | Kind::Decimal));
                let empty = |x: &Value| matches!(x, Value::Nil | Value::Unset);
                let (left, right) = (&v[0], &v[1]);
                let alike = match (left, right) {
                    (Value::Flag(_), _) | (_, Value::Flag(_)) => self.stands_true(left) == self.stands_true(right),
                    // Nothing whatever is equal to the worth nothing is
                    // equal to, itself least of all, and text spelling
                    // its name no more than anything else. A flag is
                    // asked first, since a flag turns the question into
                    // whether the other side is true, and it is.
                    (x, y) if math::no_order(x, y) => false,
                    (one, other) | (other, one) if empty(one) && counts(other) => !self.stands_true(other),
                    (one, Value::Text(s)) | (Value::Text(s), one) if empty(one) => s.is_empty(),
                    (one, Value::Vector(items)) | (Value::Vector(items), one) if empty(one) => items.is_empty(),
                    (one, Value::Dict(pairs)) | (Value::Dict(pairs), one) if empty(one) => pairs.is_empty(),
                    (Value::Text(_), Value::Text(_)) => match (self.number_said(left), self.number_said(right)) {
                        (Some(x), Some(y)) => x.equals(&y),
                        _ => left.equals(right),
                    },
                    (Value::Text(s), other) | (other, Value::Text(s)) if counts(other) => match self.number_said(left).or_else(|| self.number_said(right)) {
                        Some(x) => self.alike_at_width(&x, other),
                        None => **s == other.bare(),
                    },
                    // Two numbers are set against each other at the
                    // width the language holds them in, as they are
                    // worked at it: a whole number past that width and
                    // the real it comes to are the one number.
                    (x, y) if counts(x) && counts(y) => self.alike_at_width(x, y),
                    _ => left.equals(right),
                };
                Value::Flag((op == Prim::Eq) == alike)
            }
            Prim::Eq => Value::Flag(v[0].equals(&v[1])),
            Prim::Ne => Value::Flag(!v[0].equals(&v[1])),
            Prim::Contains | Prim::Absent => {
                let present = match (&v[0], &v[1]) {
                    (needle, Value::Vector(hay)) => hay.iter().any(|item| needle.equals(item)),
                    (key, Value::Dict(entries)) => entries.iter().any(|(k, _)| key.equals(k)),
                    (Value::Text(part), Value::Text(text)) => text.contains(part.as_ref()),
                    _ => return Err(self.table.single("ext.op.in.unsupported").unwrap_or_default().to_string()),
                };
                Value::Flag(if op == Prim::Absent { !present } else { present })
            }
            Prim::Mod if self.table.flag("ext.op.rem.formats_text") && matches!(v[0], Value::Text(_)) => {
                let Value::Text(pattern) = &v[0] else { unreachable!() };
                Value::text(&self.text_remainder(pattern, &v[1])?)
            }
            Prim::Selfsame | Prim::Unlike if self.table.has_any("ext.op.identical.negated") => {
                let identical = match (&v[0], &v[1]) {
                    (Value::Vector(a), Value::Vector(b)) => Rc::ptr_eq(a, b),
                    (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
                    (Value::Thing(a), Value::Thing(b)) => Rc::ptr_eq(a, b),
                    (Value::Blueprint(a), Value::Blueprint(b)) => Rc::ptr_eq(a, b),
                    (Value::Nil, Value::Nil) | (Value::Ellipsis, Value::Ellipsis) => true,
                    (Value::Flag(a), Value::Flag(b)) => a == b,
                    (Value::Small(n), Value::Small(m)) if *n >= -5 && *n <= 256 => n == m,
                    _ if !v[0].selfsame(&v[1]) => false,
                    _ => return Err(self.table.single("ext.op.identical.unsupported").unwrap_or_default().to_string()),
                };
                Value::Flag(if op == Prim::Unlike { !identical } else { identical })
            }
            Prim::Selfsame => Value::Flag(v[0].selfsame(&v[1])),
            Prim::Unlike => Value::Flag(!v[0].selfsame(&v[1])),
            Prim::Join => Value::text(&format!("{}{}", v[0].render(w), v[1].render(w))),
            Prim::At => self.element(&v[0], &v[1], Reading::Plain)?,
            Prim::Apart => self.element(&v[0], &v[1], Reading::Apart)?,
            Prim::Toward => self.element(&v[0], &v[1], Reading::Toward)?,
            // A glance has nothing to say about what is not there.
            Prim::Glance => {
                self.quieted += 1;
                let seen = self.element(&v[0], &v[1], Reading::Plain).unwrap_or(Value::Nil);
                self.quieted -= 1;
                seen
            }
            Prim::Standing => Value::Flag(v.iter().all(|x| !matches!(x, Value::Nil | Value::Unset))),
            Prim::Hollow => {
                n(1)?;
                Value::Flag(!self.stands_true(&v[0]))
            }
            // A value made one of another kind. Numbers give up what
            // lies past the point, text is read for the number it opens
            // with, and anything that is not an array becomes an array
            // holding only itself.
            Prim::AsChars => Value::text(&v[0].render(w)),
            Prim::UnheldText => return Err(v[0].bare()),
            Prim::RenderField => Value::text(&v[0].in_field(w, &v[1].bare(), &v[2].bare())),
            Prim::AsTruth => Value::Flag(self.stands_true(&v[0])),
            Prim::AsNothing => Value::Nil,
            Prim::AsVector => match v[0].clone() {
                held @ (Value::Vector(_) | Value::Dict(_)) => held,
                Value::Nil | Value::Unset => Value::Vector(std::rc::Rc::new(Vec::new())),
                held => Value::Vector(std::rc::Rc::new(vec![held])),
            },
            Prim::AsWhole => {
                let worth = self.worth_of(&v[0]);
                let whole = math::whole_part(&worth).unwrap_or_else(|| BigInt::from(0));
                self.at_width(Value::from_big(whole))
            }
            Prim::AsDecimal => {
                let worth = self.worth_of(&v[0]);
                let made = math::to_decimal(&worth, self.real_figures()).unwrap_or(Value::Small(0));
                self.at_width(made)
            }
            // Reaching in makes the place where nothing is there yet,
            // which is what a write into it asks for.
            Prim::Inward if matches!(&v[1], Value::Span(_)) => return Err(self.span_complaint("detached")),
            Prim::Inward if !self.builds_places => self.element(&v[0], &v[1], Reading::Plain)?,
            Prim::Inward => {
                self.quieted += 1;
                let reached = self.element(&v[0], &v[1], Reading::Plain).unwrap_or(Value::Nil);
                self.quieted -= 1;
                match reached {
                    Value::Nil | Value::Unset => Value::Vector(std::rc::Rc::new(Vec::new())),
                    already => already,
                }
            }
            // Adding text joins it only where the language has no
            // operator of its own for joining; where it has one, adding
            // is arithmetic.
            Prim::Plus
                if !self.table.has_any("op.concat") && (matches!(v[0], Value::Text(_)) || matches!(v[1], Value::Text(_))) =>
            {
                Value::text(&format!("{}{}", v[0].render(w), v[1].render(w)))
            }
            // Text that spells a number is worked with as that number,
            // fractions included. Text that spells one and then says
            // something more is worth what it opens with, and text that
            // spells none is worth nothing; a language with a word for a
            // warning hears of both instead of being stopped.
            Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power
            | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::BitsUp | Prim::BitsDown
                // Only where every piece of text will give up a number,
                // so that what is worked out below is never text again,
                // and, for the two shifts, only where the language has
                // them read each side for a number at all.
                if (matches!(v[0], Value::Text(_)) || matches!(v[1], Value::Text(_)))
                    && (!matches!(op, Prim::BitsUp | Prim::BitsDown) || self.table.flag("ext.op.bit.shift.numbers"))
                    && [&v[0], &v[1]].iter().all(|x| {
                        !matches!(x, Value::Text(_))
                            || number_spelled_in(x).is_some()
                            || self.complaint_words.iter().any(|(k, _)| *k == "warning")
                    }) =>
            {
                // A language may refuse text spelling no number at all
                // where a number is wanted, naming the kinds it was
                // handed rather than working with nothing.
                if let Some(words) = self.table.single("ext.system.fault.operands") {
                    let unnumbered = |x: &Value| matches!(x, Value::Text(_)) && number_opening_in(x).0.is_none();
                    if unnumbered(&v[0]) || unnumbered(&v[1]) {
                        let said = format!("{}: {} {} {}", words, self.kind_called(&v[0]), self.written_as(&op), self.kind_called(&v[1]));
                        return Err(said.into());
                    }
                }
                let warns = self.complaint_words.iter().any(|(k, _)| *k == "warning");
                let worth = |x: &Value| -> Value {
                    let Value::Text(_) = x else { return x.clone() };
                    match number_opening_in(x) {
                        (Some(n), true) => n,
                        (Some(n), false) => {
                            if warns {
                                self.grumble("warning", "A non-numeric value encountered");
                                return n;
                            }
                            x.clone()
                        }
                        (None, _) => {
                            if warns {
                                self.grumble("warning", "A non-numeric value encountered");
                                return Value::Small(0);
                            }
                            x.clone()
                        }
                    }
                };
                let pair = [worth(&v[0]), worth(&v[1])];
                return self.prim(op, name, &pair);
            }
            // Carrying the bits along. Each side is read as a whole
            // number of sixty-four bits, sign and all, save where the
            // reading above has already made numbers of them.
            Prim::BitsUp | Prim::BitsDown => {
                let (bits, by) = (self.bits_told(&v[0])?, self.bits_told(&v[1])?);
                match math::carried_bits(bits, by, op == Prim::BitsUp) {
                    Some(carried) => Value::Small(carried),
                    // Carrying by a count under nought is no carrying,
                    // and a language may have its own words for that.
                    None => match self.table.single("ext.system.fault.shift") {
                        Some(words) => return Err(words.to_string()),
                        None => return Err("Bit shift by a negative number".to_string()),
                    },
                }
            }
            // A language whose remainder is taken between whole numbers
            // brings what it is given to one first, the way it brings a
            // value to the bits it works on.
            Prim::Mod
                if self.table.flag("op.mod.whole")
                    && (matches!(v[0], Value::Frac(_)) || matches!(v[1], Value::Frac(_))) =>
            {
                let pair = [Value::Small(self.bits_told(&v[0])?), Value::Small(self.bits_told(&v[1])?)];
                return self.prim(op, name, &pair);
            }
            // Two whole numbers dividing evenly leave a whole one, in a
            // language whose division says as much rather than always
            // leaving a real behind.
            Prim::OverReal
                if self.table.lone("op.div.result") == Some("whole_or_real")
                    && matches!(v[0], Value::Small(_) | Value::Huge(_))
                    && matches!(v[1], Value::Small(_) | Value::Huge(_))
                    && matches!(math::compute(Calc::Over, &v[0], &v[1]), Some(Ok(Value::Small(_) | Value::Huge(_)))) =>
            {
                let evenly = match math::compute(Calc::Over, &v[0], &v[1]) {
                    Some(r) => r?,
                    None => return Err("Division requires numeric operands".to_string()),
                };
                self.at_width(evenly)
            }
            Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power => {
                let sum = match op {
                    Prim::Plus => Calc::Plus,
                    Prim::Minus => Calc::Minus,
                    Prim::Times => Calc::Times,
                    Prim::Over => Calc::Over,
                    Prim::OverReal => Calc::OverReal,
                    Prim::IntDiv => Calc::IntDiv,
                    Prim::Mod => Calc::Remainder,
                    _ => Calc::Power,
                };
                // Where a language holds its reals to a width of bits,
                // a whole number meeting a real is brought to that
                // width first, so the two are worked as it works them.
                let (left, right) = match self.holds_reals_to_width() && (self.a_real(&v[0]) || self.a_real(&v[1])) {
                    true => (self.at_width(self.as_wide_real(&v[0])), self.at_width(self.as_wide_real(&v[1]))),
                    false => (v[0].clone(), v[1].clone()),
                };
                let worked = match math::compute(sum, &left, &right) {
                    // A language may tell the remainder by nought apart
                    // from the division by it and word that its own
                    // way. The kind of fault is the same for both, so
                    // it is the wording alone that is stood in.
                    Some(Err(told)) if sum == Calc::Remainder && told == "Division by zero" => {
                        let its_own = self.table.single("ext.system.fault.modulo");
                        return Err(its_own.map_or(told, str::to_string));
                    }
                    Some(r) => r?,
                    None => match sum {
                        Calc::Plus => Value::from_big(left.as_big()? + right.as_big()?),
                        Calc::Minus => Value::from_big(left.as_big()? - right.as_big()?),
                        Calc::Times => Value::from_big(left.as_big()? * right.as_big()?),
                        Calc::Over | Calc::OverReal => return Err("Division requires numeric operands".to_string()),
                        Calc::IntDiv => return Err("Integer quotient requires numeric operands".to_string()),
                        Calc::Remainder => return Err("Modulo requires numeric operands".to_string()),
                        Calc::Power => return Err("Exponentiation requires numeric operands".to_string()),
                    },
                };
                self.at_width(worked)
            }
            Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge => {
                // Two numbers are set against each other at the width
                // the language holds them in, as they are worked at it.
                let (left, right) = match self.holds_reals_to_width() && (self.a_real(&v[0]) || self.a_real(&v[1])) {
                    true => (self.at_width(self.as_wide_real(&v[0])), self.at_width(self.as_wide_real(&v[1]))),
                    false => (v[0].clone(), v[1].clone()),
                };
                let below = |a: &Value, b: &Value| -> Result<bool, String> {
                    match math::below(a, b) {
                        Some(r) => Ok(r),
                        None => Ok(a.as_big()? < b.as_big()?),
                    }
                };
                Value::Flag(match op {
                    Prim::Lt => below(&left, &right)?,
                    Prim::Gt => below(&right, &left)?,
                    Prim::Le => !below(&right, &left)?,
                    _ => !below(&left, &right)?,
                })
            }
            Prim::Echo => {
                n(1)?;
                match &v[0] {
                    Value::Text(s) => self.utter(s),
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
                Value::Nil
            }
            Prim::Say => {
                self.utter(&format!("{}\n", self.show(v)));
                Value::Nil
            }
            Prim::Out => {
                self.utter(&self.show(v));
                Value::Nil
            }
            Prim::Tell => {
                let w = self.wording();
                let text: String = v.iter().map(|x| x.render(w)).collect();
                self.utter(&text);
                Value::Nil
            }
            Prim::Define => return Err(format!("{}() needs a quoted name as its first argument", name)),
            Prim::Dump => {
                for x in v {
                    self.utter(&format!("{}\n", with_kind(x, 0, self.table.count("ext.system.real.bits").is_some(), self.wording())));
                }
                Value::Nil
            }
            Prim::Listed => {
                match v.len() {
                    0 => Value::Vector(Rc::new(Vec::new())),
                    1 => Value::Vector(Rc::new(self.gathered_members(&v[0])?)),
                    _ => return Err(format!("{}() expects 1 argument, got {}", name, v.len())),
                }
            }
            Prim::SomeTrue => {
                n(1)?;
                let members = self.gathered_members(&v[0])?;
                Value::Flag(members.iter().any(|item| self.stands_true(item)))
            }
            Prim::Total => {
                if !(1..=2).contains(&v.len()) { return Err(format!("{}() expects one or two arguments", name)); }
                let members = self.gathered_members(&v[0])?;
                let start = v.get(1).cloned().unwrap_or(Value::Small(0));
                let number = |x| match x { Value::Flag(flag) => Value::Small(flag as i64), x => x };
                members.into_iter().try_fold(number(start), |prior, item| {
                    math::compute(Calc::Plus, &prior, &number(item)).unwrap_or_else(|| Err(self.table.single("ext.builtin.sum.non_number").unwrap_or("Invalid collection argument").into()))
                })?
            }
            Prim::Span if self.table.flag("ext.builtin.range.value") => {
                let wrong = || self.argument_fault("ext.syntax.call.amiss", None);
                if !(1..=3).contains(&v.len()) { return Err(wrong()); }
                let integer = |item: &Value| match item {
                    Value::Huge(big) => Ok((**big).clone()),
                    Value::Small(small) => Ok(BigInt::from(*small)),
                    Value::Flag(flag) => Ok(BigInt::from(if *flag { 1 } else { 0 })),
                    _ => Err(self.argument_fault("ext.builtin.range.integer", None)),
                };
                let (first, limit) = if v.len() == 1 { (BigInt::from(0), integer(&v[0])?) }
                    else { (integer(&v[0])?, integer(&v[1])?) };
                let stride = v.get(2).map(integer).transpose()?.unwrap_or_else(|| BigInt::from(1));
                if stride == BigInt::from(0) { return Err(self.argument_fault("ext.builtin.range.zero", None)); }
                Value::Progression(Rc::new(crate::data::Progression { first, limit, stride, word: name.to_owned() }))
            }
            Prim::Span => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Prim::MakeReal => {
                if v.is_empty() || v.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, v.len()));
                }
                let digits = match v.get(1) {
                    None => math::DEFAULT_PLACES,
                    Some(Value::Small(d)) if *d >= 0 => *d as usize,
                    Some(Value::Small(_)) | Some(Value::Huge(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                math::to_decimal(&v[0], digits).ok_or_else(|| format!("{}() requires a number, rational, or real argument", name))?
            }
            Prim::Places => {
                n(1)?;
                match &v[0] {
                    Value::Frac(e) if e.places.is_some() => Value::Small(e.places.unwrap() as i64),
                    _ => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Prim::AsText if self.table.single("ext.builtin.to_string.object").is_some() && v.len() != 1 => {
                if v.is_empty() { Value::text("") } else { return Err(self.argument_fault("ext.builtin.to_string.unready", None)); }
            }
            Prim::AsText => {
                n(1)?;
                Value::text(&v[0].render(w))
            }
            Prim::AsInt if self.table.single("ext.builtin.to_int.base").is_some() => self.whole_from_call(v)?,
            Prim::AsInt => {
                n(1)?;
                let whole = math::whole_part(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::from_big(whole)
            }
            Prim::AsReal if self.table.flag("ext.builtin.to_real.text") && (v.is_empty() || matches!(v.first(), Some(Value::Text(_)))) => {
                if v.len() > 1 { return Err(self.argument_fault("ext.syntax.call.amiss", None)); }
                let failure = || self.argument_fault("ext.builtin.to_real.text.amiss", None);
                let worth = if v.is_empty() { Value::Small(0) } else { number_spelled_in(&v[0]).ok_or_else(failure)? };
                math::to_decimal(&worth, math::DEFAULT_PLACES).ok_or_else(failure)?
            }
            Prim::AsReal => {
                n(1)?;
                match &v[0] {
                    x @ Value::Frac(e) if e.places.is_some() => x.clone(),
                    x => math::to_decimal(x, math::DEFAULT_PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?,
                }
            }
            Prim::Length => {
                n(1)?;
                match &v[0] {
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Vector(l) => Value::Small(l.len() as i64),
                    Value::Dict(entries) => Value::Small(entries.len() as i64),
                    Value::Progression(p) => Value::from_big(p.count()),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Prim::CharAtIndex => {
                n(2)?;
                match (&v[0], &v[1]) {
                    (Value::Text(s), Value::Small(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::text(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Text(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Prim::CodeOf => {
                n(1)?;
                match &v[0] {
                    Value::Text(s) => match s.chars().next() {
                        Some(c) => Value::Small(c as i64),
                        None => return Err(format!("{}() requires a non-empty string", name)),
                    },
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
            }
            Prim::CharOf => {
                n(1)?;
                let code = match &v[0] {
                    Value::Small(i) => u32::try_from(*i).ok(),
                    Value::Huge(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                }
                .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::text(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            // Saying the run is over is done where the call is made,
            // since it is not a value to be worked out.
            Prim::Quit => return Err("the run is over".to_string()),
            Prim::Raise => {
                n(1)?;
                return match &v[0] {
                    Value::Text(s) => Err(s.to_string()),
                    _ => Err(format!("{}() argument must be a string", name)),
                };
            }
            Prim::SortOf => {
                n(1)?;
                // A thing is of no kind the core knows, so a language
                // with a word of its own for one answers with that.
                if let (Value::Thing(_), Some(word)) = (&v[0], self.table.single("ext.system.kind.object")) {
                    return Ok(Value::text(word));
                }
                let Some(sort) = v[0].kind() else {
                    return Err(format!("{}(): unknown value type", name));
                };
                // Where a language says a kind in words, the word it
                // gives that kind is the answer, not a value standing
                // for the kind itself.
                if !self.table.flag("ext.system.kind.spelled") {
                    Value::KindOf(sort)
                } else {
                    match KIND_LABELS.iter().find(|(_, k)| *k == sort).and_then(|(label, _)| self.table.single(label)) {
                        Some(word) => Value::text(word),
                        None => return Err(format!("{}(): the language has no word for that kind", name)),
                    }
                }
            }
            Prim::Numer => {
                n(1)?;
                Value::from_big(math::ratio_of(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?.above)
            }
            Prim::Denom => {
                n(1)?;
                Value::from_big(math::ratio_of(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?.beneath)
            }
            Prim::Fetch => {
                n(2)?;
                self.element(&v[0], &v[1], Reading::Plain)?
            }
            Prim::External => {
                let target = match v.first() {
                    Some(Value::Text(s)) => s.to_string(),
                    Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
                    None => return Err(format!("{} requires at least one argument (function name)", name)),
                };
                if !matches!(target.as_str(), "print_native" | "debug_info" | "value_type") {
                    return Err(format!("Unknown external function: {}", target));
                }
                if v.len() != 2 {
                    return Err(format!("{} expects 1 argument, got {}", target, v.len() - 1));
                }
                let x = &v[1];
                match target.as_str() {
                    "print_native" => self.utter(&format!("{}\n", x.render(w))),
                    "debug_info" => eprintln!("[DEBUG] {}", x.render(w)),
                    _ => {
                        return match x.kind() {
                            Some(Kind::Whole | Kind::Fraction | Kind::Decimal) => Ok(Value::Small(0)),
                            Some(Kind::Truth) => Ok(Value::Small(1)),
                            Some(Kind::Chars) => Ok(Value::Small(2)),
                            _ => Err("Unknown value type".to_string()),
                        }
                    }
                }
                x.clone()
            }
            Prim::Seq | Prim::Choose | Prim::Both | Prim::Either | Prim::Yield | Prim::Leave | Prim::Resume | Prim::Append | Prim::Replace
            | Prim::Front | Prim::Spawn | Prim::Ask | Prim::Bid | Prim::Hurl | Prim::Otherwise => unreachable!("handled in eval"),
        })
    }

    /// A source read in and run: text given outright, or a file
    /// sought beside the one asking for it before it is sought where
    /// the run began.
    fn read_in(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        if v.len() != 1 {
            return Err(format!("{}() expects 1 argument, got {}", name, v.len()));
        }
                let w = self.wording();
                let given = v[0].render(w);
                let mut came_out_of = None;
                let source = match op {
                    // Text handed over to be run is code already; a file
                    // is text with code marked out inside it, so only
                    // the first wants the mark that opens code.
                    Prim::Weigh => match self.table.single("lexical.prologue") {
                        Some(open) => format!("{}\n{}", open, given),
                        None => given,
                    },
                    // A file is sought beside the one asking for it
                    // before it is sought where the run began: a program
                    // naming a file beside itself means that one.
                    _ => {
                        let near = std::path::Path::new(self.written_in.as_ref()).parent().map(|place| place.join(&given));
                        let near = near.filter(|place| place.exists());
                        came_out_of = Some(match &near {
                            Some(place) => place.to_string_lossy().into_owned(),
                            None => given.clone(),
                        });
                        let held = match near {
                            Some(place) => std::fs::read(place),
                            None => std::fs::read(&given),
                        };
                        // A file asked for once only is read the first
                        // time and passed over after, under whatever
                        // name it was asked for, since it is the file
                        // and not the name that stands.
                        if op == Prim::BringOnce {
                            let place = came_out_of.clone().unwrap_or_else(|| given.clone());
                            let whole = std::fs::canonicalize(&place).map(|p| p.to_string_lossy().into_owned()).unwrap_or(place);
                            if !self.read_before.borrow_mut().insert(whole) {
                                return Ok(Value::Flag(true));
                            }
                        }
                        match held {
                            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                            // A file that will not be read is not there
                            // as far as the run can tell. It says so of
                            // the file; then a word that will not go on
                            // without it stops the run, and one that
                            // will says so of the reading in besides and
                            // answers false.
                            Err(_) => {
                                let page = self.word_page(name);
                                self.grumble("warning", &format!("{}(){}: Failed to open stream: No such file or directory", name, page));
                                let insisted = self.table.strings("ext.builtin.include.demanded").iter().any(|word| word == name);
                                if let (true, Some((opens, closes))) = (insisted, self.table.around("ext.builtin.include.demanded.missing")) {
                                    return Err(format!("{opens}{given}{closes}"));
                                }
                                let told = format!("{}(){}: Failed opening '{}' for inclusion (include_path='.')", name, page, given);
                                self.grumble("warning", &told);
                                return Ok(Value::Flag(false));
                            }
                        }
                    }
                };
                return self.run_source(&source, came_out_of);
    }

    /// A key as this language takes one.
    fn as_key(&self, at: &Value) -> Value {
        match self.plain_keys {
            true => key_as_taken(at.clone()),
            false => at.clone(),
        }
    }

    /// The same, with a word said where a place is named by nothing at
    /// all and the language asks for that to be written out. A place
    /// being taken away is named without a word, as the reference has it.
    fn as_key_spoken(&self, at: &Value) -> Value {
        if let (Value::Nil | Value::Unset, Some(said)) = (at, self.table.single("ext.op.index.nothing")) {
            self.said_regardless("deprecated", &said.to_string());
        }
        self.as_key(at)
    }

    fn element(&self, target: &Value, at: &Value, how: Reading) -> Result<Value, String> {
        match (self.table.flag("ext.op.index.from_end"), target, at) {
            (true, Value::Vector(values), Value::Small(n)) if *n < 0 && n.unsigned_abs() <= values.len() as u64 => {
                return self.element(target, &Value::Small(values.len() as i64 + n), how);
            }
            (true, Value::Text(chars), Value::Small(n)) if *n < 0 => {
                let count = chars.chars().count() as i64;
                if count + n >= 0 { return self.element(target, &Value::Small(count + n), how); }
            }
            _ => (),
        }
        if let Value::Progression(walk) = target {
            match at {
                Value::Small(_) | Value::Huge(_) | Value::Flag(_) => return walk.item(&at.as_big()?).ok_or_else(|| self.argument_fault("ext.builtin.range.index", None)),
                Value::Span(_) => return Err(self.span_complaint("unsupported")),
                _ => return Err(self.argument_fault("ext.builtin.range.integer", None)),
            }
        }
        // A place holding a cell that names share reads as whatever the
        // cell holds: the sharing lies between the names, not in the
        // value itself.
        return self.element_within(target, at, how).map(|found| match found {
            Value::Shared(cell) => cell.borrow().clone(),
            held => held,
        });
    }

    fn span_complaint(&self, part: &str) -> String {
        self.table.single(&format!("ext.op.index.slice.{}", part)).unwrap_or("").to_owned()
    }

    fn span_number(&self, part: &Value) -> Result<Option<BigInt>, String> {
        match part {
            Value::Nil => Ok(None),
            Value::Huge(whole) => Ok(Some(whole.as_ref().clone())),
            Value::Small(whole) => Ok(Some(BigInt::from(*whole))),
            Value::Flag(truth) => Ok(Some(BigInt::from(u8::from(*truth)))),
            _ => Err(self.span_complaint("bounds")),
        }
    }

    /// Count the places first, then gather them. The step need never
    /// reach farther than the row's whole length and one place beyond.
    fn span_selection(&self, bounds: &[Value], length: usize) -> Result<(std::ops::Range<usize>, Vec<usize>, bool), String> {
        let step = self.span_number(&bounds[2])?.unwrap_or_else(|| BigInt::from(1));
        if step == BigInt::from(0) {
            return Err(self.span_complaint("zero"));
        }
        let unit = step == BigInt::from(1);
        let reverse = step < BigInt::from(0);
        let extent = length as i128;
        let reach = BigInt::from(extent + 1);
        let stride = step.max(-&reach).min(reach).to_i128().expect("a step within the row");
        let mut ends = [0_i128; 2];
        for side in 0..2 {
            let omitted = match (side, reverse) {
                (0, true) => extent - 1,
                (0, false) => 0,
                (_, true) => -1,
                (_, false) => extent,
            };
            ends[side] = match self.span_number(&bounds[side])? {
                None => omitted,
                Some(mut whole) => {
                    if whole < BigInt::from(0) {
                        whole += BigInt::from(extent);
                    }
                    let least = if reverse { -1 } else { 0 };
                    let most = if reverse { extent - 1 } else { extent };
                    whole.max(BigInt::from(least)).min(BigInt::from(most)).to_i128().expect("a bound within the row")
                }
            };
        }
        let distance = if reverse { ends[0] - ends[1] } else { ends[1] - ends[0] };
        let many = if distance <= 0 { 0 } else { (distance - 1) / stride.abs() + 1 };
        let picked = (0..many).map(|turn| (ends[0] + turn * stride) as usize).collect();
        let begin = ends[0].max(0) as usize;
        let end = ends[1].max(ends[0]).max(0) as usize;
        Ok((begin..end, picked, unit))
    }

    fn span_written(&self, held: &mut Value, bounds: &[Value], handed: &Value) -> Result<(), String> {
        let Value::Vector(row) = held else { return Err(self.span_complaint("unsupported")) };
        let (span, picked, unit) = self.span_selection(bounds, row.len())?;
        let coming: Vec<Value> = match handed {
            Value::Text(letters) => letters.chars().map(|c| Value::text(&c.to_string())).collect(),
            Value::Dict(entries) => entries.iter().map(|entry| entry.0.clone()).collect(),
            Value::Vector(values) => values.iter().cloned().collect(),
            _ => return Err(self.span_complaint("assign")),
        };
        if !unit && picked.len() != coming.len() {
            let wording = self.table.strings("ext.op.index.slice.length");
            let before = wording.first().map(String::as_str).unwrap_or("");
            let between = wording.get(1).map(String::as_str).unwrap_or("");
            return Err(format!("{before} {} {between} {}", coming.len(), picked.len()));
        }
        let written = Rc::make_mut(row);
        if unit {
            let mut tail = written.split_off(span.end);
            written.truncate(span.start);
            written.extend(coming);
            written.append(&mut tail);
        } else {
            for (value, place) in coming.into_iter().zip(picked) {
                written[place] = value;
            }
        }
        Ok(())
    }

    fn element_within(&self, target: &Value, at: &Value, how: Reading) -> Result<Value, String> {
        if let Value::Span(bounds) = at {
            let row = match target {
                Value::Vector(values) => values.as_ref().clone(),
                Value::Text(text) if self.table.flag("op.index.strings") => {
                    text.chars().map(|letter| Value::text(&letter.to_string())).collect()
                }
                _ => return Err(self.span_complaint("unsupported")),
            };
            let (_, picked, _) = self.span_selection(bounds, row.len())?;
            let selected: Vec<Value> = picked.iter().map(|&i| row[i].clone()).collect();
            return Ok(if matches!(target, Value::Text(_)) {
                Value::text(&selected.iter().map(Value::bare).collect::<String>())
            } else {
                Value::Vector(Rc::new(selected))
            });
        }
        // A language may say that a place an array does not hold reads as
        // nothing rather than stopping the program.
        // A language that reads an absent place as nothing, and has a
        // word for a warning, says which place was missing first.
        let missing = |told: String, key: &Value| {
            if !self.table.flag("ext.op.index.absent") {
                return Err(told);
            }
            let named = match key {
                Value::Text(s) => format!("\"{}\"", s),
                other => other.bare(),
            };
            self.grumble("warning", &format!("Undefined array key {}", named));
            Ok(Value::Nil)
        };
        // Reaching into nothing at all is said differently from reaching
        // for a place an array does not hold: there is no array to hold
        // it in the first place.
        if self.table.flag("ext.op.index.absent") && matches!(target, Value::Nil | Value::Unset) {
            self.grumble("warning", "Trying to access array offset on null");
            return Ok(Value::Nil);
        }
        if let Value::Dict(entries) = target {
            let at = &self.as_key_spoken(at);
            let found = entries.iter().find(|(k, _)| k.equals(at));
            return match found {
                Some((_, v)) => Ok(v.clone()),
                None => missing(format!("Undefined array key {}", at.bare()), at),
            };
        }
        // Text naming a place in text counts for the number it opens
        // with, since text counts as a number wherever one is wanted.
        let spelled = match (target, at) {
            (Value::Text(_), Value::Text(said)) if self.letter_places => {
                let (opens, outright) = number_opening_in(at);
                // Where the text naming the place is no number outright,
                // the run says so rather than counting it quietly.
                if !outright && opens.is_some() {
                    self.grumble("warning", &format!("Illegal string offset \"{}\"", said));
                }
                opens
            }
            _ => None,
        };
        let i = match as_index(spelled.as_ref().unwrap_or(at)) {
            Ok(i) => i,
            Err(told) => return missing(told, at),
        };
        match target {
            Value::Vector(l) => match l.get(i) {
                Some(v) => Ok(v.clone()),
                None => missing(format!("Array index {} out of bounds (length: {})", i, l.len()), at),
            },
            Value::Text(s) if self.table.flag("op.index.strings") => s
                .chars()
                .nth(i)
                .map(|c| Value::text(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", i, s.chars().count())),
            // A value with no places whatever. A language reading a
            // place an array does not hold as nothing reads a place of
            // such a value the same way, and names the kind it was
            // asked of; a thing is another matter and is turned down.
            _ if self.table.flag("ext.op.index.absent") && how != Reading::Toward && !matches!(target, Value::Thing(_)) => {
                let kind = self.kind_called(target);
                let told = match how {
                    Reading::Apart => format!("Cannot use {} as array", kind),
                    _ => format!("Trying to access array offset on {}", kind),
                };
                self.grumble("warning", &told);
                Ok(Value::Nil)
            }
            _ => Err(self.no_places()),
        }
    }

    fn gathered_members(&self, source: &Value) -> Result<Vec<Value>, String> {
        Ok(match source {
            Value::Text(word) => word.chars().map(|letter| Value::text(&String::from(letter))).collect(),
            Value::Vector(values) => values.to_vec(),
            Value::Dict(entries) => entries.iter().map(|entry| entry.0.clone()).collect(),
            Value::Progression(walk) => {
                let mut values = Vec::new();
                let mut position = BigInt::from(0);
                let count = walk.count();
                while position < count {
                    values.push(Value::from_big(&walk.first + &walk.stride * &position));
                    position += 1;
                }
                values
            }
            Value::Shared(held) => return self.gathered_members(&held.borrow()),
            _ => return Err(self.table.single("ext.syntax.collection.unwalkable").unwrap_or("Cannot gather members from this value").to_string()),
        })
    }

    fn show(&self, v: &[Value]) -> String {
        let w = self.wording();
        let holes = self.table.strings("builtin.print.placeholder");
        let find = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|p| (p, h.len()))).min();
        if let (Some(Value::Text(t)), true) = (v.first(), v.len() > 1) {
            if find(t).is_some() {
                let mut out = String::new();
                let mut rest = v[1..].iter();
                let mut s: &str = t;
                while let Some((p, k)) = find(s) {
                    out.push_str(&s[..p]);
                    match rest.next() {
                        Some(x) => out.push_str(&x.render(w)),
                        None => out.push_str(&s[p..p + k]),
                    }
                    s = &s[p + k..];
                }
                out.push_str(s);
                return out;
            }
        }
        v.iter().map(|x| x.render(w)).collect::<Vec<_>>().join(" ")
    }
}

fn as_index(v: &Value) -> Result<usize, String> {
    match v {
        Value::Small(i) => usize::try_from(*i).map_err(|_| "Array index out of bounds".to_string()),
        Value::Huge(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}

/// A value shown with its kind the way PHP's var_dump does: numbers as
/// `int(n)` and `float(x)`, text with its length in bytes, a vector
/// one entry per line, each nested level two spaces further in.
/// How far a member is reached from, written beside its name where a
/// thing is shown: nothing where it is open to everything, the word for
/// one a class shares only with those built on it, or the declaring
/// class's name and the word for one it keeps to itself.
fn written_reach(of: &Blueprint, filed: &str, w: Names) -> String {
    let (member, owner) = crate::data::holder_of(filed);
    if let Some(declared) = owner {
        return w.alone_word.map_or_else(String::new, |word| format!(":\"{declared}\":{word}"));
    }
    match of.reach_of(member) {
        Some((Reach::Within, _)) => w.within_word.map_or_else(String::new, |word| format!(":{word}")),
        _ => String::new(),
    }
}

/// The head of a complaint: the break of line one opens with, the word
/// naming the kind, and what the complaint says. Where the language
/// gives words for dressing a complaint for a reader of markup the word
/// is wrapped in them; where it gives none the bare words go out.
pub fn complaint_head(table: &Table, word: &str, about: &str) -> String {
    match table.strings("ext.system.complaint.markup.kind") {
        [ahead, opens, closes, ..] => format!("{ahead}\n{opens}{word}{closes}{about}"),
        _ => format!("\n{word}: {about}"),
    }
}

/// The tail of a complaint: where it was raised and on what line, each
/// wrapped as the language would have it, and the break of line that
/// ends a complaint after the two.
pub fn complaint_tail(table: &Table, place: &str, row: u32) -> String {
    let wrapped = |label: &str, held: String| match table.around(label) {
        Some((opens, closes)) => format!("{opens}{held}{closes}"),
        None => held,
    };
    let file = wrapped("ext.system.complaint.markup.place", place.to_string());
    let at = wrapped("ext.system.complaint.markup.line", row.to_string());
    format!(" in {file} on line {at}\n")
}

/// Whether a thing still keeps a member at a place. Taking one off
/// empties its place and leaves it there, so that a walk already under
/// way finds the rest of the members where it left them.
pub fn kept(x: &Value) -> bool {
    !matches!(x, Value::Unset)
}

fn with_kind(v: &Value, level: usize, binary_reals: bool, w: Names) -> String {
    let lead = "  ".repeat(level);
    // Something standing inside a value is marked as shared when its
    // cell is one that some name still reaches besides the value
    // holding it. A cell no other name reaches shows plainly, and so
    // does a value shown on its own.
    let tied = |x: &Value| match x {
        Value::Shared(cell) if Rc::strong_count(cell) > 1 => "&",
        _ => "",
    };
    match v {
        // A cell that names share is shown as what it holds.
        Value::Shared(cell) => with_kind(&cell.borrow(), level, binary_reals, w),
        Value::Small(_) | Value::Huge(_) => format!("int({})", v.bare()),
        // Shown with its kind, a binary real is written in the fewest
        // figures that read back as the same number.
        // A nought under nought is written so, at any width.
        Value::Frac(e) if e.under && num_traits::Zero::is_zero(&e.above) => "float(-0)".to_string(),
        Value::Frac(e) if binary_reals => format!("float({})", crate::data::figured(crate::data::nearest_binary(&e.above, &e.beneath), None)),
        Value::Frac(_) => format!("float({})", v.bare()),
        Value::Text(s) => {
            // Kept as bytes, a character is a byte and the width is the
            // count of them; kept as letters, it is what they are
            // spelled with that is counted.
            let wide = if w.kept_as_bytes { s.chars().count() } else { s.len() };
            format!("string({}) \"{}\"", wide, s)
        }
        Value::Flag(b) => format!("bool({})", b),
        Value::Vector(items) => {
            let entries: Vec<String> = items.iter().enumerate().map(|(i, x)| format!("{lead}  [{i}]=>\n{lead}  {}{}\n", tied(x), with_kind(x, level + 1, binary_reals, w))).collect();
            format!("array({}) {{\n{}{lead}}}", items.len(), entries.concat())
        }
        Value::Dict(entries) => {
            // A key of text is shown in quotes, a number bare.
            let shown: Vec<String> = entries
                .iter()
                .map(|(k, x)| {
                    let key = match k {
                        Value::Text(s) => format!("\"{}\"", s),
                        other => other.bare(),
                    };
                    format!("{lead}  [{key}]=>\n{lead}  {}{}\n", tied(x), with_kind(x, level + 1, binary_reals, w))
                })
                .collect();
            format!("array({}) {{\n{}{lead}}}", entries.len(), shown.concat())
        }
        Value::Thing(thing) => {
            let held = thing.holds.borrow();
            let shown: Vec<String> = held
                .iter()
                .filter(|(_, x)| kept(x))
                .map(|(filed, x)| {
                    let how = written_reach(&thing.of, filed, w);
                    let member = crate::data::holder_of(filed).0;
                    format!("{lead}  [\"{member}\"{how}]=>\n{lead}  {}{}\n", tied(x), with_kind(x, level + 1, binary_reals, w))
                })
                .collect();
            format!("object({})#{} ({}) {{\n{}{lead}}}", thing.of.name, thing.turn, shown.len(), shown.concat())
        }
        _ => "NULL".to_string(),
    }
}

/// The values of a literal: a map when one of them is a couple or the
/// literal asks for one, else a list. A value with no key of its own
/// takes the next whole number.
fn assembled(values: Vec<Value>, map_wanted: bool, plain_keys: bool) -> Value {
    if !map_wanted && !values.iter().any(|x| matches!(x, Value::Couple(_))) {
        return Value::Vector(Rc::new(values));
    }
    let mut entries: Vec<(Value, Value)> = Vec::with_capacity(values.len());
    for value in values {
        match value {
            Value::Couple(e) => {
                let key = if plain_keys { key_as_taken(e.0.clone()) } else { e.0.clone() };
                set_key(&mut entries, key, e.1.clone())
            }
            other => {
                let key = Value::Small(after_keys(&entries));
                entries.push((key, other));
            }
        }
    }
    Value::Dict(Rc::new(entries))
}

/// One past the highest whole-number key, or nought when there is none.
fn after_keys(entries: &[(Value, Value)]) -> i64 {
    entries
        .iter()
        .filter_map(|(k, _)| match k {
            Value::Small(n) => Some(n + 1),
            _ => None,
        })
        .chain(std::iter::once(0))
        .max()
        .unwrap_or(0)
}

/// Write a key over what it holds, or add it at the end.
fn set_key(entries: &mut Vec<(Value, Value)>, key: Value, value: Value) {
    match entries.iter_mut().find(|(k, _)| k.equals(&key)) {
        // A place keeping a cell that names share is written through,
        // not written over.
        Some(entry) => match &entry.1 {
            Value::Shared(cell) => *cell.borrow_mut() = value,
            _ => entry.1 = value,
        },
        None => entries.push((key, value)),
    }
}

/// A value over lines the way PHP's print_r writes it: a scalar on its
/// own, an array as the word `Array` with its places in brackets, every
/// array within set eight spaces further along and followed by a gap.
fn over_lines(v: &Value, along: usize, w: Names) -> String {
    let held;
    let (called, places): (String, Vec<(String, &Value)>) = match v {
        Value::Vector(items) => ("Array".to_string(), items.iter().enumerate().map(|(at, x)| (at.to_string(), x)).collect()),
        Value::Dict(entries) => ("Array".to_string(), entries.iter().map(|(k, x)| (k.render(w), x)).collect()),
        Value::Thing(thing) => {
            held = thing.holds.borrow();
            let named = |k: &String| format!("{}{}", crate::data::holder_of(k).0, written_reach(&thing.of, k, w));
            (format!("{} Object", thing.of.name), held.iter().filter(|(_, x)| kept(x)).map(|(k, x)| (named(k), x)).collect())
        }
        other => return other.render(w),
    };
    let lead = " ".repeat(along);
    let mut out = format!("{called}\n{lead}(\n");
    for (key, item) in places {
        // An array within ends its own line, so this newline is the gap
        // that follows it; after a scalar it is the end of the line.
        let shown = over_lines(item, along + 8, w);
        out.push_str(&format!("{lead}    [{key}] => {shown}\n"));
    }
    out.push_str(&format!("{lead})\n"));
    out
}

/// Write a value into an array held in a binding: at a key, or at the
/// end when no key is given. A list written where it already reaches
/// stays a list; any other key turns it into a map, its places becoming
/// the keys.
/// Text with one of its places holding a different letter, given the
/// text a language lets a program write into. Only the first letter of
/// what is handed over is put there; a place beyond the end is reached
/// over spaces, and one counted from the end reaches back from it.
fn letter_put(had: &str, at: &Value, put: &str) -> Result<(Value, bool), String> {
    let mut spelling = put.chars();
    let Some(letter) = spelling.next() else {
        return Err("Cannot write nothing into a place in text".to_string());
    };
    // Whether more was handed over than the place has room for, which
    // the caller is the one placed to say anything about.
    let over = spelling.next().is_some();
    let mut letters: Vec<char> = had.chars().collect();
    // Text standing for a place counts as the number it opens with,
    // the way text counts as a number anywhere else.
    let named = match at {
        Value::Text(_) => match number_opening_in(at).0 {
            Some(counted) => counted,
            None => return Err("A place in text is named by a whole number".to_string()),
        },
        other => other.clone(),
    };
    let step = match &named {
        Value::Small(n) if *n < 0 => letters.len().checked_sub(n.unsigned_abs() as usize),
        other => as_index(other).ok(),
    };
    let Some(step) = step else {
        return Err("A place in text is named by a whole number".to_string());
    };
    if letters.len() <= step {
        letters.resize(step + 1, ' ');
    }
    letters[step] = letter;
    Ok((Value::text(&letters.into_iter().collect::<String>()), over))
}

/// Where the cell a walk handed out is to be found in the array now.
/// The place it was handed out at is tried first, since the body will
/// mostly have left it there; nothing is answered where the item is no
/// longer in the array at all.
fn found_at(walked: &Value, cell: &Rc<RefCell<Value>>, stood: usize) -> Option<usize> {
    let itself = |x: &Value| matches!(x, Value::Shared(other) if Rc::ptr_eq(other, cell));
    match walked {
        Value::Vector(items) => match items.get(stood).map_or(false, &itself) {
            true => Some(stood),
            false => items.iter().position(itself),
        },
        Value::Dict(entries) => match entries.get(stood).map_or(false, |(_, x)| itself(x)) {
            true => Some(stood),
            false => entries.iter().position(|(_, x)| itself(x)),
        },
        _ => None,
    }
}

/// Values put before everything an array holds, in the order given.
/// The places keep their words where they are named by words, and are
/// numbered afresh from nought where they are named by numbers, the
/// newcomers taking the first numbers. The answer is how many places
/// there are afterwards.
fn put_before(held: &mut Value, coming: Vec<Value>, name: &str) -> Result<usize, String> {
    let mut numbered = 0i64;
    let mut afresh = |k: Option<&Value>| match k {
        Some(Value::Text(word)) => Value::Text(word.clone()),
        _ => {
            numbered += 1;
            Value::Small(numbered - 1)
        }
    };
    let mut all: Vec<(Value, Value)> = coming.into_iter().map(|x| (afresh(None), x)).collect();
    match &*held {
        Value::Vector(items) => all.extend(items.iter().map(|x| (afresh(None), x.clone()))),
        Value::Dict(entries) => all.extend(entries.iter().map(|(k, x)| (afresh(Some(k)), x.clone()))),
        other => return Err(format!("{}() cannot put a value in front of {}", name, other.bare())),
    }
    let many = all.len();
    // Where the places run nought, one, two and so on, the array is
    // the plain one it looks like and is given back as such.
    let plain = all.iter().enumerate().all(|(at, (k, _))| matches!(k, Value::Small(n) if *n == at as i64));
    *held = match plain {
        true => Value::Vector(Rc::new(all.into_iter().map(|(_, x)| x).collect())),
        false => Value::Dict(Rc::new(all)),
    };
    Ok(many)
}

fn written_into(held: &mut Value, key: Option<Value>, value: Value, no_places: &str, builds: bool, letter: Option<String>) -> Result<bool, String> {
    // Where a language writes into text, a named place in text takes a
    // letter and the name goes on holding text.
    if let (Value::Text(had), Some(put), Some(at)) = (&*held, &letter, &key) {
        let (made, over) = letter_put(had, at, put)?;
        *held = made;
        return Ok(over);
    }
    if builds && matches!(held, Value::Nil | Value::Unset) {
        *held = Value::Vector(Rc::new(Vec::new()));
    }
    let stays = match (&*held, &key) {
        (Value::Vector(items), Some(k)) => as_index(k).map_or(false, |at| at < items.len()),
        (Value::Vector(_), None) => true,
        _ => false,
    };
    if let (Value::Vector(items), true) = (&mut *held, stays) {
        // A place keeping a cell that names share is written through,
        // not written over.
        if let Some(k) = &key {
            if let Some(Value::Shared(cell)) = items.get(as_index(k)?) {
                *cell.borrow_mut() = value;
                return Ok(false);
            }
        }
        let items = Rc::make_mut(items);
        match key {
            Some(k) => items[as_index(&k)?] = value,
            None => items.push(value),
        }
        return Ok(false);
    }
    if let Value::Vector(items) = &*held {
        let spread = items.iter().enumerate().map(|(at, x)| (Value::Small(at as i64), x.clone())).collect();
        *held = Value::Dict(Rc::new(spread));
    }
    let Value::Dict(entries) = held else {
        // A value with no places at all is written to in the words the
        // language has for that, as reading such a place is.
        return Err(no_places.to_string());
    };
    let entries = Rc::make_mut(entries);
    let key = key.unwrap_or_else(|| Value::Small(after_keys(entries)));
    set_key(entries, key, value);
    Ok(false)
}

/// The place an array holds, made a shared cell, so that a name tied to
/// it writes into the array itself. A walk counts places, so a map is
/// reached by its position as a vector is.
fn shared_item(held: &mut Value, at: &Value) -> Result<Rc<RefCell<Value>>, String> {
    let i = as_index(at)?;
    // A thing's members are counted as an array's places are, but they
    // are kept behind a shared holding, so the cell is made within it.
    if let Value::Thing(thing) = held {
        let mut holds = thing.holds.borrow_mut();
        let reach = holds.len();
        let Some((_, place)) = holds.get_mut(i) else {
            return Err(format!("Array index {} out of bounds (length: {})", i, reach));
        };
        if let Value::Shared(cell) = place {
            return Ok(cell.clone());
        }
        let cell = Rc::new(RefCell::new(std::mem::replace(place, Value::Nil)));
        *place = Value::Shared(cell.clone());
        return Ok(cell);
    }
    let place: &mut Value = match held {
        Value::Vector(items) => {
            let items = Rc::make_mut(items);
            let reach = items.len();
            items.get_mut(i).ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, reach))?
        }
        Value::Dict(entries) => {
            let entries = Rc::make_mut(entries);
            let reach = entries.len();
            let entry = entries.get_mut(i).ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, reach))?;
            &mut entry.1
        }
        _ => return Err("Cannot walk a value that is not an array".to_string()),
    };
    if let Value::Shared(cell) = place {
        return Ok(cell.clone());
    }
    let cell = Rc::new(RefCell::new(std::mem::replace(place, Value::Nil)));
    *place = Value::Shared(cell.clone());
    Ok(cell)
}

/// The cell of the place a chain of keys names, the arrays and the
/// places along the way made where they are not there yet and the
/// language makes what a write needs.
fn shared_deep(held: &mut Value, keys: &[Value], makes: bool) -> Result<Rc<RefCell<Value>>, String> {
    let Some((last, first)) = keys.split_last() else {
        return Err("No place was named".to_string());
    };
    let mut spot = held;
    for k in first {
        spot = place_within(spot, k, makes)?;
    }
    let place = place_within(spot, last, makes)?;
    if let Value::Shared(cell) = place {
        return Ok(cell.clone());
    }
    let cell = Rc::new(RefCell::new(std::mem::replace(place, Value::Nil)));
    *place = Value::Shared(cell.clone());
    Ok(cell)
}

fn place_within<'a>(held: &'a mut Value, at: &Value, makes: bool) -> Result<&'a mut Value, String> {
    if makes && matches!(held, Value::Nil | Value::Unset) {
        *held = Value::Vector(Rc::new(Vec::new()));
    }
    // A list holds only the places it already has: a key that is no
    // whole number, or one past the last, makes it a map, which is what
    // a plain write into such a place does too.
    let beyond = match (&*held, as_index(at)) {
        (Value::Vector(items), Ok(i)) => i >= items.len(),
        _ => true,
    };
    if matches!(held, Value::Vector(_)) && beyond {
        let Value::Vector(items) = &*held else { unreachable!("a list") };
        let spread = items.iter().enumerate().map(|(i, x)| (Value::Small(i as i64), x.clone())).collect();
        *held = Value::Dict(Rc::new(spread));
    }
    let place: &mut Value = match held {
        Value::Vector(items) => {
            let i = as_index(at)?;
            let items = Rc::make_mut(items);
            let reach = items.len();
            items.get_mut(i).ok_or_else(|| format!("Array index {} out of bounds (length: {})", i, reach))?
        }
        Value::Dict(entries) => {
            let entries = Rc::make_mut(entries);
            let found = entries.iter().position(|(k, _)| k.equals(at));
            let where_at = match found {
                Some(i) => i,
                None if makes => {
                    entries.push((at.clone(), Value::Nil));
                    entries.len() - 1
                }
                None => return Err(format!("Undefined array key {}", at.bare())),
            };
            &mut entries[where_at].1
        }
        _ => return Err("Cannot take a cell from a place in something that is not an array".to_string()),
    };
    Ok(place)
}

/// The number a piece of text spells, whole or fractional, with room for
/// a sign and for space around it; anything that is not such text spells
/// no number at all.
/// Text walked along its letters: the last one moves on, `z` coming
/// round to `a` and carrying into the one before, `Z` to `A` and `9` to
/// `0` likewise. A mark that is neither letter nor digit halts the walk
/// where it stands, and a carry off the front sets a fresh `a`, `A` or
/// `1` before what came round first.
fn letters_walked(s: &str) -> String {
    if s.is_empty() {
        return "1".to_string();
    }
    let mut marks: Vec<u8> = s.as_bytes().to_vec();
    let (mut at, mut carried) = (marks.len(), true);
    while carried && at > 0 {
        at -= 1;
        match marks[at] {
            b'z' => marks[at] = b'a',
            b'Z' => marks[at] = b'A',
            b'9' => marks[at] = b'0',
            m @ (b'a'..=b'y' | b'A'..=b'Y' | b'0'..=b'8') => {
                marks[at] = m + 1;
                carried = false;
            }
            // Anything else does not move, and halts the walk.
            _ => return String::from_utf8_lossy(&marks).into_owned(),
        }
    }
    if carried {
        marks.insert(0, match marks.first() {
            Some(b'a') => b'a',
            Some(b'A') => b'A',
            _ => b'1',
        });
    }
    String::from_utf8_lossy(&marks).into_owned()
}

fn number_spelled_in(v: &Value) -> Option<Value> {
    let Value::Text(s) = v else { return None };
    let text = s.trim();
    // A number may carry a power of ten after it: 1e2, 1.5E-3.
    if let Some(at) = text.find(['e', 'E']) {
        let (front, back) = text.split_at(at);
        let power: i32 = back[1..].parse().ok()?;
        // A power of ten written after it makes a real of it, whole or
        // not, as it does where the number is written in a program.
        let (above, beneath) = match number_spelled_in(&Value::text(front))? {
            Value::Frac(e) => (e.above.clone(), e.beneath.clone()),
            Value::Small(n) => (BigInt::from(n), BigInt::from(1)),
            Value::Huge(n) => ((*n).clone(), BigInt::from(1)),
            _ => return None,
        };
        let scale = BigInt::from(10).pow(power.unsigned_abs());
        let (above, beneath) = match power < 0 {
            true => (above, beneath * scale),
            false => (above * scale, beneath),
        };
        return Some(math::make_number(above, beneath, Some(15)));
    }
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let Some((before, after)) = digits.split_once('.') else {
        return digits.parse::<BigInt>().ok().map(|n| Value::from_big(n * sign));
    };
    if after.is_empty() {
        return before.parse::<BigInt>().ok().map(|n| Value::from_big(n * sign));
    }
    let scale = BigInt::from(10).pow(after.len() as u32);
    let above: BigInt = if before.is_empty() { BigInt::from(0) } else { before.parse().ok()? };
    let below: BigInt = after.parse().ok()?;
    let digits_told = (before.len() + after.len()).max(15);
    Some(math::make_number((above * &scale + below) * sign, scale, Some(digits_told)))
}


/// A value as a whole number of sixty-four bits. Text spelling a number
/// stands for it and text spelling none stands for nothing; what lies
/// past the point is dropped towards nothing, so -1.5 stands for -1.
fn sixty_four(v: &Value) -> Result<i64, String> {
    let number = match v {
        Value::Small(n) => return Ok(*n),
        Value::Flag(yes) => return Ok(i64::from(*yes)),
        Value::Nil | Value::Unset => return Ok(0),
        Value::Huge(n) => return Ok(n.to_i64().unwrap_or(i64::MIN)),
        Value::Text(_) => match number_spelled_in(v) {
            Some(n) => n,
            None => return Ok(0),
        },
        other => other.clone(),
    };
    match math::whole_part(&number) {
        // Dividing whole numbers cuts towards nothing, which is what
        // dropping what lies past the point comes to. A number too wide
        // for the bits at all comes to the lowest of them, as it does on
        // a machine that holds numbers to a width.
        Some(n) => Ok(n.to_i64().unwrap_or(i64::MIN)),
        None => Err("Working on bits needs a whole number".to_string()),
    }
}

/// A key as a language whose keys are plain takes one: text spelling a
/// whole number is that number, so `a['7']` and `a[7]` name one place;
/// a number with a point stands for the whole number towards nothing; a
/// flag stands for 1 or 0, and nothing for text with nothing in it.
/// Text spelling a number any other way stays as it was written.
fn key_as_taken(v: Value) -> Value {
    match &v {
        Value::Text(s) => match whole_number_spelled(s) {
            Some(n) => Value::Small(n),
            None => v,
        },
        Value::Flag(yes) => Value::Small(i64::from(*yes)),
        Value::Nil | Value::Unset => Value::text(""),
        Value::Frac(_) => match sixty_four(&v) {
            Ok(n) => Value::Small(n),
            Err(_) => v,
        },
        _ => v,
    }
}

/// The whole number a piece of text spells, where it spells one the way
/// a whole number is written out: digits, a minus before them at most,
/// no space around them and no nought leading.
fn whole_number_spelled(s: &str) -> Option<i64> {
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => (-1i64, rest),
        None => (1, s),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    if sign < 0 && digits == "0" {
        return None;
    }
    digits.parse::<i64>().ok().map(|n| n * sign)
}

/// The number a piece of text opens with, and whether that is the whole
/// of it: "12" is twelve and the whole, "12abc" twelve and not, and
/// "abc" no number at all.
fn number_opening_in(v: &Value) -> (Option<Value>, bool) {
    if let Some(whole) = number_spelled_in(v) {
        return (Some(whole), true);
    }
    let Value::Text(s) = v else { return (None, false) };
    let text = s.trim_start();
    let letters = text.as_bytes();
    let mut end = usize::from(matches!(letters.first(), Some(b'-') | Some(b'+')));
    let mut had_point = false;
    while end < letters.len() {
        let c = letters[end];
        if c.is_ascii_digit() {
            end += 1;
        } else if c == b'.' && !had_point {
            had_point = true;
            end += 1;
        } else {
            break;
        }
    }
    // A power of ten goes with the number before it, so long as digits
    // come after it: `123e5xyz` opens with a number, `123exyz` opens
    // with 123.
    if end > 0 && matches!(letters.get(end), Some(b'e') | Some(b'E')) {
        let mut past = end + 1;
        if matches!(letters.get(past), Some(b'-') | Some(b'+')) {
            past += 1;
        }
        let first = past;
        while letters.get(past).map_or(false, u8::is_ascii_digit) {
            past += 1;
        }
        if past > first {
            end = past;
        }
    }
    match number_spelled_in(&Value::text(&text[..end])) {
        Some(opening) => (Some(opening), false),
        None => (None, false),
    }
}

impl Machine<'_> {
    /// Imported text is built above the world's old addresses. The new
    /// names are then filed away, while its forms still reach their cells.
    fn load_namespace(&mut self, path: &str) -> Result<Value, String> {
        if let Some(value) = self.imported.get(path) { return Ok(value.clone()); }
        if path.starts_with('.') {
            return Err(self.table.single("ext.stmt.import.relative.unready").unwrap_or_default().to_string());
        }
        let text = self.library_sources.get(path).cloned().ok_or_else(|| {
            let (before, after) = self.table.around("ext.stmt.import.missing").unwrap_or(("", ""));
            format!("{before}{path}{after}")
        })?;
        let split = path.rsplit_once('.');
        if let Some((owner, _)) = split { self.load_namespace(owner)?; }
        let beginning = self.idents.len();
        let hidden: Vec<String> = (0..beginning).map(|n| format!("\0prior/{n}")).collect();
        let scanned = crate::scan::scan(&text, self.table)?;
        let ready = crate::indent::indent(scanned, self.table, 0).map_err(|(words, _)| words)?;
        let built = crate::build::build(&ready, self.table, &hidden, HashMap::new(), false, 0)?;
        let exported = &built.globals[beginning..];
        self.idents.extend(exported.iter().map(|name| format!("\0import/{path}/{name}")));
        let mut members = Vec::with_capacity(exported.len());
        {
            let mut world = self.outermost.cells.borrow_mut();
            world.resize(self.idents.len(), Value::Unset);
            for (position, name) in exported.iter().enumerate() {
                let initial = match self.table.strings("ext.system.module.name").contains(name) {
                    true => Value::text(path), false => self.fault_roots.get(name.trim_end_matches(crate::form::OF_A_CLASS)).cloned().unwrap_or(Value::Unset),
                };
                let link = Value::Shared(Rc::new(RefCell::new(initial)));
                members.push((name.clone(), link.clone()));
                world[beginning + position] = link;
            }
        }
        for word in self.table.strings("ext.system.module.name") {
            if !members.iter().any(|(name, _)| name == word) { members.push((word.clone(), Value::text(path))); }
        }
        let kind = Blueprint {
            name: path.into(), under: None, methods: Vec::new(), constants: Vec::new(),
            shared: RefCell::new(Vec::new()), fields: Vec::new(), answers: Vec::new(), reaches: Vec::new(),
        };
        self.made += 1;
        let value = Value::Thing(Rc::new(Thing { of: Rc::new(kind), holds: RefCell::new(members), turn: self.made }));
        self.imported.insert(path.into(), value.clone());
        let scope = self.outermost.clone();
        if let Err(stopped) = self.value_of(&built.program.body, &scope) {
            self.imported.remove(path);
            self.refresh_import_table();
            return Err(match stopped { Escape::Error(said) => said, other => { self.got_away = Some(other); "module did not finish".into() } });
        }
        if let Some((owner, name)) = split {
            if let Some(Value::Thing(parent)) = self.imported.get(owner) {
                let mut holdings = parent.holds.borrow_mut();
                if let Some(at) = holdings.iter().position(|entry| entry.0 == name) {
                    if let Value::Shared(link) = &holdings[at].1 { *link.borrow_mut() = value.clone(); }
                    else { holdings[at].1 = value.clone(); }
                } else { holdings.push((name.into(), value.clone())); }
            }
        }
        self.refresh_import_table();
        Ok(value)
    }

    fn namespace_answers(&self, value: &Value) -> bool {
        let Value::Thing(space) = value else { return false };
        self.table.single("ext.system.module.getattr").and_then(|name| self.attribute(value, name)).is_some()
            && self.imported.values().any(|other| matches!(other, Value::Thing(held) if Rc::ptr_eq(space, held)))
    }

    fn ask_namespace(&mut self, value: &Value, missing: &str) -> Result<Option<Value>, String> {
        let Some(named) = self.table.single("ext.system.module.getattr") else { return Ok(None) };
        let Value::Thing(space) = value else { return Ok(None) };
        let belongs = self.imported.values().any(|stored| match stored {
            Value::Thing(other) => Rc::ptr_eq(space, other), _ => false,
        });
        if !belongs { return Ok(None); }
        let Some(answer) = self.attribute(value, named) else { return Ok(None) };
        let (routine, scope) = match answer {
            Value::Bound(body, scope) => (body, scope),
            _ => return Ok(None),
        };
        match self.invoke(routine, scope, vec![Value::text(missing)]) {
            Ok(value) => Ok(Some(value)),
            Err(Escape::Error(words)) => Err(words),
            Err(escape) => { self.got_away = Some(escape); Err("module member did not finish".into()) }
        }
    }

    fn namespace_item(&mut self, value: &Value, path: &str, wanted: &str) -> Result<Value, String> {
        if let Value::Thing(space) = value {
            for (name, cell) in space.holds.borrow().iter() {
                if name != wanted { continue; }
                let read = match cell { Value::Shared(link) => link.borrow().clone(), worth => worth.clone() };
                if !matches!(read, Value::Unset) { return Ok(read); }
            }
        }
        let full = format!("{path}.{wanted}");
        if self.library_sources.contains_key(&full) { return self.load_namespace(&full); }
        if let Some(answer) = self.ask_namespace(value, wanted)? { return Ok(answer); }
        let (head, tail) = self.table.around("ext.stmt.import.member.missing").unwrap_or(("", ""));
        Err(format!("{head}{wanted}{tail}"))
    }
}

fn belongs_to(worth: &Value, kind: &Value) -> bool {
    match kind {
        Value::Vector(choices) => choices.iter().any(|choice| belongs_to(worth, choice)),
        Value::Blueprint(class) => {
            let Value::Thing(object) = worth else { return false };
            let mut current = object.of.clone();
            loop {
                if Rc::ptr_eq(&current, class) { return true; }
                match current.under.clone() { Some(parent) => current = parent, None => return false }
            }
        },
        Value::KindOf(tag) => worth.kind() == Some(*tag),
        _ => false,
    }
}

impl Machine<'_> {
    fn copy_worth(&mut self, value: &Value, descend: bool, known: &mut Vec<(usize, Value)>) -> Value {
        if let Value::Thing(original) = value {
            let key = Rc::as_ptr(original) as usize;
            if descend {
                if let Some((_, held)) = known.iter().find(|(at, _)| *at == key) { return held.clone(); }
            }
            self.made += 1;
            let target = Rc::new(Thing { of: original.of.clone(), turn: self.made, holds: RefCell::new(Vec::new()) });
            let answer = Value::Thing(target.clone());
            known.push((key, answer.clone()));
            for (name, field) in original.holds.borrow().iter() {
                let item = if descend { self.copy_worth(field, true, known) } else { field.clone() };
                target.holds.borrow_mut().push((name.clone(), item));
            }
            return answer;
        }
        match value {
            Value::Shared(cell) => self.copy_worth(&cell.borrow(), descend, known),
            Value::Vector(items) if descend => {
                let items = items.iter().map(|item| self.copy_worth(item, true, known)).collect();
                Value::Vector(Rc::new(items))
            }
            Value::Dict(pairs) if descend => {
                let mut copied = Vec::with_capacity(pairs.len());
                for (key, item) in pairs.iter() {
                    copied.push((self.copy_worth(key, true, known), self.copy_worth(item, true, known)));
                }
                Value::Dict(Rc::new(copied))
            }
            _ => value.clone(),
        }
    }
}

impl Machine<'_> {
    fn refresh_import_table(&self) {
        let names = self.table.strings("ext.system.module.cache");
        if names.len() != 2 { return; }
        if let Some(Value::Thing(namespace)) = self.imported.get(&names[0]) {
            let dictionary = Value::Dict(Rc::new(self.imported.iter().map(|(key, worth)| (Value::text(key), worth.clone())).collect()));
            for (key, worth) in namespace.holds.borrow_mut().iter_mut() {
                if key == &names[1] {
                    if let Value::Shared(cell) = worth { *cell.borrow_mut() = dictionary; }
                    else { *worth = dictionary; }
                    break;
                }
            }
        }
    }
}

impl Machine<'_> {
    fn keeping_fault(&self, which: usize) -> String {
        self.table.strings("ext.builtin.pickle.amiss").get(which).cloned().unwrap_or_default()
    }

    fn state_answer(&mut self, thing: &Rc<Thing>, word: &str, values: Vec<Value>) -> Result<Option<Value>, String> {
        if let Some(routine) = thing.of.method(word).cloned() {
            let call = Form::Apply(Callee::Code(Box::new(Form::Const(Value::Method(routine, thing.clone())))), values.into_iter().map(Form::Const).collect());
            let frame = Rc::clone(&self.outermost);
            return match self.value_of(&call, &frame) {
                Ok(answer) => Ok(Some(answer)),
                Err(Escape::Error(message)) => Err(message),
                Err(escape) => { self.got_away = Some(escape); Err(self.keeping_fault(0)) }
            };
        }
        Ok(None)
    }

    fn keep_worth(&mut self, worth: &Value, numbered: &mut Vec<usize>, level: usize) -> Result<serde_json::Value, String> {
        use serde_json::{Value as J, json};
        if level == 257 { return Err(self.keeping_fault(0)); }
        if let Value::Shared(place) = worth {
            let inner = place.borrow().clone();
            return self.keep_worth(&inner, numbered, level + 1);
        }
        let pointer = match worth {
            Value::Thing(x) => Rc::as_ptr(x) as usize,
            Value::Vector(x) => Rc::as_ptr(x) as usize,
            Value::Dict(x) => Rc::as_ptr(x) as usize,
            _ => 0,
        };
        let ordinal = numbered.len();
        if pointer != 0 {
            if let Some(previous) = numbered.iter().position(|p| *p == pointer) { return Ok(json!(["ref", previous])); }
            numbered.push(pointer);
        }
        let result = match worth {
            Value::Text(chars) => json!(["str", chars.to_string()]),
            Value::Flag(truth) => json!(["bool", truth]),
            Value::Nil => json!(["none"]),
            Value::Huge(_) | Value::Small(_) => json!(["int", worth.bare()]),
            Value::Frac(number) => match number.places {
                Some(precision) => json!(["real", number.above.to_string(), number.beneath.to_string(), precision, number.under, number.float_style]),
                None => json!(["ratio", number.above.to_string(), number.beneath.to_string()]),
            },
            Value::Vector(sequence) => {
                let children: Result<Vec<J>, String> = sequence.iter().map(|part| self.keep_worth(part, numbered, level + 1)).collect();
                json!(["array", ordinal, children?])
            }
            Value::Dict(entries) => {
                let mut contents = vec![];
                for entry in entries.as_ref() {
                    let key = self.keep_worth(&entry.0, numbered, level + 1)?;
                    let item = self.keep_worth(&entry.1, numbered, level + 1)?;
                    contents.push(J::Array(vec![key, item]));
                }
                json!(["map", ordinal, contents])
            }
            Value::Thing(instance) => {
                let agrees = |v: &Value| match v {
                    Value::Blueprint(b) => Rc::ptr_eq(b, &instance.of),
                    Value::Shared(p) => matches!(&*p.borrow(), Value::Blueprint(b) if Rc::ptr_eq(b, &instance.of)),
                    _ => false,
                };
                let origin = self.imported.iter().find_map(|(name, namespace)| {
                    if let Value::Thing(scope) = namespace {
                        if scope.holds.borrow().iter().any(|(_, v)| agrees(v)) { return Some(name.clone()); }
                    }
                    None
                });
                if origin.is_none() && !self.outermost.cells.borrow().iter().any(agrees) { return Err(self.keeping_fault(0)); }
                let methods = self.table.strings("ext.builtin.pickle.hooks");
                if methods.len() > 2 && instance.of.method(&methods[2]).is_some() { return Err(self.keeping_fault(0)); }
                let alternate = if methods.is_empty() { None } else { self.state_answer(instance, &methods[0], Vec::new())? };
                let changed = alternate.is_some();
                let state = match alternate {
                    Some(state) => state,
                    None => Value::Dict(Rc::new(instance.holds.borrow().iter().map(|(name, item)| (Value::text(name), item.clone())).collect())),
                };
                let encoded = self.keep_worth(&state, numbered, level + 1)?;
                json!(["object", ordinal, origin.unwrap_or_default(), instance.of.name, changed, encoded])
            }
            Value::Bound(_, _) | Value::Method(_, _) | Value::Routine(_) => return Err(self.keeping_fault(2)),
            _ => return Err(self.keeping_fault(0)),
        };
        Ok(result)
    }

    fn restore_worth(&mut self, record: &serde_json::Value, restored: &mut Vec<(usize, Value)>, level: usize) -> Result<Value, String> {
        let fault = self.keeping_fault(1);
        if level > 256 { return Err(fault); }
        let list = record.as_array().ok_or_else(|| fault.clone())?;
        let word = |position| list.get(position).and_then(serde_json::Value::as_str).ok_or_else(|| fault.clone());
        let count = |position| list.get(position).and_then(serde_json::Value::as_u64).and_then(|n| usize::try_from(n).ok()).ok_or_else(|| fault.clone());
        let number = |position| -> Result<BigInt, String> { word(position)?.parse().map_err(|_| fault.clone()) };
        let tag = word(0)?;
        if tag == "ref" && list.len() == 2 {
            let wanted = count(1)?;
            return restored.iter().find(|(n, _)| *n == wanted).map(|(_, v)| v.clone()).ok_or(fault);
        }
        let scalar = match (tag, list.len()) {
            ("str", 2) => Some(Value::text(word(1)?)),
            ("int", 2) => Some(Value::from_big(number(1)?)),
            ("none", 1) => Some(Value::Nil),
            ("bool", 2) => Some(Value::Flag(list[1].as_bool().ok_or_else(|| fault.clone())?)),
            ("real", 6) => Some(Value::Frac(Rc::new(crate::data::Ratio {
                above: number(1)?, beneath: number(2)?, places: Some(count(3)?),
                float_style: list[5].as_bool().ok_or_else(|| fault.clone())?, under: list[4].as_bool().ok_or_else(|| fault.clone())?,
            }))),
            ("ratio", 3) => {
                let denominator = number(2)?;
                if denominator <= BigInt::from(1) { return Err(fault); }
                Some(Value::Frac(Rc::new(crate::data::Ratio { above: number(1)?, beneath: denominator, places: None, float_style: false, under: false })))
            }
            _ => None,
        };
        if let Some(scalar) = scalar { return Ok(scalar); }
        let ordinal = count(1)?;
        if restored.iter().any(|(old, _)| *old == ordinal) { return Err(fault); }
        if (tag == "array" || tag == "map") && list.len() == 3 {
            let entries = list[2].as_array().ok_or_else(|| fault.clone())?;
            let mut sequence = Vec::new();
            let mut mapping = Vec::new();
            for entry in entries {
                if tag == "array" { sequence.push(self.restore_worth(entry, restored, level + 1)?); }
                else {
                    let parts = entry.as_array().filter(|e| e.len() == 2).ok_or_else(|| fault.clone())?;
                    let key = self.restore_worth(&parts[0], restored, level + 1)?;
                    let item = self.restore_worth(&parts[1], restored, level + 1)?;
                    mapping.push((key, item));
                }
            }
            let value = if tag == "array" { Value::Vector(Rc::new(sequence)) } else { Value::Dict(Rc::new(mapping)) };
            restored.push((ordinal, value.clone()));
            return Ok(value);
        }
        if tag != "object" || list.len() != 6 { return Err(fault); }
        let owner = word(2)?;
        let title = word(3)?;
        let find_class = |v: &Value| {
            let held = if let Value::Shared(c) = v { c.borrow().clone() } else { v.clone() };
            match held { Value::Blueprint(b) if b.name == title => Some(b), _ => None }
        };
        let shape = if owner.is_empty() { self.outermost.cells.borrow().iter().find_map(find_class) }
        else {
            match self.load_namespace(owner)? {
                Value::Thing(scope) => scope.holds.borrow().iter().find_map(|(_, v)| find_class(v)),
                _ => None,
            }
        }.ok_or_else(|| fault.clone())?;
        self.made += 1;
        let fresh = Rc::new(Thing { turn: self.made, of: shape, holds: RefCell::new(vec![]) });
        let result = Value::Thing(Rc::clone(&fresh));
        restored.push((ordinal, result.clone()));
        let state = self.restore_worth(&list[5], restored, level + 1)?;
        let names = self.table.strings("ext.builtin.pickle.hooks");
        if names.len() >= 2 && self.state_answer(&fresh, &names[1], vec![state.clone()])?.is_some() { return Ok(result); }
        match state {
            Value::Nil => (),
            Value::Dict(entries) => for (key, data) in entries.iter() {
                match key {
                    Value::Text(name) => fresh.holds.borrow_mut().push((name.to_string(), data.clone())),
                    _ => return Err(fault),
                }
            },
            _ => return Err(fault),
        }
        Ok(result)
    }
}
