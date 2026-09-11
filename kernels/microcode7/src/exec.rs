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

use num_traits::{ToPrimitive, Zero};

use crate::math::{self, Calc};
use crate::table::Table;
use crate::form::{Input, Traps, Form, Prim, Routine, Address, Callee};
use crate::data::{Adornment, IteratorKind, IteratorState, Blueprint, Env, Kind, Names, Reach, Thing, Value};

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

/// The forms still owed by a yielding routine, innermost work last.
enum Owed {
    Find(Form),
    Store(Address),
    Drop,
    Apply(Callee, usize),
    Call(usize),
    Select(Form, Form),
    Test(Rc<Form>),
    Decide(Rc<Form>),
    Turn(Rc<Form>, usize),
    HandOut,
    From,
    Finish,
    Truth(Prim, Form),
}

pub struct Suspension {
    frame: Rc<Env>,
    owed: Vec<Owed>,
    found: Vec<Value>,
    begun: bool,
    ended: bool,
    receiving: bool,
    result: Value,
    inner: Option<Value>,
    members: Option<std::vec::IntoIter<Value>>,
    ready: Option<Value>,
    /// The cell of the map the members were taken from, and how many
    /// pairs it held then, so a step may notice the map has grown or
    /// shrunk under the walk.
    overseen: Option<(Rc<RefCell<Value>>, usize)>,
}

impl Suspension {
    fn body(program: &Routine, frame: Rc<Env>) -> Self {
        Self { frame, owed: vec![Owed::Find(program.body.clone())], found: Vec::new(),
            begun: false, ended: false, receiving: false, result: Value::Nil,
            inner: None, members: None, ready: None, overseen: None }
    }
}

pub struct Machine<'a> {
    pub library_sources: HashMap<String, String>,
    imported: HashMap<String, Value>,
    /// The dictionary kept beside the outermost names once a program
    /// has asked for it: from then on those names are read out of it
    /// and written into it, so either side sees the other's writing.
    world_book: Option<Rc<RefCell<Value>>>,
    /// Each text read into dictionaries handed over: the slots it was
    /// given, the dictionary its names live in, and the outer one for
    /// what the near one lacks and for its declared globals.
    readings: Vec<Namebook>,
    /// The reading the run stands inside at present, if any.
    reading_now: Option<usize>,
    /// The dictionary of builtin words, and the blueprint of a code
    /// value, each made when first wanted.
    natives_book: Option<Rc<RefCell<Value>>>,
    code_kind: Option<Rc<Blueprint>>,
    ancestor: Option<Rc<Blueprint>>,
    /// The blueprint of properties, once one has been asked for.
    property_kind: Option<Rc<Blueprint>>,
    /// The blueprints standing for native kinds, one for each word a class has stood on.
    native_kinds: Vec<(String, Rc<Blueprint>)>,
    routine_members: Vec<(Value, Rc<Thing>)>,
    table: &'a Table,
    fault_kinds: HashMap<String, Value>,
    pub outermost: Rc<Env>,
    idents: Vec<String>,
    memo: HashMap<String, Value>,
    identities: Vec<(String, u64)>,
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
    /// Whether a member is being read only to learn if it is there.
    asking_presence: bool,
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
    /// How many calls stand under way, one in tail position counted as
    /// deepening the run though it takes the place of the call before.
    standing: usize,
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
    fn given_faults(table: &Table) -> HashMap<String, Value> {
        let mut chain: Vec<Rc<Blueprint>> = Vec::new();
        for (number, word) in table.strings("ext.builtin.exceptions").iter().enumerate() {
            let parent = match number {
                0 => None, 1 | 17 | 18 => Some(0), 3 | 4 => Some(2),
                6 | 7 => Some(5), 11 => Some(10), 14 | 21 => Some(13),
                22 => Some(9), 25..=35 => Some(24), _ => Some(1),
            };
            let mut seed = Vec::new();
            match number {
                0 => seed.push(("\0fault-kind".to_string(), Value::Flag(true))),
                7 => seed.push(("\0key-fault".to_string(), Value::Flag(true))),
                _ => {}
            }
            let kind = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
                name: word.clone(), under: parent.and_then(|p| chain.get(p).cloned()),
                fields: seed, reaches: vec![], answers: vec![], methods: vec![],
                shared: RefCell::new(vec![]), constants: vec![],
            };
            chain.push(Rc::new(kind));
        }
        chain.into_iter().map(|b| (b.name.clone(), Value::Blueprint(b))).collect()
    }

    fn is_fault_kind(&self, kind: &Blueprint) -> bool {
        self.table.has_any("ext.builtin.exceptions") && kind.every_field().iter().any(|(k, _)| k == "\0fault-kind")
    }

    fn make_fault(&mut self, kind: Rc<Blueprint>, row: Vec<Value>, because: Value) -> Value {
        self.made += 1;
        let values = Value::Arguments(Rc::new(row));
        let mut holds = kind.every_field();
        let seeded = [
            ("ext.builtin.exceptions.args", values.clone()), ("ext.builtin.exceptions.cause", because),
            ("ext.builtin.exceptions.context", Value::Nil), ("ext.builtin.exceptions.suppress", Value::Flag(false)),
        ];
        for (label, value) in seeded {
            if let Some(key) = self.table.single(label) { holds.push((key.to_string(), value)); }
        }
        holds.push(("\0raised-values".into(), values));
        Value::Thing(Rc::new(Thing { of: kind, turn: self.made, holds: RefCell::new(holds) }))
    }

    /// What was being handled when a value is raised stays with that
    /// value as its context, where the table names a member for it: the
    /// innermost fault held, unless that is the value itself. Should the
    /// contexts behind the held fault lead back round to the value
    /// raised, the link that would is emptied first, so that no fault
    /// ever stands behind itself.
    fn keep_context(&self, raised: &Value) {
        let Some(key) = self.table.single("ext.builtin.exceptions.context") else { return };
        let Value::Thing(thing) = raised.settled() else { return };
        let Some(Value::Thing(handled)) = self.holding_fault.last().map(Value::settled) else { return };
        if Rc::ptr_eq(&handled, &thing) { return; }
        let context_of = |of: &Rc<Thing>| of.holds.borrow().iter().find(|(k, _)| k == key).map(|(_, v)| v.settled());
        let mut step = handled.clone();
        while let Some(Value::Thing(older)) = context_of(&step) {
            if Rc::ptr_eq(&older, &thing) {
                if let Some((_, slot)) = step.holds.borrow_mut().iter_mut().find(|(k, _)| k == key) { *slot = Value::Nil; }
                break;
            }
            step = older;
        }
        let mut holds = thing.holds.borrow_mut();
        match holds.iter_mut().find(|(k, _)| k == key) {
            Some((_, slot)) => *slot = Value::Thing(handled),
            None => holds.push((key.to_string(), Value::Thing(handled))),
        }
    }

    /// A fault of the kernel's own, met where exceptions are furnished,
    /// is raised as a value of the class the table names for it, so a
    /// clause may take it; every other outcome passes as it came.
    fn raised_if_error<T>(&mut self, outcome: Result<T, Escape>) -> Result<T, Escape> {
        match outcome {
            Err(Escape::Error(told)) if self.table.has_any("ext.builtin.exceptions") => match self.as_raised(&told) {
                Some(value) => Err(Escape::Thrown(value)),
                None => Err(Escape::Error(told)),
            },
            other => other,
        }
    }

    /// One more call under way, unless the table's limit on them is
    /// reached already, when the words it gives for that are the fault.
    fn deeper(&mut self) -> Result<(), Escape> {
        if let (Some(limit), Some(words)) = (self.table.count("ext.system.recursion.limit"), self.table.single("ext.system.recursion.exceeded")) {
            if self.standing >= limit { return Err(format!("\0{words}").into()); }
        }
        self.standing += 1;
        Ok(())
    }

    /// The words for a value that cannot manage a context, its kind set
    /// between them, or the plain wrong-answer complaint where the table
    /// gives no such words.
    fn no_manager(&self, kind: &str) -> String {
        match self.table.strings("ext.stmt.with.invalid") {
            [before, after] => format!("\0{before}{kind}{after}"),
            _ => self.bad_answer(),
        }
    }

    fn fault_descends(kind: &Rc<Blueprint>, ancestor: &Rc<Blueprint>) -> bool {
        Rc::ptr_eq(kind, ancestor) || kind.under.as_ref().map_or(false, |parent| Self::fault_descends(parent, ancestor))
    }

    fn fault_methods(kind: &Blueprint) -> bool {
        let mut current = Some(kind);
        while let Some(class) = current {
            if !class.methods.is_empty() { return true; }
            current = class.under.as_deref();
        }
        false
    }

    fn raise_class(&mut self, value: Value, frame: &Rc<Env>) -> Res {
        match value {
            Value::Blueprint(kind) if self.is_fault_kind(&kind) => {
                let call = Form::Apply(Callee::Prim(Prim::Spawn, Rc::from("")), vec![Form::Const(Value::Blueprint(kind))]);
                self.value_of(&call, frame)
            }
            Value::Blueprint(_) => Err(self.table.single("ext.stmt.catch.invalid").unwrap_or_default().to_string().into()),
            other => Ok(other),
        }
    }

    pub fn new(table: &'a Table, idents: Vec<String>) -> Machine<'a> {
        let find = |key: &str| table.single(key).and_then(|n| idents.iter().position(|x| x == n));
        let outermost = Env::make(idents.len(), None);
        if table.has_any("ext.stmt.catch.as") {
            if let Some(at) = find("ext.system.fault.class.value") {
                let blueprint = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
                    name: idents[at].clone(), under: None, answers: Vec::new(),
                    fields: Vec::new(), constants: Vec::new(), methods: Vec::new(),
                    shared: RefCell::new(Vec::new()), reaches: Vec::new(),
                };
                outermost.cells.borrow_mut()[at] = Value::Blueprint(Rc::new(blueprint));
            }
        }
        let fault_kinds = Self::given_faults(table);
        for (i, word) in idents.iter().enumerate() {
            if let Some(value) = fault_kinds.get(word) { outermost.cells.borrow_mut()[i] = value.clone(); }
        }
        Machine {
            fault_kinds,
            library_sources: HashMap::new(),
            imported: HashMap::new(),
            world_book: None,
            readings: Vec::new(),
            reading_now: None,
            natives_book: None,
            code_kind: None,
            ancestor: None, property_kind: None, native_kinds: Vec::new(), routine_members: Vec::new(),
            table,
            outermost,
            args_cell: find("system.args"),
            memo_cell: find("system.memoization"),
            idents,
            memo: HashMap::new(),
            identities: Vec::new(),
            reads_handed: ["ext.builtin.args.all", "ext.builtin.args.count", "ext.builtin.args.at"]
                .iter()
                .any(|label| table.single(label).is_some()),
            handed: Vec::new(),
            pending: Vec::new(),
            made: 0,
            row: 0,
            holding_fault: Vec::new(),
            asking_presence: false,
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
            standing: 0,
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
            Value::Vector(items) | Value::Tuple(items) => items.iter().enumerate().map(|(at, x)| (Value::Small(at as i64), x.clone())).collect(),
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
            Value::Mutable(place, _) => self.stands_true(&place.borrow()),
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
        // A list that a lazy walk begins on is walked in its own cell
        // rather than settled into a copy, so a member the body adds is
        // reached in its turn and one the body takes away is passed over.
        if matches!(op, Prim::Walked) && self.table.flag("ext.stmt.yield.suspends") {
            if let Some(living @ IteratorKind::Living(..)) = v.first().and_then(Self::live_walk) {
                return Ok(Self::cursor_value(living));
            }
        }
        if self.table.flag("ext.syntax.call.bind_names") && v.iter().any(|x| matches!(x, Value::Shared(_))) {
            let items: Vec<Value> = v.iter().map(collection_read).collect();
            return self.walking(op, name, &items);
        }
        if v.iter().any(|item| matches!(item, Value::Mutable(..) | Value::Window(..))) {
            let settled: Vec<Value> = v.iter().map(Value::settled).collect();
            return self.walking(op, name, &settled);
        }
        let n = |want: usize| match v.len() == want {
            true => Ok(()),
            false => Err(Escape::Error(format!("{}() expects {} argument(s), got {}", name, want, v.len()))),
        };
        Ok(match op {
            // A thing may be its own walk, or may hand another over to
            // be walked for it. Either way a walk begins here.
            Prim::Walked => {
                n(1)?;
                if let Value::Blueprint(kind) = &v[0] {
                    // What the class hands over is walked as any walk begins,
                    // here, where the walking is done.
                    if let Some(yielded) = self.blueprint_walk(&kind.clone())? { return self.walking(&Prim::Walked, name, &[yielded]); }
                }
                if let Some(walk) = self.begin_set_walk(&v[0]) { return Ok(walk); }
                // An iterator is walked one member per pass, never gathered
                // up front, so a loop that leaves early leaves the rest.
                if self.table.flag("ext.stmt.yield.suspends") && !matches!(v[0], Value::Thing(_)) {
                    if let Value::Iterator(_) = v[0] { return Ok(v[0].clone()); }
                    return self.make_iterator(v[0].clone());
                }
                let mut walking = v[0].clone();
                if self.table.has_any("ext.stmt.class.special") && matches!(walking, Value::Thing(_) | Value::Cursor(_)) {
                    let iterator = self.user_operation(Prim::Iterator, std::slice::from_ref(&walking))?.ok_or_else(|| self.bad_answer())?;
                    return Ok(Value::Traversal(Rc::new(iterator), Rc::new(RefCell::new(None))));
                }
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
                if let Some(walk) = self.begin_set_walk(&walking) { return Ok(walk); }
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
                if let Value::Generator(state) = &v[0] {
                    let item = self.resume(state, Value::Nil)?;
                    let more = item.is_some();
                    state.borrow_mut().ready = item;
                    return Ok(Value::Flag(more));
                }
                if matches!(v[0], Value::Iterator(_)) { return Ok(Value::Flag(self.iterator_has_more(&v[0])?)); }
                if let Some(contents) = self.check_set_walk(&v[0])? {
                    let more = as_index(&v[1]).map_or(false, |position| position < contents.borrow().entries.len());
                    return Ok(Value::Flag(more));
                }
                if let Value::Traversal(source, present) = &v[0] {
                    let item = self.advance_object(source)?;
                    let more = item.is_some();
                    *present.borrow_mut() = item;
                    return Ok(Value::Flag(more));
                }
                match self.walk_asked(&v[0], self.table.single("ext.op.walk.more").map(str::to_string))? {
                    Some(answer) => Value::Flag(self.stands_true(&answer)),
                    None if matches!(&v[0], Value::Progression(_)) => {
                        let Value::Progression(walk) = &v[0] else { unreachable!() };
                        let count = walk.count();
                        Value::Flag(v[1].as_big().map_or(false, |place| place < count && place >= BigInt::from(0)))
                    }
                    None => {
                        let far = match &v[0] {
                            Value::Vector(items) | Value::Tuple(items) => items.len(),
                            Value::Set(store) => store.borrow().keys.len(),
                            Value::Octets { cell, .. } => cell.borrow().len(),
                            Value::TextRow(row, _) => row.len(),
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
                if let Value::Generator(state) = &v[0] {
                    return Ok(if names { v[1].clone() } else { state.borrow_mut().ready.take().unwrap_or(Value::Nil) });
                }
                if let Value::Traversal(_, present) = &v[0] {
                    return Ok(if names { v[1].clone() } else { present.borrow().clone().ok_or_else(|| self.bad_answer())? });
                }
                let asked = match names {
                    true => "ext.op.walk.key",
                    false => "ext.op.walk.this",
                };
                match self.walk_asked(&v[0], self.table.single(asked).map(str::to_string))? {
                    None if matches!(v[0], Value::Iterator(_)) => if names { v[1].clone() } else { self.next_value(&v[0])?.ok_or_else(|| self.core_complaint("core.exhausted", ""))? },
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
        if matches!(x, Value::TextRow(..) | Value::Octets { .. } | Value::Iterator(_) | Value::Tuple(_) | Value::Set(_) | Value::Vector(_) | Value::Dict(_) | Value::Thing(_) | Value::Progression(_)) {
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
        if self.table.has_any("ext.builtin.exceptions") {
            if let found @ Some(Value::Blueprint(_)) = self.lookup(name) { return found; }
            if let Some(held) = self.fault_kinds.get(name) { return Some(held.clone()); }
        }
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
            Some(r) => math::made_number(r.above, r.beneath, Some(self.real_figures()), r.under).keeping_point(r.pointed),
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
        crate::data::at_binary_width(v, self.table.count("ext.system.real.bits"), self.real_figures(), self.table.lone("system.real.render") == Some("shortest"))
    }

    /// The number a piece of text says, brought to the width the
    /// language holds its numbers in. Without that, a number written
    /// out and the same number said in text would be told apart, one
    /// having been brought to the width and the other not.
    fn number_said(&self, v: &Value) -> Option<Value> {
        number_spelled_in(v).map(|n| self.at_width(n))
    }

    fn quoted_remainder(&self, item: &Value) -> Result<String, String> {
        if let Value::Shared(held) = item {
            return self.quoted_remainder(&held.borrow());
        }
        match item {
            Value::Vector(elements) | Value::Tuple(elements) => {
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
        if self.table.has_any("ext.builtin.format") {
            return crate::formatting::Layout { table: self.table, names: self.wording() }.remainder(pattern, rhs);
        }
        let unsupported = self.table.single("ext.op.rem.format.unsupported").unwrap_or_default();
        let mismatch = self.table.single("ext.op.rem.format.arguments").unwrap_or_default();
        let supplied = match rhs { Value::Vector(list) | Value::Tuple(list) => list.as_slice(), _ => std::slice::from_ref(rhs) };
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
            brief_reals: self.table.lone("system.real.render") == Some("shortest"),
            // A language may show nothing as no text at all, as PHP does,
            // rather than as the word a program writes for it.
            real_figures: self.table.count("ext.system.real.bits").and(self.table.count("ext.system.real.digits")),
            bit_reals: self.table.count("ext.system.real.bits").is_some(),
            nil: match self.table.flag("literal.null.silent") {
                true => "",
                false => self.table.single("literal.null").unwrap_or("null"),
            },
            within_word: self.table.single("ext.stmt.class.guarded"),
            alone_word: self.table.single("ext.stmt.class.hidden"),
            kept_as_bytes: self.table.flag("ext.system.text.bytes"),
            keys_by_worth: self.table.flag("ext.syntax.map.value_keys"),
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
        let told = told.trim_start_matches('\0');
        if self.table.has_any("ext.builtin.exceptions") {
            for kind in self.table.strings("ext.builtin.exceptions") {
                if told.starts_with(&format!("{kind}:")) { return Some(kind.clone()); }
            }
            let label = if self.table.single("ext.stmt.catch.invalid") == Some(told) { Some("ext.system.fault.class.kind") }
                else if told.starts_with("Undefined variable") { Some("ext.system.fault.class.name") }
                else if told.starts_with("Undefined array key") { Some("ext.system.fault.class.key") }
                else if told.contains("index") && told.contains("out of bounds") { Some("ext.system.fault.class.index") }
                else if told.starts_with("Undefined property") || told.starts_with("Cannot read property") { Some("ext.system.fault.class.attribute") }
                else if told.starts_with("Cannot") || told.contains("requires") || told.contains("must be") { Some("ext.system.fault.class.kind") }
                else { None };
            if let Some(label) = label { return self.table.single(label).map(str::to_string); }
        }
        let told_of = |label: &str| self.table.single(label) == Some(told);
        let by_kind = match told {
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
            .or_else(|| if self.table.has_any("ext.builtin.exceptions") { None } else { self.table.single("ext.system.fault.class") })
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

    fn fault_words(&self, raw: &str, kind: &str) -> String {
        let raw = raw.trim_start_matches('\0');
        if self.table.single("ext.stmt.catch.invalid") == Some(raw) { return raw.into(); }
        let prefix = format!("{kind}: ");
        if let Some(words) = raw.strip_prefix(&prefix) { return words.to_string(); }
        for (class_key, words_key) in [("ext.system.fault.class.division", "ext.system.fault.division"),
            ("ext.system.fault.class.index", "ext.system.fault.index"),
            ("ext.system.fault.class.kind", "ext.system.fault.kind")] {
            if self.table.single(class_key) == Some(kind) {
                return self.table.single(words_key).unwrap_or(raw).to_string();
            }
        }
        if self.table.single("ext.system.fault.class.name") == Some(kind) {
            let name = raw.trim_start_matches("Undefined variable").trim_start_matches(':').trim().trim_matches('\'');
            if let [head, tail] = self.table.strings("ext.system.fault.name") { return format!("{head}{name}{tail}"); }
        }
        if self.table.single("ext.system.fault.class.key") == Some(kind) {
            return raw.strip_prefix("Undefined array key ").unwrap_or(raw).into();
        }
        if self.table.single("ext.system.fault.class.attribute") == Some(kind) {
            if let [before, after] = self.table.strings("ext.system.fault.attribute") {
                let word = raw.split("::$").nth(1).or_else(|| raw.split("'").nth(1)).unwrap_or(raw);
                return format!("{before}{word}{after}");
            }
        }
        raw.into()
    }

    fn as_raised(&mut self, told: &str) -> Option<Value> {
        if self.table.single("ext.builtin.core.exhausted") == Some(told) && self.table.has_any("ext.stmt.class.special.stop") {
            if let Some(Value::Blueprint(kind)) = self.fault_kinds.get(self.table.single("ext.stmt.class.special.stop")?).cloned() {
                return Some(self.make_fault(kind, vec![], Value::Nil));
            }
            let of = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None, name: self.table.single("ext.stmt.class.special.stop")?.to_owned(), under: None, fields: vec![], methods: vec![], shared: RefCell::new(vec![]), reaches: vec![], answers: vec![], constants: vec![] };
            self.made += 1;
            return Some(Value::Thing(Rc::new(Thing { of: Rc::new(of), holds: RefCell::new(vec![]), turn: self.made })));
        }
        let key_value = if let Some(text) = told.strip_prefix("\0absent-text=") { Some(Value::text(text)) }
            else if let Some(number) = told.strip_prefix("\0absent-number=") { number.parse::<BigInt>().ok().map(Value::from_big) }
            else { None };
        if let Some(key) = key_value {
            let kind = self.table.single("ext.system.fault.class.key")?;
            let Value::Blueprint(kind) = self.fault_kinds.get(kind)?.clone() else { return None };
            return Some(self.make_fault(kind, vec![key], Value::Nil));
        }
        let named = self.class_of_fault(told)?;
        let Some(Value::Blueprint(of)) = self.fault_kinds.get(&named).cloned().or_else(|| self.class_bound(&named)) else { return None };
        if self.is_fault_kind(&of) {
            let words = self.fault_words(told, &named);
            let values = if words.is_empty() || words == format!("{named}:") { vec![] } else { vec![Value::text(&words)] };
            let raised = self.make_fault(of, values, Value::Nil);
            self.keep_context(&raised);
            match &raised {
                Value::Thing(object) if self.table.single("ext.stmt.catch.invalid") == Some(told) => {
                    object.holds.borrow_mut().push(("\0former-complaint".into(), Value::text(told)));
                }
                _ => {}
            }
            return Some(raised);
        }
        self.made += 1;
        let mut holds = of.every_field();
        // A fault of the kernel's own carries the words said and the
        // place in the program they were said of.
        let carried = [
            ("message", Value::text(told)),
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
        let ran = match ran {
            Err(Escape::Error(words)) if self.table.has_any("ext.builtin.exceptions") && (words.starts_with('\0') || words.starts_with("Division by zero") || words.starts_with("Undefined property") || words.starts_with("Cannot read property") || words.starts_with("Array index")) => {
                self.as_raised(&words).map_or(Err(Escape::Error(words)), |value| Err(Escape::Thrown(value)))
            }
            result => result,
        };
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
                let original = thing.holds.borrow().iter().find(|(key, _)| key == "\0former-complaint").map(|(_, value)| value.bare());
                if let Some(original) = original { return Err(original); }
                if let Some(words) = Value::Thing(thing.clone()).raised_words(self.wording()).filter(|_| thing.holds.borrow().iter().any(|(key, _)| key == "\0raised-values")) {
                    return Err(if words.is_empty() { format!("\0{}", thing.of.name) } else { format!("\0{}: {}", thing.of.name, words) });
                }
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
        if Rc::ptr_eq(f, &self.outermost) {
            if let Some(found) = self.booked_read(slot.at, &slot.ident) { return found; }
        }
        let mut v = f.cells.borrow()[slot.at].clone();
        if let Value::Shared(cell) = &v {
            if self.table.flag("ext.syntax.call.bind_names") && !(Rc::ptr_eq(f, &self.outermost) && self.idents[slot.at].starts_with("\0import/")) { return Ok(v); }
            let held = cell.borrow().clone();
            if !self.table.flag("ext.stmt.function.closes_over") { return Ok(held); }
            v = held;
        }
        if matches!(v, Value::Unset) && self.table.flag("ext.stmt.function.closes_over") && !Rc::ptr_eq(&f, &self.outermost) {
            let label = if slot.up == 0 { "ext.stmt.function.local.unbound" } else { "ext.stmt.function.free.unbound" };
            return Err(self.argument_fault(label, Some(&slot.ident)));
        }
        if !matches!(v, Value::Unset) {
            return Ok(v);
        }
        if let Some(g) = slot.fallback {
            if let Some(found) = self.booked_read(g, &slot.ident) { return found; }
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
        if self.has_class_order() {
            if slot.ident.as_ref()==self.detail("root") {if let Some(c)=&self.ancestor{return Ok(Value::Blueprint(c.clone()));}}
            if self.table.prims.contains_key(slot.ident.as_ref()){return Ok(Value::Wrapped(8,Rc::new(vec![Value::text(&slot.ident)])));}
        }
        Err(format!("Undefined variable: {}", slot.ident))
    }

    fn store(&self, slot: &Address, frame: &Rc<Env>, value: Value) -> Result<(), String> {
        if self.table.flag("ext.syntax.call.bind_names") {
            let destination = ascend(frame, slot.up);
            let stored = self.collection_cell(value);
            if Rc::ptr_eq(destination, &self.outermost) && self.idents[slot.at].starts_with("\0import/") {
                if let Value::Shared(cell) = &destination.cells.borrow()[slot.at] { *cell.borrow_mut() = stored; return Ok(()); }
            }
            if Rc::ptr_eq(destination, &self.outermost) { self.booked_write(slot.at, &slot.ident, Some(stored.clone())); }
            destination.cells.borrow_mut()[slot.at] = stored;
            return Ok(());
        }
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

    fn suspension_fault(&self, fault: Escape) -> String {
        match fault {
            Escape::Error(words) | Escape::Stopped(words) => words,
            Escape::Thrown(value) => format!("Uncaught {}", value.render(self.wording())),
            _ => self.generator_words("unsupported"),
        }
    }

    fn generator_words(&self, suffix: &str) -> String {
        self.table.single(&format!("ext.stmt.yield.{}", suffix)).unwrap_or_default().to_string()
    }

    fn make_iterator(&mut self, source: Value) -> Res {
        if let Value::Generator(_) = source { return Ok(source); }
        let members = self.gathered_members(&source)?;
        Ok(self.walk_over(&source, members))
    }

    /// A walk handing out members taken from a value. Taken from a map
    /// in a cell, the walk keeps an eye on the map where the table has
    /// words for one that changes size: a step after such a change
    /// stops with those words.
    fn walk_over(&self, source: &Value, members: Vec<Value>) -> Value {
        let overseen = match Self::dict_cell(source) {
            Some(cell) if self.table.has_any("ext.syntax.map.resized") => { let size = Self::dict_extent(&cell); Some((cell, size)) }
            _ => None,
        };
        Value::Generator(Rc::new(RefCell::new(Suspension {
            frame: self.outermost.clone(), owed: Vec::new(), found: Vec::new(),
            begun: false, ended: false, receiving: false, result: Value::Nil,
            inner: None, members: Some(members.into_iter()), ready: None, overseen,
        })))
    }

    /// The cell a map lives in, given the cell, a name for it, or a view
    /// of its keys, values or pairs.
    fn dict_cell(value: &Value) -> Option<Rc<RefCell<Value>>> {
        match value {
            Value::Mutable(cell, _) | Value::Shared(cell) => match &*cell.borrow() {
                Value::Dict(_) => Some(cell.clone()),
                deeper @ (Value::Mutable(..) | Value::Shared(_)) => Self::dict_cell(deeper),
                _ => None,
            },
            Value::Window(owner, _) => Self::dict_cell(owner),
            _ => None,
        }
    }

    /// How many pairs the map in a cell holds now.
    fn dict_extent(cell: &Rc<RefCell<Value>>) -> usize {
        match &*cell.borrow() {
            Value::Dict(entries) => entries.len(),
            Value::Mutable(deeper, _) | Value::Shared(deeper) => Self::dict_extent(deeper),
            _ => 0,
        }
    }

    /// Whether two keys stand for one place in a map: by worth where the
    /// table says keys do, a flag for its number and a whole number one
    /// with its real; by plain equality elsewhere.
    fn keys_match(&self, one: &Value, other: &Value) -> bool {
        // The very thing matches itself; a hashed key matches by what it wraps.
        if one.one_place(other) { return true; }
        let bare = |v: &Value| match v { Value::Keyed(inner, _) => inner.as_ref().clone(), other => other.clone() };
        let (a, b) = (bare(one), bare(other));
        if a.one_place(&b) { return true; }
        if self.table.flag("ext.syntax.map.value_keys") { self.equal_contents(&a, &b) } else { a.equals(&b) }
    }

    /// Whether a map holds the key, and the map without it: each stored
    /// key asked as the program would ask it, a thing by its own equality.
    fn key_held(&mut self, entries: &[(Value, Value)], wanted: &Value) -> Result<bool, String> {
        for (stored, _) in entries { if self.keys_agree(stored, wanted)? { return Ok(true); } }
        Ok(false)
    }
    fn without_key(&mut self, entries: &[(Value, Value)], wanted: &Value) -> Result<Vec<(Value, Value)>, String> {
        let mut kept = Vec::with_capacity(entries.len());
        for pair in entries { if !self.keys_agree(&pair.0, wanted)? { kept.push(pair.clone()); } }
        Ok(kept)
    }

    /// The words refusing a key no map can hold, naming the kind of what
    /// was offered: a list, a dict or a set, at any depth inside a tuple.
    /// Nothing where the key will do, or the table has no such words.
    fn cannot_key(&self, key: &Value) -> Option<String> {
        let words = self.table.strings("ext.syntax.map.unhashable");
        let [head, tail] = words else { return None };
        fn culprit(value: &Value) -> Option<String> {
            match value {
                Value::Mutable(cell, _) | Value::Shared(cell) => culprit(&cell.borrow()),
                Value::Vector(_) | Value::Dict(_) | Value::Set(_) => Some(value.kind_word()),
                Value::Tuple(items) | Value::Row(items) => items.iter().find_map(culprit),
                _ => None,
            }
        }
        culprit(key).map(|kind| format!("\0{head}{kind}{tail}"))
    }

    /// The words for a key a map lacks, carried under the class the
    /// table gives such a fault.
    fn absent_key(&self, at: &Value) -> String {
        match at.settled() {
            Value::Text(t) => format!("\0absent-text={t}"),
            Value::Small(_) | Value::Huge(_) => format!("\0absent-number={}", at.bare()),
            _ => self.argument_fault("ext.builtin.exceptions.unready", None),
        }
    }

    /// The pairs a value offers a map: its own where it is a map, else
    /// one for each two-item member. Those before an ill-shaped member
    /// come back beside the words about it, to be written first.
    fn pairs_offered(&self, source: &Value) -> (Vec<(Value, Value)>, Option<String>) {
        let mut pairs = Vec::new();
        let stopped = match source.settled() {
            Value::Dict(entries) => { pairs.extend(entries.iter().cloned()); None }
            Value::Vector(items) | Value::Tuple(items) => {
                let mut fault = None;
                for (at, item) in items.iter().enumerate() {
                    match item.settled() {
                        Value::Vector(pair) | Value::Tuple(pair) | Value::Row(pair) if pair.len() == 2 => pairs.push((pair[0].clone(), pair[1].clone())),
                        other => {
                            let width = match other { Value::Vector(p) | Value::Tuple(p) | Value::Row(p) => p.len(), _ => 0 };
                            fault = Some(match self.table.strings("ext.builtin.core.dict.pair") {
                                [head, middle, tail] => format!("{head}{at}{middle}{width}{tail}"),
                                _ => self.table.single("ext.system.fault.operands").unwrap_or_default().to_string(),
                            });
                            break;
                        }
                    }
                }
                fault
            }
            _ => Some(self.table.single("ext.system.fault.operands").unwrap_or_default().to_string()),
        };
        (pairs, stopped)
    }

    fn end_generator(&mut self, generator: &Rc<RefCell<Suspension>>) -> Res<()> {
        let mut state = generator.try_borrow_mut().map_err(|_| self.generator_words("busy"))?;
        if let Some(Value::Generator(child)) = state.inner.take() { self.end_generator(&child)?; }
        state.ended = true;
        state.ready = None;
        state.owed.clear();
        state.found.clear();
        state.members = None;
        Ok(())
    }

    fn resume(&mut self, generator: &Rc<RefCell<Suspension>>, sent: Value) -> Res<Option<Value>> {
        let mut state = generator.try_borrow_mut().map_err(|_| self.generator_words("busy"))?;
        if state.ended { state.result = Value::Nil; return Ok(None); }
        if !state.begun && !matches!(sent, Value::Nil) { return Err(self.generator_words("unstarted").into()); }
        state.begun = true;
        if state.ready.is_some() { return Ok(state.ready.take()); }
        // A map that changed size under the walk stops the step.
        if let (Some(_), Some((cell, size))) = (&state.members, &state.overseen) {
            if Self::dict_extent(cell) != *size {
                state.ended = true;
                return Err(format!("\0{}", self.table.single("ext.syntax.map.resized").unwrap_or_default()).into());
            }
        }
        if let Some(members) = &mut state.members {
            if !matches!(sent, Value::Nil) { return Err(self.generator_words("unsupported").into()); }
            let next = members.next();
            state.ended = next.is_none();
            return Ok(next);
        }
        let row = self.row;
        let outcome = self.unfold(&mut state, sent);
        self.row = row;
        if outcome.is_err() || matches!(outcome, Ok(None)) {
            state.ended = true;
            state.owed.clear();
            state.found.clear();
        }
        outcome
    }

    fn unfold(&mut self, state: &mut Suspension, mut sent: Value) -> Res<Option<Value>> {
        if state.receiving {
            state.receiving = false;
            state.found.push(sent.clone());
        }
        let frame = state.frame.clone();
        while let Some(work) = state.owed.pop() {
            match work {
                Owed::Find(node) => match node {
                    Form::OnLine(row, body) => { self.row = row; state.owed.push(Owed::Find(*body)); }
                    Form::Write(place, body) => { state.owed.push(Owed::Store(place)); state.owed.push(Owed::Find(*body)); }
                    Form::Apply(Callee::Prim(Prim::Seq, _), parts) => {
                        if parts.is_empty() { state.found.push(Value::Nil); }
                        for (at, part) in parts.into_iter().enumerate().rev() {
                            if at > 0 { state.owed.push(Owed::Find(part)); state.owed.push(Owed::Drop); }
                            else { state.owed.push(Owed::Find(part)); }
                        }
                    }
                    Form::Apply(Callee::Prim(Prim::Suspend, _), mut parts) => {
                        state.owed.push(Owed::HandOut);
                        state.owed.push(Owed::Find(parts.remove(0)));
                    }
                    Form::Apply(Callee::Prim(Prim::Delegate, _), mut parts) => {
                        state.owed.push(Owed::From);
                        state.owed.push(Owed::Find(parts.remove(0)));
                    }
                    Form::Apply(Callee::Prim(Prim::Yield, _), mut parts) => {
                        state.owed.push(Owed::Finish);
                        state.owed.push(Owed::Find(if parts.is_empty() { Form::Const(Value::Nil) } else { parts.remove(0) }));
                    }
                    Form::Apply(Callee::Prim(Prim::Leave | Prim::Resume, _), _) => {
                        let continuing = matches!(node, Form::Apply(Callee::Prim(Prim::Resume, _), _));
                        let at = state.owed.iter().rposition(|w| matches!(w, Owed::Turn(..))).ok_or_else(|| self.generator_words("unsupported"))?;
                        let Owed::Turn(_, floor) = &state.owed[at] else { unreachable!() };
                        state.found.truncate(*floor);
                        if continuing { state.owed.truncate(at + 1); state.found.push(Value::Nil); }
                        else { state.owed.truncate(at); state.found.push(Value::Nil); }
                    }
                    Form::Apply(Callee::Prim(Prim::Choose, _), mut arms) => {
                        let no = arms.pop().expect("else arm");
                        let yes = arms.pop().expect("then arm");
                        state.owed.push(Owed::Select(yes, no));
                        state.owed.push(Owed::Find(arms.remove(0)));
                    }
                    Form::Apply(Callee::Prim(op @ (Prim::Both | Prim::Either), _), mut parts) => {
                        let right = parts.pop().expect("right operand");
                        state.owed.push(Owed::Truth(op, right));
                        state.owed.push(Owed::Find(parts.remove(0)));
                    }
                    Form::Apply(Callee::Code(target), args) => {
                        state.owed.push(Owed::Call(args.len()));
                        for arg in args.into_iter().rev() { state.owed.push(Owed::Find(arg)); }
                        state.owed.push(Owed::Find(*target));
                    }
                    Form::Apply(callee, args) => {
                        state.owed.push(Owed::Apply(callee, args.len()));
                        for arg in args.into_iter().rev() { state.owed.push(Owed::Find(arg)); }
                    }
                    Form::Cycle { after: false, .. } => state.owed.push(Owed::Test(Rc::new(node))),
                    Form::Dyad { op, name, a, b } => {
                        let form = |input| match input { Input::Form(f) => *f, Input::Address(a) => Form::Read(a), Input::Const(v) => Form::Const(v) };
                        state.owed.push(Owed::Find(Form::Apply(Callee::Prim(op, name), vec![form(a), form(b)])));
                    }
                    other => {
                        if suspension_within(&other) { return Err(self.generator_words("unsupported").into()); }
                        state.found.push(self.value_of(&other, &frame)?);
                    }
                },
                Owed::Store(place) => self.store(&place, &frame, state.found.last().cloned().unwrap_or(Value::Nil))?,
                Owed::Drop => { state.found.pop(); }
                Owed::Apply(callee, count) => {
                    let args = state.found.split_off(state.found.len() - count).into_iter().map(Form::Const).collect();
                    state.found.push(self.value_of(&Form::Apply(callee, args), &frame)?);
                }
                Owed::Call(count) => {
                    let args = state.found.split_off(state.found.len() - count).into_iter().map(Form::Const).collect();
                    let target = state.found.pop().expect("called value");
                    state.found.push(self.value_of(&Form::Apply(Callee::Code(Box::new(Form::Const(target))), args), &frame)?);
                }
                Owed::Select(yes, no) => {
                    let test = state.found.pop().unwrap_or(Value::Nil);
                    state.owed.push(Owed::Find(unwrapped_arm(if self.stands_true(&test) { yes } else { no })));
                }
                Owed::Truth(op, right) => {
                    let left = state.found.pop().unwrap_or(Value::Nil);
                    let holds = self.stands_true(&left);
                    if (op == Prim::Both && !holds) || (op == Prim::Either && holds) { state.found.push(Value::Flag(holds)); }
                    else {
                        state.owed.push(Owed::Apply(Callee::Prim(Prim::AsTruth, Rc::from("")), 1));
                        state.owed.push(Owed::Find(unwrapped_arm(right)));
                    }
                }
                Owed::Test(cycle) => {
                    if let Some(over) = self.past_its_time() { return Err(over); }
                    if let Some(over) = self.past_its_room() { return Err(over); }
                    let Form::Cycle { test, .. } = cycle.as_ref() else { unreachable!() };
                    state.owed.push(Owed::Decide(cycle.clone()));
                    state.owed.push(Owed::Find(*test.clone()));
                }
                Owed::Decide(cycle) => {
                    let test = state.found.pop().unwrap_or(Value::Nil);
                    let Form::Cycle { body, otherwise, .. } = cycle.as_ref() else { unreachable!() };
                    if self.stands_true(&test) {
                        state.owed.push(Owed::Turn(cycle.clone(), state.found.len()));
                        state.owed.push(Owed::Find(*body.clone()));
                    } else if let Some(arm) = otherwise { state.owed.push(Owed::Find(*arm.clone())); }
                    else { state.found.push(Value::Nil); }
                }
                Owed::Turn(cycle, floor) => {
                    state.found.truncate(floor);
                    let Form::Cycle { step, .. } = cycle.as_ref() else { unreachable!() };
                    state.owed.push(Owed::Test(cycle.clone()));
                    if let Some(step) = step { state.owed.push(Owed::Drop); state.owed.push(Owed::Find(*step.clone())); }
                }
                Owed::HandOut => {
                    state.receiving = true;
                    return Ok(Some(state.found.pop().unwrap_or(Value::Nil)));
                }
                Owed::From => {
                    if state.inner.is_none() {
                        let source = state.found.pop().unwrap_or(Value::Nil);
                        state.inner = Some(self.make_iterator(source)?);
                        sent = Value::Nil;
                    }
                    let Some(Value::Generator(inner)) = state.inner.clone() else { unreachable!() };
                    if let Some(item) = self.resume(&inner, std::mem::replace(&mut sent, Value::Nil))? {
                        state.owed.push(Owed::From);
                        return Ok(Some(item));
                    }
                    state.found.push(inner.borrow().result.clone());
                    state.inner = None;
                }
                Owed::Finish => { state.result = state.found.pop().unwrap_or(Value::Nil); return Ok(None); }
            }
        }
        Ok(None)
    }

    fn value_of(&mut self, node: &Form, frame: &Rc<Env>) -> Res {
        if self.has_class_order() && self.ancestor.is_none() { self.common_ancestor(); }
        // A complaint raised where the run was only reading waits to be
        // handed over; here, before the next step, is where the run can
        // reach back into the program to hand it on.
        if self.any_unheard.get() {
            self.hand_over_unheard()?;
        }
        match node {
            Form::Const(Value::Routine(p)) => Ok(Value::Bound(p.clone(), frame.clone())),
            Form::Const(v) => Ok(self.collection_cell(v.clone())),
            Form::Read(slot) => Ok(self.fetch(slot, frame)?),
            Form::Glance(slot) => {
                let f = ascend(frame, slot.up);
                let held = f.cells.borrow()[slot.at].clone();
                let held = match (&held, slot.fallback) {
                    (Value::Unset, Some(g)) => self.outermost.cells.borrow()[g].clone(),
                    _ => held,
                };
                Ok(match held {
                    Value::Shared(cell) if !self.table.flag("ext.syntax.call.bind_names") => cell.borrow().clone(),
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
                    None => {
                        let made = self.prim(*op, name, &[av, bv]);
                        // What a thing's own method raised on the way
                        // is raised on, not the words that stood in.
                        if let Some(away) = self.got_away.take() { return Err(away); }
                        Ok(made?)
                    }
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
                // A namespace is not a field kept under that name but a
                // view of the thing's own, and is written into as one.
                if self.has_class_order() && called.as_ref() == self.detail("namespace") && matches!(thing, Value::Routine(_) | Value::Bound(..) | Value::Method(..) | Value::Thing(_)) {
                    return self.read_class_member(thing, called, false);
                }
                let Value::Thing(thing) = thing else {
                    if matches!(thing, Value::Routine(_) | Value::Bound(..) | Value::Method(..)) && self.table.has_any("ext.system.scope.unready") {
                        return Err(self.table.single("ext.system.scope.unready").unwrap_or_default().to_string().into());
                    }
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
                if Rc::ptr_eq(f, &self.outermost) { self.booked_write(slot.at, &slot.ident, None); }
                let mut places = f.cells.borrow_mut();
                match &places[slot.at] {
                    Value::Shared(cell) if self.table.flag("ext.stmt.function.closes_over") && !self.table.flag("ext.syntax.call.bind_names") => *cell.borrow_mut() = Value::Unset,
                    _ => places[slot.at] = Value::Unset,
                }
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
                let target = match &holder { Value::Shared(c) => c.borrow().clone(), value => value.clone() };
                if let Value::Attributes(t) = &target {
                    let Value::Text(key) = named else { return Err(self.bad_answer().into()) };
                    let mut fields = t.holds.borrow_mut();
                    let position = fields.iter().position(|(n, _)| n == key.as_ref()).ok_or_else(|| self.bad_answer())?;
                    fields.remove(position);
                    return Ok(Value::Nil);
                }
                if matches!(&target, Value::Dict(entries) if entries.iter().any(|(k, _)| matches!(k, Value::Keyed(..)))) {
                    let remaining = self.user_operation(Prim::Erase, &[target, named])?.ok_or_else(|| self.bad_answer())?;
                    if let Value::Shared(cell) = holder { *cell.borrow_mut() = remaining; return Ok(Value::Nil); }
                    return Err(self.bad_answer().into());
                }
                if self.appointed(&target, 13).is_some() {
                    let asked = self.ask_special(&target, 13, &[named]);
                    if let Some(away) = self.got_away.take() { return Err(away); }
                    asked?;
                    return Ok(Value::Nil);
                }
                let named = match &named {
                    Value::Span(bounds) if self.table.has_any("ext.builtin.slice") => {
                        let settled = self.span_settled(bounds);
                        if let Some(away) = self.got_away.take() { return Err(away); }
                        Value::Span(Rc::new(settled?))
                    }
                    _ => named,
                };
                let at = self.as_key_spoken(&named);
                let Value::Shared(cell) = holder else {
                    return Err("Cannot take a place out of something that is not an array".to_string().into());
                };
                let mut inside = cell.borrow_mut();
                let left = match &*inside {
                    // Whatever a span picks out goes, and the rest
                    // closes up in the order it stood.
                    Value::Vector(items) if matches!(at, Value::Span(_)) && self.table.has_any("ext.builtin.slice") => {
                        let Value::Span(bounds) = &at else { unreachable!() };
                        let (_, picked, _) = self.span_selection(bounds, items.len())?;
                        let retained = items.iter().enumerate().filter(|(j, _)| !picked.contains(j)).map(|(_, v)| v.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) if self.table.has_any("ext.stmt.del") => {
                        let offset = (match &at { Value::Flag(b) => Some(if *b { 1 } else { 0 }), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None }).ok_or_else(|| self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string())?;
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string().into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, v)| v.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) | Value::Tuple(items) => {
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
                        if let Some(words) = self.cannot_key(&at) { return Err(words.into()); }
                        // The key is hashed once, as the reference hashes it, before it is sought.
                        let wanted = self.hash_key(&at)?;
                        let present = self.key_held(pairs, &wanted)?;
                        if self.table.has_any("ext.stmt.del") && !present {
                            let words = if self.table.has_any("ext.builtin.exceptions") { self.absent_key(&at) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() };
                            return Err(words.into());
                        }
                        Value::Dict(Rc::new(pairs.iter().filter(|(k, _)| !self.keys_match(k, &at)).cloned().collect()))
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
            Form::Fits { value, test, slots, tuple } => {
                let subject = self.value_of(value, frame)?;
                let result = fit_case(test, &subject, *tuple).map_err(|()| self.table.single("ext.stmt.match.unready").unwrap_or_default().to_owned())?;
                if let Some(captures) = result {
                    for (name, address) in slots {
                        self.store(address, frame, captures.get(name).expect("a captured name").clone())?;
                    }
                    Ok(Value::Flag(true))
                } else { Ok(Value::Flag(false)) }
            }
            Form::Again => {
                match self.holding_fault.last() {
                    Some(value) => Err(Escape::Thrown(value.clone())),
                    None => Err(format!("\0{}", self.table.single("ext.stmt.throw.empty").unwrap_or_default()).into()),
                }
            }
            Form::Assert { condition, message } => {
                let tested = self.value_of(condition, frame)?;
                if self.object_truth(&tested)? { return Ok(Value::Nil); }
                let held = self.value_of(message, frame)?;
                self.raised_on = self.row;
                // A furnished class is raised as any fault of its kind is,
                // the message its one argument where there is one; a table
                // furnishing none gets a class of the name and nothing else.
                let raised = match self.table.single("ext.stmt.assert.kind").and_then(|n| self.fault_kinds.get(n)).cloned() {
                    Some(Value::Blueprint(base)) => {
                        let unsaid = matches!(&held, Value::Text(words) if words.is_empty());
                        self.make_fault(base, if unsaid { Vec::new() } else { vec![held] }, Value::Nil)
                    }
                    _ => {
                        let kind = Blueprint {
                            ancestry: vec![], parents: vec![], presentation: None,
                            name: self.table.single("ext.stmt.assert.kind").unwrap_or_default().to_string(),
                            fields: Vec::new(), methods: Vec::new(), constants: Vec::new(),
                            shared: RefCell::new(Vec::new()), reaches: Vec::new(), under: None, answers: Vec::new(),
                        };
                        Value::Thing(Rc::new(Thing { of: Rc::new(kind), turn: 0, holds: RefCell::new(vec![("message".to_string(), held)]) }))
                    }
                };
                self.keep_context(&raised);
                Err(Escape::Thrown(raised))
            }
            Form::Attempt { context, body, clauses, last, otherwise } => {
                if self.table.has_any("ext.stmt.catch.group.unsupported") && clauses.iter().any(|part| part.grouped) {
                    return Err(self.table.single("ext.stmt.catch.group.unsupported").unwrap_or_default().to_string().into());
                }
                let preceding = self.holding_fault.len();
                let body_result = match self.value_of(body, frame) {
                    // What a thing's own method raised on the way is what
                    // the body ended in, not the words that stood in for it.
                    Err(Escape::Error(_)) if self.got_away.is_some() => Err(self.got_away.take().expect("what got away")),
                    Err(Escape::Error(told)) => match self.as_raised(&told) {
                        Some(value) => Err(Escape::Thrown(value)),
                        None => Err(Escape::Error(told)),
                    },
                    result => result,
                };
                if let Some(address) = context {
                    let manager = self.fetch(address, frame)?;
                    if !matches!(&manager, Value::Thing(_)) { return body_result; }
                    if matches!(&body_result, Err(Escape::Error(_))) || matches!(&body_result, Err(Escape::Thrown(value)) if !matches!(value, Value::Thing(_))) {
                        return Err(self.table.single("ext.stmt.class.special.unready").unwrap_or_default().to_owned().into());
                    }
                    let arguments = if let Err(Escape::Thrown(v)) = &body_result {
                        let kind = match v { Value::Thing(t) => Value::Blueprint(t.of.clone()), _ => Value::Nil };
                        vec![kind, v.clone(), Value::Backtrace(Rc::from(self.table.single("ext.stmt.class.special.unready").unwrap_or_default()))]
                    } else { vec![Value::Nil; 3] };
                    // The raised value stays held while the manager lets
                    // the body go, so whatever the leaving raises keeps it
                    // as context; and what the leaving raises comes back
                    // as raised rather than as a wrong answer.
                    if let Err(Escape::Thrown(value)) = &body_result { self.holding_fault.push(value.clone()); }
                    let asked = self.ask_special(&manager, 34, &arguments).map_err(|told| self.got_away.take().unwrap_or(Escape::Error(told)));
                    let asked = self.raised_if_error(asked);
                    self.holding_fault.truncate(preceding);
                    let answer = asked?.ok_or_else(|| self.bad_answer())?;
                    return match body_result {
                        Err(Escape::Thrown(_)) if self.object_truth(&answer)? => Ok(Value::Nil),
                        outcome => outcome,
                    };
                }
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
                                            // A namespace's binding is a cell; the class is what it holds.
                                            let class = self.value_of(choice, frame)?.settled();
                                            if !matches!(class, Value::Unset | Value::Blueprint(_)) {
                                                return Err(self.table.single("ext.stmt.catch.invalid").unwrap_or("A catch needs a class").to_string().into());
                                            }
                                            if let Value::Blueprint(kind) = class {
                                                if self.table.has_any("ext.builtin.exceptions") && !self.is_fault_kind(&kind) { return Err(self.table.single("ext.stmt.catch.invalid").unwrap_or_default().to_string().into()); }
                                                if let Value::Thing(value) = &raised {
                                                    fits |= match self.table.has_any("ext.builtin.exceptions") {
                                                        true => Self::fault_descends(&value.of, &kind),
                                                        false => value.of.goes_by(&kind.name, self.classes_either_way),
                                                    };
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
                                    // A fault the clause itself met is raised
                                    // while the value it took is still held.
                                    let answer = self.raised_if_error(answer);
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
                let ending = match ending {
                    Err(Escape::Error(words)) if self.table.has_any("ext.builtin.exceptions") => {
                        self.as_raised(&words).map_or(Err(Escape::Error(words)), |value| Err(Escape::Thrown(value)))
                    }
                    other => other,
                };
                if let Some(limb) = last {
                    if let Err(Escape::Thrown(value)) = &ending { self.holding_fault.push(value.clone()); }
                    let final_result = self.value_of(limb, frame);
                    let final_result = self.raised_if_error(final_result);
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
                        // A native kind the table lets a class stand on.
                        Some(Value::Intrinsic(word)) if self.table.spells("ext.stmt.class.builtin", &word) => Some(self.native_kind(&word)),
                        // The property builtin, stood on as a class.
                        Some(named) if self.has_class_order() && self.spells_property_kind(&named) => Some(self.property_blueprint()),
                        Some(Value::Intrinsic(word)) if self.table.spells("ext.builtin.bool", &word) && self.table.has_any("ext.builtin.bool.base") => {
                            return Err(self.table.single("ext.builtin.bool.base").unwrap_or_default().to_owned().into());
                        }
                        _ => return Err(if self.has_class_order(){self.detail("unready").to_owned()}else{format!("Class {} cannot be built on that", plan.name)}.into()),
                    },
                };
                let mut answers = Vec::with_capacity(plan.answers);
                for _ in 0..plan.answers {
                    match given.next() {
                        Some(Value::Blueprint(b)) => answers.push(b),
                        Some(Value::Intrinsic(word)) if self.table.spells("ext.stmt.class.builtin", &word) => { let kind = self.native_kind(&word); answers.push(kind); }
                        Some(named) if self.has_class_order() && self.spells_property_kind(&named) => { let kind = self.property_blueprint(); answers.push(kind); }
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
                if self.has_class_order() {
                    let mut entries=shared;
                    entries.extend(plan.methods.iter().map(|(key,p)|(key.clone(),Value::Routine(p.clone()))));
                    let mut parents=Vec::new();parents.extend(under);parents.extend(answers);
                    return self.build_class_value(plan.name.clone(),parents,entries);
                }
                Ok(Value::Blueprint(Rc::new(Blueprint {
                    ancestry: vec![], parents: vec![], presentation: None,
                    name: plan.name.clone(),
                    under,
                    answers,
                    fields,
                    reaches: plan.field_reach.clone(),
                    methods: plan.methods.clone(),
                    constants,
                    shared: RefCell::new(shared),
                })))
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
                        if !self.object_truth(&told)? {
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
                        if self.object_truth(&told)? {
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
                if let Value::Adorned(adornment) = &stands {
                    let given = self.value_list(args, frame)?;
                    return self.call_adornment(adornment, given, frame);
                }
                if let Some(answer) = self.text_called(&stands, args, frame) { return answer; }
                if self.has_class_order() && matches!(&stands,Value::Wrapped(..)|Value::Thing(_)|Value::Routine(_)) {let values=self.value_list(args,frame)?;return self.apply_class_member(stands,values);}
                if let Value::Method(body, object) = &stands {
                    let mut given = vec![Value::Thing(object.clone())];
                    given.extend(self.value_list(args, frame)?);
                    return self.invoke(body.clone(), self.outermost.clone(), given).map_err(Escape::from);
                }
                if let Value::Member(receiver, operation) = &stands {
                    let raw = self.value_list(args, frame)?;
                    let (given, named) = self.open_arguments(raw)?;
                    return self.value_member(receiver, operation, given, named);
                }
                if self.appointed(&stands, 17).is_some() {
                    let values = self.value_list(args, frame)?;
                    let answer = self.ask_special(&stands, 17, &values)?.unwrap();
                    return Ok(answer);
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
                Prim::BindValueMethod => {
                    let receiver = self.value_of(&args[0], frame)?.keep(false);
                    let operation = self.value_of(&args[1], frame)?.bare();
                    // The parts of a number are members read, not methods called.
                    if ["numerator", "denominator", "real", "imag"].contains(&operation.as_str()) {
                        match receiver.settled() {
                            Value::Complex(pair) => return Ok(crate::complex::decimal_value(if operation == "real" { pair.0 } else { pair.1 })),
                            Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Flag(_) => return self.value_member(&receiver, &operation, Vec::new(), Vec::new()),
                            _ => {}
                        }
                    }
                    Ok(Value::Member(Rc::new(receiver), operation))
                }
                Prim::SortedValues => {
                    let raw = self.value_list(args, frame)?;
                    let (given, keywords) = self.open_arguments(raw)?;
                    if given.len() != 1 { return Err(self.method_fault("arguments").into()); }
                    let sorted = self.ordered_members(&given[0], &keywords)?;
                    Ok(Value::Vector(Rc::new(sorted)).keep(true))
                }
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
                Prim::Weigh if !Rc::ptr_eq(frame, &self.outermost) && !self.reads_manners() => {
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
                    let left = self.object_truth(&seen)?;
                    if (*op == Prim::Both && !left) || (*op == Prim::Either && left) {
                        return Ok(Value::Flag(left));
                    }
                    // The right side is read in place and evaluated only here.
                    let right = match self.value_of(&args[1], frame)? {
                        Value::Bound(p, env) => self.invoke(p, env, Vec::new())?,
                        v => v,
                    };
                    Ok(Value::Flag(self.object_truth(&right)?))
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
                    let mut values = self.value_list(args, frame)?.into_iter();
                    let raised = values.next().ok_or_else(|| format!("{}() needs a value to raise", name))?;
                    let raised = if self.table.has_any("ext.builtin.exceptions") { self.raise_class(raised, frame)? } else {
                    let raised = match raised {
                        Value::Blueprint(class) if self.table.has_any("ext.stmt.class.special") => self.make_instance(class, Vec::new())?,
                        Value::Blueprint(of) if self.table.has_any("ext.stmt.catch.as") => {
                            self.made += 1;
                            Value::Thing(Rc::new(Thing { of, holds: RefCell::new(Vec::new()), turn: self.made }))
                        }
                        worth => worth,

                    };
                        raised
                    };
                    // Where the table furnishes exceptions and has words
                    // for it, nothing but an instance of one is raised.
                    if let Some(words) = self.table.single("ext.stmt.throw.invalid") {
                        if self.table.has_any("ext.builtin.exceptions") && !matches!(raised.settled(), Value::Thing(of) if self.is_fault_kind(&of.of)) {
                            return Err(format!("\0{words}").into());
                        }
                    }
                    if let Some(cause) = values.next() {
                        let cause = self.raise_class(cause, frame)?;
                        match &cause {
                            Value::Nil => {},
                            Value::Thing(object) if self.is_fault_kind(&object.of) => {},
                            _ => return Err(self.argument_fault("ext.builtin.exceptions.unready", None).into()),
                        }
                        // A cause stated, nothing included, hides the
                        // context without forgetting it.
                        if let Value::Thing(thing) = &raised {
                            let mut holds = thing.holds.borrow_mut();
                            for (label, worth) in [("ext.builtin.exceptions.cause", cause), ("ext.builtin.exceptions.suppress", Value::Flag(true))] {
                                let Some(key) = self.table.single(label) else { continue };
                                match holds.iter_mut().find(|(k, _)| k == key) {
                                    Some((_, value)) => *value = worth, None => holds.push((key.into(), worth)),
                                }
                            }
                        }
                    }
                    self.keep_context(&raised);
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
                    if self.is_fault_kind(&class) {
                        if Self::fault_methods(&class) { return Err(self.argument_fault("ext.builtin.exceptions.unready", None).into()); }
                        return Ok(self.make_fault(class, values, Value::Nil));
                    }
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
                    if let Value::Generator(generator) = subject {
                        if self.table.spells("ext.stmt.yield.close", &called) && values.is_empty() {
                            self.end_generator(&generator)?;
                            return Ok(Value::Nil);
                        }
                        if self.table.spells("ext.stmt.yield.send", &called) && values.len() == 1 {
                            return self.resume(&generator, values.remove(0))?.ok_or_else(|| self.generator_words("exhausted").into());
                        }
                        let fault = if self.table.spells("ext.stmt.yield.throw", &called) { "throw.unavailable" } else { "unsupported" };
                        return Err(self.generator_words(fault).into());
                    }
                    if self.table.spells("ext.text.format", &called) {
                        if let Value::Text(pattern) = &subject {
                            let (positions, keywords) = self.open_arguments(values)?;
                            let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
                            return Ok(Value::text(&layout.interpolate(pattern, &positions, &keywords)?));
                        }
                    }
                    if self.has_class_order() && matches!(&subject, Value::Thing(_) | Value::Blueprint(_) | Value::Routine(_) | Value::Bound(..) | Value::Wrapped(..)){let target=self.read_class_member(subject,&called,false)?;return self.apply_class_member(target,values);}
                    if self.table.flag("ext.op.member.pipes") {
                        let read = self.stands_for_property(Prim::Of, &[subject.clone(), Value::text(&called)])?;
                        if let Some(target) = read.or_else(|| self.attribute(&subject, &called)) {
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
                    let parent = values.remove(0);
                    let called = values.remove(0).bare();
                    if self.has_class_order(){if let Value::Text(owner)=&parent{return self.next_ancestor_call(subject,owner,&called,values);}}
                    let holder = self.class_it_spells(parent);
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
                    if self.table.has_any("ext.stmt.class.special") {
                        let original = self.fetch(slot, frame)?;
                        // A thing over a native worth is written into
                        // through that worth, where its blueprint has no
                        // method of its own for the writing.
                        let (target, worth_cell) = match self.underlying_unless(&original.settled(), &[12]) {
                            Some(Value::Mutable(cell, _)) => { let held = cell.borrow().clone(); (held, Some(cell)) }
                            _ => (original.settled(), None),
                        };
                        if let (Some(key), Value::Dict(entries)) = (&key, &target) {
                            if let Some(words) = self.cannot_key(key) { return Err(words.into()); }
                            let key = self.hash_key(key)?;
                            let mut entries = entries.to_vec();
                            let mut position = 0;
                            while position < entries.len() && !self.keys_agree(&entries[position].0, &key)? { position += 1; }
                            if position < entries.len() { entries[position].1 = value; } else { entries.push((key, value)); }
                            let changed = Value::Dict(Rc::new(entries));
                            match (worth_cell, original) {
                                (Some(cell), _) => { cell.replace(changed); }
                                (None, Value::Shared(cell) | Value::Mutable(cell, _)) => { cell.replace(changed); }
                                _ => self.store(slot, frame, changed)?,
                            }
                            return Ok(Value::Nil);
                        }
                        if let (Some(index), Some(cell)) = (&key, &worth_cell) {
                            let letter = self.letter_places.then(|| value.render(self.wording()));
                            written_into(&mut cell.borrow_mut(), Some(index.clone()), value, &self.no_places(), self.builds_places, letter, !self.table.flag("ext.syntax.call.bind_names"))?;
                            return Ok(Value::Nil);
                        }
                        if let (Some(Value::Text(word)), Value::Attributes(t)) = (&key, &target) {
                            let mut slots = t.holds.borrow_mut();
                            match slots.iter_mut().find(|(n, _)| n == word.as_ref()) {
                                Some((_, old)) => *old = value,
                                None => slots.push((word.to_string(), value)),
                            }
                            return Ok(Value::Nil);
                        }
                        if let (Some(index), true) = (&key, self.appointed(&target, 12).is_some()) {
                            self.ask_special(&target, 12, &[index.clone(), value])?;
                            return Ok(Value::Nil);
                        }
                    }
                    if let Some(Value::Span(bounds)) = &key {
                        let value = collection_read(&value);
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
                    let held = f.cells.borrow()[i].clone();
                    let held = if let Value::Shared(cell) = held { cell.borrow().clone() } else { held };
                    if let Value::Octets { cell, changeable, .. } = held {
                        if !changeable { return Err(self.octet_error("immutable").into()); }
                        let byte = self.octet_item(&value)?;
                        if let Some(index) = key {
                            let index = self.octet_at(&index, cell.borrow().len())?;
                            cell.borrow_mut()[index] = byte;
                        } else { cell.borrow_mut().push(byte); }
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
                            written_into(&mut held, key, value, &self.no_places(), self.builds_places, letter, !self.table.flag("ext.syntax.call.bind_names"))?
                        }
                        None => {
                            let mut slots = f.cells.borrow_mut();
                            written_into(&mut slots[i], key, value, &self.no_places(), self.builds_places, letter, !self.table.flag("ext.syntax.call.bind_names"))?
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
                    // The names about a call are only to be seen from
                    // here, where the frame is at hand.
                    if args.is_empty() && self.reads_manners() && matches!(op, Prim::HereBook | Prim::MembersOf | Prim::ClassWork(8) | Prim::WorldBook) {
                        return self.names_here(frame, *op).map_err(Escape::Error);
                    }
                    let mut values = self.value_list(args, frame)?;
                    // A list that a lazy walk is to begin on is not gathered
                    // into a copy: it goes on in its own cell, so the walk
                    // reaches what the body adds and misses what it takes.
                    if *op == Prim::Iterated && self.table.flag("ext.stmt.yield.suspends") {
                        if let Some(living @ IteratorKind::Living(..)) = values.first().and_then(Self::live_walk) { return Ok(Self::cursor_value(living)); }
                    }
                    // The property builtin takes its accessors by name; making the property sorts them out.
                    if *op == Prim::ClassWork(11) && self.has_class_order() && !self.detail("descriptor.get").is_empty() { return self.work_on_class(11, values); }
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
                    if self.has_class_order() {
                        match op {
                            Prim::ClassWork(k)=>return self.work_on_class(*k,values),
                            Prim::SortOf if values.len()==3 || values.first().map_or(false, |v| matches!(v, Value::Thing(_) | Value::Blueprint(_) | Value::Routine(_) | Value::Bound(..) | Value::Wrapped(..))) =>return self.class_from_type(values),
                            Prim::Of if values.len()==2 && (matches!(&values[0], Value::Thing(t) if t.of.presentation.is_some()) || matches!(&values[0], Value::Blueprint(c) if c.presentation.is_some()) || matches!(&values[0], Value::Routine(_) | Value::Method(..) | Value::Bound(..) | Value::Wrapped(..)) || matches!(&values[0], Value::Intrinsic(word) if self.table.spells("ext.stmt.class.builtin", word))) =>return self.read_class_member(values[0].clone(),&values[1].bare(),false),
                            Prim::Onto if values.len()==3=>return self.alter_class_member(values[0].clone(),&values[1].bare(),Some(values[2].clone()),false),
                            Prim::Pluck if values.len()==2=>return self.alter_class_member(values[0].clone(),&values[1].bare(),None,false),
                            _=>{}
                        }
                    }
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
    fn call_adornment(&mut self, member: &Adornment, mut given: Vec<Value>, frame: &Rc<Env>) -> Res<Value> {
        match member.manner {
            's' => (),
            'b' => given.insert(0, member.extra.clone().expect("a receiver")),
            _ => return Err(self.table.single("ext.stmt.class.unready").unwrap_or_default().to_string().into()),
        }
        let expressions = given.into_iter().map(Form::Const).collect();
        let apply = Form::Apply(Callee::Code(Box::new(Form::Const(member.target.clone()))), expressions);
        self.value_of(&apply, frame)
    }

    fn property_member(&self, subject: &Value, called: &str) -> Option<Rc<Adornment>> {
        if let Value::Thing(thing) = subject {
            let owner = thing.of.keeper(called)?;
            for (name, value) in owner.shared.borrow().iter() {
                if name == called {
                    if let Value::Adorned(member) = value {
                        if member.manner == 'p' { return Some(member.clone()); }
                    }
                }
            }
        }
        None
    }

    fn stands_for_property(&mut self, op: Prim, values: &[Value]) -> Res<Option<Value>> {
        if matches!((op, values.len()), (Prim::Of, 2) | (Prim::Onto, 3)) {
            if let Some(member) = self.property_member(&values[0], &values[1].bare()) {
                let target = if op == Prim::Of { member.target.clone() } else {
                    member.extra.clone().ok_or_else(|| self.table.single("ext.stmt.class.unready").unwrap_or_default().to_string())?
                };
                let mut args = vec![Form::Const(values[0].clone())];
                if op == Prim::Onto { args.push(Form::Const(values[2].clone())); }
                let call = Form::Apply(Callee::Code(Box::new(Form::Const(target))), args);
                let answer = self.value_of(&call, &self.outermost.clone())?;
                return Ok(Some(if op == Prim::Of { answer } else { Value::Nil }));
            }
        }
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
        if held || (self.table.has_any("ext.stmt.class.special") && self.attribute(&values[0], &called).is_some()) {
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
    fn text_called(&mut self, stands: &Value, args: &[Form], frame: &Rc<Env>) -> Option<Res> {
        let Value::TextCall { subject, work, name } = stands else { return None };
        Some((|| {
            let received = self.value_list(args, frame)?;
            let (mut positional, named) = self.open_arguments(received)?;
            if *work != crate::text::Work::MAKETRANS { positional.insert(0, Value::Text(subject.clone())); }
            crate::text::fit_names(self.table, *work, &mut positional, named)?;
            Ok(crate::text::apply(self.table, *work, name, &positional, self.wording())?)
        })())
    }

    fn word_it_spells(&mut self, stands: &Value, args: &[Form], frame: &Rc<Env>) -> Option<Res<Value>> {
        let word = match stands { Value::Text(word) | Value::Intrinsic(word) => word, _ => return None };
        let op = self.table.prims.get(word.as_ref()).copied()?;
        let name = word.to_string();
        Some((|| {
            let mut values = self.value_list(args, frame)?;
            if op == Prim::ClassWork(11) && self.has_class_order() && !self.detail("descriptor.get").is_empty() { return self.work_on_class(11, values); }
            if matches!(stands, Value::Intrinsic(_)) && self.table.flag("ext.syntax.call.bind_names") {
                let (mut positions, names) = self.open_arguments(values)?;
                if let Some(answer) = self.builtin_names(op, &name, &mut positions, names)? { return Ok(answer); }
                values = positions;
            }
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
        let names = self.table.strings("ext.stmt.class.special");
        if let Value::Span(bounds) = value {
            return self.span_bound_named(name).map(|i| bounds[i].clone());
        }
        match value {
            Value::Thing(t) if names.get(35).map_or(false, |s| s == name) => return Some(Value::Blueprint(t.of.clone())),
            Value::Thing(t) if names.get(36).map_or(false, |s| s == name) => return Some(Value::Attributes(t.clone())),
            Value::Blueprint(c) if names.get(37).map_or(false, |s| s == name) => return Some(Value::text(&c.name)),
            _ => (),
        }
        if self.table.single("ext.builtin.class.name") == Some(name) {
            if let Value::Blueprint(kind) = value { return Some(Value::text(&kind.name)); }
            if let Value::Intrinsic(word) = value { return Some(Value::text(word)); }
            if let Value::KindOf(kind) = value { return Some(Value::text(Value::word_for_kind(*kind))); }
        }
        if let (Value::Text(subject), Some(Prim::Textual(work))) = (value, self.table.prims.get(name)) {
            return Some(Value::TextCall { subject: subject.clone(), work: *work, name: Rc::from(name) });
        }
        let class = match value {
            Value::Thing(thing) => {
                let fields = thing.holds.borrow();
                if self.is_fault_kind(&thing.of) && self.table.single("ext.builtin.exceptions.cause") == Some(name) {
                    return Some(fields.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone()).unwrap_or(Value::Nil));
                }
                if self.is_fault_kind(&thing.of) && self.table.single("ext.builtin.exceptions.args") == Some(name) {
                    let arguments = fields.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone());
                    return arguments.or_else(|| Some(Value::Arguments(Rc::new(fields.iter().filter(|(key, _)| key == "message").map(|(_, value)| value.clone()).collect()))));
                }
                if let Some(at) = self.member_place(&fields, name) {
                    return Some(match &fields[at].1 {
                        Value::Shared(cell) => cell.borrow().clone(),
                        held => held.clone(),
                    });
                }
                &thing.of
            }
            Value::Blueprint(class) => class,
            _ => return None,
        };
        if let Some(keeper) = class.keeper(name) {
            return keeper.shared.borrow().iter().find(|(n, _)| n == name).map(|(_, x)| {
                match (value, x) {
                    (_, Value::Adorned(member)) => match member.manner {
                        's' => member.target.clone(),
                        'c' => Value::Adorned(Rc::new(Adornment {
                            manner: 'b', target: member.target.clone(), extra: Some(Value::Blueprint(class.clone())),
                        })),
                        _ => x.clone(),
                    },
                    (Value::Thing(object), Value::Routine(_) | Value::Bound(..)) => Value::Adorned(Rc::new(Adornment {
                        manner: 'b', target: x.clone(), extra: Some(Value::Thing(object.clone())),
                    })),
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

    fn make_instance(&mut self, class: Rc<Blueprint>, args: Vec<Value>) -> Res<Value> {
        // A blueprint holding a program for being called answers the call
        // in place of a new thing.
        if let Some(answering) = self.table.single("ext.stmt.class.called").and_then(|word| self.inherited_entry(&class, word)) {
            let mut given = vec![Value::Blueprint(class.clone())];
            given.extend(args);
            return self.apply_class_member(answering, given);
        }
        if self.is_fault_kind(&class) {
            if Self::fault_methods(&class) {
                return Err(self.argument_fault("ext.builtin.exceptions.unready", None).into());
            }
            return Ok(self.make_fault(class, args, Value::Nil));
        }
        if self.has_class_order(){return self.construct_ordered(class,args);}
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
                if let Value::Adorned(adornment) = &stands {
                    let given = self.value_list(args, frame)?;
                    return self.call_adornment(adornment, given, frame).map(Next::Value);
                }
                if let Some(answer) = self.text_called(&stands, args, frame) { return Ok(Next::Value(answer?)); }
                if self.has_class_order() && matches!(&stands,Value::Wrapped(..)|Value::Thing(_)|Value::Routine(_)){let given=self.value_list(args,frame)?;return Ok(Next::Value(self.apply_class_member(stands,given)?));}
                if let Value::Method(body, object) = &stands {
                    let mut given = self.value_list(args, frame)?;
                    given.insert(0, Value::Thing(object.clone()));
                    let value = self.invoke(body.clone(), self.outermost.clone(), given)?;
                    return Ok(Next::Value(value));
                }
                if self.appointed(&stands, 17).is_some() {
                    let values = self.value_list(args, frame)?;
                    let answer = self.ask_special(&stands, 17, &values)?.unwrap();
                    return Ok(Next::Value(answer));
                }
                if let Some(done) = self.paired_call(&stands, args, frame) {
                    return Ok(Next::Value(done?));
                }
                if let Value::Member(receiver, operation) = &stands {
                    let raw = self.value_list(args, frame)?;
                    let (positions, keywords) = self.open_arguments(raw)?;
                    return Ok(Next::Value(self.value_member(receiver, operation, positions, keywords)?));
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
        let test = self.object_truth(&asked)?;
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

    fn method_fault(&self, kind: &str) -> String {
        self.table.single(&format!("ext.builtin.method.error.{kind}")).unwrap_or_default().to_string()
    }

    fn value_member(&mut self, receiver: &Value, name: &str, arguments: Vec<Value>, keywords: Vec<(String, Value)>) -> Res<Value> {
        if let Value::Span(bounds) = receiver.settled() {
            if !keywords.is_empty() { return Err(self.method_fault("arguments").into()); }
            return match (name, arguments.len()) {
                ("indices", 1) => {
                    let clipped = self.span_clipped(&bounds, &arguments[0]);
                    if let Some(away) = self.got_away.take() { return Err(away); }
                    Ok(Value::Tuple(Rc::new(clipped?.into_iter().map(Value::from_big).collect())))
                }
                ("slice_hash", 0) => self.span_hashed(&receiver.settled()).map_err(Escape::from),
                _ => Err(self.method_fault("arguments").into()),
            };
        }
        let actual = receiver.settled();
        if name == "encode" && matches!(&actual, Value::Text(_)) {
            let mut options = arguments;
            for (key, value) in keywords {
                let place = match key.as_str() { "encoding"=>0, "errors"=>1, _=>return Err(self.octet_error("unready").into()) };
                if options.len() > place { return Err(self.octet_error("arguments").into()); }
                while options.len() < place { options.push(Value::text("utf-8")); }
                options.push(value);
            }
            if options.len() > 2 { return Err(self.octet_error("arguments").into()); }
            if let Some(error_mode) = options.get(1) { if !matches!(error_mode, Value::Text(s) if s.as_ref()=="strict") { return Err(self.octet_error("unready").into()); } }
            options.truncate(1); options.insert(0, actual);
            return self.octet_routine(2, &options).map_err(Escape::from);
        }
        if let Value::Octets { cell, changeable: true, .. } = &actual {
            if name == "append" && arguments.len() == 1 && keywords.is_empty() { cell.borrow_mut().push(self.octet_item(&arguments[0])?); return Ok(Value::Nil); }
        }
        if matches!(&actual, Value::Octets { .. }) || name == "encode" && matches!(&actual, Value::Text(_)) {
            let operation = match name { "encode"=>Some(2), "decode"=>Some(3), "hex"=>Some(4), "upper"=>Some(6), "lower"=>Some(7), "split"=>Some(8), "join"=>Some(9), "startswith"=>Some(10), "replace"=>Some(11), "strip"=>Some(12), "find"=>Some(13), _=>None };
            if let Some(operation) = operation {
                if !keywords.is_empty() { return Err(self.octet_error("unready").into()); }
                let mut values = vec![actual]; values.extend(arguments);
                return self.octet_routine(operation, &values).map_err(Escape::from);
            }
        }
        if matches!(&actual, Value::Text(_)) {
            let key = format!("ext.builtin.text.{}", name);
            if let Some((_, Prim::Textual(work))) = crate::table::BUILTIN_LABELS.iter().find(|(label, _)| *label == key) {
                let mut given = vec![actual]; given.extend(arguments.into_iter().map(|v| match v.settled() { Value::Tuple(row)=>Value::Vector(row), other=>other }));
                if *work == crate::text::Work::JOIN && given.len() == 2 { given[1] = Value::Vector(Rc::new(self.gathered_members(&given[1])?)); }
                crate::text::fit_names(self.table, *work, &mut given, keywords)?;
                return crate::text::apply(self.table, *work, name, &given, self.wording()).map_err(Escape::from);
            }
        }
        if matches!(receiver.settled(), Value::Set(_)) {
            let code = match name { "remove" => Some(2), "pop" => Some(4), "clear" => Some(5), "copy" => Some(6), "update" => Some(7), _ => None };
            if let Some(code) = code {
                if !keywords.is_empty() { return Err(self.set_complaint("arguments", "").into()); }
                let mut values = vec![receiver.settled()]; values.extend(arguments);
                return self.work_set(code, &values).map_err(Escape::from);
            }
        }
        if name == "conjugate" {
            if !arguments.is_empty() || !keywords.is_empty() { return Err(self.method_fault("arguments").into()); }
            let subject = receiver.settled();
            let pair = crate::complex::coordinates(&subject).ok_or_else(|| crate::complex::complaint(self.table, "unready"))?;
            if !matches!(subject, Value::Complex(_)) {
                return Ok(if let Value::Flag(b) = subject { Value::Small(if b { 1 } else { 0 }) } else { subject });
            }
            return Ok(crate::complex::pair(self.table, pair.0, -pair.1));
        }
        let mut found = Vec::new();
        for (word, _) in &keywords {
            if found.contains(word) { return Err(self.method_fault("arguments").into()); }
            found.push(word.clone());
        }
        if !keywords.is_empty() && !["sort", "split", "rsplit", "format", "update", "encode"].contains(&name) {
            let spelling = self.table.single(&format!("ext.builtin.method.{name}")).unwrap_or(name);
            return Err(self.builtin_keyword_fault(spelling).into());
        }

        let keywords = if name == "split" || name == "rsplit" {
            keywords.into_iter().map(|(written, value)| {
                let purpose = if self.table.spells("ext.builtin.method.split.sep", &written) { "sep" }
                    else if self.table.spells("ext.builtin.method.split.maxsplit", &written) { "maxsplit" } else { "" };
                (purpose.to_string(), value)
            }).collect()
        } else { keywords };
        if name != "sort" {
            let says = |kind: &str| self.method_fault(kind);
            return crate::members::Request { target: receiver, operation: name, given: arguments, named: &keywords, names: self.wording(), complaint: &says }.answer().map_err(Escape::from);
        }
        if !arguments.is_empty() || !matches!(receiver.settled(), Value::Vector(_)) { return Err(self.method_fault("arguments").into()); }
        let Value::Mutable(place, _) = receiver else { return Err(self.method_fault("unready").into()) };
        if !self.table.has_any("ext.builtin.method.sort.modified") {
            let ordered = self.ordered_members(receiver, &keywords)?;
            place.replace(Value::Vector(Rc::new(ordered)));
            return Ok(Value::Nil);
        }
        // The row stands aside while it is being ordered and an empty one
        // waits in its cell, so a key that reaches for the row finds
        // nothing in it. Should the key raise, the row goes back as it
        // stood; should the key have written into the waiting row, the
        // ordered row is put in place and the meddling told of after.
        let bare = Rc::new(Vec::new());
        let kept = place.replace(Value::Vector(Rc::clone(&bare)));
        let outcome = self.ordered_members(&kept, &keywords);
        let meddled = !matches!(&*place.borrow(), Value::Vector(row) if Rc::ptr_eq(row, &bare));
        let ordered = match outcome { Err(away) => { place.replace(kept); return Err(away); }, Ok(row) => row };
        place.replace(Value::Vector(Rc::new(ordered)));
        match meddled {
            false => Ok(Value::Nil),
            true => Err(format!("\0{}", self.table.single("ext.builtin.method.sort.modified").unwrap_or_default()).into()),
        }
    }

    fn ordered_members(&mut self, receiver: &Value, keywords: &[(String, Value)]) -> Res<Vec<Value>> {
        let mut reverse = false;
        let mut using = Value::Nil;
        let mut seen = Vec::new();
        for (word, value) in keywords {
            if seen.contains(word) { return Err(self.method_fault("arguments").into()); }
            seen.push(word.clone());
            if self.table.spells("ext.builtin.method.sort.key", word) { using = value.clone(); }
            else if self.table.spells("ext.builtin.method.sort.reverse", word) {
                if !matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { return Err(self.method_fault("arguments").into()); }
                reverse = self.stands_true(value);
            }
            else { return Err(self.method_fault("arguments").into()); }
        }
        let items = self.core_collect(receiver)?;
        self.arranged(items, &using, reverse).map_err(Escape::from)
    }

    /// The members arranged and left steady: each is set against those
    /// already placed and walks back only for as long as it weighs
    /// below them, so that two weighing alike keep the order in which
    /// they arrived. What the key or the weighing raises is carried out
    /// whole, since a raised fault is the program's answer and not a
    /// sign that the arranging could not be done.
    fn arranged(&mut self, items: Vec<Value>, using: &Value, reverse: bool) -> Result<Vec<Value>, String> {
        let mut weighed = Vec::with_capacity(items.len());
        for item in items {
            let mark = match using { Value::Nil => item.clone(), work => self.core_run(work, vec![item.clone()])? };
            weighed.push((mark, item));
        }
        let mut placed: Vec<(Value, Value)> = Vec::with_capacity(weighed.len());
        for pair in weighed {
            let mut at = placed.len();
            while at != 0 {
                let both = if reverse { [placed[at-1].0.clone(), pair.0.clone()] } else { [pair.0.clone(), placed[at-1].0.clone()] };
                let below = self.prim(Prim::Lt, "", &both)?;
                if !self.object_truth(&below)? { break; }
                at -= 1;
            }
            placed.insert(at, pair);
        }
        Ok(placed.into_iter().map(|entry| entry.1).collect())
    }

    /// Which bound a name reads, where the table names the three.
    fn span_bound_named(&self, name: &str) -> Option<usize> {
        ["ext.builtin.slice.start", "ext.builtin.slice.stop", "ext.builtin.slice.step"].iter()
            .position(|label| self.table.single(label).map_or(false, |word| !word.is_empty() && word == name))
    }

    /// The whole number a bound stands for. A thing is asked through the
    /// method the table names; what is not a thing must be whole already.
    fn span_whole(&mut self, bound: &Value) -> Result<BigInt, String> {
        let told = match bound {
            Value::Thing(thing) => match self.table.single("ext.op.index.integer").and_then(|word| self.inherited_entry(&thing.of, word)) {
                Some(method) => self.apply_within(method, vec![bound.clone()])?,
                None => bound.clone(),
            },
            other => other.settled(),
        };
        match told {
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) => told.as_big(),
            _ => Err(self.span_complaint("bounds")),
        }
    }

    /// The bounds with every thing among them settled to the whole
    /// number it stands for; a bound not given stays not given, and a
    /// number stays for the row to judge.
    fn span_settled(&mut self, bounds: &[Value]) -> Result<Vec<Value>, String> {
        let mut told = Vec::with_capacity(bounds.len());
        for bound in bounds {
            told.push(match bound {
                Value::Thing(_) => Value::from_big(self.span_whole(bound)?),
                other => other.clone(),
            });
        }
        Ok(told)
    }

    /// The bounds clipped to a length, as the indices method tells them:
    /// no step of nought, and a bound not given falls to the end the step
    /// sets out from.
    fn span_clipped(&mut self, bounds: &[Value], length: &Value) -> Result<Vec<BigInt>, String> {
        use num_traits::Signed;
        let extent = self.span_whole(length)?;
        if extent.is_negative() { return Err(self.table.single("ext.builtin.slice.length").unwrap_or_default().to_owned()); }
        let stride = match bounds.get(2) { Some(Value::Nil) | None => BigInt::from(1), Some(bound) => self.span_whole(bound)? };
        if stride.is_zero() { return Err(self.span_complaint("zero")); }
        let backward = stride.is_negative();
        let (floor, ceiling): (BigInt, BigInt) = if backward { (BigInt::from(-1), &extent - 1) } else { (BigInt::from(0), extent.clone()) };
        let mut told = Vec::with_capacity(3);
        for (i, bound) in bounds.iter().take(2).enumerate() {
            let end = if matches!(bound, Value::Nil) {
                if (i == 0) == backward { ceiling.clone() } else { floor.clone() }
            } else {
                let mut n = self.span_whole(bound)?;
                if n.is_negative() { n += &extent; }
                n.max(floor.clone()).min(ceiling.clone())
            };
            told.push(end);
        }
        told.push(stride);
        Ok(told)
    }

    /// A span hashes as its bounds do, or not at all where one cannot.
    fn span_hashed(&self, span: &Value) -> Result<Value, String> {
        let Value::Span(bounds) = span else { return Err(self.span_complaint("unsupported")) };
        if let Some(bound) = bounds.iter().find(|bound| bound.hash_number().is_none()) {
            return Err(self.core_complaint("core.unhashable", &bound.kind_word()));
        }
        Ok(Value::Small(span.hash_number().expect("the bounds hash")))
    }

    /// The namespace a builtin is routed through, loaded if it is not yet.
    fn namespace_for(&mut self, path: &str) -> Result<Value, String> {
        self.load_namespace(path)
    }

    /// Apply a value the program could apply, in the outermost scope,
    /// handing on whatever it raises as the program would see it.
    fn apply_held(&mut self, target: Value, arguments: Vec<Value>) -> Res {
        let call = Form::Apply(Callee::Code(Box::new(Form::Const(target))), arguments.into_iter().map(Form::Const).collect());
        let scope = self.outermost.clone();
        self.value_of(&call, &scope)
    }

    /// As `apply_held`, for an operation that answers plain words: an
    /// escape that is not words is put by to be raised once it returns.
    fn apply_within(&mut self, target: Value, arguments: Vec<Value>) -> Result<Value, String> {
        match self.apply_held(target, arguments) {
            Ok(answer) => Ok(answer),
            Err(Escape::Error(words)) => Err(words),
            Err(away) => { self.got_away = Some(away); Err(self.argument_fault("ext.builtin.stream.failed", None)) }
        }
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
        // Compile alone takes its arguments by name; the readers of text
        // and the handing out of names take theirs in order only.
        if self.reads_manners() && matches!(op, Prim::Prepare | Prim::Perform | Prim::Weigh | Prim::Summon | Prim::WorldBook | Prim::HereBook) {
            let formals = table.strings("ext.builtin.compile.parameters");
            for (key, worth) in keywords {
                let place = formals.iter().position(|word| word == &key).filter(|_| op == Prim::Prepare)
                    .ok_or_else(|| self.argument_fault("ext.syntax.call.amiss.unknown", Some(&key)))?;
                if positional.get(place).map_or(false, |held| !matches!(held, Value::Unset)) { return Err(self.argument_fault("ext.syntax.call.amiss.duplicate", Some(&key)).into()); }
                if positional.len() <= place { positional.resize(place + 1, Value::Unset); }
                positional[place] = worth;
            }
            for held in positional.iter_mut() { if matches!(held, Value::Unset) { *held = Value::Nil; } }
            return Ok(None);
        }
        if let Prim::Octets(which @ (14 | 15)) = op {
            let mut negative_allowed = false;
            for (key, worth) in keywords {
                if !table.spells("ext.builtin.bytes.signed", &key) { return Err(self.octet_error("unready").into()); }
                negative_allowed = worth.is_true();
            }
            return self.octet_work(which, positional, negative_allowed).map(Some).map_err(Into::into);
        }
        if op == Prim::Say && table.single("ext.builtin.print.sep").is_some() {
            let route = table.strings("ext.builtin.print.redirect");
            let mut join = String::from(" ");
            let mut tail = String::from("\n");
            let mut channel = 1;
            let mut sink = Value::Nil;
            let mut drained = false;
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
                    if route.len() == 3 { sink = value; continue; }
                    match value {
                        Value::Channel(port) => channel = port,
                        Value::Nil => channel = 1,
                        _ => return Err(self.argument_fault("ext.builtin.print.file.unready", None).into()),
                    }
                } else if table.spells("ext.builtin.print.flush", &key) {
                    drained = self.stands_true(&value);
                } else {
                    return Err(self.argument_fault("ext.syntax.call.amiss.unknown", Some(&key)).into());
                }
            }
            // Routed, the text goes to the writer of whatever the module
            // holds as its stream just now, unless a file was named. A
            // stream of nothing takes the print and shows nothing, and
            // the values are not even put into words for it.
            if route.len() == 3 {
                if matches!(sink, Value::Nil) {
                    let namespace = self.namespace_for(&route[0])?;
                    sink = self.attribute(&namespace, &route[1]).unwrap_or(Value::Nil);
                    if matches!(sink, Value::Nil) { return Ok(Some(Value::Nil)); }
                }
                let mut written = String::new();
                for (at, item) in positional.iter().enumerate() {
                    if at != 0 { written.push_str(&join); }
                    written.push_str(&self.object_words(item, false)?);
                }
                written.push_str(&tail);
                let writer = self.attribute(&sink, &route[2]).ok_or_else(|| self.argument_fault("ext.builtin.print.file.unready", None))?;
                self.apply_held(writer, vec![Value::text(&written)])?;
                if drained {
                    if let Some(method) = table.single("ext.builtin.print.flush").and_then(|word| self.attribute(&sink, word)) {
                        self.apply_held(method, Vec::new())?;
                    }
                }
                return Ok(Some(Value::Nil));
            }
            let mut written = String::new();
            for (at, item) in positional.iter().enumerate() {
                if at != 0 { written.push_str(&join); }
                written.push_str(&self.object_words(item, false)?);
            }
            written.push_str(&tail);
            match channel {
                2 => eprint!("{}", written),
                _ => self.utter(&written),
            }
            return Ok(Some(Value::Nil));
        }
        if op == Prim::ComplexMade && !keywords.is_empty() {
            if positional.len() > 2 { return Err(crate::complex::complaint(table,"arguments").into()); }
            let mut parts: [Option<Value>; 2] = [positional.first().cloned(), positional.get(1).cloned()];
            for (key, value) in keywords {
                let which = match () {
                    _ if table.spells("ext.builtin.complex.imag", &key) => 1,
                    _ if table.spells("ext.builtin.complex.real", &key) => 0,
                    _ => return Err(crate::complex::complaint(table,"arguments").into()),
                };
                if parts[which].replace(value).is_some() { return Err(crate::complex::complaint(table,"arguments").into()); }
            }
            let mut given = vec![parts[0].take().unwrap_or(Value::Small(0))];
            if let Some(imaginary) = parts[1].take() { given.push(imaginary); }
            return crate::complex::create(table,&given).map(Some).map_err(Escape::Error);
        }
        // A map walked backwards from its cell is watched as a loop over
        // it is.
        if op == Prim::Backwards && keywords.is_empty() && self.table.has_any("ext.syntax.map.resized") {
            if let Some(source) = positional.first().filter(|source| Self::dict_cell(source).is_some()) {
                let mut keys = self.gathered_members(source)?;
                keys.reverse();
                return Ok(Some(self.walk_over(source, keys)));
            }
        }
        if Self::is_core_primitive(op) {
            return self.core_primitive(op, name, positional.clone(), keywords).map(Some).map_err(Escape::Error);
        }
        if let Prim::Textual(work) = op {
            crate::text::fit_names(table, work, positional, keywords)?;
            return Ok(None);
        }
        for (key, value) in keywords {
            let index = match op {
                Prim::Total if table.spells("ext.builtin.start", &key) => 1,
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
        let invalid_text = || {
            if let Some((head, middle)) = self.table.around("ext.builtin.to_int.text.detail") {
                format!("{head}{radix}{middle}{}", self.quoted_remainder(&values[0]).unwrap_or_default())
            } else { complaint("text.amiss") }
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
                    if !was_digit { return Err(invalid_text()); }
                    was_digit = false;
                } else {
                    if !c.is_ascii() || c.to_digit(base).is_none() { return Err(invalid_text()); }
                    digits.push(c);
                    was_digit = true;
                }
            }
            if !was_digit || (radix == 0 && !has_prefix && digits.starts_with('0') && digits.bytes().any(|b| b != b'0')) {
                return Err(invalid_text());
            }
            // Too many figures in a base that is slow to read are refused
            // before the reading, with the limit named.
            if let Some(limit) = self.table.count("ext.builtin.to_int.digits") {
                if limit > 0 && !base.is_power_of_two() && digits.len() > limit { return Err(self.too_many_figures(limit)); }
            }
            let mut number = BigInt::parse_bytes(digits.as_bytes(), base).ok_or_else(|| invalid_text())?;
            if negative { number = -number; }
            return Ok(Value::from_big(number));
        }
        if values.len() > 1 { return Err(complaint("text.required")); }
        match &values[0] {
            Value::Flag(truth) => Ok(Value::Small(if *truth { 1 } else { 0 })),
            // A real past the numbers holds no whole number; the table may
            // give the words for each of the two.
            Value::Frac(ratio) if ratio.past_numbers() && self.table.has_any(if ratio.answers_none() { "ext.builtin.to_int.nan" } else { "ext.builtin.to_int.infinity" }) => {
                Err(self.table.single(if ratio.answers_none() { "ext.builtin.to_int.nan" } else { "ext.builtin.to_int.infinity" }).unwrap_or_default().to_owned())
            }
            other => math::whole_part(other).map(Value::from_big).ok_or_else(|| self.argument_fault("ext.syntax.call.amiss", None)),
        }
    }

    /// Whether a whole number may be written out at all: the table may
    /// put a limit on how many figures it is allowed to have.
    fn figures_allowed(&self, value: &Value) -> Result<(), String> {
        let Some(limit) = self.table.count("ext.builtin.to_int.digits") else { return Ok(()) };
        match value.settled() {
            Value::Huge(whole) if limit > 0 && num_traits::Signed::abs(&*whole).to_str_radix(10).len() > limit => Err(self.too_many_figures(limit)),
            _ => Ok(()),
        }
    }

    /// The words refusing a whole number of more figures than the table
    /// allows in text, the limit set in the middle of them.
    fn too_many_figures(&self, limit: usize) -> String {
        let (head, tail) = self.table.around("ext.builtin.to_int.digits.amiss").unwrap_or(("", ""));
        format!("{head}{limit}{tail}")
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
    fn open_arguments(&mut self, values: Vec<Value>) -> Res<(Vec<Value>, Vec<(String, Value)>)> {
        let mut positions = Vec::new();
        let mut names = Vec::new();
        for worth in values {
            let Value::Couple(pair) = worth else { positions.push(worth); continue };
            match &pair.0 {
                Value::Text(key) => names.push((key.to_string(), pair.1.clone())),
                Value::Flag(true) => {
                    let fault = || self.argument_fault("ext.syntax.call.spread.pairs.amiss", None);
                    if let Value::Dict(entries) = pair.1.settled() {
                        for (k, v) in entries.iter() {
                            match k {
                                Value::Text(text) => names.push((text.to_string(), v.clone())),
                                _ => return Err(fault().into()),
                            }
                        }
                    } else { return Err(fault().into()); }
                }
                Value::Flag(false) => {
                    match &pair.1.settled() {
                        Value::Iterator(_) => positions.extend(self.core_collect(&pair.1)?),
                        Value::Progression(walk) => {
                            let mut place = BigInt::from(0);
                            while place < walk.count() {
                                if let Some(item) = walk.item(&place) { positions.push(item); }
                                place += 1;
                            }
                        }
                        Value::Vector(values) | Value::Tuple(values) => positions.extend(values.iter().cloned()),
                        Value::Set(store) => positions.extend(store.borrow().values()),
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

    fn fit_arguments(&mut self, program: &Routine, manners: &[char], values: Vec<Value>) -> Res<Vec<Value>> {
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
        if program.generator && self.table.flag("ext.stmt.yield.suspends") {
            return Ok(Value::Generator(Rc::new(RefCell::new(Suspension::body(&program, frame)))));
        }
        // A call that would stand deeper than the table allows is refused
        // before it runs, in the table's own words, so that a clause may
        // take the fault and the run go on beneath the limit. An arm of
        // a branch is no call of anybody's and is not counted.
        let mut counted = 0usize;
        if !program.frameless {
            self.deeper()?;
            counted += 1;
        }
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
                    if p.generator && self.table.flag("ext.stmt.yield.suspends") {
                        break Ok(Value::Generator(Rc::new(RefCell::new(Suspension::body(&p, f)))));
                    }
                    program = p;
                    frame = f;
                    // A call in tail position deepens the run as any
                    // call does, though it takes the place of the last.
                    if !program.frameless {
                        if let Err(too_deep) = self.deeper() { break Err(too_deep); }
                        counted += 1;
                    }
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
        self.standing -= counted;
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

    fn octet_error(&self, reason: &str) -> String {
        self.table.single(&format!("ext.system.bytes.{reason}")).unwrap_or("").to_owned()
    }

    fn octets(&self, values: Vec<u8>, changeable: bool) -> Value {
        let lead = self.table.strings("ext.system.bytes.repr")[if changeable { 1 } else { 0 }].clone();
        Value::Octets { cell: Rc::new(RefCell::new(values)), changeable, lead: lead.into() }
    }

    fn octet_type(&self, changeable: bool) -> Value {
        let label = if changeable { "ext.builtin.bytearray" } else { "ext.builtin.bytes" };
        let parts = self.table.strings("ext.system.bytes.type");
        Value::OctetKind { changeable, shown: format!("{}{}{}", parts[0], self.table.single(label).unwrap_or(""), parts[1]).into() }
    }

    fn octet_whole(&self, value: &Value) -> Result<BigInt, String> {
        match value.kind() {
            Some(Kind::Whole | Kind::Truth) => value.as_big(),
            _ => Err(self.octet_error("arguments")),
        }
    }

    fn octet_item(&self, value: &Value) -> Result<u8, String> {
        self.octet_whole(value)?.to_u8().ok_or_else(|| self.octet_error("range"))
    }

    fn octet_contents(&self, source: &Value, iterable: bool) -> Result<Vec<u8>, String> {
        if let Value::Octets { cell, .. } = source { return Ok(cell.borrow().to_vec()); }
        if iterable {
            if let Value::Vector(items) = source {
                let mut result = Vec::with_capacity(items.len());
                for item in items.iter() { result.push(self.octet_item(item)?); }
                return Ok(result);
            }
        }
        Err(self.octet_error("arguments"))
    }

    fn octet_at(&self, index: &Value, length: usize) -> Result<usize, String> {
        let raw = self.octet_whole(index)?;
        let adjusted = if raw < BigInt::zero() { raw + length } else { raw };
        match adjusted.to_usize() {
            Some(i) if i < length => Ok(i),
            _ => Err(self.octet_error("index")),
        }
    }

    fn octet_encoding(&self, argument: Option<&Value>) -> Result<usize, String> {
        match argument {
            None => Ok(0),
            Some(Value::Text(encoding)) => {
                let normalized = encoding.to_lowercase().replace('_', "-");
                let entries = self.table.strings("ext.system.bytes.encodings");
                entries.iter().enumerate().find_map(|(i, e)| (e == &normalized).then_some(i / 2))
                    .ok_or_else(|| self.octet_error("unready"))
            }
            _ => Err(self.octet_error("arguments")),
        }
    }

    fn octets_from_text(&self, text: &str, alphabet: usize) -> Result<Vec<u8>, String> {
        if alphabet != 0 {
            let chars = text.chars().collect::<Vec<_>>();
            if let Some(first) = chars.iter().position(|c| *c as u32 > 127) {
                let last = chars[first..].iter().position(|c| c.is_ascii()).map_or(chars.len(), |i| first + i);
                let location = if last - first > 1 { format!("characters in position {}-{}", first, last - 1) }
                    else {
                        let code = chars[first] as u32;
                        let escape = match code { 0..=255 => format!("\\x{code:02x}"), 256..=65535 => format!("\\u{code:04x}"), _ => format!("\\U{code:08x}") };
                        format!("character '{}' in position {}", escape, first)
                    };
                let name = &self.table.strings("ext.system.bytes.encodings")[2];
                return Err(format!("{}'{}' codec can't encode {}: ordinal not in range(128)", self.octet_error("encode"), name, location));
            }
        }
        Ok(Vec::from(text.as_bytes()))
    }

    fn octets_to_text(&self, numbers: &[u8], alphabet: usize) -> Result<Value, String> {
        let mut fault = None;
        if alphabet == 1 {
            for (i, &number) in numbers.iter().enumerate() {
                if number > 127 { fault = Some((i, i + 1, "ordinal not in range(128)")); break; }
            }
        } else if let Err(error) = std::str::from_utf8(numbers) {
            let first = error.valid_up_to();
            let (last, why) = match error.error_len() {
                None => (numbers.len(), "unexpected end of data"),
                Some(n) if numbers[first] >= 194 && numbers[first] <= 244 => (first + n, "invalid continuation byte"),
                Some(n) => (first + n, "invalid start byte"),
            };
            fault = Some((first, last, why));
        }
        match fault {
            Some((first, last, why)) => {
                let subject = match last - first {
                    1 => format!("byte 0x{:02x} in position {}", numbers[first], first),
                    _ => format!("bytes in position {}-{}", first, last - 1),
                };
                Err(format!("{}'{}' codec can't decode {}: {}", self.octet_error("decode"), self.table.strings("ext.system.bytes.encodings")[alphabet * 2], subject, why))
            }
            None => Ok(Value::text(std::str::from_utf8(numbers).map_err(|_| self.octet_error("unready"))?)),
        }
    }

    fn octet_routine(&self, operation: u8, values: &[Value]) -> Result<Value, String> {
        let normalized: Vec<Value> = values.iter().map(Value::settled).collect();
        let values = normalized.as_slice();
        self.octet_work(operation, values, false)
    }

    fn octet_work(&self, operation: u8, values: &[Value], negative_allowed: bool) -> Result<Value, String> {
        if operation < 4 && values.len() == 3 {
            match &values[2] {
                Value::Text(word) if self.table.spells("ext.system.bytes.strict", word) => return self.octet_work(operation, &values[..2], negative_allowed),
                _ => return Err(self.octet_error("unready")),
            }
        }
        let refusal = || self.octet_error("unready");
        let wrong = || self.octet_error("arguments");
        let amount = |value: Option<&Value>| -> Result<usize, String> {
            match value { None => Ok(usize::MAX), Some(v) => { let n = self.octet_whole(v)?; Ok(n.to_usize().unwrap_or(usize::MAX)) } }
        };
        let seek = |hay: &[u8], part: &[u8]| (0..=hay.len()).find(|&i| hay[i..].starts_with(part));
        match operation {
            0 | 1 => {
                let content = match values.len() {
                    0 => Vec::new(),
                    1 if matches!(values[0].kind(), Some(Kind::Whole | Kind::Truth)) => {
                        let quantity = self.octet_whole(&values[0])?;
                        if quantity < BigInt::zero() { return Err(self.octet_error("negative")); }
                        let length = quantity.to_usize().ok_or_else(refusal)?;
                        let mut content = Vec::new();
                        content.try_reserve(length).map_err(|_| refusal())?;
                        content.resize(length, 0); content
                    }
                    1 => self.octet_contents(&values[0], true)?,
                    2 => {
                        let Value::Text(s) = &values[0] else { return Err(wrong()); };
                        self.octets_from_text(s, self.octet_encoding(values.get(1))?)?
                    }
                    _ => return Err(refusal()),
                };
                return Ok(self.octets(content, operation != 0));
            }
            2 => {
                if values.is_empty() || values.len() > 2 { return Err(refusal()); }
                let Value::Text(source) = &values[0] else { return Err(wrong()); };
                let content = self.octets_from_text(source, self.octet_encoding(values.get(1))?)?;
                return Ok(self.octets(content, false));
            }
            5 => {
                let [Value::Text(source)] = values else { return Err(wrong()); };
                let mut pending = None;
                let mut content = Vec::new();
                for (position, ch) in source.chars().enumerate() {
                    if pending.is_none() && (ch.is_ascii_whitespace() || ch == '\x0b') { continue; }
                    let digit = ch.to_digit(16).filter(|_| ch.is_ascii()).ok_or_else(|| format!("{}{}", self.octet_error("hex"), position))? as u8;
                    if let Some((high, _)) = pending.take() { content.push(high * 16 + digit); }
                    else { pending = Some((digit, position)); }
                }
                if pending.is_some() { return Err(format!("{}{}", self.octet_error("hex"), source.chars().count())); }
                return Ok(self.octets(content, false));
            }
            14 | 15 => {
                let maximum = if operation == 14 { 3 } else { 2 };
                if values.is_empty() || values.len() > maximum { return Err(refusal()); }
                let order = self.table.strings("ext.system.bytes.order");
                let reversed = match values.get(maximum - 1) {
                    None => false,
                    Some(Value::Text(name)) if name.as_ref() == order[0] => false,
                    Some(Value::Text(name)) if name.as_ref() == order[1] => true,
                    _ => return Err(self.octet_error("bad_order")),
                };
                if operation == 15 {
                    let mut content = self.octet_contents(&values[0], true)?;
                    if reversed { content.reverse(); }
                    let value = if negative_allowed { BigInt::from_signed_bytes_be(&content) }
                        else { BigInt::from_bytes_be(num_bigint::Sign::Plus, &content) };
                    return Ok(Value::from_big(value));
                }
                let number = self.octet_whole(&values[0])?;
                if !negative_allowed && number < BigInt::zero() { return Err(self.octet_error("unsigned")); }
                let width = match values.get(1) { None => 1, Some(n) => self.octet_whole(n)?.to_usize().ok_or_else(wrong)? };
                let mut content = if number.is_zero() { Vec::new() } else if negative_allowed { number.to_signed_bytes_be() } else { number.to_bytes_be().1 };
                if content.len() > width { return Err(self.octet_error("overflow")); }
                content.reverse();
                content.try_reserve(width - content.len()).map_err(|_| refusal())?;
                content.resize(width, if number < BigInt::zero() { u8::MAX } else { 0 });
                if !reversed { content.reverse(); }
                return Ok(self.octets(content, false));
            }
            16 => {
                let [object, Value::OctetKind { changeable: wanted, .. }] = values else { return Err(refusal()); };
                return Ok(Value::Flag(matches!(object, Value::Octets { changeable, .. } if changeable == wanted)));
            }
            17 => {
                let [Value::Octets { cell, changeable, .. }] = values else { return Err(refusal()); };
                if *changeable { return Err(self.octet_error("unhashable")); }
                let number = cell.borrow().iter().fold(0i64, |n, &b| n.wrapping_mul(1_000_003) ^ i64::from(b));
                return Ok(Value::Small(if number == -1 { -2 } else { number }));
            }
            _ => {}
        }
        let Some(Value::Octets { cell, changeable, .. }) = values.first() else { return Err(refusal()); };
        let content = cell.borrow().to_vec();
        let args = &values[1..];
        let result;
        match operation {
            3 if args.len() < 2 => return self.octets_to_text(&content, self.octet_encoding(args.first())?),
            4 if args.is_empty() => {
                let mut text = String::with_capacity(content.len() * 2);
                for n in content { text.push_str(&format!("{n:02x}")); }
                return Ok(Value::text(&text));
            }
            6 | 7 if args.is_empty() => {
                result = if operation == 6 { content.to_ascii_uppercase() } else { content.to_ascii_lowercase() };
            }
            8 if args.len() <= 2 => {
                let maximum = amount(args.get(1))?;
                let mut chunks = Vec::new();
                if args.first().map_or(true, |v| matches!(v, Value::Nil)) {
                    let mut rest = content.as_slice();
                    while !rest.is_empty() && (rest[0].is_ascii_whitespace() || rest[0] == 11) { rest = &rest[1..]; }
                    while !rest.is_empty() {
                        if chunks.len() == maximum { chunks.push(rest.to_vec()); break; }
                        let end = rest.iter().position(|n| n.is_ascii_whitespace() || *n == 11).unwrap_or(rest.len());
                        chunks.push(rest[..end].to_vec()); rest = &rest[end..];
                        while !rest.is_empty() && (rest[0].is_ascii_whitespace() || rest[0] == 11) { rest = &rest[1..]; }
                    }
                } else {
                    let separator = self.octet_contents(&args[0], false)?;
                    if separator.is_empty() { return Err(self.octet_error("separator")); }
                    let mut remaining = content.as_slice();
                    while chunks.len() < maximum {
                        match seek(remaining, &separator) {
                            None => break,
                            Some(i) => { chunks.push(remaining[..i].to_vec()); remaining = &remaining[i + separator.len()..]; }
                        }
                    }
                    chunks.push(remaining.to_vec());
                }
                return Ok(Value::Vector(Rc::new(chunks.into_iter().map(|chunk| self.octets(chunk, *changeable)).collect())));
            }
            9 if args.len() == 1 => {
                let Value::Vector(items) = &args[0] else { return Err(refusal()); };
                let pieces = items.iter().map(|item| self.octet_contents(item, false)).collect::<Result<Vec<_>, _>>()?;
                result = pieces.join(content.as_slice());
            }
            10 | 13 if args.len() == 1 => {
                let part = self.octet_contents(&args[0], false)?;
                let location = seek(&content, &part);
                return Ok(if operation == 13 { Value::Small(location.map_or(-1, |i| i as i64)) }
                    else { Value::Flag(location == Some(0)) });
            }
            11 if (2..=3).contains(&args.len()) => {
                let old = self.octet_contents(&args[0], false)?;
                let new = self.octet_contents(&args[1], false)?;
                let maximum = amount(args.get(2))?;
                let mut made = Vec::new();
                let mut cursor = 0;
                for _ in 0..maximum {
                    let Some(relative) = seek(&content[cursor..], &old) else { break; };
                    let found = cursor + relative;
                    made.extend_from_slice(&content[cursor..found]);
                    made.extend_from_slice(&new);
                    cursor = found + old.len();
                    if old.is_empty() {
                        if cursor >= content.len() { break; }
                        made.push(content[cursor]); cursor += 1;
                    }
                }
                made.extend_from_slice(&content[cursor..]);
                result = made;
            }
            12 if args.len() <= 1 => {
                let set = match args.first() { None | Some(Value::Nil) => vec![32, 9, 10, 13, 11, 12], Some(v) => self.octet_contents(v, false)? };
                let mut remaining = content.as_slice();
                while remaining.first().map_or(false, |n| set.contains(n)) { remaining = &remaining[1..]; }
                while remaining.last().map_or(false, |n| set.contains(n)) { remaining = &remaining[..remaining.len() - 1]; }
                result = remaining.to_vec();
            }
            _ => return Err(refusal()),
        }
        Ok(self.octets(result, *changeable))
    }

    fn bit_integer(&self, held: &Value) -> Result<BigInt, String> {
        let number = match held {
            Value::Flag(true) => BigInt::from(1),
            Value::Flag(false) => BigInt::from(0),
            Value::Huge(large) => large.as_ref().clone(),
            Value::Small(small) => BigInt::from(*small),
            _ => return Err(self.table.single("ext.op.bit.integer").unwrap_or_default().to_owned()),
        };
        Ok(number)
    }

    fn all_bits(&self, operation: Prim, values: &[Value]) -> Result<Value, String> {
        let first = self.bit_integer(&values[0])?;
        if operation == Prim::BitsOver { return Ok(Value::from_big(!first)); }
        let second = self.bit_integer(&values[1])?;
        if matches!(operation, Prim::BitsUp | Prim::BitsDown) {
            if second < BigInt::from(0) {
                return Err(self.table.single("ext.system.fault.shift").unwrap_or_default().to_owned());
            }
            let falls = operation == Prim::BitsDown;
            if falls && second >= BigInt::from(first.bits()) {
                return Ok(Value::Small(-i64::from(first < BigInt::from(0))));
            }
            if first == BigInt::from(0) { return Ok(Value::Small(0)); }
            let distance = second.to_usize().ok_or_else(|| self.table.single("ext.op.bit.beyond").unwrap_or_default().to_owned())?;
            return Ok(Value::from_big(if falls { first >> distance } else { first << distance }));
        }
        let combined = match operation {
            Prim::BitsEither => first | second,
            Prim::BitsOne => first ^ second,
            _ => first & second,
        };
        match (&values[0], &values[1]) {
            (Value::Flag(_), Value::Flag(_)) => Ok(Value::Flag(combined != BigInt::from(0))),
            _ => Ok(Value::from_big(combined)),
        }
    }

    fn powered_real(&self, pair: &[Value]) -> Result<Option<Value>, String> {
        let take = |v: &Value| match v {
            Value::Flag(t) => math::ratio_of(&Value::Small(if *t { 1 } else { 0 })),
            _ => math::ratio_of(v),
        };
        let Some(base) = take(&pair[0]) else { return Ok(None) };
        let Some(exponent) = take(&pair[1]) else { return Ok(None) };
        if base.places.or(exponent.places).is_none() && exponent.above >= BigInt::from(0) {
            return Ok(None);
        }
        let near = |r: &crate::data::Ratio| {
            if r.under && r.above == BigInt::from(0) { -0.0 }
            else { crate::data::nearest_binary(&r.above, &r.beneath) }
        };
        let b = near(&base);
        let e = near(&exponent);
        let fault = |label| self.table.single(label).unwrap_or_default().to_string();
        if b == 0.0 && e < 0.0 { return Err(fault("ext.op.pow.zero")); }
        if b < 0.0 && b.is_finite() && e.is_finite() && e.trunc() != e {
            return Err(fault("ext.op.pow.nonreal"));
        }
        let result = b.powf(e);
        if b.is_finite() && e.is_finite() && result.is_infinite() {
            return Err(fault("ext.op.pow.overflow"));
        }
        Ok(Some(crate::data::worth_of_binary(result, math::DEFAULT_PLACES)))
    }

    fn hash_key(&mut self, value: &Value) -> Result<Value, String> {
        if self.table.has_any("ext.stmt.class.special") && matches!(value, Value::Thing(_)) {
            let hash = self.user_operation(Prim::Hashed, std::slice::from_ref(value))?.ok_or_else(|| self.bad_answer())?;
            Ok(Value::Keyed(Rc::new(value.clone()), Rc::new(hash)))
        } else { Ok(value.clone()) }
    }

    fn keys_agree(&mut self, first: &Value, second: &Value) -> Result<bool, String> {
        // A key is found by the very thing before anything is asked of it.
        if first.one_place(second) { return Ok(true); }
        let objects = match (first, second) {
            (Value::Keyed(a, ah), Value::Keyed(b, bh)) => { if !ah.equals(bh) { return Ok(false); } (a.as_ref(), b.as_ref()) }
            (Value::Keyed(a, hash), b) | (b, Value::Keyed(a, hash)) => {
                // A hashed thing beside a plain number agrees only where its hash does.
                if matches!(b, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) && !hash.equals(b) { return Ok(false); }
                (a.as_ref(), b)
            }
            _ => return Ok(self.keys_match(first, second)),
        };
        if matches!(objects, (Value::Thing(a), Value::Thing(b)) if Rc::ptr_eq(a, b)) { return Ok(true); }
        if !matches!(objects.0, Value::Thing(_)) && !matches!(objects.1, Value::Thing(_)) { return Ok(self.keys_match(objects.0, objects.1)); }
        let test = self.prim(Prim::Eq, "", &[objects.0.clone(), objects.1.clone()])?;
        self.object_truth(&test)
    }

    fn appointment(&self, subject: &Value, index: usize) -> Option<Value> {
        let names = self.table.strings("ext.stmt.class.special");
        let word = names.get(index)?;
        let Value::Thing(thing) = subject else { return None };
        let mut blueprint = &thing.of;
        loop {
            let own = blueprint.shared.borrow().iter().find(|(key, _)| key == word).map(|(_, v)| v.clone());
            if own.is_some() { return own; }
            if let Some((_, body)) = blueprint.methods.iter().find(|(key, _)| key == word) { return Some(Value::Routine(body.clone())); }
            // Equality given, in the methods or the namespace, without a
            // hash: the things cannot be hashed.
            if index == 8 && names.get(2).map_or(false, |equal| blueprint.methods.iter().any(|(key, _)| key == equal) || blueprint.shared.borrow().iter().any(|(key, _)| key == equal)) { return Some(Value::Nil); }
            blueprint = blueprint.under.as_ref()?;
        }
    }

    fn appointed(&self, subject: &Value, index: usize) -> Option<Rc<Routine>> {
        match self.appointment(subject, index) {
            Some(Value::Routine(body) | Value::Bound(body, _)) => Some(body),
            _ => None,
        }
    }

    fn bad_answer(&self) -> String {
        self.table.single("ext.stmt.class.special.amiss").unwrap_or("").to_owned()
    }

    fn ask_special(&mut self, subject: &Value, index: usize, tail: &[Value]) -> Result<Option<Value>, String> {
        if index == 3 && self.appointment(subject, index).is_none() {
            return match self.ask_special(subject, 2, tail)? {
                None => Ok(None),
                Some(v @ Value::Refusal(_)) => Ok(Some(v)),
                Some(v) => self.object_truth(&v).map(|b| Some(Value::Flag(!b))),
            };
        }
        match self.appointment(subject, index) {
            None => Ok(None),
            Some(Value::Routine(body) | Value::Bound(body, _)) => {
                let arguments = std::iter::once(subject.clone()).chain(tail.iter().cloned()).collect();
                let scope = self.outermost.clone();
                match self.invoke(body, scope, arguments) {
                    Ok(value) => Ok(Some(value)),
                    Err(Escape::Error(message)) => Err(message),
                    Err(escape) => {
                        self.got_away = Some(escape);
                        Err(self.bad_answer())
                    }
                }
            }
            Some(_) => Err(self.bad_answer()),
        }
    }

    fn carries_instance(value: &Value) -> bool {
        Self::carries_instance_past(value, &mut Vec::new())
    }

    /// The scan proper, remembering the cells passed through so that a
    /// collection reaching itself is not looked into without end.
    fn carries_instance_past(value: &Value, passed: &mut Vec<usize>) -> bool {
        match value {
            Value::Backtrace(_) | Value::Keyed(..) | Value::Attributes(_) | Value::Thing(_) => true,
            Value::Shared(cell) | Value::Mutable(cell, _) => {
                let address = Rc::as_ptr(cell) as usize;
                if passed.contains(&address) { return false; }
                passed.push(address);
                Self::carries_instance_past(&cell.borrow(), passed)
            }
            Value::Row(v) | Value::Tuple(v) | Value::Vector(v) => v.iter().any(|item| Self::carries_instance_past(item, passed)),
            Value::Dict(d) => d.iter().flat_map(|(k, v)| [k, v]).any(|item| Self::carries_instance_past(item, passed)),
            _ => false,
        }
    }

    fn object_words(&mut self, subject: &Value, quoted: bool) -> Result<String, String> {
        let celled = match subject {
            Value::Mutable(place, represented) => Some((place.clone(), *represented)),
            Value::Shared(place) => Some((place.clone(), false)),
            _ => None,
        };
        if let Some((place, represented)) = celled {
            let inner = place.borrow().clone();
            // A collection of plain values is shown from its cell, so that
            // one reaching itself is met on the way round.
            if !quoted && !represented && matches!(inner, Value::Vector(_) | Value::Dict(_)) && !Self::carries_instance(&inner) {
                return Ok(self.show(std::slice::from_ref(&Value::Mutable(place, false))));
            }
            return self.object_words(&inner, quoted || represented);
        }
        if let Value::Backtrace(words) = subject { return Err(words.to_string()); }
        if self.table.strings("ext.stmt.class.special").is_empty() || (!quoted && !Self::carries_instance(subject)) { return Ok(self.show(std::slice::from_ref(subject))); }
        match subject {
            Value::Keyed(value, _) => self.object_words(value, quoted),
            Value::Attributes(t) => {
                let pairs = t.holds.borrow().iter().filter(|(name, v)| !matches!(v, Value::Unset) && !name.starts_with('\0')).map(|(name, v)| (Value::text(name), v.clone())).collect();
                self.object_words(&Value::Dict(Rc::new(pairs)), true)
            }
            Value::Thing(t) => {
                if self.is_fault_kind(&t.of) && self.appointment(subject, usize::from(quoted)).is_none() {
                    return Ok(if quoted { subject.representation(self.wording()) } else { subject.render(self.wording()) });
                }
                // A thing over a native worth shows as that worth where
                // its blueprint says nothing of how it is shown.
                if let Some(under) = Self::underlying(subject) {
                    let own = self.appointment(subject, 1).is_some() || (!quoted && self.appointment(subject, 0).is_some());
                    if !own { return self.object_words(&under, quoted); }
                }
                let chosen = usize::from(quoted || self.appointment(subject, 0).is_none());
                match self.ask_special(subject, chosen, &[])? {
                    None => Ok(format!("<{} object>", t.of.name)),
                    Some(Value::Text(s)) => Ok(s.to_string()),
                    _ => Err(self.bad_answer()),
                }
            }
            Value::Vector(v) => {
                let pieces = v.iter().map(|x| self.object_words(x, true)).collect::<Result<Vec<_>, _>>()?;
                Ok(String::from("[") + &pieces.join(", ") + "]")
            }
            Value::Dict(d) => {
                let pieces = d.iter().map(|(k, v)| {
                    Ok(self.object_words(k, true)? + ": " + &self.object_words(v, true)?)
                }).collect::<Result<Vec<_>, String>>()?;
                Ok(String::from("{") + &pieces.join(", ") + "}")
            }
            Value::Text(s) if quoted => {
                let mut written = String::from("'");
                for c in s.chars() {
                    match c {
                        '\'' => written.push_str("\\'"), '\\' => written.push_str("\\\\"),
                        '\n' => written.push_str("\\n"), '\r' => written.push_str("\\r"),
                        '\t' => written.push_str("\\t"), _ => written.push(c),
                    }
                }
                written.push('\'');
                Ok(written)
            }
            _ => Ok(subject.render(self.wording())),
        }
    }

    fn object_truth(&mut self, subject: &Value) -> Result<bool, String> {
        if let Value::Attributes(t) = subject { return Ok(t.holds.borrow().iter().any(|(name, x)| !matches!(x, Value::Unset) && !name.starts_with('\0'))); }
        if matches!(subject, Value::Refusal(_)) { return Err(self.table.single("ext.stmt.class.special.unready").unwrap_or_default().to_string()); }
        match self.ask_special(subject, 9, &[])? {
            Some(Value::Flag(b)) => return Ok(b),
            Some(other) => return Err(match self.table.single("ext.builtin.bool.result") {
                Some(opening) if !opening.is_empty() => format!("{}{}", opening, other.kind_word()),
                _ => self.bad_answer(),
            }),
            None => (),
        }
        if let Some(length) = self.ask_special(subject, 10, &[])? {
            if !matches!(length, Value::Small(_) | Value::Huge(_)) { return Err(self.bad_answer()); }
            let number = length.as_big()?;
            if number < BigInt::from(0) { return Err(self.bad_answer()); }
            return Ok(number != BigInt::from(0));
        }
        if let Some(under) = Self::underlying(subject) { return self.object_truth(&under.settled()); }
        Ok(self.stands_true(subject))
    }

    fn advance_object(&mut self, source: &Value) -> Result<Option<Value>, String> {
        if matches!(source, Value::Iterator(_) | Value::Generator(_)) { return self.next_value(source); }
        match source {
            Value::Cursor(c) => Ok(c.borrow_mut().pop_front()),
            _ => {
                let routine = self.appointed(source, 16).ok_or_else(|| self.bad_answer())?;
                match self.invoke(routine, self.outermost.clone(), vec![source.clone()]) {
                    Ok(v) => Ok(Some(v)),
                    Err(Escape::Thrown(Value::Thing(t))) if self.table.strings("ext.stmt.class.special.stop").iter().any(|name| t.of.goes_by(name, false)) => Ok(None),
                    Err(Escape::Error(s)) => Err(s),
                    Err(away) => { self.got_away = Some(away); Err(self.bad_answer()) }
                }
            }
        }
    }

    fn object_members(&mut self, subject: &Value) -> Result<Vec<Value>, String> {
        if let Some(under) = self.underlying_unless(subject, &[15]) { return self.object_members(&under); }
        match subject {
            Value::Attributes(t) => Ok(t.holds.borrow().iter().filter(|(n, x)| !matches!(x, Value::Unset) && !n.starts_with('\0')).map(|(n, _)| Value::text(n)).collect()),
            Value::Cursor(c) => {
                let mut c = c.borrow_mut();
                let answer = c.drain(..).collect();
                Ok(answer)
            }
            Value::Thing(_) => {
                let other = match self.ask_special(subject, 15, &[])? { Some(walk) => walk, None => self.placed_walk(subject).ok_or_else(|| self.bad_answer())? };
                let mut members = Vec::new();
                while let Some(value) = self.advance_object(&other)? { members.push(value); }
                Ok(members)
            }
            _ => self.gathered_members(subject),
        }
    }

    fn user_operation(&mut self, operation: Prim, operands: &[Value]) -> Result<Option<Value>, String> {
        if self.table.strings("ext.stmt.class.special").is_empty() { return Ok(None); }
        if Self::is_core_primitive(operation) && !operands.iter().any(|v| Self::carries_instance(v) || matches!(v, Value::Cursor(_))) { return Ok(None); }
        if operation == Prim::Belongs { return Ok(None); }
        let pair = match operation {
            Prim::Plus => Some((18, 26)), Prim::Minus => Some((19, 27)), Prim::Times => Some((20, 28)),
            Prim::Over | Prim::OverReal => Some((21, 29)), Prim::IntDiv => Some((22, 30)),
            Prim::Mod => Some((23, 31)), Prim::Power => Some((24, 32)),
            Prim::Lt => Some((4, 6)), Prim::Gt => Some((6, 4)), Prim::Le => Some((5, 7)), Prim::Ge => Some((7, 5)),
            Prim::Eq => Some((2, 2)), Prim::Ne => Some((3, 3)), Prim::At => Some((11, usize::MAX)),
            _ => None,
        };
        if let (Some((forward, reverse)), [left, right]) = (pair, operands) {
            let descendant = match (left, right) {
                (Value::Thing(a), Value::Thing(b)) => a.of.name != b.of.name && b.of.goes_by(&a.of.name, false),
                _ => false,
            };
            let changed = match (self.appointed(left, reverse), self.appointed(right, reverse)) {
                (Some(a), Some(b)) => !Rc::ptr_eq(&a, &b),
                (None, Some(_)) => true,
                _ => false,
            };
            let reverse_first = descendant && (forward < 8 || changed);
            let mut attempts = vec![(left, forward, right)];
            let same = matches!((left, right), (Value::Thing(a), Value::Thing(b)) if Rc::ptr_eq(&a.of, &b.of));
            if reverse_first { attempts.insert(0, (right, reverse, left)); }
            else if forward < 8 || !same { attempts.push((right, reverse, left)); }
            for (subject, index, argument) in attempts {
                if let Some(result) = self.ask_special(subject, index, std::slice::from_ref(argument))? {
                    if !matches!(result, Value::Refusal(_)) { return Ok(Some(result)); }
                }
            }

        }
        // A thing over a native worth is written into through that
        // worth, answers an absent key through the method the table
        // names for it, and for whatever its blueprint did not answer
        // above is the worth itself.
        if let (Prim::Placed | Prim::Erase, Some(target)) = (operation, operands.first()) {
            if let Some(Value::Mutable(cell, _)) = Self::underlying(target).filter(|_| self.appointment(target, if operation == Prim::Placed { 12 } else { 13 }).is_none()) {
                let mut inner = operands.to_vec();
                inner[0] = cell.borrow().clone();
                let result = self.prim(operation, "", &inner)?;
                *cell.borrow_mut() = result;
                return Ok(Some(target.clone()));
            }
        }
        if let (Prim::At, [subject @ Value::Thing(t), key]) = (operation, operands) {
            if let (Some(Value::Dict(entries)), Some(method)) = (Self::underlying(subject).map(|w| w.settled()), self.table.single("ext.stmt.class.missing").and_then(|word| self.inherited_entry(&t.of, word))) {
                let wanted = self.hash_key(key)?;
                let mut found = None;
                for (stored, value) in entries.iter() { if self.keys_agree(stored, &wanted)? { found = Some(value.clone()); break; } }
                return Ok(Some(match found {
                    Some(value) => value,
                    None => self.apply_class_member(method, vec![subject.clone(), key.clone()]).map_err(|fault| self.suspension_fault(fault))?,
                }));
            }
        }
        let free_at: &[usize] = match operation {
            Prim::Length => &[10], Prim::Hashed => &[8], Prim::Truthful | Prim::AsTruth => &[9, 10], Prim::NextItem => &[16],
            Prim::AsText => &[0, 1], Prim::Quoted => &[1], Prim::AsInt => &[38], Prim::AsReal => &[39], Prim::Magnitude => &[40],
            Prim::Contains | Prim::Absent => &[14],
            Prim::Iterator | Prim::Listed | Prim::Ordered | Prim::Tupling | Prim::Uniques | Prim::Backwards | Prim::Numbered | Prim::Zipped | Prim::Mapped | Prim::Filtered | Prim::EveryTrue | Prim::SomeTrue | Prim::Least | Prim::Greatest => &[15],
            Prim::Rounded | Prim::QuotRem | Prim::Powered | Prim::Hexadecimal | Prim::Octal | Prim::Binary => &[],
            Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power | Prim::At | Prim::Fetch => &[],
            _ => &[usize::MAX],
        };
        if free_at != [usize::MAX] {
            let mut settled = Vec::with_capacity(operands.len());
            let mut changed = false;
            for operand in operands {
                match self.underlying_unless(operand, free_at) {
                    Some(worth) => { settled.push(worth); changed = true; }
                    None => settled.push(operand.clone()),
                }
            }
            if changed && !settled.iter().any(|v| Self::carries_instance(v) || matches!(v, Value::Cursor(_))) {
                let word = self.table.prims.iter().find(|(_, p)| **p == operation).map(|(w, _)| w.clone()).unwrap_or_default();
                return Ok(Some(self.prim(operation, &word, &settled)?));
            }
        }
        let value = match (operation, operands) {
            (Prim::Of | Prim::HasMember, [Value::Backtrace(words), _]) => return Err(words.to_string()),
            // What whole number, real, magnitude, or positive form a thing
            // stands for, by its own methods where it has them.
            (Prim::AsInt | Prim::AsReal | Prim::Magnitude | Prim::Positive, [subject @ Value::Thing(_)]) => {
                let index = match operation { Prim::AsInt => 38, Prim::AsReal => 39, Prim::Magnitude => 40, _ => 41 };
                match self.ask_special(subject, index, &[])? { Some(answer) => answer, None => return Ok(None) }
            }
            (Prim::StartContext, [manager]) => {
                let plain = manager.settled();
                if let Value::Thing(thing) = &plain {
                    if self.appointment(&plain, 34).is_none() { return Err(self.no_manager(&thing.of.name)); }
                    self.ask_special(&plain, 33, &[])?.ok_or_else(|| self.no_manager(&thing.of.name))?
                } else if self.table.strings("ext.stmt.with.invalid").len() == 2 {
                    // What is no thing has no such methods at all, and a
                    // table with words for that says so by kind.
                    return Err(self.no_manager(&plain.kind_word()));
                } else { manager.clone() }
            }
            (Prim::Onto, [_, Value::Text(name), _]) if self.table.strings("ext.stmt.class.special").get(35..38).map_or(false, |members| members.iter().any(|word| word == name.as_ref())) => {
                return Err(self.table.single("ext.stmt.class.special.unready").unwrap_or_default().to_owned());
            }
            (Prim::Contains | Prim::Absent, [needle, Value::Dict(entries)]) => {
                let key = self.hash_key(needle)?;
                let mut found = false;
                for (stored, _) in entries.iter() { if self.keys_agree(stored, &key)? { found = true; break; } }
                Value::Flag(found != (operation == Prim::Absent))
            }
            (Prim::Placed, [Value::Dict(entries), key, value]) => {
                if let Some(words) = self.cannot_key(key) { return Err(words); }
                let keyed = self.hash_key(key)?;
                let mut result = entries.to_vec();
                let mut at = 0;
                while at < result.len() && !self.keys_agree(&result[at].0, &keyed)? { at += 1; }
                if at == result.len() { result.push((keyed, value.clone())); } else { result[at].1 = value.clone(); }
                Value::Dict(Rc::new(result))
            }
            (Prim::Erase, [Value::Dict(entries), key]) => {
                if let Some(words) = self.cannot_key(key) { return Err(words); }
                let hashed = self.hash_key(key)?;
                let mut remaining = Vec::new();
                let mut removed = false;
                for (old, item) in entries.iter() {
                    if self.keys_agree(old, &hashed)? { removed = true; } else { remaining.push((old.clone(), item.clone())); }
                }
                if !removed {
                    return Err(if self.table.has_any("ext.builtin.exceptions") { self.absent_key(key) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() });
                }
                Value::Dict(Rc::new(remaining))
            }
            (Prim::Contains | Prim::Absent, [needle, haystack]) if self.appointed(haystack, 14).is_some() => {
                let found = self.ask_special(haystack, 14, std::slice::from_ref(needle))?.unwrap();
                Value::Flag(self.object_truth(&found)? != (operation == Prim::Absent))
            }
            (Prim::At, [Value::Attributes(t), Value::Text(key)]) => {
                t.holds.borrow().iter().find(|(n, _)| n == key.as_ref()).map(|(_, value)| value.clone()).ok_or_else(|| self.bad_answer())?
            }
            (Prim::Erase, [one @ Value::Attributes(t), Value::Text(key)]) => {
                let mut attributes = t.holds.borrow_mut();
                let index = attributes.iter().position(|(word, _)| word == key.as_ref()).ok_or_else(|| self.bad_answer())?;
                attributes.remove(index);
                one.clone()
            }
            (Prim::Erase, [one, key]) if self.appointed(one, 13).is_some() => {
                self.ask_special(one, 13, std::slice::from_ref(key))?;
                one.clone()
            }
            (Prim::Length, [Value::Attributes(t)]) => Value::Small(t.holds.borrow().iter().filter(|(n, x)| !matches!(x, Value::Unset) && !n.starts_with('\0')).count() as i64),
            (Prim::At | Prim::Fetch, [Value::Dict(entries), key]) => {
                let hashed = self.hash_key(key)?;
                for (candidate, value) in entries.iter() {
                    if self.keys_agree(candidate, &hashed)? { return Ok(Some(value.clone())); }
                }
                return Ok(None);
            }
            (Prim::Invert, [one]) => Value::Flag(!self.object_truth(one)?),
            (Prim::Negate, [one]) if self.appointed(one, 25).is_some() => self.ask_special(one, 25, &[])?.unwrap(),
            (Prim::Quoted, [one]) => Value::text(&self.object_words(one, true)?),
            (Prim::AsText, [one]) => {
                self.figures_allowed(one)?;
                Value::text(&self.object_words(one, false)?)
            }
            (Prim::Truthful, []) => Value::Flag(false),
            (Prim::Truthful | Prim::AsTruth, [one]) => Value::Flag(self.object_truth(one)?),
            (Prim::Length, [one]) if self.appointed(one, 10).is_some() => {
                let length = self.ask_special(one, 10, &[])?.unwrap();
                if !matches!(length, Value::Small(_) | Value::Huge(_)) || length.as_big()? < BigInt::from(0) { return Err(self.bad_answer()); }
                length
            }
            (Prim::Hashed, [one]) if matches!(self.appointment(one, 8), Some(Value::Nil)) => {
                // Equality without a hash method, or the hash method set to
                // nothing: such a thing cannot be a key.
                return Err(self.core_complaint("core.unhashable", &one.kind_word()));
            }
            (Prim::Hashed, [one]) => match self.ask_special(one, 8, &[])? {
                Some(number @ (Value::Small(_) | Value::Huge(_))) => number,
                Some(_) => return Err(self.bad_answer()),
                None => match one {
                    Value::Thing(t) if self.appointed(one, 2).is_none() => Value::Small(t.turn as i64),
                    Value::Small(_) | Value::Huge(_) => one.clone(),
                    Value::Flag(b) => Value::Small(*b as i64),
                    _ => return Err(self.bad_answer()),
                },
            },
            (Prim::DistinctObjects, [set @ Value::Set(_)]) => set.clone(),
            (Prim::DistinctObjects, [Value::Vector(members)]) => {
                let mut result: Vec<Value> = Vec::new();
                for candidate in members.iter() {
                    let mut duplicate = false;
                    if matches!(candidate, Value::Thing(_)) {
                        let hashed = self.hash_key(candidate)?;
                        for old in &result {
                            if self.keys_agree(old, &hashed)? { duplicate = true; break; }
                        }
                        if !duplicate { result.push(hashed); }
                    } else { result.push(candidate.clone()); }
                }
                Value::Vector(Rc::new(result.into_iter().map(|v| match v { Value::Keyed(raw, _) => raw.as_ref().clone(), v => v }).collect()))
            }
            (Prim::Ordered, [one]) => {
                let input = self.object_members(one)?;
                let mut ordered: Vec<Value> = Vec::new();
                for item in input {
                    let mut place = ordered.len();
                    while place != 0 {
                        let below = self.prim(Prim::Lt, "", &[item.clone(), ordered[place - 1].clone()])?;
                        if !self.object_truth(&below)? { break; }
                        place -= 1;
                    }
                    ordered.insert(place, item);
                }
                Value::Vector(Rc::new(ordered)).keep(true)
            }
            (Prim::Iterated, [one @ Value::Thing(_)]) => one.clone(),
            (Prim::Listed, [one]) => {
                let result = Value::Vector(Rc::new(self.object_members(one)?));
                // Members an iterator hands out are kept quoted, as a window's are.
                if matches!(one, Value::Window(..) | Value::Mutable(_, true) | Value::Text(_) | Value::Iterator(_) | Value::Generator(_)) { result.keep(true) } else { result }
            },
            (Prim::Iterator, [one @ Value::Cursor(_)]) => one.clone(),
            (Prim::Iterator, [one]) => match self.ask_special(one, 15, &[])? {
                Some(iterator) => iterator,
                None => match self.placed_walk(one) {
                    Some(places) => places,
                    None => Value::Cursor(Rc::new(RefCell::new(self.gathered_members(one)?.into_iter().collect()))),
                },
            },
            (Prim::Iterator, [_, _]) => return Ok(None),
            // A thing with a method for walking backwards answers the
            // reversed builtin with that walk.
            (Prim::Backwards, [one @ Value::Thing(_)]) => match self.ask_special(one, 42, &[])? {
                Some(walk @ Value::Thing(_)) => walk,
                Some(walk) => self.iterated_value(&walk)?,
                None => return Ok(None),
            },
            (Prim::NextItem, [one, otherwise]) => self.advance_object(one)?.unwrap_or_else(|| otherwise.clone()),
            (Prim::NextItem, [Value::Cursor(cursor)]) => {
                if let Some(value) = cursor.borrow_mut().pop_front() { value } else {
                    let class = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
                        name: self.table.single("ext.stmt.class.special.stop").unwrap_or_default().to_owned(),
                        under: None, fields: vec![], methods: vec![], shared: RefCell::new(vec![]), reaches: vec![], answers: vec![], constants: vec![],
                    };
                    self.made += 1;
                    let value = Value::Thing(Rc::new(Thing { of: Rc::new(class), holds: RefCell::new(vec![]), turn: self.made }));
                    self.got_away = Some(Escape::Thrown(value));
                    return Err(self.table.single("ext.stmt.class.special.stop").unwrap_or_default().to_string());
                }
            },
            (Prim::NextItem, [one]) => self.ask_special(one, 16, &[])?.ok_or_else(|| self.bad_answer())?,
            (Prim::Belongs, [one, Value::Blueprint(class)]) => {
                Value::Flag(matches!(one, Value::Thing(t) if t.of.goes_by(&class.name, false)))
            }
            (Prim::Quoted | Prim::Truthful | Prim::Hashed | Prim::Ordered | Prim::Iterator | Prim::NextItem | Prim::Belongs, _) => return Err(self.bad_answer()),
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    /// Two rows are weighed member against member, and so are two
    /// tuples: the first pair of members that are not each other's equal
    /// says which goes first, and where no pair differed the shorter of
    /// the two goes before the longer. A thing whose blueprint appointed
    /// no method for the ordering is refused and named by its blueprint,
    /// since the kernel invents no order among things. Anything else is
    /// handed back untouched, for the plain working knows its own kinds.
    fn weighed_member_wise(&mut self, op: Prim, one: &Value, two: &Value) -> Result<Option<Value>, String> {
        let members = |value: &Value| match value { Value::Vector(held) | Value::Tuple(held) | Value::Row(held) => Some(Rc::clone(held)), _ => None };
        let rows = matches!(one, Value::Vector(_)) == matches!(two, Value::Vector(_));
        if let (Some(left), Some(right), true) = (members(one), members(two), rows) {
            for place in 0..left.len().min(right.len()) {
                if contained_equal(&left[place], &right[place]) { continue; }
                let same = self.prim(Prim::Eq, "", &[left[place].clone(), right[place].clone()])?;
                if self.object_truth(&same)? { continue; }
                return self.prim(op, "", &[left[place].clone(), right[place].clone()]).map(Some);
            }
            let rank = left.len().cmp(&right.len());
            let settled = match op { Prim::Lt => rank.is_lt(), Prim::Le => rank.is_le(), Prim::Gt => rank.is_gt(), _ => rank.is_ge() };
            return Ok(Some(Value::Flag(settled)));
        }
        if matches!(one, Value::Thing(_)) || matches!(two, Value::Thing(_)) {
            return Err(self.unordered_complaint(op, one, two));
        }
        Ok(None)
    }

    /// The complaint that two values stand in no order at all, carrying
    /// the sign that was asked for and the name of each of the kinds.
    fn unordered_complaint(&self, op: Prim, one: &Value, two: &Value) -> String {
        let sign = match op { Prim::Le => "<=", Prim::Gt => ">", Prim::Ge => ">=", _ => "<" };
        match self.table.strings("ext.op.order.unsupported") {
            [before, between, and, after] => format!("{before}{sign}{between}{}{and}{}{after}", one.kind_word(), two.kind_word()),
            _ => String::new(),
        }
    }

    fn equal_contents(&self, one: &Value, other: &Value) -> bool {
        match one { Value::Shared(cell) => return self.equal_contents(&cell.borrow(), other), _ => {} }
        match other { Value::Shared(cell) => return self.equal_contents(one, &cell.borrow()), _ => {} }
        // Within a container a value is equal to itself before anything
        // is asked of it, and a flag stands for its number.
        let held_alike = |x: &Value, y: &Value| x.one_place(y) || self.equal_contents(x, y);
        match (one, other) {
            (Value::Flag(truth), rhs) => return self.equal_contents(&Value::Small(i64::from(*truth)), rhs),
            (lhs, Value::Flag(truth)) => return self.equal_contents(lhs, &Value::Small(i64::from(*truth))),
            _ => (),
        }
        if let (Value::Dict(entries), Value::Dict(against)) = (one, other) {
            if entries.len() != against.len() { return false; }
            for (key, value) in entries.iter() {
                let found = against.iter().find(|entry| held_alike(key, &entry.0));
                match found {
                    Some(entry) if held_alike(value, &entry.1) => (),
                    _ => return false,
                }
            }
            return true;
        }
        if let (Value::Vector(values), Value::Vector(against)) | (Value::Tuple(values), Value::Tuple(against)) = (one, other) {
            return values.len() == against.len() && (0..values.len())
                .all(|i| held_alike(&values[i], &against[i]));
        }
        one.equals(other)
    }

    fn dictionary(&mut self, positional: &[Value], keywords: Vec<(String, Value)>) -> Result<Value, String> {
        let positional: Vec<Value> = positional.iter().map(collection_read).collect();
        if positional.len() > 1 {
            return Err(self.table.single("ext.builtin.map.arguments.amiss").unwrap_or("A map takes at most one source").to_string());
        }
        let mut result: Vec<(Value, Value)> = Vec::new();
        let source_pairs = match positional.first() {
            None => Vec::new(),
            Some(Value::Dict(entries)) => entries.to_vec(),
            Some(value) => {
                let mut pairs = Vec::new();
                for item in self.gathered_members(value)? {
                    let members = self.gathered_members(&item)?;
                    if members.len() != 2 {
                        return Err(self.table.single("ext.builtin.map.pair.amiss").unwrap_or("A map item needs two values").to_string());
                    }
                    pairs.push((members[0].clone(), members[1].clone()));
                }
                pairs
            }
        };
        let mut new_names = std::collections::HashSet::new();
        let mut additions = source_pairs;
        for (name, value) in keywords {
            if !new_names.insert(name.clone()) { return Err(self.argument_fault("ext.syntax.call.amiss.duplicate", Some(&name))); }
            additions.push((Value::text(&name), value));
        }
        for (key, value) in additions {
            if let Some((_, previous)) = result.iter_mut().find(|(known, _)| known.equals(&key)) {
                *previous = value;
            } else {
                result.push((key, value));
            }
        }
        Ok(Value::Dict(Rc::new(result)))
    }

    fn collection_cell(&self, value: Value) -> Value {
        match value {
            Value::Vector(_) | Value::Dict(_) if self.table.flag("ext.syntax.call.bind_names") =>
                Value::Shared(Rc::new(RefCell::new(value))),
            _ => value,
        }
    }

    /// Literal members keep their cells. Other operations ask the
    /// contents of their arguments, leaving those cells where they were.
    fn prim(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        // Two spans are alike when their bounds are, a pair of things
        // asked as the program asks; a span is always alike to itself.
        if let ([Value::Span(left), Value::Span(right)], true) = (v, matches!(op, Prim::Eq | Prim::Ne)) {
            let mut alike = Rc::ptr_eq(left, right);
            if !alike {
                alike = true;
                for (a, b) in left.iter().zip(right.iter()) {
                    let same = if a.equals(b) { true } else {
                        let answer = self.prim(Prim::Eq, name, &[a.clone(), b.clone()])?;
                        self.stands_true(&answer)
                    };
                    if !same { alike = false; break; }
                }
            }
            return Ok(Value::Flag(alike == (op == Prim::Eq)));
        }
        // A span reaching into a row has each thing among its bounds
        // asked for the whole number it stands for before the row is
        // read; a progression sliced is a progression over the places
        // the bounds pick out.
        if let (Prim::At, [target, Value::Span(bounds)], true) = (op, v, self.table.has_any("ext.builtin.slice")) {
            let target = target.settled();
            if !matches!(target, Value::Thing(_) | Value::Dict(_)) {
                if bounds.iter().any(|bound| matches!(bound, Value::Thing(_))) {
                    let bounds = self.span_settled(bounds)?;
                    return self.prim(op, name, &[target, Value::Span(Rc::new(bounds))]);
                }
                if let Value::Progression(walk) = &target {
                    let clipped = self.span_clipped(bounds, &Value::from_big(walk.count()))?;
                    let [from, to, by] = <[BigInt; 3]>::try_from(clipped).expect("three bounds");
                    let picked = crate::data::Progression { first: &walk.first + &from * &walk.stride, limit: &walk.first + &to * &walk.stride, stride: &walk.stride * by, word: walk.word.clone() };
                    return Ok(Value::Progression(Rc::new(picked)));
                }
            }
        }
        // A key handed out with its hash is, for ordering and arithmetic,
        // the key itself.
        if matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power)
            && v.iter().any(|item| matches!(item, Value::Keyed(..))) {
            let bare: Vec<Value> = v.iter().map(|item| match item { Value::Keyed(key, _) => key.as_ref().clone(), other => other.clone() }).collect();
            return self.prim(op, name, &bare);
        }
        // Two texts stand in the order of their letters' code points, where
        // the table says texts are ordered.
        if let ([Value::Text(left), Value::Text(right)], true) = (v, matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) && self.table.flag("ext.op.order.text")) {
            let rank = left.chars().cmp(right.chars());
            return Ok(Value::Flag(match op { Prim::Lt => rank.is_lt(), Prim::Le => rank.is_le(), Prim::Gt => rank.is_gt(), _ => rank.is_ge() }));
        }
        // Kinds that stand in no order to one another are refused with
        // both named, where the table gives the four pieces of words;
        // numbers order among themselves, and texts among themselves.
        if let ([left, right], [before, between, and, after]) = (v, self.table.strings("ext.op.order.unsupported")) {
            if matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
                let (left, right) = (left.settled(), right.settled());
                let counts = |x: &Value| matches!(x, Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Flag(_));
                let texts = matches!((&left, &right), (Value::Text(_), Value::Text(_)));
                // Two of one kind may still order themselves, as sets and
                // rows do; two of different kinds, or of a kind without any
                // order, cannot.
                let orderless = left.kind_word() != right.kind_word() || matches!(left, Value::Nil | Value::Dict(_));
                if orderless && !(counts(&left) && counts(&right)) && !texts && !matches!((&left, &right), (Value::Thing(_), _) | (_, Value::Thing(_))) && math::below(&left, &right).is_none() {
                    let sign = match op { Prim::Lt => "<", Prim::Le => "<=", Prim::Gt => ">", _ => ">=" };
                    return Err(format!("{before}{sign}{between}{}{and}{}{after}", left.kind_word(), right.kind_word()));
                }
            }
        }
        if self.table.flag("ext.syntax.call.bind_names") {
            let result = if matches!(op, Prim::ExtendLiteral(_, false)) {
                self.prim_values(op, name, &[collection_read(&v[0]), v[1].clone()])?
            } else if matches!(op, Prim::MakeArray | Prim::MakeMap | Prim::Couple | Prim::SpanOf | Prim::SliceBounds | Prim::IdentityOf | Prim::ValueMethod | Prim::Perform | Prim::Weigh | Prim::Prepare) {
                self.prim_values(op, name, v)?
            } else if matches!(op, Prim::SetAssign(0) | Prim::Backwards | Prim::Iterated | Prim::Erase | Prim::Pointed) && v.first().map_or(false, |first| Self::dict_cell(first).is_some()) {
                // A map keeps its cell here: written into in place, a
                // key taken out of it, walked under watch forwards or
                // backwards, or handed on after a compound write.
                self.prim_values(op, name, v)?
            } else {
                let unwrapped: Vec<Value> = v.iter().map(collection_read).collect();
                self.prim_values(op, name, &unwrapped)?
            };
            return Ok(self.collection_cell(result));
        }
        self.prim_values(op, name, v)
    }

    fn prim_values(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        if self.reads_manners() && matches!(op, Prim::Perform | Prim::Weigh | Prim::Prepare | Prim::Summon | Prim::WorldBook | Prim::HereBook) {
            return self.text_operation(op, name, v);
        }
        if matches!(op, Prim::Added | Prim::Placed) {
            if let Some(Value::Mutable(cell, _)) = v.first() {
                let mut arguments = v.to_vec();
                arguments[0] = v[0].settled();
                let changed = self.prim(op, name, &arguments)?;
                cell.replace(changed);
                return Ok(v[0].clone());
            }
        }
        if let Prim::ExtendLiteral(_, expanded) = op {
            if matches!(v.first(), Some(Value::Mutable(..))) || expanded && matches!(v.get(1), Some(Value::Mutable(..))) {
                let mut arguments = v.to_vec();
                arguments[0] = arguments[0].settled();
                if expanded { arguments[1] = arguments[1].settled(); }
                return self.prim(op, name, &arguments);
            }
        }
        // A map written into with the bit-or sign in its compound form is
        // written in its own cell, so every name for it sees the pairs
        // it took; a map or any row of pairs may stand on the right, and
        // the pairs before an ill-shaped one are written before it stops.
        if let (Prim::SetAssign(0), [left, right]) = (op, v) {
            if let Some(cell) = Self::dict_cell(left).filter(|_| self.table.flag("ext.op.bit.or.maps")) {
                let (pairs, stopped) = self.pairs_offered(right);
                let mut entries = match &*cell.borrow() { Value::Dict(held) => held.to_vec(), _ => Vec::new() };
                for (key, value) in pairs {
                    match entries.iter_mut().find(|entry| self.keys_match(&entry.0, &key)) {
                        Some(entry) => entry.1 = value,
                        None => entries.push((key, value)),
                    }
                }
                cell.replace(Value::Dict(Rc::new(entries)));
                return match stopped { Some(words) => Err(words), None => Ok(left.clone()) };
            }
        }
        // A key taken out of a map held in a cell is taken out of the
        // map in that cell, so every name for the map sees it gone.
        if let (Prim::Erase, [target, key]) = (op, v) {
            if let Some(cell) = Self::dict_cell(target).filter(|_| self.table.flag("ext.syntax.call.bind_names")) {
                let at = self.as_key(key);
                if let Some(words) = self.cannot_key(&at) { return Err(words); }
                let entries = match &*cell.borrow() { Value::Dict(held) => held.to_vec(), _ => Vec::new() };
                let wanted = self.hash_key(&at)?;
                if self.table.has_any("ext.stmt.del") && !self.key_held(&entries, &wanted)? {
                    return Err(if self.table.has_any("ext.builtin.exceptions") { self.absent_key(&at) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() });
                }
                let kept: Vec<(Value, Value)> = self.without_key(&entries, &wanted)?;
                cell.replace(Value::Dict(Rc::new(kept)));
                return Ok(target.clone());
            }
        }
        // A map walked from its cell, forwards for a loop or backwards
        // from its last key to its first, is watched for a change of
        // size on the way.
        if let (Prim::Iterated | Prim::Backwards, [source]) = (op, v) {
            if self.table.flag("ext.stmt.yield.suspends") && self.table.has_any("ext.syntax.map.resized") && Self::dict_cell(source).is_some() {
                let mut keys = self.gathered_members(source)?;
                if op == Prim::Backwards { keys.reverse(); }
                return Ok(self.walk_over(source, keys));
            }
        }
        // A view of a map's keys or pairs meets a set, or another view,
        // as a set would under the set signs.
        if matches!(op, Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne | Prim::Minus) && v.iter().any(|value| matches!(value, Value::Window(..))) {
            let mut as_sets = Vec::new();
            for value in v {
                as_sets.push(match value {
                    Value::Window(..) => Value::Set(Rc::new(RefCell::new(self.gather_set(Some(&value.settled()))?))),
                    other => other.clone(),
                });
            }
            return self.prim(op, name, &as_sets);
        }
        if v.iter().any(|value| matches!(value, Value::Mutable(..) | Value::Window(..))) && !matches!(op, Prim::Say | Prim::Out | Prim::Listed | Prim::MakeArray | Prim::MakeMap | Prim::Couple | Prim::ExtendLiteral(..) | Prim::Added | Prim::Placed | Prim::ValueMethod) {
            let settled: Vec<Value> = v.iter().map(Value::settled).collect();
            return self.prim(op, name, &settled);
        }
        if let Some(result) = self.user_operation(op, v)? { return Ok(result); }
        if let ([one, two], true) = (v, matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) && self.table.strings("ext.op.order.unsupported").len() == 4) {
            let (one, two) = (one.clone(), two.clone());
            if let Some(answer) = self.weighed_member_wise(op, &one, &two)? { return Ok(answer); }
        }
        if Self::is_core_primitive(op) { return self.core_primitive(op, name, v.to_vec(), Vec::new()); }
        if let Prim::Octets(which) = op { return self.octet_routine(which, v); }
        if v.len() == 2 {
            if let (Value::Octets { cell: x, changeable, .. }, Value::Octets { cell: y, .. }) = (&v[0], &v[1]) {
                match op {
                    Prim::Plus => { let mut bytes = x.borrow().to_vec(); bytes.extend_from_slice(&y.borrow()); return Ok(self.octets(bytes, *changeable)); }
                    Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge => {
                        let ordering = x.borrow().cmp(&y.borrow());
                        return Ok(Value::Flag(match op { Prim::Lt => ordering.is_lt(), Prim::Le => !ordering.is_gt(), Prim::Gt => ordering.is_gt(), _ => !ordering.is_lt() }));
                    }
                    _ => {}
                }
            }
            let has_octets = v.iter().any(|item| matches!(item, Value::Octets { .. }));
            if has_octets && op == Prim::Times {
                let (bytes, times) = if matches!(&v[0], Value::Octets { .. }) { (&v[0], &v[1]) } else { (&v[1], &v[0]) };
                let Value::Octets { cell, changeable, .. } = bytes else { unreachable!() };
                let n = self.octet_whole(times)?;
                let quantity = if n < BigInt::zero() { 0 } else { n.to_usize().ok_or_else(|| self.octet_error("unready"))? };
                let cell = cell.borrow();
                let mut result = Vec::new();
                let size = quantity.checked_mul(cell.len()).ok_or_else(|| self.octet_error("unready"))?;
                result.try_reserve_exact(size).map_err(|_| self.octet_error("unready"))?;
                if cell.len() > 0 { for _ in 0..quantity { result.extend_from_slice(&cell); } }
                return Ok(self.octets(result, *changeable));
            }
            if has_octets && matches!(op, Prim::Plus | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Mod | Prim::Join) {
                return Err(self.octet_error("unready"));
            }
        }
        if self.table.flag("ext.op.arithmetic.flags") && v.iter().any(|x| matches!(x, Value::Flag(_))) {
            let arithmetic = matches!(op, Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power | Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Negate | Prim::AsReal);
            if arithmetic {
                let counted: Vec<Value> = v.iter().map(|x| match x {
                    Value::Flag(false) => Value::Small(0), Value::Flag(true) => Value::Small(1), other => other.clone(),
                }).collect();
                return self.prim(op, name, &counted);
            }
        }
        if op == Prim::Power && self.table.flag("ext.op.pow.real_exponent") {
            if let Some(result) = self.powered_real(v)? { return Ok(result); }
        }
        let formatting = op == Prim::Mod && self.table.flag("ext.op.rem.formats_text") && matches!(v.first(), Some(Value::Text(_)));
        if self.table.has_any("ext.op.div.zero") && !formatting && matches!(op, Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod) {
            let zero = match &v[1] {
                Value::Flag(false) => true,
                other => math::ratio_of(other).map_or(false, |r| r.above == BigInt::from(0) && r.beneath != BigInt::from(0)),
            };
            if zero {
                let reals = v.iter().any(|x| matches!(x, Value::Frac(r) if r.places.is_some()));
                let label = match op {
                    Prim::IntDiv if reals => "ext.op.quot.real_zero",
                    Prim::IntDiv => "ext.op.quot.zero",
                    Prim::Mod if reals => "ext.op.rem.real_zero",
                    Prim::Mod => "ext.system.fault.modulo",
                    _ => "ext.op.div.zero",
                };
                return Err(self.table.single(label).unwrap_or_default().to_owned());
            }
        }
        let w = self.wording();
        let n = |k: usize| -> Result<(), String> {
            if v.len() == k { Ok(()) } else { Err(format!("{}() expects {} argument{}, got {}", name, k, if k == 1 { "" } else { "s" }, v.len())) }
        };
        if v.len() == 2 && v.iter().any(|item| matches!(item, Value::Set(_))) {
            let rule = match op { Prim::BitsEither => Some(0), Prim::BitsBoth => Some(1), Prim::Minus => Some(2), Prim::BitsOne => Some(3), _ => None };
            if rule.is_some() || matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
                let (Value::Set(x), Value::Set(y)) = (&v[0], &v[1]) else { return Err(self.set_complaint("operands", "")); };
                let x = x.borrow();
                let y = y.borrow();
                return Ok(match rule {
                    Some(rule) => Value::Set(Rc::new(RefCell::new(x.merge(&y, rule)))),
                    None => Value::Flag(match op {
                        Prim::Lt => x.keys.len() < y.keys.len() && x.keys.is_subset(&y.keys),
                        Prim::Le => x.keys.is_subset(&y.keys),
                        Prim::Gt => x.keys.len() > y.keys.len() && x.keys.is_superset(&y.keys),
                        _ => x.keys.is_superset(&y.keys),
                    }),
                });
            }
        }
        if matches!(op, Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power
            | Prim::Positive | Prim::NumberAlone | Prim::Negate | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne | Prim::BitsOver | Prim::BitsUp | Prim::BitsDown) {
            if v.iter().any(|value| matches!(value, Value::Complex(_))) { return crate::complex::reckon(self.table, op, v); }
            for value in v {
                if let Value::Imaginary { unready, .. } = value { return Err(unready.to_string()); }
            }
        }
        if self.table.flag("ext.op.bit.whole") && matches!(op,
            Prim::BitsOver | Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne | Prim::BitsUp | Prim::BitsDown)
            && !(op == Prim::BitsEither && self.table.flag("ext.op.bit.or.maps") && matches!(v, [Value::Dict(_), Value::Dict(_)]))
        {
            let fault = || self.table.single("ext.system.fault.operands").unwrap_or("Working on bits needs a whole number").to_string();
            let number = |value: &Value| {
                match value.kind() {
                    Some(Kind::Whole) | Some(Kind::Truth) => value.as_big(),
                    _ => Err(fault()),
                }
            };
            let left = number(&v[0])?;
            if op == Prim::BitsOver {
                return Ok(Value::from_big(!left));
            }
            let right = number(&v[1])?;
            let answer = if matches!(op, Prim::BitsUp | Prim::BitsDown) {
                if right.sign() == num_bigint::Sign::Minus {
                    return Err(self.table.single("ext.system.fault.shift").unwrap_or("Bit shift by a negative number").to_string());
                }
                match op {
                    Prim::BitsDown if right >= BigInt::from(left.bits()) => BigInt::from(i32::from(left.sign() == num_bigint::Sign::Minus) * -1),
                    _ if left == BigInt::from(0) => left,
                    Prim::BitsUp => {
                        let by = right.to_usize().filter(|count| {
                            (*count as u128) + u128::from(left.bits()) < isize::MAX as u128
                        }).ok_or_else(|| self.table.single("ext.op.bit.whole.room").unwrap_or("Bit shift count is too large").to_owned())?;
                        left << by
                    },
                    _ => left >> right.to_usize().ok_or_else(fault)?,
                }
            } else {
                match op {
                    Prim::BitsOne => left ^ right,
                    Prim::BitsBoth => left & right,
                    _ => left | right,
                }
            };
            let flags = v.iter().all(|x| x.kind() == Some(Kind::Truth));
            return Ok(if flags && !matches!(op, Prim::BitsUp | Prim::BitsDown) {
                Value::Flag(answer != BigInt::from(0))
            } else {
                Value::from_big(answer)
            });
        }
        Ok(match op {
            Prim::Positive => { n(1)?; v[0].clone() }
            Prim::OctetAssign(times) => {
                let changed = self.prim(if times { Prim::Times } else { Prim::Plus }, name, v)?;
                match (&v[0], &changed) {
                    (Value::Octets { cell, changeable: true, .. }, Value::Octets { cell: content, .. }) => {
                        cell.replace(content.borrow().to_vec());
                        v[0].clone()
                    }
                    _ => changed,
                }
            }
            Prim::ClassWork(work) => return self.work_on_class(work, v.to_vec()).map_err(|e| self.suspension_fault(e)),
            Prim::Pointed => v[0].clone().keeping_point(true),
            Prim::ComplexMade => crate::complex::create(self.table, v)?,
            Prim::NumberAlone => match &v[0] {
                Value::Small(_) | Value::Huge(_) | Value::Frac(_) => v[0].clone(),
                Value::Flag(b) => Value::Small(if *b { 1 } else { 0 }),
                _ => return Err(self.table.single("ext.op.plus.non_number").unwrap_or("").to_owned()),
            },
            Prim::MatrixProduct => {
                let words = self.table.single("ext.op.matrix.unready").unwrap_or_default();
                return Err(words.to_owned());
            }
            Prim::Dictionary => self.dictionary(v, Vec::new())?,
            // A method spelled with its class in front is asked of its
            // first argument, as it would be of a value of that class.
            Prim::ValueMethod if name.contains('.') => {
                let operation = name.rsplit('.').next().unwrap_or(name).to_owned();
                if v.is_empty() { return Err(self.method_fault("arguments")); }
                return self.value_member(&v[0], &operation, v[1..].to_vec(), Vec::new()).map_err(|fault| self.suspension_fault(fault));
            }
            Prim::ValueMethod | Prim::BindValueMethod | Prim::SortedValues => return Err(self.method_fault("attribute")),
            Prim::SetAssign(operation) => {
                n(2)?;
                let ordinary = [Prim::BitsEither, Prim::BitsBoth, Prim::Minus, Prim::BitsOne][operation as usize];
                let answer = self.prim(ordinary, name, v)?;
                match (&v[0], &answer) {
                    (Value::Set(place), Value::Set(updated)) => {
                        place.replace(updated.borrow().clone());
                        v[0].clone()
                    }
                    _ => answer,
                }
            }
            Prim::SetCall(which) => self.work_set(which, v)?,
            Prim::EmptySet => Value::Set(Rc::new(RefCell::new(self.gather_set(None)?))),
            Prim::StartContext | Prim::DistinctObjects => return Err(self.bad_answer()),
            Prim::Textual(work) => { let values: Vec<Value> = v.iter().map(|x| match x.settled() { Value::Tuple(row)=>Value::Vector(row), other=>other }).collect(); crate::text::apply(self.table, work, name, &values, self.wording())? },
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
                    Value::Generator(state) => {
                        let mut yielded = Vec::new();
                        loop {
                            if star.is_none() && yielded.len() > wanted { break; }
                            match self.resume(state, Value::Nil).map_err(|e| self.suspension_fault(e))? {
                                Some(item) => yielded.push(item),
                                None => break,
                            }
                        }
                        yielded
                    }
                    walk @ Value::Iterator(_) => {
                        let mut taken = Vec::new();
                        while star.is_some() || taken.len() <= wanted {
                            match self.next_value(walk)? { Some(item) => taken.push(item), None => break }
                        }
                        taken
                    }
                    Value::Set(set) => set.borrow().values(),
                    Value::Tuple(items) | Value::Row(items) => items.to_vec(),
                    Value::TextRow(..) | Value::Octets { .. } | Value::Progression(_) => self.gathered_members(&v[0])?,
                    Value::Text(s) => s.chars().map(|letter| Value::text(&letter.to_string())).collect(),
                    Value::Dict(entries) => entries.iter().map(|entry| match &entry.0 { Value::Keyed(v, _) => v.as_ref().clone(), key => key.clone() }).collect(),
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
            Prim::Iterated => match self.begin_set_walk(&v[0]) {
                None if matches!(v[0], Value::Iterator(_)) => v[0].clone(),
                None => Value::Vector(Rc::new(self.gathered_members(&v[0])?)),
                Some(walk) => walk,
            },
            Prim::CheckUnpack(count) | Prim::BindingWidth(count) => {
                let values = if let Value::Generator(generator) = &v[0] {
                    let mut taken = Vec::new();
                    for _ in 0..=count {
                        let item = self.resume(generator, Value::Nil).map_err(|fault| self.suspension_fault(fault))?;
                        if let Some(item) = item { taken.push(item); } else { break; }
                    }
                    taken
                } else { self.gathered_members(&v[0])? };
                if values.len() != count {
                    let label = if matches!(op, Prim::BindingWidth(_)) { "ext.stmt.binding.unrun" } else { "ext.op.comprehension.unpack.amiss" };
                    return Err(self.table.single(label).unwrap_or("Comprehension target and item have different lengths").into());
                }
                Value::Vector(Rc::new(values))
            }
            Prim::ExtendLiteral(dictionary, expanded) => {
                if let Value::Set(kept) = &v[0] {
                    let additions = if expanded { self.gathered_members(&v[1])? } else { vec![v[1].clone()] };
                    for addition in additions {
                        if matches!(addition, Value::Thing(_)) {
                            let key = self.hash_key(&addition)?;
                            let previous = kept.borrow().values();
                            let mut duplicate = false;
                            for entry in previous { if self.keys_agree(&entry, &key)? { duplicate = true; break; } }
                            if !duplicate { let address = format!("instance:{}", kept.borrow().entries.len()); kept.borrow_mut().put(address, key); }
                        } else { kept.borrow_mut().put(self.hash_for_set(&addition)?, addition); }
                    }
                    v[0].clone()
                } else if !dictionary {
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
                    for (key, value) in incoming {
                        if let Some(words) = self.cannot_key(&key) { return Err(words); }
                        if !self.table.has_any("ext.stmt.class.special") { set_key(&mut combined, key, value); continue; }
                        let key = self.hash_key(&key)?;
                        let mut position = 0;
                        while position < combined.len() && !self.keys_agree(&combined[position].0, &key)? { position += 1; }
                        if position == combined.len() { combined.push((key, value)); }
                        else { combined[position].1 = value; }
                    }
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
                if self.check_set_walk(&v[0])?.is_some() { return self.element(&v[0], &v[1], Reading::Plain); }
                let wants_key = op == Prim::KeyAt;
                match &v[0] {
                    Value::Octets { cell, .. } => if wants_key { Value::Small(at as i64) }
                        else { Value::Small(i64::from(*cell.borrow().get(at).ok_or_else(|| self.octet_error("index"))?)) },
                    Value::Progression(walk) => {
                        if wants_key { Value::Small(at as i64) }
                        else { walk.item(&BigInt::from(at)).ok_or_else(|| self.argument_fault("ext.builtin.range.index", None))? }
                    }
                    Value::TextRow(row, _) => {
                        let word = row.get(at).ok_or_else(||self.span_complaint("bounds"))?;
                        if wants_key {Value::Small(at as i64)} else {Value::text(word)}
                    }
                    Value::Set(members) => members.borrow().values().get(at).cloned().ok_or_else(|| self.set_complaint("missing", &at.to_string()))?,
                    Value::Vector(items) | Value::Tuple(items) => match items.get(at) {
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
                    Value::Octets { cell, .. } => Value::Small(cell.borrow().len() as i64),
                    Value::Progression(walk) => Value::from_big(walk.count()),
                    Value::Set(set) => Value::Small(set.borrow().keys.len() as i64),
                    Value::Arguments(items) | Value::Vector(items) | Value::Tuple(items) => Value::Small(items.len() as i64),
                    Value::TextRow(row, _) => Value::Small(row.len() as i64),
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
            Prim::Adorn(manner) => {
                let member = if manner == 'w' {
                    n(2)?;
                    match &v[0] {
                        Value::Adorned(old) if old.manner == 'p' => Adornment {
                            manner: 'p', target: old.target.clone(), extra: Some(v[1].clone()),
                        },
                        _ => return Err(self.table.single("ext.stmt.class.unready").unwrap_or_default().to_string()),
                    }
                } else {
                    n(1)?;
                    Adornment { manner, target: v[0].clone(), extra: None }
                };
                Value::Adorned(Rc::new(member))
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
                if self.has_class_order() { return self.build_class_value(title, vec![ancestor], holdings).map_err(|e| self.suspension_fault(e)); }
                let heir = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
                    under: Some(ancestor), name: title, shared: RefCell::new(holdings),
                    methods: vec![], constants: vec![], reaches: vec![], answers: vec![], fields: vec![],
                };
                Value::Blueprint(Rc::new(heir))
            }
            Prim::CopyWorth => {
                n(2)?;
                let deep = matches!(v[1], Value::Flag(true));
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
                match self.attribute(&v[0], &word).or_else(|| v.get(2).cloned()) {
                    Some(found) => found,
                    None => {
                        // A namespace may answer for a name it has not,
                        // through the routine the table names for it.
                        let space = match &v[0] { Value::Shared(cell) => cell.borrow().clone(), other => other.clone() };
                        let answerer = match (&space, self.table.single("ext.system.module.getattr")) {
                            (Value::Thing(t), Some(name)) => t.holds.borrow().iter().find(|(n, _)| n == name).map(|(_, held)| match held { Value::Shared(cell) => cell.borrow().clone(), other => other.settled() }),
                            _ => None,
                        };
                        match answerer {
                            Some(routine @ (Value::Routine(_) | Value::Bound(..))) => self.apply_class_member(routine, vec![Value::text(&word)]).map_err(|fault| self.suspension_fault(fault))?,
                            _ => {
                                let (opening, ending) = self.table.around("ext.builtin.member.absent").unwrap_or(("", ""));
                                return Err(format!("{opening}{word}{ending}"));
                            }
                        }
                    }
                }
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
            Prim::HasMember => {
                n(2)?;
                if self.has_class_order() && matches!(&v[0],Value::Thing(_)|Value::Blueprint(_)|Value::Routine(_)|Value::Bound(..)|Value::Method(..)|Value::Wrapped(..)){return Ok(Value::Flag(true));}
                let word = v[1].bare();
                // A kind value and an intrinsic are each named, where the
                // table has a member for a name.
                if matches!(&v[0], Value::KindOf(_) | Value::Intrinsic(_)) && self.table.spells("ext.builtin.class.name", &word) { return Ok(Value::Flag(true)); }
                // A native kind's word has a maker and a name, where a class may stand on it.
                if let Value::Intrinsic(kind) = &v[0] {
                    if self.table.spells("ext.stmt.class.builtin", kind) && (word == self.detail("allocate") || word == self.detail("name") || self.table.spells("ext.builtin.class.name", &word)) { return Ok(Value::Flag(true)); }
                }
                let (class, own) = match &v[0] {
                    Value::Complex(_) => (None, self.table.spells("ext.builtin.complex.real", &word) || self.table.spells("ext.builtin.complex.imag", &word)),
                    Value::Span(_) => (None, self.span_bound_named(&word).is_some()),
                    Value::Thing(o) => (Some(&o.of), self.member_place(&o.holds.borrow(), &word).is_some()),
                    Value::Blueprint(c) => (Some(c), false),
                    _ => (None, false),
                };
                let native = matches!(v[0], Value::Text(_)) && matches!(self.table.prims.get(&word), Some(Prim::Textual(work)) if *work != crate::text::Work::REPR);
                Value::Flag(native || (self.table.has_any("ext.builtin.exceptions") && matches!(&v[0], Value::Blueprint(_) | Value::Thing(_))) || matches!(v[0], Value::Member(..)) || own || (self.table.has_any("ext.stmt.class.special") && class.is_some()) || class.map_or(false, |c| c.keeper(&word).is_some() || c.program(&word).is_some() || c.constant(&word).is_some()))
            }
            Prim::Of => {
                n(2)?;
                let called = v[1].bare();
                if let Value::Complex(pair) = &v[0] {
                    if self.table.spells("ext.builtin.complex.real", &called) { return Ok(crate::complex::decimal_value(pair.0)); }
                    if self.table.spells("ext.builtin.complex.imag", &called) { return Ok(crate::complex::decimal_value(pair.1)); }
                    return Err(crate::complex::complaint(self.table, "unready"));
                }
                if matches!(v[0], Value::Member(..)) {
                    return Err(self.table.single("ext.stmt.class.unready").unwrap_or_default().to_owned());
                }
                if matches!(v[0], Value::Text(_)) && self.table.spells("ext.text.format", &called) {
                    return Err(self.table.single("ext.text.format.unready").unwrap_or_default().to_owned());
                }
                if self.table.flag("ext.op.member.pipes") {
                    if let Some(found) = self.attribute(&v[0], &called) { return Ok(found); }
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
                            // A namespace may answer for a name it has not,
                            // through the routine the table names for it.
                            None if self.table.single("ext.system.module.getattr").map_or(false, |word| thing.holds.borrow().iter().any(|(n, _)| n == word)) => {
                                let word = self.table.single("ext.system.module.getattr").unwrap_or_default();
                                let answerer = thing.holds.borrow().iter().find(|(n, _)| n == word).map(|(_, held)| match held { Value::Shared(cell) => cell.borrow().clone(), other => other.settled() });
                                match answerer {
                                    Some(routine @ (Value::Routine(_) | Value::Bound(..))) => self.apply_class_member(routine, vec![Value::text(&called)]).map_err(|fault| self.suspension_fault(fault))?,
                                    _ => return Err(format!("Undefined property: {}::${}", thing.of.name, called)),
                                }
                            }
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
                        if self.is_fault_kind(&thing.of) && self.table.single("ext.builtin.exceptions.args") == Some(called.as_str()) {
                            let contents = match &v[2] {
                                Value::Arguments(r) | Value::Vector(r) => Value::Arguments(r.clone()),
                                _ => return Err(self.argument_fault("ext.builtin.exceptions.unready", None)),
                            };
                            let mut places = thing.holds.borrow_mut();
                            for word in [called.as_str(), "\0raised-values"] {
                                if let Some(entry) = places.iter_mut().find(|(k, _)| k == word) { entry.1 = contents.clone(); }
                                else { places.push((word.into(), contents.clone())); }
                            }
                            return Ok(Value::Nil);
                        }
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
                    Value::Vector(items) | Value::Tuple(items) => {
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
                if matches!(v[0], Value::Tuple(_) | Value::Set(_)) { return Err(self.core_complaint("core.immutable", &v[0].kind_word())); }
                match &v[0] {
                    Value::Octets { cell, changeable, .. } => {
                        if !changeable { return Err(self.octet_error("immutable")); }
                        let index = self.octet_at(&v[1], cell.borrow().len())?;
                        let byte = self.octet_item(&v[2])?;
                        cell.borrow_mut()[index] = byte;
                        v[0].clone()
                    }
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
                    Value::Vector(items) | Value::Tuple(items) => {
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
            // The readers of text and the books of names are answered
            // before the arguments are opened, where the table spells
            // the manners text may be read in; spelled without them,
            // the words are refused here.
            Prim::WorldBook | Prim::HereBook | Prim::Perform | Prim::Prepare | Prim::Summon => return Err(self.source_refused()),
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
            Prim::FaultHeld => {
                if v.len() > 1 { return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()); }
                let held = v.first().cloned().or_else(|| self.holding_fault.last().cloned());
                let Some(fault) = held else { return Ok(Value::Vector(Rc::new(vec![Value::Nil, Value::Nil]))) };
                let kind = match &fault { Value::Thing(thing) => Value::text(&thing.of.name), _ => Value::Nil };
                let words = self.object_words(&fault, false)?;
                Value::Vector(Rc::new(vec![kind, Value::text(&words)]))
            }
            Prim::PathSort => {
                n(1)?;
                let w = self.wording();
                let named = v[0].render(w);
                let meta = std::fs::metadata(&named).ok();
                Value::Small(match meta { Some(m) if m.is_file() => 1, Some(m) if m.is_dir() => 2, _ => 0 })
            }
            Prim::HostRow => {
                n(0)?;
                let here = match std::env::current_dir() {
                    Ok(path) => path.to_str().map_or(Value::Nil, Value::text),
                    Err(_) => Value::Nil,
                };
                let mut surroundings = Vec::new();
                for (key, worth) in std::env::vars_os() {
                    if let (Some(key), Some(worth)) = (key.to_str(), worth.to_str()) { surroundings.push((Value::text(key), Value::text(worth))); }
                }
                let row = vec![here, Value::text(std::env::consts::OS), Value::text(std::env::consts::ARCH), Value::Dict(Rc::new(surroundings))];
                Value::Vector(Rc::new(row))
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
                        Prim::ClassMethods => class.methods.iter().map(|(called, _)| called.clone()).chain(class.shared.borrow().iter().filter(|(_,v)| matches!(v,Value::Routine(_) | Value::Bound(..) | Value::Adorned(_) | Value::Wrapped(..))).map(|(n,_)| n.clone())).collect(),
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
                // With one flag, where the table allows, the answer is a
                // real: seconds since the machine's own steady origin
                // when the flag stands, seconds since the epoch when not.
                if v.len() == 1 && self.table.flag("ext.builtin.clock.parts") {
                    let Value::Flag(steady) = v[0] else {
                        return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string());
                    };
                    let seconds = match steady {
                        true => {
                            static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                            ORIGIN.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
                        }
                        false => std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0),
                    };
                    let figures = self.table.count("ext.system.real.digits").unwrap_or(15);
                    let mut worth = crate::data::worth_of_binary(seconds, figures);
                    if let Value::Frac(ratio) = &mut worth { Rc::make_mut(ratio).float_style = true; }
                    return Ok(worth);
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
                    // A span names several places at once, and the rest
                    // close up in the order they stood.
                    Value::Vector(items) if matches!(&v[1], Value::Span(_)) && self.table.has_any("ext.builtin.slice") => {
                        let Value::Span(bounds) = &v[1] else { unreachable!() };
                        let bounds = self.span_settled(bounds)?;
                        let (_, picked, _) = self.span_selection(&bounds, items.len())?;
                        let retained = items.iter().enumerate().filter(|(at, _)| !picked.contains(at)).map(|(_, x)| x.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) if self.table.has_any("ext.stmt.del") => {
                        let offset = (match &v[1] { Value::Flag(b) => Some(if *b { 1 } else { 0 }), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None }).ok_or_else(|| self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string())?;
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string().into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, v)| v.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) | Value::Tuple(items) => {
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
                        if let Some(words) = self.cannot_key(&at) { return Err(words); }
                        let wanted = self.hash_key(&at)?;
                        if self.table.has_any("ext.stmt.del") && !self.key_held(entries, &wanted)? {
                            return Err(if self.table.has_any("ext.builtin.exceptions") { self.absent_key(&at) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() });
                        }
                        let kept: Vec<(Value, Value)> = self.without_key(entries, &wanted)?;
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
            Prim::BitsOver | Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne | Prim::BitsUp | Prim::BitsDown
                if self.table.flag("ext.op.bit.unbounded") && !(op == Prim::BitsEither && self.table.flag("ext.op.bit.or.maps") && matches!(v, [Value::Dict(_), Value::Dict(_)])) => self.all_bits(op, v)?,
            Prim::BitsOver => match &v[0] {
                Value::Text(s) => {
                    let over: Vec<u8> = self.table.raw_of(s).iter().map(|b| !b).collect();
                    Value::text(&self.table.said_of(&over))
                }
                other => Value::Small(!self.bits_told(other)?),
            },
            Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne
                if self.table.single("ext.op.bit.operands").is_some() && !(op == Prim::BitsEither && self.table.flag("ext.op.bit.or.maps") && matches!(v, [Value::Dict(_), Value::Dict(_)])) =>
            {
                let whole = |value: &Value| match value {
                    Value::Small(_) | Value::Huge(_) | Value::Flag(_) => value.as_big(),
                    _ => Err(self.table.single("ext.op.bit.operands").unwrap().to_owned()),
                };
                let mut answer = whole(&v[0])?;
                let rhs = whole(&v[1])?;
                if op == Prim::BitsOne { answer ^= rhs; }
                else if op == Prim::BitsBoth { answer &= rhs; }
                else { answer |= rhs; }
                match (&v[0], &v[1]) {
                    (Value::Flag(_), Value::Flag(_)) => Value::Flag(answer != BigInt::from(0)),
                    _ => Value::from_big(answer),
                }
            }
            // Two pieces of text meet letter by letter. The shorter one
            // says how far it goes, save where either bit will do, and
            // there the longer one carries on alone.
            Prim::BitsEither if self.table.flag("ext.op.bit.or.maps")
                && matches!(v, [Value::Dict(_), Value::Dict(_)]) => {
                return self.prim(Prim::ExtendLiteral(true, true), name, v);
            }
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
                        math::made_number(now.above.clone(), now.beneath.clone(), now.places, !was.under).keeping_point(was.pointed)
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
            Prim::Eq | Prim::Ne if self.table.flag("ext.op.eq.maps.unordered") => {
                Value::Flag(self.equal_contents(&v[0], &v[1]) != (op == Prim::Ne))
            }
            Prim::Membership => {
                n(2)?;
                let present = if let (Value::Text(needle), Value::Text(text)) = (&v[0], &v[1]) {
                    text.contains(needle.as_ref())
                } else if matches!(&v[1], Value::Text(_)) {
                    return Err(self.table.single("ext.syntax.collection.unwalkable").unwrap_or("Membership needs an iterable").to_string());
                } else {
                    self.gathered_members(&v[1])?.iter().any(|item| self.equal_contents(&v[0], item))
                };
                Value::Flag(present)
            }
            Prim::Eq => Value::Flag(v[0].equals(&v[1])),
            Prim::Ne => Value::Flag(!v[0].equals(&v[1])),
            Prim::Contains | Prim::Absent => {
                // A complex number whose imaginary part is nought is asked
                // for as the real number it equals.
                let sought = match &v[0] {
                    Value::Complex(pair) if pair.1 == 0.0 && !pair.0.is_nan() => crate::complex::decimal_value(pair.0),
                    other => other.clone(),
                };
                let present = match (&sought, &v[1]) {
                    (needle, Value::Progression(sequence)) => {
                        match needle.as_big().ok().filter(|whole| contained_equal(needle, &Value::from_big(whole.clone()))) {
                            None => false,
                            Some(whole) => {
                                let offset = &whole - &sequence.first;
                                let position = &offset / &sequence.stride;
                                (&offset % &sequence.stride) == BigInt::from(0)
                                    && position >= BigInt::from(0) && position < sequence.count()
                            }
                        }
                    },
                    (needle, Value::Vector(hay) | Value::Tuple(hay)) => hay.iter().any(|item| contained_equal(needle, item)),
                    (key, Value::Dict(entries)) => entries.iter().any(|(k, _)| contained_equal(key, k) || self.keys_match(key, k)),
                    (item, Value::Set(hay)) => hay.borrow().keys.contains(&self.hash_for_set(item)?),
                    (item, Value::Octets { cell, .. }) => {
                        let numbers = cell.borrow();
                        if let Value::Octets { cell: needle, .. } = item {
                            let needle = needle.borrow();
                            (0..=numbers.len()).any(|i| numbers[i..].starts_with(&needle))
                        } else { numbers.contains(&self.octet_item(item)?) }
                    }
                    (needle, Value::TextRow(words, _)) => words.iter().any(|s| needle.equals(&Value::text(s))),
                    (Value::Text(part), Value::Text(text)) => text.contains(part.as_ref()),
                    // An iterator gives up members until the one sought
                    // turns up, and stands after it thereafter.
                    (needle, walk @ Value::Iterator(_)) => {
                        let mut seen = false;
                        while let Some(item) = self.next_value(walk)? { if contained_equal(needle, &item) { seen = true; break; } }
                        seen
                    }
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
                    // Every value is itself: a small number by its worth,
                    // whatever lives behind a pointer by that pointer.
                    (Value::Small(n), Value::Small(m)) => n == m,
                    (Value::Huge(a), Value::Huge(b)) => Rc::ptr_eq(a, b),
                    (Value::Frac(a), Value::Frac(b)) => Rc::ptr_eq(a, b),
                    (Value::Text(a), Value::Text(b)) => Rc::ptr_eq(a, b),
                    (Value::Tuple(a), Value::Tuple(b)) => Rc::ptr_eq(a, b),
                    (Value::Span(a), Value::Span(b)) => Rc::ptr_eq(a, b),
                    (Value::Progression(a), Value::Progression(b)) => Rc::ptr_eq(a, b),
                    (Value::Iterator(a), Value::Iterator(b)) => Rc::ptr_eq(a, b),
                    (Value::Generator(a), Value::Generator(b)) => Rc::ptr_eq(a, b),
                    (Value::Refusal(_), Value::Refusal(_)) => true,
                    (Value::Intrinsic(a), Value::Intrinsic(b)) => a == b,
                    (Value::OctetKind { changeable: x, .. }, Value::OctetKind { changeable: y, .. }) => x == y,
                    (Value::Octets { cell: x, .. }, Value::Octets { cell: y, .. }) => Rc::ptr_eq(x, y),
                    (Value::Vector(a), Value::Vector(b)) => Rc::ptr_eq(a, b),
                    (Value::Set(a), Value::Set(b)) => Rc::ptr_eq(a, b),
                    (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
                    (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a,b),
                    (Value::Bound(a,_), Value::Bound(b,_)) => Rc::ptr_eq(a,b),
                    // Every read of a method ties it afresh: two reads are never one value.
                    (Value::Method(..), Value::Method(..)) => false,
                    (Value::Wrapped(k,a), Value::Wrapped(l,b)) => k==l && Rc::ptr_eq(a,b),
                    (Value::Thing(a), Value::Thing(b)) => Rc::ptr_eq(a, b),
                    (Value::Blueprint(a), Value::Blueprint(b)) => Rc::ptr_eq(a, b),
                    (Value::Nil, Value::Nil) | (Value::Ellipsis, Value::Ellipsis) => true,
                    (Value::Flag(a), Value::Flag(b)) => a == b,
                    _ if !v[0].selfsame(&v[1]) => false,
                    _ => return Err(self.table.single("ext.op.identical.unsupported").unwrap_or_default().to_string()),
                };
                Value::Flag(if op == Prim::Unlike { !identical } else { identical })
            }
            Prim::Selfsame => Value::Flag(v[0].selfsame(&v[1])),
            Prim::Unlike => Value::Flag(!v[0].selfsame(&v[1])),
            Prim::Join => Value::text(&format!("{}{}", v[0].render(w), v[1].render(w))),
            Prim::At if self.has_class_order() && matches!(&v[0],Value::Blueprint(_)) => {
                let Value::Blueprint(class)=&v[0] else{unreachable!()};
                if self.inherited_entry(class,self.detail("getitem")).is_none(){return Err(self.detail("unready").to_owned());}
                v[0].clone()
            }
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
            Prim::RenderField => {
                let result = if self.table.has_any("ext.builtin.format") {
                    crate::formatting::Layout { table: self.table, names: w }.present(&v[0], &v[1].bare(), &v[2].bare())?
                } else { v[0].in_field(w, &v[1].bare(), &v[2].bare()).ok_or_else(|| self.table.single("ext.lexical.string.format.unavailable").unwrap_or("This formatted value is not supported").to_owned())? };
                Value::text(&result)
            }
            Prim::FormatValue => {
                let layout = crate::formatting::Layout { table: self.table, names: w };
                if v.is_empty() || v.len() > 2 { return Err(self.table.single("ext.syntax.call.amiss").unwrap_or_default().to_owned()); }
                let spec = match v.get(1) {
                    Some(Value::Text(s)) => s.as_ref(),
                    Some(item) => return Err(layout.complain("ext.text.format.spec.type", &[layout.typename(item)])),
                    None => "",
                };
                Value::text(&layout.present(&v[0], spec, "")?)
            }
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
            Prim::Times if self.table.flag("ext.builtin.text.repeat") && v.iter().any(|x| matches!(x,Value::Text(_))) => {
                let (word, multiplier) = match (&v[0],&v[1]) { (Value::Text(t),x)|(x,Value::Text(t)) => (t,x), _ => unreachable!() };
                let number = match multiplier {
                    Value::Flag(b) => if *b { 1 } else { 0 }, Value::Small(i) => *i,
                    Value::Huge(i) => i.to_i64().unwrap_or_else(|| if **i < BigInt::from(0) { i64::MIN } else { i64::MAX }),
                    _ => return Err(crate::text::complaint(self.table,"integer").into()),
                };
                let mut repeated = String::new();
                if number > 0 && !word.is_empty() {
                    let length = (number as usize).checked_mul(word.len()).ok_or_else(||crate::text::complaint(self.table,"room"))?;
                    repeated.try_reserve(length).map_err(|_|crate::text::complaint(self.table,"room"))?;
                    for _ in 0..number { repeated.push_str(word); }
                }
                Value::text(&repeated)
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
                let binary = if self.table.flag("ext.op.arithmetic.binary") { math::binary_work(sum, &left, &right) } else { None };
                let worked = match binary.or_else(|| math::compute(sum, &left, &right)) {
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
            // The line comes from the reader the table names, so a
            // program that has put another source in the reader's place
            // is answered from that source.
            // One bound stops; two or three start, stop and step; what
            // is not given stands for nothing.
            Prim::SpanOf => {
                if v.is_empty() || v.len() > 3 { return Err(self.table.single("ext.builtin.slice.arity").unwrap_or_default().to_owned()); }
                let mut bounds = vec![Value::Nil; 3];
                match v.len() { 1 => bounds[1] = v[0].clone(), count => bounds[..count].clone_from_slice(v) }
                Value::Span(Rc::new(bounds))
            }
            Prim::Inquire => {
                let words = self.table.strings("ext.builtin.input.reader").to_vec();
                if words.len() != 2 { return Err(self.argument_fault("ext.builtin.stream.amiss", None)); }
                let namespace = self.namespace_for(&words[0])?;
                let reader = self.attribute(&namespace, &words[1]).ok_or_else(|| self.argument_fault("ext.builtin.stream.amiss", None))?;
                match self.apply_within(reader, v.to_vec())? {
                    line @ Value::Text(_) => line,
                    _ => return Err(self.argument_fault("ext.builtin.stream.amiss", None)),
                }
            }
            // Text and whether it is for the error channel; the count
            // of characters poured is answered.
            Prim::PourOut => {
                use std::io::Write;
                let (Some(Value::Text(content)), true) = (v.first(), v.len() == 2) else {
                    return Err(self.argument_fault("ext.builtin.stream.amiss", None));
                };
                if self.stands_true(&v[1]) {
                    let mut channel = std::io::stderr().lock();
                    let poured = channel.write_all(content.as_bytes()).and_then(|_| channel.flush());
                    if poured.is_err() { return Err(self.argument_fault("ext.builtin.stream.failed", None)); }
                } else {
                    self.utter(content);
                }
                Value::Small(content.chars().count() as i64)
            }
            // A count and whether to stop at a line's end. A count below
            // nought means all there is. Each character is drawn whole:
            // its first byte says how many more belong to it.
            Prim::DrawIn => {
                use std::io::Read;
                let (Some(Value::Small(limit)), true) = (v.first(), v.len() == 2) else {
                    return Err(self.argument_fault("ext.builtin.stream.amiss", None));
                };
                let (limit, stop_at_line) = (*limit, self.stands_true(&v[1]));
                let failed = || self.argument_fault("ext.builtin.stream.failed", None);
                let mut channel = std::io::stdin().lock();
                let mut drawn: Vec<u8> = Vec::new();
                let mut characters = 0;
                while limit < 0 || characters < limit {
                    let mut lead = [0u8];
                    match channel.read(&mut lead) {
                        Ok(0) => break,
                        Ok(_) => (),
                        Err(_) => return Err(failed()),
                    }
                    let more = match lead[0] { 0x00..=0x7f => 0, 0xc2..=0xdf => 1, 0xe0..=0xef => 2, 0xf0..=0xf4 => 3, _ => return Err(failed()) };
                    let mut rest = vec![0u8; more];
                    if channel.read_exact(&mut rest).is_err() { return Err(failed()); }
                    drawn.push(lead[0]);
                    drawn.extend(rest);
                    characters += 1;
                    if stop_at_line && lead[0] == b'\n' { break; }
                }
                Value::text(&String::from_utf8(drawn).map_err(|_| failed())?)
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
            Prim::Suspend | Prim::Delegate => return Err(self.generator_words("unsupported")),
            Prim::MakeTuple => Value::Tuple(Rc::new(v.to_vec())),
            Prim::Following => {
                if !(1..=2).contains(&v.len()) { return Err(self.generator_words("unsupported")); }
                let Value::Generator(state) = &v[0] else { return Err(self.table.single("ext.syntax.collection.unwalkable").unwrap_or_default().into()); };
                match self.resume(state, Value::Nil).map_err(|fault| self.suspension_fault(fault))? {
                    Some(item) => item,
                    None => v.get(1).cloned().ok_or_else(|| self.generator_words("exhausted"))?,
                }
            }
            Prim::Tupled => {
                if v.is_empty() { Value::Tuple(Rc::new(Vec::new())) }
                else { n(1)?; Value::Tuple(Rc::new(self.gathered_members(&v[0])?)) }
            }
            Prim::Belongs | Prim::Tupling | Prim::Uniques | Prim::Ordered | Prim::Backwards | Prim::Numbered | Prim::Zipped | Prim::Mapped | Prim::Filtered | Prim::EveryTrue | Prim::Least | Prim::Greatest | Prim::Magnitude | Prim::Rounded | Prim::QuotRem | Prim::Powered | Prim::Hexadecimal | Prim::Octal | Prim::Binary | Prim::Quoted | Prim::Truthful | Prim::CallableValue | Prim::IdentityOf | Prim::Hashed | Prim::Iterator | Prim::NextItem | Prim::HasAttribute | Prim::GetMember | Prim::SetMember | Prim::DropMember | Prim::MembersOf => unreachable!(),
            Prim::Listed => {
                match v.len() {
                    0 => Value::Vector(Rc::new(Vec::new())),
                    1 => {let result=Value::Vector(Rc::new(self.gathered_members(&v[0])?)); if matches!(v[0],Value::Window(..)|Value::Mutable(_,true)|Value::Text(_)|Value::Iterator(_)|Value::Generator(_)){result.keep(true)}else{result}},
                    _ => return Err(format!("{}() expects 1 argument, got {}", name, v.len())),
                }
            }
            Prim::SomeTrue => {
                n(1)?;
                if matches!(v[0], Value::Iterator(_)) {
                    while let Some(item) = self.next_value(&v[0])? { if self.stands_true(&item) { return Ok(Value::Flag(true)); } }
                    return Ok(Value::Flag(false));
                } else if self.table.flag("ext.stmt.yield.suspends") {
                    let Value::Generator(walk) = self.make_iterator(v[0].clone()).map_err(|fault| self.suspension_fault(fault))? else { unreachable!() };
                    loop {
                        match self.resume(&walk, Value::Nil).map_err(|fault| self.suspension_fault(fault))? {
                            None => return Ok(Value::Flag(false)),
                            Some(item) if self.stands_true(&item) => return Ok(Value::Flag(true)),
                            _ => {}
                        }
                    }
                }
                let members = self.gathered_members(&v[0])?;
                Value::Flag(members.iter().any(|item| self.stands_true(item)))
            }
            Prim::Total => {
                if !(1..=2).contains(&v.len()) { return Err(format!("{}() expects one or two arguments", name)); }
                if self.table.flag("ext.stmt.yield.suspends") {
                    let Value::Generator(walk) = self.make_iterator(v[0].clone()).map_err(|fault| self.suspension_fault(fault))? else { unreachable!() };
                    let counted = |value| if let Value::Flag(flag) = value { Value::Small(flag as i64) } else { value };
                    let mut answer = counted(v.get(1).cloned().unwrap_or(Value::Small(0)));
                    loop {
                        let item = self.resume(&walk, Value::Nil).map_err(|fault| self.suspension_fault(fault))?;
                        let Some(item) = item else { return Ok(answer) };
                        answer = math::compute(Calc::Plus, &answer, &counted(item)).unwrap_or_else(|| Err(self.table.single("ext.builtin.sum.non_number").unwrap_or_default().to_string()))?;
                    }
                }
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
            Prim::Iterate => {
                n(1)?;
                let source = v[0].clone();
                if let Value::Thing(thing) = &source {
                    if thing.holds.borrow().iter().any(|(k, _)| k == "\0walked") { return Ok(source); }
                }
                if !matches!(source, Value::Vector(_) | Value::Arguments(_) | Value::Dict(_) | Value::Text(_) | Value::Progression(_)) {
                    return Err(self.argument_fault("ext.builtin.exceptions.unready", None));
                }
                let kind = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None, name: name.into(), under: None, answers: vec![], reaches: vec![],
                    fields: vec![], shared: RefCell::new(vec![]), constants: vec![], methods: vec![] };
                self.made += 1;
                Value::Thing(Rc::new(Thing { of: Rc::new(kind), turn: self.made,
                    holds: RefCell::new(vec![("\0walked".into(), source), ("\0walk-step".into(), Value::Small(0))]) }))
            }
            Prim::NextOne => {
                if !(1..=2).contains(&v.len()) { return Err(self.argument_fault("ext.builtin.exceptions.unready", None)); }
                let refuse = || self.argument_fault("ext.builtin.exceptions.unready", None);
                let Value::Thing(thing) = &v[0] else { return Err(refuse()) };
                let mut held = thing.holds.borrow_mut();
                let source = held.iter().find(|(k, _)| k == "\0walked").ok_or_else(refuse)?.1.clone();
                let cursor = held.iter_mut().find(|(k, _)| k == "\0walk-step").ok_or_else(refuse)?;
                let Value::Small(position) = &mut cursor.1 else { return Err(refuse()) };
                let answer = match source {
                    Value::Progression(p) => p.item(&BigInt::from(*position)),
                    Value::Text(t) => t.chars().nth(*position as usize).map(|x| Value::text(&x.to_string())),
                    Value::Dict(d) => d.get(*position as usize).map(|entry| entry.0.clone()),
                    Value::Vector(r) | Value::Arguments(r) => r.get(*position as usize).cloned(),
                    _ => return Err(refuse()),
                };
                if let Some(answer) = answer { *position += 1; answer }
                else if let Some(fallback) = v.get(1) { fallback.clone() }
                else { return Err(format!("\0{}:", self.table.single("ext.system.fault.class.stop").unwrap_or_default())); }
            }
            Prim::Repr => {
                n(1)?;
                Value::text(&v[0].representation(self.wording()))
            }
            Prim::AsText => {
                n(1)?;
                self.figures_allowed(&v[0])?;
                Value::text(&v[0].render(w))
            }
            Prim::AsInt if matches!(v.first(), Some(Value::Complex(_))) => return Err(crate::complex::complaint(self.table, "integer")),
            Prim::AsInt if self.table.single("ext.builtin.to_int.base").is_some() => self.whole_from_call(v)?,
            Prim::AsInt => {
                n(1)?;
                let whole = math::whole_part(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::from_big(whole)
            }
            Prim::AsReal if self.table.flag("ext.builtin.to_real.text") && (v.is_empty() || matches!(v.first(), Some(Value::Text(_)))) => {
                if v.len() > 1 { return Err(self.argument_fault("ext.syntax.call.amiss", None)); }
                if let Some(Value::Text(source)) = v.first() {
                    let source = source.trim().to_ascii_lowercase();
                    let letters = source.strip_prefix(['+', '-']).unwrap_or(&source);
                    let special = if self.table.spells("ext.builtin.to_real.nan", letters) { Some(f64::NAN) }
                        else if self.table.spells("ext.builtin.to_real.infinity", letters) {
                            Some(if source.starts_with('-') { -f64::INFINITY } else { f64::INFINITY })
                        } else { None };
                    if let Some(real) = special { return Ok(crate::data::worth_of_binary(real, math::DEFAULT_PLACES)); }
                }
                let failure = || self.argument_fault("ext.builtin.to_real.text.amiss", None);
                // Figures may be grouped with a separator, which has to
                // stand between two of them; anywhere else it is a fault.
                let regrouped = match v.first() {
                    Some(Value::Text(chars)) => Some(Value::text(&crate::data::ungrouped_figures(chars, &self.table.letters("ext.lexical.number.separator")).ok_or_else(failure)?)),
                    _ => None,
                };
                let v: Vec<Value> = regrouped.into_iter().chain(v.iter().skip(1).cloned()).collect();
                if let Some(Value::Text(text)) = v.first() {
                    let lower = text.trim().to_ascii_lowercase();
                    let letters = lower.trim_start_matches(['+', '-']);
                    let sign_count = lower.len() - letters.len();
                    let infinity = letters == "inf" || letters == "infinity";
                    if sign_count <= 1 && (infinity || letters == "nan") {
                        let x = if !infinity { f64::NAN } else if lower.starts_with('-') { f64::NEG_INFINITY } else { f64::INFINITY };
                        return Ok(crate::data::past_the_numbers(x, math::DEFAULT_PLACES));
                    }
                    }
                if let Some(Value::Text(chars)) = v.first() {
                    if let Ok(binary) = chars.trim().to_ascii_lowercase().parse::<f64>() {
                        if self.table.lone("system.real.render") == Some("shortest") {
                            return Ok(crate::data::worth_of_binary(binary, math::DEFAULT_PLACES));
                        }
                        if !binary.is_finite() { return Ok(crate::data::past_the_numbers(binary, math::DEFAULT_PLACES)); }
                    }
                }
                let worth = if v.is_empty() { Value::Small(0) } else { number_spelled_in(&v[0]).ok_or_else(failure)? };
                self.at_width(math::to_decimal(&worth, math::DEFAULT_PLACES).ok_or_else(failure)?)
            }
            Prim::AsReal => {
                n(1)?;
                match &v[0] {
                    x @ Value::Frac(e) if e.places.is_some() => x.clone(),
                    x => self.at_width(math::to_decimal(x, math::DEFAULT_PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?),
                }
            }
            Prim::Length => {
                n(1)?;
                if let Some(held) = self.check_set_walk(&v[0])? { return Ok(Value::Small(held.borrow().entries.len() as i64)); }
                match &v[0] {
                    Value::Octets { cell, .. } => Value::Small(cell.borrow().len() as i64),
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Vector(l) | Value::Tuple(l) => Value::Small(l.len() as i64),
                    Value::Set(members) => Value::Small(members.borrow().keys.len() as i64),
                    Value::TextRow(words, _) => Value::Small(words.len() as i64),
                    Value::Dict(entries) => Value::Small(entries.len() as i64),
                    Value::Progression(p) => Value::from_big(p.count()),
                    measureless => return Err(match self.table.strings("ext.builtin.core.unsized") {
                        [before, after] => format!("{}{}{}", before, measureless.kind_word(), after),
                        _ => format!("{}() requires a string or array argument", name),
                    }),
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
                if self.table.has_any("ext.builtin.isinstance") {
                    if let Value::Thing(t) = &v[0] { return Ok(Value::Blueprint(t.of.clone())); }
                    let wanted = match &v[0] {
                        Value::Complex(_) => Some(Prim::ComplexMade),
                        Value::Text(_) => Some(Prim::AsText), Value::Flag(_) => Some(Prim::Truthful),
                        Value::Vector(_) => Some(Prim::Listed), Value::Dict(_) => Some(Prim::Dictionary),
                        Value::Set(_) => Some(Prim::Uniques), Value::Tuple(_) => Some(Prim::Tupling),
                        Value::Small(_) | Value::Huge(_) => Some(Prim::AsInt), Value::Frac(_) => Some(Prim::AsReal), _ => None,
                    };
                    if let Some(operation) = wanted {
                        if let Some((word,_)) = self.table.prims.iter().find(|(_,p)| **p == operation) { return Ok(Value::Intrinsic(Rc::from(word.as_str()))); }
                    }
                }
                if let Value::Octets { changeable, .. } = &v[0] { return Ok(self.octet_type(*changeable)); }
                // A trace is of no kind the core knows either; where the
                // table names one, a blueprint of that name is the answer.
                if let (Value::Backtrace(_), Some(word)) = (&v[0], self.table.single("ext.builtin.exceptions.traceback")) {
                    return Ok(Value::Blueprint(Rc::new(Blueprint {
                        ancestry: vec![], parents: vec![], presentation: None, name: word.to_string(),
                        fields: Vec::new(), methods: Vec::new(), constants: Vec::new(),
                        shared: RefCell::new(Vec::new()), reaches: Vec::new(), under: None, answers: Vec::new(),
                    })));
                }
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
            Prim::Octets(_) | Prim::Seq | Prim::Choose | Prim::Both | Prim::Either | Prim::Yield | Prim::Leave | Prim::Resume | Prim::Append | Prim::Replace
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
        if matches!(target, Value::Mutable(..) | Value::Window(..)) { return self.element(&target.settled(), at, how); }
        if let Some(store) = self.check_set_walk(target)? {
            let position = as_index(at)?;
            return store.borrow().entries.get(position).map(|(_, item)| item.clone()).ok_or_else(|| self.set_complaint("missing", &position.to_string()));
        }
        if self.table.has_any("ext.builtin.exceptions") {
            if let Value::Dict(entries) = target {
                if let Some(words) = self.cannot_key(at) { return Err(words); }
                if !entries.iter().any(|entry| self.keys_match(&entry.0, at)) { return Err(self.absent_key(at)); }
            }
        }
        if let Value::Arguments(row) = target {
            let place = if let Value::Small(n) = at {
                let n = if *n < 0 { row.len() as i128 + i128::from(*n) } else { i128::from(*n) };
                usize::try_from(n).ok()
            } else { as_index(at).ok() };
            return place.and_then(|p| row.get(p)).cloned().ok_or_else(|| format!("\0{}: {}", self.table.single("ext.system.fault.class.index").unwrap_or_default(), self.table.single("ext.system.fault.index").unwrap_or_default()));
        }
        if let Value::Tuple(values)=target {
            let selected=if let Value::Span(bounds)=at {
                let (_,places,_)=self.span_selection(bounds,values.len())?;
                Value::Tuple(Rc::new(places.into_iter().map(|i|values[i].clone()).collect()))
            }else{
                let index=at.as_big()?;let index=if index<BigInt::from(0){index+values.len()}else{index};
                index.to_usize().and_then(|i|values.get(i)).cloned().ok_or_else(||self.detail("unready").to_owned())?
            };
            return Ok(selected);
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
            Value::Shared(cell) if !self.table.flag("ext.syntax.call.bind_names") => cell.borrow().clone(),
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
        if let Value::Shared(cell) = handed {
            let contents = cell.borrow().clone();
            return self.span_written(held, bounds, &contents);
        }
        if let Value::Mutable(cell, _) = held { return self.span_written(&mut cell.borrow_mut(), bounds, handed); }
        let settled = handed.settled();
        let handed = &settled;
        if let Value::Octets { cell, changeable, .. } = held {
            if !*changeable { return Err(self.octet_error("immutable")); }
            let incoming = self.octet_contents(handed, true)?;
            let (range, positions, contiguous) = self.span_selection(bounds, cell.borrow().len())?;
            if contiguous { cell.borrow_mut().splice(range, incoming); }
            else {
                if positions.len() != incoming.len() { return Err(self.octet_error("arguments")); }
                for (index, byte) in positions.into_iter().zip(incoming) { cell.borrow_mut()[index] = byte; }
            }
            return Ok(());
        }
        let Value::Vector(row) = held else { return Err(self.span_complaint("unsupported")) };
        let (span, picked, unit) = self.span_selection(bounds, row.len())?;
        let coming: Vec<Value> = match handed {
            Value::Set(contents) => contents.borrow().values(),
            Value::Progression(walk) => (0..).map_while(|place| walk.item(&BigInt::from(place))).collect(),
            Value::Text(letters) => letters.chars().map(|c| Value::text(&c.to_string())).collect(),
            Value::Dict(entries) => entries.iter().map(|entry| match &entry.0 { Value::Keyed(v, _) => v.as_ref().clone(), key => key.clone() }).collect(),
            Value::Vector(values) | Value::Tuple(values) => values.iter().cloned().collect(),
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
        if matches!(target, Value::Set(_)) { return Err(self.core_complaint("core.unindexable", &target.kind_word())); }
        if let Value::Octets { cell, changeable, .. } = target {
            let numbers = cell.borrow();
            return match at {
                Value::Span(bounds) => {
                    let (_, chosen, _) = self.span_selection(bounds, numbers.len())?;
                    Ok(self.octets(chosen.iter().map(|&i| numbers[i]).collect(), *changeable))
                }
                index => Ok(Value::Small(i64::from(numbers[self.octet_at(index, numbers.len())?]))),
            };
        }
        if self.table.flag("ext.op.index.text.negative") && !matches!(at,Value::Span(_)) {
            if let Value::Text(word) = target {
                let number = match at {
                    Value::Flag(f) => if *f {1} else {0}, Value::Small(i) => *i,
                    Value::Huge(i) => i.to_i64().ok_or_else(||crate::text::complaint(self.table,"index"))?,
                    _ => return Err(crate::text::complaint(self.table,"integer")),
                };
                let place = if number < 0 {(word.chars().count() as i64).saturating_add(number)} else {number};
                return word.chars().nth(place as usize).map(|c|Value::text(&String::from(c))).ok_or_else(||crate::text::complaint(self.table,"index"));
            }
        }
        if let Value::TextRow(row, closed) = target {
            if let Value::Span(bounds) = at {
                let (_, selection, _) = self.span_selection(bounds, row.len())?;
                let words = selection.iter().map(|&i| row[i].clone()).collect();
                return Ok(Value::TextRow(Rc::new(words), *closed));
            }
            let offset = match at { Value::Small(i) => *i, Value::Flag(f) => if *f {1} else {0}, _ => return Err(crate::text::complaint(self.table,"integer")) };
            let place = if offset >= 0 {offset} else {(row.len() as i64).saturating_add(offset)};
            return match row.get(place as usize) {Some(word) => Ok(Value::text(word)),None => Err(self.span_complaint("bounds"))};
        }
        if let Value::Span(bounds) = at {
            let row = match target {
                Value::Vector(values) | Value::Tuple(values) => values.as_ref().clone(),
                Value::Text(text) if self.table.flag("op.index.strings") => {
                    text.chars().map(|letter| Value::text(&letter.to_string())).collect()
                }
                _ => return Err(self.span_complaint("unsupported")),
            };
            let (_, picked, _) = self.span_selection(bounds, row.len())?;
            let selected: Vec<Value> = picked.iter().map(|&i| row[i].clone()).collect();
            return Ok(if matches!(target, Value::Text(_)) {
                Value::text(&selected.iter().map(Value::bare).collect::<String>())
            } else if matches!(target, Value::Tuple(_)) {
                Value::Tuple(Rc::new(selected))
            } else { Value::Vector(Rc::new(selected)) });
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
            Value::Vector(l) | Value::Tuple(l) => match l.get(i) {
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

    fn begin_set_walk(&self, value: &Value) -> Option<Value> {
        if let Value::SetCursor { .. } = value { return Some(value.clone()); }
        match value {
            Value::Set(store) => Some(Value::SetCursor { source: store.clone(), count: store.borrow().entries.len() }),
            _ => None,
        }
    }

    /// Each step asks whether the size still agrees with the first one.
    fn check_set_walk(&self, value: &Value) -> Result<Option<Rc<RefCell<crate::data::SetStore>>>, String> {
        let Value::SetCursor { source, count } = value else { return Ok(None) };
        if source.borrow().keys.len() == *count { Ok(Some(source.clone())) }
        else { Err(self.set_complaint("changed", "")) }
    }

    fn set_complaint(&self, suffix: &str, insert: &str) -> String {
        let label = format!("ext.builtin.set.{suffix}");
        let parts = self.table.strings(&label);
        let mut words = parts.first().cloned().unwrap_or_default();
        words.push_str(insert);
        if let Some(end) = parts.get(1) { words.push_str(end); }
        words
    }

    fn hash_for_set(&self, item: &Value) -> Result<String, String> {
        match item.hash_address() {
            Ok(address) => Ok(address),
            Err("") => Err(self.set_complaint("unsupported", "")),
            Err(kind) => Err(self.set_complaint("unhashable", kind)),
        }
    }

    fn gather_set(&mut self, source: Option<&Value>) -> Result<crate::data::SetStore, String> {
        let mut gathered = crate::data::SetStore::new(self.table.single("ext.builtin.set").unwrap_or(""));
        if let Some(source) = source {
            for item in self.gathered_members(source)? { gathered.put(self.hash_for_set(&item)?, item); }
        }
        Ok(gathered)
    }

    fn work_set(&mut self, which: u8, values: &[Value]) -> Result<Value, String> {
        let wrong = || self.set_complaint("arguments", "");
        if which == 0 {
            if values.len() > 1 { return Err(wrong()); }
            return self.gather_set(values.first()).map(|items| Value::Set(Rc::new(RefCell::new(items))));
        }
        if which == 18 {
            if values.len() != 1 { return Err(wrong()); }
            let mut row = self.gathered_members(&values[0])?;
            let mut fault = false;
            row.sort_by(|a, b| {
                if let (Value::Text(x), Value::Text(y)) = (a, b) { return x.cmp(y); }
                let quantity = |v: &Value| match v {
                    Value::Flag(yes) => math::ratio_of(&Value::Small(if *yes { 1 } else { 0 })),
                    _ => math::ratio_of(v),
                };
                match (quantity(a), quantity(b)) {
                    (Some(x), Some(y)) if !x.past_numbers() && !y.past_numbers() => (x.above * y.beneath).cmp(&(y.above * x.beneath)),
                    _ => { fault = true; std::cmp::Ordering::Equal }
                }
            });
            if fault { return Err(self.set_complaint("unsortable", "")); }
            return Ok(Value::Vector(Rc::new(row)));
        }
        let Some(Value::Set(target)) = values.first() else { return Err(self.set_complaint("operands", "")); };
        match which {
            4..=6 if values.len() != 1 => return Err(wrong()),
            1..=3 | 11..=14 | 17 if values.len() != 2 => return Err(wrong()),
            _ => (),
        }
        if which == 7 {
            for input in values.iter().skip(1) {
                let additions = self.gathered_members(input)?;
                for value in additions {
                    let address = self.hash_for_set(&value)?;
                    target.borrow_mut().put(address, value);
                }
            }
            return Ok(Value::Nil);
        }
        match which {
            1..=3 => {
                let address = self.hash_for_set(&values[1])?;
                if which == 1 { target.borrow_mut().put(address, values[1].clone()); }
                else {
                    let taken = target.borrow_mut().take(&address);
                    if taken.is_none() && which == 2 {
                        let member = values[1].set_member_spelling(self.wording());
                        return Err(self.set_complaint("missing", &member));
                    }
                }
                Ok(Value::Nil)
            }
            4 => {
                let first = target.borrow().entries.first().map(|(k, _)| k.clone());
                match first {
                    Some(address) => Ok(target.borrow_mut().take(&address).unwrap()),
                    None => Err(self.set_complaint("empty", "")),
                }
            }
            5 => {
                let mut contents = target.borrow_mut();
                contents.entries.clear();
                contents.keys.clear();
                Ok(Value::Nil)
            }
            _ => {
                let mut answer = target.borrow().clone();
                for source in values.iter().skip(1) {
                    let operand = self.gather_set(Some(source))?;
                    let truth = match which {
                        12 => Some(answer.keys.is_subset(&operand.keys)),
                        13 => Some(answer.keys.is_superset(&operand.keys)),
                        14 => Some(answer.keys.is_disjoint(&operand.keys)),
                        _ => None,
                    };
                    if let Some(truth) = truth { return Ok(Value::Flag(truth)); }
                    let operation = match which { 9 | 15 => 1, 10 | 16 => 2, 11 | 17 => 3, _ => 0 };
                    answer = answer.merge(&operand, operation);
                }
                if matches!(which, 7 | 15..=17) {
                    *target.borrow_mut() = answer;
                    Ok(Value::Nil)
                } else { Ok(Value::Set(Rc::new(RefCell::new(answer)))) }
            }
        }
    }

    /// What walking a blueprint yields, through the program it holds for
    /// that, or nothing where it holds none.
    fn blueprint_walk(&mut self, kind: &Rc<Blueprint>) -> Result<Option<Value>, String> {
        let Some(handing) = self.table.single("ext.stmt.class.walked").and_then(|word| self.inherited_entry(kind, word)) else { return Ok(None) };
        self.apply_within(handing, vec![Value::Blueprint(kind.clone())]).map(Some)
    }

    fn gathered_members(&mut self, source: &Value) -> Result<Vec<Value>, String> {
        if let Some(under) = self.underlying_unless(source, &[15]) { return self.gathered_members(&under); }
        if let Value::Blueprint(kind) = source {
            if let Some(yielded) = self.blueprint_walk(&kind.clone())? { return self.gathered_members(&yielded); }
        }
        Ok(match source {
            Value::Octets { cell, .. } => cell.borrow().iter().copied().map(|n| Value::Small(i64::from(n))).collect(),
            Value::Text(word) => word.chars().map(|letter| Value::text(&String::from(letter))).collect(),
            Value::Generator(state) => {
                let mut members = Vec::new();
                while let Some(item) = self.resume(state, Value::Nil).map_err(|fault| self.suspension_fault(fault))? { members.push(item); }
                members
            }
            Value::Row(values) | Value::Tuple(values) | Value::Vector(values) => values.to_vec(),
            Value::Mutable(cell, _) => return self.gathered_members(&cell.borrow()),
            Value::Window(..) => return self.gathered_members(&source.settled()),
            Value::Iterator(_) => return self.core_collect(source),
            Value::Set(items) => items.borrow().values(),
            Value::Dict(entries) => entries.iter().map(|entry| match &entry.0 { Value::Keyed(v, _) => v.as_ref().clone(), key => key.clone() }).collect(),
            Value::TextRow(words, _) => words.iter().map(|s| Value::text(s)).collect(),
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
        let argument = |x: &Value| {
            let text = x.render(w);
            match (self.table.flag("ext.builtin.print.real_point"), x.point_kept()) {
                (true, true) if text.trim_start_matches('-').bytes().all(|c| c.is_ascii_digit()) => format!("{}.0", text),
                _ => text,
            }
        };
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
                        Some(x) => out.push_str(&argument(x)),
                        None => out.push_str(&s[p..p + k]),
                    }
                    s = &s[p + k..];
                }
                out.push_str(s);
                return out;
            }
        }
        v.iter().map(argument).collect::<Vec<_>>().join(" ")
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
        Value::Vector(items) | Value::Tuple(items) => {
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
        Value::Vector(items) | Value::Tuple(items) => ("Array".to_string(), items.iter().enumerate().map(|(at, x)| (at.to_string(), x)).collect()),
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
        Value::Vector(items) | Value::Tuple(items) => match items.get(stood).map_or(false, &itself) {
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
        Value::Vector(items) | Value::Tuple(items) => all.extend(items.iter().map(|x| (afresh(None), x.clone()))),
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

fn written_into(held: &mut Value, key: Option<Value>, value: Value, no_places: &str, builds: bool, letter: Option<String>, cells_are_places: bool) -> Result<bool, String> {
    if let Value::Mutable(cell, _) = held { return written_into(&mut cell.borrow_mut(), key, value, no_places, builds, letter, cells_are_places); }
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
        if let Some(k) = key.as_ref().filter(|_| cells_are_places) {
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
    if cells_are_places {
        set_key(entries, key, value);
    } else if let Some(entry) = entries.iter_mut().find(|entry| entry.0.equals(&key)) {
        entry.1 = value;
    } else { entries.push((key, value)); }
    Ok(false)
}

/// The place an array holds, made a shared cell, so that a name tied to
/// it writes into the array itself. A walk counts places, so a map is
/// reached by its position as a vector is.
fn shared_item(held: &mut Value, at: &Value) -> Result<Rc<RefCell<Value>>, String> {
    if let Value::Mutable(cell, _) = held { return shared_item(&mut cell.borrow_mut(), at); }
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
        Value::Vector(items) | Value::Tuple(items) => {
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
    if let Value::Mutable(cell, _) = held { return shared_deep(&mut cell.borrow_mut(), keys, makes); }
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
        Value::Vector(items) | Value::Tuple(items) => {
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

fn collection_read(held: &Value) -> Value {
    if let Value::Shared(inside) = held {
        inside.borrow().clone()
    } else {
        held.clone()
    }
}

/// Try a case without touching any cell until all its parts have agreed.
fn fit_case(test: &crate::form::CaseTest, value: &Value, tuple: bool) -> Result<Option<HashMap<String, Value>>, ()> {
    use crate::form::CaseTest;
    let mut gathered = HashMap::new();
    if tuple && matches!(test, CaseTest::Keep(_) | CaseTest::Also { .. }) { return Err(()); }
    match test {
        CaseTest::Ignore => {}
        CaseTest::Keep(name) => { gathered.insert(name.clone(), value.clone()); }
        CaseTest::Equal(wanted) => {
            let equal = match (wanted, value) {
                (Value::Nil | Value::Flag(_), _) => wanted.selfsame(value),
                (_, Value::Flag(flag)) => wanted.equals(&Value::Small(if *flag { 1 } else { 0 })),
                _ => wanted.equals(value),
            };
            if !equal { return Ok(None); }
        }
        CaseTest::Pending(_) => return Err(()),
        CaseTest::AnyOf(choices) => {
            for next in choices {
                let answer = fit_case(next, value, tuple)?;
                if answer.is_some() { return Ok(answer); }
            }
            return Ok(None);
        }
        CaseTest::Also { test: within, name } => {
            let Some(found) = fit_case(within, value, tuple)? else { return Ok(None); };
            gathered = found;
            gathered.insert(name.clone(), value.clone());
        }
        CaseTest::Series { members, spread } => {
            if let Value::Shared(cell) = value {
                return fit_case(test, &cell.borrow(), tuple);
            }
            let Value::Vector(values) = value else { return Ok(None); };
            let minimum = if spread.is_some() { members.len() - 1 } else { members.len() };
            if values.len() < minimum { return Ok(None); }
            if spread.is_none() && minimum != values.len() { return Ok(None); }
            let mut position = 0;
            for (ordinal, member) in members.iter().enumerate() {
                let next = match spread {
                    Some(star) if ordinal == *star => {
                        let end = values.len() - (members.len() - ordinal - 1);
                        let portion = Value::Vector(Rc::new(values[position..end].to_vec()));
                        position = end;
                        portion
                    }
                    _ => {
                        let portion = values[position].clone();
                        position += 1;
                        portion
                    }
                };
                match fit_case(member, &next, false)? {
                    None => return Ok(None),
                    Some(found) => gathered.extend(found),
                }
            }
        }
    }
    Ok(Some(gathered))
}


/// A container's comparison keeps a shared nonreflexive item findable.
fn contained_equal(near: &Value, far: &Value) -> bool {
    match near {
        Value::Shared(storage) => return contained_equal(&storage.borrow(), far),
        _ => {},
    }
    match far {
        Value::Shared(storage) => return contained_equal(near, &storage.borrow()),
        _ => {},
    }
    if let Value::Flag(bit) = near { return contained_equal(&Value::Small(if *bit { 1 } else { 0 }), far); }
    if let Value::Flag(bit) = far { return contained_equal(near, &Value::Small(if *bit { 1 } else { 0 })); }
    match (near, far) {
        (Value::Frac(x), Value::Frac(y)) if Rc::ptr_eq(x, y) => return true,
        (Value::Vector(left), Value::Vector(right)) => {
            if Rc::ptr_eq(left, right) { return true; }
            if left.len() != right.len() { return false; }
            return left.iter().zip(right.iter()).all(|(l, r)| contained_equal(l, r));
        }
        (Value::Dict(left), Value::Dict(right)) => {
            if Rc::ptr_eq(left, right) { return true; }
            if left.len() != right.len() { return false; }
            for (key, value) in left.iter() {
                if !right.iter().any(|(k, v)| contained_equal(key, k) && contained_equal(value, v)) { return false; }
            }
            return true;
        }
        // A tuple and a pair read out of a map are one when their items are.
        (Value::Tuple(left) | Value::Row(left), Value::Tuple(right) | Value::Row(right)) => {
            return left.len() == right.len() && left.iter().zip(right.iter()).all(|(l, r)| contained_equal(l, r));
        }
        _ => {}
    }
    near.equals(far)
}

fn unwrapped_arm(form: Form) -> Form {
    match form {
        Form::Const(Value::Routine(program)) if program.frameless => program.body.clone(),
        other => other,
    }
}

fn suspension_within(form: &Form) -> bool {
    match form {
        Form::Apply(Callee::Prim(Prim::Suspend | Prim::Delegate, _), _) => true,
        Form::Apply(Callee::Prim(_, _), args) => args.iter().any(suspension_within),
        Form::Apply(Callee::Code(target), args) => suspension_within(target) || args.iter().any(suspension_within),
        Form::Const(Value::Routine(body)) if body.frameless => suspension_within(&body.body),
        Form::Write(_, inner) | Form::OnLine(_, inner) | Form::Muted(inner) | Form::Silenced(inner) => suspension_within(inner),
        Form::Cycle { test, body, step, otherwise, .. } => suspension_within(test) || suspension_within(body)
            || step.as_deref().map_or(false, suspension_within) || otherwise.as_deref().map_or(false, suspension_within),
        Form::Attempt { body, clauses, last, otherwise, .. } => suspension_within(body)
            || clauses.iter().any(|c| suspension_within(&c.body) || c.choices.as_ref().map_or(false, |parts| parts.iter().any(suspension_within)))
            || last.as_deref().map_or(false, suspension_within) || otherwise.as_deref().map_or(false, suspension_within),
        Form::Assert { condition, message } => suspension_within(condition) || suspension_within(message),
        Form::Dyad { a, b, .. } => [a, b].iter().any(|input| matches!(input, Input::Form(f) if suspension_within(f))),
        _ => false,
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
                    true => Value::text(path), false => self.fault_kinds.get(name).cloned().unwrap_or_else(|| if self.table.prims.contains_key(name) { Value::Intrinsic(Rc::from(name.as_str())) } else { Value::Unset }),
                };
                let link = Value::Shared(Rc::new(RefCell::new(initial)));
                members.push((name.clone(), link.clone()));
                world[beginning + position] = link;
            }
        }
        for word in self.table.strings("ext.system.module.name") {
            if !members.iter().any(|(name, _)| name == word) { members.push((word.clone(), Value::text(path))); }
        }
        let kind = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
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
        let (head, tail) = self.table.around("ext.stmt.import.member.missing").unwrap_or(("", ""));
        Err(format!("{head}{wanted}{tail}"))
    }
}

/// A text read into dictionaries handed over. Its names have slots of
/// their own among the outermost cells, yet what they hold lives in the
/// dictionaries: the near one first; then the outer one, which also
/// takes whatever the text declared global.
struct Namebook {
    near: Rc<RefCell<Value>>,
    outer: Option<Rc<RefCell<Value>>>,
    from: usize,
    upto: usize,
    declared: Vec<String>,
}

/// Which dictionary a slot's value lives in, when it lives in one.
#[derive(Clone, Copy)]
enum Held {
    World,
    Reading(usize),
}

/// Whether a dictionary's key is this name, however it is wrapped.
fn spells_key(key: &Value, name: &str) -> bool {
    match key {
        Value::Text(word) => word.as_ref() == name,
        Value::Keyed(inner, _) => spells_key(inner, name),
        Value::Shared(cell) => spells_key(&cell.borrow(), name),
        _ => false,
    }
}

/// The value a dictionary in a cell keeps under a name.
fn looked_up(book: &Rc<RefCell<Value>>, name: &str) -> Option<Value> {
    let Value::Dict(entries) = &*book.borrow() else { return None };
    entries.iter().find(|(key, _)| spells_key(key, name)).map(|(_, worth)| worth.clone())
}

/// Set a value down under a name in such a dictionary; given nothing,
/// strike the name out.
fn set_down(book: &Rc<RefCell<Value>>, name: &str, worth: Option<Value>) {
    let Value::Dict(entries) = &mut *book.borrow_mut() else { return };
    let entries = Rc::make_mut(entries);
    let place = entries.iter().position(|(key, _)| spells_key(key, name));
    match (place, worth) {
        (Some(place), Some(worth)) => entries[place].1 = worth,
        (Some(place), None) => { entries.remove(place); }
        (None, Some(worth)) => entries.push((Value::text(name), worth)),
        (None, None) => {}
    }
}

impl<'a> Machine<'a> {
    /// Whether the table spells the manners text may be read in, which
    /// is what brings the readers of text and the books of names alive.
    fn reads_manners(&self) -> bool {
        self.table.has_any("ext.builtin.compile.modes")
    }

    /// A name the program can write, rather than a cell of the builder's.
    fn visible_name(word: &str) -> bool {
        !word.contains('\0') && !word.starts_with('#')
    }

    /// The dictionary an outermost slot lives in, if any: that of the
    /// reading it belongs to, else the world's once that has been asked for.
    fn book_holding(&self, at: usize) -> Option<Held> {
        if self.world_book.is_none() && self.readings.is_empty() { return None; }
        let ident = self.idents.get(at)?;
        let bare = ident.rsplit('/').next().unwrap_or(ident);
        if !Self::visible_name(bare) { return None; }
        if let Some(which) = self.readings.iter().position(|book| (book.from..book.upto).contains(&at)) { return Some(Held::Reading(which)); }
        if self.world_book.is_some() && Self::visible_name(ident) { return Some(Held::World); }
        None
    }

    /// Whether the world's dictionary passed this name over when it was
    /// made — a builtin under its own name, a native kind, or a cell
    /// never written — so that it goes on being read from the cell.
    fn passed_over(&self, name: &str, held: &Value) -> bool {
        match held {
            Value::Intrinsic(word) => word.as_ref() == name,
            Value::Blueprint(kind) => kind.name == name && (self.fault_kinds.contains_key(name) || name == self.detail("root")),
            Value::KindOf(_) | Value::Unset => true,
            _ => false,
        }
    }

    /// What a builtin word names: a fault kind, a native operation, or
    /// the root class.
    fn native_of(&self, name: &str) -> Option<Value> {
        if let Some(kind) = self.fault_kinds.get(name) { return Some(kind.clone()); }
        if self.table.prims.contains_key(name) { return Some(Value::Intrinsic(Rc::from(name))); }
        if self.has_class_order() && name == self.detail("root") { return self.ancestor.clone().map(Value::Blueprint); }
        None
    }

    /// Read a slot's name out of the dictionary it lives in; nothing
    /// means the cell itself is to be read after all.
    fn booked_read(&self, at: usize, name: &str) -> Option<Result<Value, String>> {
        match self.book_holding(at)? {
            Held::World => {
                let book = self.world_book.as_ref()?;
                if let Some(worth) = looked_up(book, name) { return Some(Ok(worth)); }
                let cell = self.outermost.cells.borrow()[at].clone();
                if self.passed_over(name, &cell) { return None; }
                Some(Err(format!("Undefined variable: {}", name)))
            }
            Held::Reading(which) => {
                let book = &self.readings[which];
                if let Some(worth) = looked_up(&book.near, name) { return Some(Ok(worth)); }
                if let Some(worth) = book.outer.as_ref().and_then(|outer| looked_up(outer, name)) { return Some(Ok(worth)); }
                // A builtin is reached through the dictionary of builtins
                // the outer dictionary names, so a program may hand over
                // one of its own and so choose what the text can reach.
                let roots = book.outer.as_ref().unwrap_or(&book.near);
                let found = match self.table.single("ext.system.module.builtins").and_then(|word| looked_up(roots, word)) {
                    Some(Value::Shared(natives)) => looked_up(&natives, name),
                    Some(_) => None,
                    None => self.native_of(name),
                };
                Some(found.ok_or_else(|| format!("Undefined variable: {}", name)))
            }
        }
    }

    /// Write a slot's name into the dictionary it lives in, or strike it out.
    fn booked_write(&self, at: usize, name: &str, worth: Option<Value>) {
        match self.book_holding(at) {
            Some(Held::World) => if let Some(book) = &self.world_book { set_down(book, name, worth) },
            Some(Held::Reading(which)) => {
                let book = &self.readings[which];
                let goes_to = match &book.outer {
                    Some(outer) if book.declared.iter().any(|word| word == name) => outer,
                    _ => &book.near,
                };
                set_down(goes_to, name, worth);
            }
            None => {}
        }
    }

    /// The dictionary of every builtin word, made once.
    fn natives_kept(&mut self) -> Rc<RefCell<Value>> {
        if let Some(book) = &self.natives_book { return book.clone(); }
        let mut words: Vec<&String> = self.table.prims.keys().chain(self.table.strings("ext.builtin.exceptions")).collect();
        words.sort();
        words.dedup();
        let entries: Vec<(Value, Value)> = words.into_iter().filter_map(|word| self.native_of(word).map(|worth| (Value::text(word), worth))).collect();
        let book = Rc::new(RefCell::new(Value::Dict(Rc::new(entries))));
        self.natives_book = Some(book.clone());
        book
    }

    /// The world's dictionary, made from the outermost cells the first
    /// time it is wanted and kept beside them after.
    fn world_kept(&mut self) -> Rc<RefCell<Value>> {
        if let Some(book) = &self.world_book { return book.clone(); }
        let mut entries = Vec::new();
        {
            let cells = self.outermost.cells.borrow();
            for (at, word) in self.idents.iter().enumerate() {
                if !Self::visible_name(word) || self.readings.iter().any(|book| (book.from..book.upto).contains(&at)) { continue; }
                let Some(held) = cells.get(at) else { continue };
                if self.passed_over(word, held) { continue; }
                entries.push((Value::text(word), held.clone()));
            }
        }
        let natives = self.natives_kept();
        for word in self.table.strings("ext.system.module.builtins") {
            if !entries.iter().any(|(key, _)| spells_key(key, word)) { entries.push((Value::text(word), Value::Shared(natives.clone()))); }
        }
        let book = Rc::new(RefCell::new(Value::Dict(Rc::new(entries))));
        self.world_book = Some(book.clone());
        book
    }

    /// The dictionary of the names where the run stands: the reading
    /// under way, else the world's. Asked for the outer names, a reading
    /// with two dictionaries answers with its outer one.
    fn book_about(&mut self, outer: bool) -> Rc<RefCell<Value>> {
        if let Some(which) = self.reading_now {
            let book = &self.readings[which];
            return match (&book.outer, outer) {
                (Some(outer), true) => outer.clone(),
                _ => book.near.clone(),
            };
        }
        self.world_kept()
    }

    /// The names about a call given nothing: the outermost dictionary;
    /// inside a routine, a fresh dictionary of its own names; and for
    /// dir, those names listed in order.
    fn names_here(&mut self, frame: &Rc<Env>, op: Prim) -> Result<Value, String> {
        let book = if op == Prim::WorldBook || Rc::ptr_eq(frame, &self.outermost) {
            self.book_about(op == Prim::WorldBook)
        } else {
            let mut entries = Vec::new();
            if let Some(program) = self.frames_named.last() {
                let cells = frame.cells.borrow();
                for (word, held) in program.idents.iter().zip(cells.iter()) {
                    if Self::visible_name(word) && !matches!(held, Value::Unset) { entries.push((Value::text(word), held.clone())); }
                }
            }
            Rc::new(RefCell::new(Value::Dict(Rc::new(entries))))
        };
        if op == Prim::ClassWork(8) {
            let mut words: Vec<String> = match &*book.borrow() {
                Value::Dict(entries) => entries.iter().map(|(key, _)| key.bare()).collect(),
                _ => Vec::new(),
            };
            words.sort();
            return Ok(Value::Vector(Rc::new(words.iter().map(|word| Value::text(word)).collect())));
        }
        Ok(Value::Shared(book))
    }

    /// The blueprint of a code value, made once.
    fn code_blueprint(&mut self) -> Rc<Blueprint> {
        if let Some(kind) = &self.code_kind { return kind.clone(); }
        let kind = Rc::new(Blueprint {
            parents: Vec::new(), ancestry: Vec::new(), presentation: None, name: self.table.single("ext.builtin.compile.kind").unwrap_or_default().to_owned(),
            under: None, answers: Vec::new(), fields: Vec::new(), reaches: Vec::new(), methods: Vec::new(), constants: Vec::new(), shared: RefCell::new(Vec::new()),
        });
        self.code_kind = Some(kind.clone());
        kind
    }

    fn source_refused(&self) -> String {
        self.table.single("ext.builtin.source.unready").unwrap_or_default().to_owned()
    }

    fn source_unreadable(&self) -> String {
        self.table.single("ext.builtin.source.syntax").unwrap_or_default().to_owned()
    }

    /// The tokens of text handed over to be read, else the words for
    /// text that cannot be read.
    fn text_tokens(&self, source: &str) -> Result<Vec<crate::scan::Token>, String> {
        crate::scan::scan_at(source, self.table)
            .and_then(|read| crate::indent::indent(read, self.table, 0))
            .map_err(|_| self.source_unreadable())
    }

    /// The readers of text, the handing out of names, and the fetching
    /// of a namespace by name.
    fn text_operation(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        match op {
            Prim::WorldBook | Prim::HereBook => {
                if !v.is_empty() { return Err(self.core_complaint("core.arity", name)); }
                Ok(Value::Shared(self.book_about(op == Prim::WorldBook)))
            }
            Prim::Summon => match v.first().map(Value::settled) {
                Some(Value::Text(path)) => self.load_namespace(&path),
                _ => Err(self.source_refused()),
            },
            Prim::Prepare => self.text_prepared(name, v),
            _ => self.text_performed(op == Prim::Weigh, v),
        }
    }

    /// Text checked ahead of time and kept as a code value with the
    /// file it stands in and the manner it is to be read in.
    fn text_prepared(&mut self, name: &str, v: &[Value]) -> Result<Value, String> {
        let formals = self.table.strings("ext.builtin.compile.parameters");
        if v.len() < 3 || v.len() > formals.len() { return Err(self.core_complaint("core.arity", name)); }
        let (Value::Text(source), Value::Text(file), Value::Text(manner)) = (v[0].settled(), v[1].settled(), v[2].settled()) else { return Err(self.source_refused()) };
        let Some(mode) = self.table.strings("ext.builtin.compile.modes").iter().position(|word| word == manner.as_ref()) else { return Err(self.source_refused()) };
        // Flags and inheritance are read and let be; optimisation beyond
        // the ordinary setting is not honoured.
        if v.get(5).map_or(false, |worth| !matches!(worth, Value::Nil | Value::Small(0) | Value::Small(-1))) { return Err(self.source_refused()); }
        let tokens = self.text_tokens(if mode == 1 { source.trim() } else { &source })?;
        self.text_built(&tokens, &[], &file, mode, &[])?;
        let kind = self.code_blueprint();
        let holds = vec![(formals[0].clone(), Value::Text(source)), (formals[1].clone(), Value::Text(file)), (formals[2].clone(), Value::Small(mode as i64))];
        self.made += 1;
        Ok(Value::Thing(Rc::new(Thing { of: kind, holds: RefCell::new(holds), turn: self.made })))
    }

    /// Text or a code value run: as one expression where it is to be
    /// weighed, else as statements; in the dictionaries handed over,
    /// else where the call stands.
    fn text_performed(&mut self, weighing: bool, v: &[Value]) -> Result<Value, String> {
        let Some(first) = v.first().map(Value::settled) else { return Err(self.source_refused()) };
        let (source, file, mode) = match first {
            Value::Text(text) => (text.to_string(), None, usize::from(weighing)),
            Value::Thing(code) if self.table.single("ext.builtin.compile.kind") == Some(code.of.name.as_str()) => {
                let holds = code.holds.borrow();
                let mode = match holds.get(2).map(|(_, worth)| worth) { Some(Value::Small(n)) => *n as usize, _ => 0 };
                (holds[0].1.bare(), Some(holds[1].1.bare()), mode)
            }
            _ => return Err(self.source_refused()),
        };
        if v.len() > 3 { return Err(self.source_refused()); }
        // An expression to be weighed may stand in from the edge of its text.
        let source = if mode == 1 { source.trim().to_owned() } else { source };
        let mut books = Vec::new();
        for place in 1..3 {
            books.push(match v.get(place) {
                None | Some(Value::Nil) => None,
                Some(Value::Shared(cell)) if matches!(&*cell.borrow(), Value::Dict(_)) => Some(cell.clone()),
                Some(_) => return Err(self.source_refused()),
            });
        }
        let (outer, near) = match (books.remove(0), books.remove(0)) {
            (None, None) => return self.perform_here(&source, file, mode),
            (Some(outer), Some(near)) if Rc::ptr_eq(&outer, &near) => (outer, None),
            (Some(outer), near) => (outer, near),
            (None, near) => (self.book_about(true), near),
        };
        // A dictionary handed over for the outer names is given the
        // builtins as well, unless it names a dictionary of its own.
        if let Some(word) = self.table.single("ext.system.module.builtins").map(str::to_owned) {
            if looked_up(&outer, &word).is_none() {
                let natives = self.natives_kept();
                set_down(&outer, &word, Some(Value::Shared(natives)));
            }
        }
        self.perform_booked(&source, file, mode, outer, near)
    }

    /// Text run where the call stands: within the reading under way, in
    /// its dictionaries; else among the outermost names.
    fn perform_here(&mut self, source: &str, file: Option<String>, mode: usize) -> Result<Value, String> {
        if let Some(which) = self.reading_now {
            let (near, outer) = (self.readings[which].near.clone(), self.readings[which].outer.clone());
            return match outer {
                Some(outer) => self.perform_booked(source, file, mode, outer, Some(near)),
                None => self.perform_booked(source, file, mode, near, None),
            };
        }
        let tokens = self.text_tokens(source)?;
        let file = file.unwrap_or_else(|| "<string>".to_owned());
        let seeded = self.idents.clone();
        let (built, shown) = self.text_built(&tokens, &seeded, &file, mode, &[])?;
        self.idents = built.globals.clone();
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        self.text_concluded(&built, &file, mode, shown)
    }

    /// Text run in dictionaries of its own: its names are given slots
    /// among the outermost cells, and a book kept for them.
    fn perform_booked(&mut self, source: &str, file: Option<String>, mode: usize, outer: Rc<RefCell<Value>>, near: Option<Rc<RefCell<Value>>>) -> Result<Value, String> {
        let tokens = self.text_tokens(source)?;
        let file = file.unwrap_or_else(|| "<string>".to_owned());
        let beginning = self.idents.len();
        let prior: Vec<String> = (0..beginning).map(|n| format!("\0prior/{n}")).collect();
        // Where the outer dictionary names a dictionary of builtins of
        // its own, every builtin word is read as a name, and the text
        // reaches only what that dictionary holds.
        let own_natives = match self.table.single("ext.system.module.builtins").and_then(|word| looked_up(&outer, word)) {
            Some(Value::Shared(cell)) => self.natives_book.as_ref().map_or(true, |natives| !Rc::ptr_eq(&cell, natives)),
            _ => false,
        };
        let shadowed: Vec<String> = if own_natives { self.table.prims.keys().cloned().collect() } else { Vec::new() };
        let (built, shown) = self.text_built(&tokens, &prior, &file, mode, &shadowed)?;
        let fresh = &built.globals[beginning..];
        self.idents.extend(fresh.iter().map(|word| format!("\0names/{beginning}/{word}")));
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        let (near, outer) = match near { Some(near) => (near, Some(outer)), None => (outer, None) };
        self.readings.push(Namebook { near, outer, from: beginning, upto: beginning + fresh.len(), declared: built.outer_aliases.clone() });
        let was_reading = self.reading_now.replace(self.readings.len() - 1);
        let answer = self.text_concluded(&built, &file, mode, shown);
        self.reading_now = was_reading;
        answer
    }

    /// A text built in its manner: one expression to be weighed; one
    /// statement shown as it runs, which is an expression written out
    /// where it is one; else statements. Says besides whether what the
    /// text leaves is to be written out.
    fn text_built(&mut self, tokens: &[crate::scan::Token], seeded: &[String], file: &str, mode: usize, shadowed: &[String]) -> Result<(crate::build::Built, bool), String> {
        let written_in: Option<Rc<str>> = Some(Rc::from(file));
        if mode == 2 {
            if let Ok(built) = crate::build::build_text(tokens, self.table, seeded, 0, written_in.clone(), true, shadowed) { return Ok((built, true)); }
        }
        crate::build::build_text(tokens, self.table, seeded, 0, written_in, mode == 1, shadowed)
            .map(|built| (built, false)).map_err(|_| self.source_unreadable())
    }

    /// Run a built text as standing in its file, and answer what it
    /// left: the value weighed, else nothing.
    fn text_concluded(&mut self, built: &crate::build::Built, file: &str, mode: usize, shown: bool) -> Result<Value, String> {
        let (was_in, was_on) = (self.written_in.clone(), self.row);
        self.written_in = Rc::from(file);
        let top = self.outermost.clone();
        let ran = self.value_of(&built.program.body, &top);
        self.written_in = was_in;
        self.row = was_on;
        let answer = match ran {
            Ok(answer) => answer,
            Err(Escape::Error(told)) => return Err(told),
            Err(Escape::Yield(answer)) => answer,
            Err(other) => { self.got_away = Some(other); return Err("the source read in did not finish".to_owned()); }
        };
        if shown && !matches!(answer, Value::Nil | Value::Unset) {
            let quoted = self.core_primitive(Prim::Quoted, "repr", vec![answer], Vec::new())?;
            self.utter(&format!("{}\n", quoted.bare()));
            return Ok(Value::Nil);
        }
        Ok(if mode == 1 { answer } else { Value::Nil })
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

// Each primitive is given the values already worked out by the caller.
// Only the few names belonging to that primitive may fill its places.
impl Machine<'_> {
    fn is_core_primitive(op: Prim) -> bool {
        use Prim::*;
        matches!(op, Belongs | Tupling | Uniques | Dictionary | Ordered | Backwards | Numbered | Zipped | Mapped | Filtered | EveryTrue | Least | Greatest | Magnitude | Rounded | QuotRem | Powered | Hexadecimal | Octal | Binary | Quoted | Truthful | CallableValue | IdentityOf | Hashed | Iterator | NextItem | HasAttribute | GetMember | SetMember | DropMember | MembersOf)
    }

    fn core_complaint(&self, key: &str, middle: &str) -> String {
        let label = format!("ext.builtin.{}", key);
        let words = self.table.strings(&label);
        match words { [] => String::new(), [one] => one.clone(), [head, tail, ..] => format!("{head}{middle}{tail}") }
    }

    fn cursor_value(kind: IteratorKind) -> Value {
        Value::Iterator(Rc::new(RefCell::new(IteratorState { kind, peek: None, done: false })))
    }

    fn iterated_value(&mut self, source: &Value) -> Result<Value, String> {
        match source {
            Value::Iterator(_) => Ok(source.clone()),
            Value::Progression(walk) => Ok(Self::cursor_value(IteratorKind::Stepping(walk.clone(), BigInt::from(0)))),
            _ => {
                let entries = self.core_collect(source)?;
                Ok(Self::cursor_value(IteratorKind::Stored(entries.into_iter().collect())))
            }
        }
    }

    fn next_value(&mut self, iterator: &Value) -> Result<Option<Value>, String> {
        if let Value::Generator(frame) = iterator { return self.resume(frame, Value::Nil).map_err(|fault| self.suspension_fault(fault)); }
        let Value::Iterator(cell) = iterator else { return Err(self.core_complaint("core.not_iterator", &iterator.kind_word())); };
        // A summoned callable may reach back into this very iterator
        // before it answers, so the iterator is not marked busy while it
        // runs, and what it answers is weighed once it is back.
        let summons = match &cell.borrow().kind { IteratorKind::Summoned { work, stop } => Some((work.clone(), stop.clone())), _ => None };
        if let Some((work, stop)) = summons {
            {
                let mut held = cell.borrow_mut();
                if held.done { return Ok(None); }
                if let Some(value) = held.peek.take() { return Ok(Some(value)); }
            }
            let answer = match self.core_run(&work, Vec::new()) {
                Ok(value) => value,
                Err(complaint) => return if self.walk_halted() { cell.borrow_mut().done = true; Ok(None) } else { Err(complaint) },
            };
            let mut held = cell.borrow_mut();
            if held.done { return Ok(None); }
            if contained_equal(&stop, &answer) { held.done = true; return Ok(None); }
            return Ok(Some(answer));
        }
        let mut kind = {
            let mut held = cell.borrow_mut();
            if held.done { return Ok(None); }
            if let Some(value) = held.peek.take() { return Ok(Some(value)); }
            if matches!(held.kind, IteratorKind::Busy) { return Err(self.core_complaint("core.unready", "next")); }
            std::mem::replace(&mut held.kind, IteratorKind::Busy)
        };
        let result = (|| -> Result<Option<Value>, String> {
            match &mut kind {
                IteratorKind::Busy | IteratorKind::Summoned { .. } => unreachable!(),
                IteratorKind::Stored(entries) => Ok(entries.pop_front()),
                IteratorKind::Living(home, at) => {
                    let item = match home.borrow().settled() { Value::Vector(items) => items.get(*at).cloned(), _ => None };
                    if item.is_some() { *at += 1; }
                    Ok(item)
                }
                IteratorKind::Watching { window, at, size } => {
                    if Self::window_extent(window) != *size { return Err(self.core_complaint("core.dict.changed", "")); }
                    let item = match window.settled() { Value::Vector(items) => items.get(*at).cloned(), _ => None };
                    if item.is_some() { *at += 1; }
                    Ok(item)
                }
                IteratorKind::Placed(thing, at) => {
                    let Some(reader) = self.appointed(thing, 11) else { return Ok(None) };
                    match self.invoke(reader, self.outermost.clone(), vec![thing.clone(), Value::from_big(at.clone())]) {
                        Ok(item) => { *at += 1; Ok(Some(item)) }
                        Err(Escape::Thrown(Value::Thing(thrown))) if self.ends_places(&thrown) => Ok(None),
                        Err(Escape::Error(complaint)) => Err(complaint),
                        Err(away) => { self.got_away = Some(away); Err(self.bad_answer()) }
                    }
                }
                IteratorKind::Stepping(walk, at) => {
                    let item = walk.item(at);
                    if item.is_some() { *at += 1; }
                    Ok(item)
                }
                IteratorKind::Count(inner, number) => match self.next_value(inner)? {
                    None => Ok(None),
                    Some(member) => {
                        let pair = vec![Value::from_big(number.clone()), member];
                        *number += 1;
                        Ok(Some(Value::Tuple(Rc::new(pair))))
                    }
                },
                IteratorKind::Parallel { inputs, mapper, exact } => {
                    if inputs.is_empty() { return Ok(None); }
                    let mut parts = Vec::with_capacity(inputs.len());
                    for (which, input) in inputs.iter().enumerate() {
                        let Some(part) = self.next_value(input)? else {
                            // Demanded to end together, a later input
                            // ending first is short, and any input still
                            // giving once the first has ended is long.
                            if *exact {
                                if which > 0 { return Err(self.unequal_zip("zip.short", which)); }
                                for (later, other) in inputs.iter().enumerate().skip(1) {
                                    if self.next_value(other)?.is_some() { return Err(self.unequal_zip("zip.long", later)); }
                                }
                            }
                            return Ok(None);
                        };
                        parts.push(part);
                    }
                    match mapper {
                        None => Ok(Some(Value::Tuple(Rc::new(parts)))),
                        Some(work) => match self.core_run(work, parts) {
                            Ok(made) => Ok(Some(made)),
                            Err(complaint) => if self.walk_halted() { Ok(None) } else { Err(complaint) },
                        },
                    }
                }
                IteratorKind::Select(inner, test) => loop {
                    let Some(part) = self.next_value(inner)? else { break Ok(None); };
                    let yes = if matches!(test, Value::Nil) { self.stands_true(&part) } else {
                        match self.core_run(test, vec![part.clone()]) {
                            Ok(answer) => self.stands_true(&answer),
                            Err(complaint) => break if self.walk_halted() { Ok(None) } else { Err(complaint) },
                        }
                    };
                    if yes { break Ok(Some(part)); }
                },
            }
        })();
        let mut held = cell.borrow_mut();
        held.kind = kind;
        held.done = matches!(result, Ok(None));
        result
    }

    /// Whether what got away from a call is the fault that ends a walk;
    /// if so it is taken back, and the walk simply ends.
    fn walk_halted(&mut self) -> bool {
        let halted = matches!(&self.got_away, Some(Escape::Thrown(Value::Thing(thrown))) if self.table.strings("ext.stmt.class.special.stop").iter().any(|name| thrown.of.goes_by(name, false)));
        if halted { self.got_away = None; }
        halted
    }

    /// Whether a thrown thing says the places of a walk are over: the
    /// index fault, or the fault that ends any walk.
    fn ends_places(&self, thrown: &Thing) -> bool {
        self.table.single("ext.system.fault.class.index").map_or(false, |name| thrown.of.goes_by(name, false))
            || self.table.strings("ext.stmt.class.special.stop").iter().any(|name| thrown.of.goes_by(name, false))
    }

    /// zip's complaint of unequal sources: which one, then the close
    /// naming the first alone or the span up to the one before.
    fn unequal_zip(&self, key: &str, which: usize) -> String {
        match self.table.strings(&format!("ext.builtin.{}", key)) {
            [opening, alone, span] => format!("{}{}{}", opening, which + 1, if which == 1 { alone.clone() } else { format!("{}{}", span, which) }),
            _ => String::new(),
        }
    }

    /// How many entries the dictionary behind a window holds.
    fn window_extent(window: &Value) -> usize {
        match window { Value::Window(owner, _) => match owner.settled() { Value::Dict(entries) => entries.len(), _ => 0 }, _ => 0 }
    }

    /// The cell a list lives in, or a window upon a dictionary, taken as
    /// iter finds them before they are settled into copies.
    fn live_walk(value: &Value) -> Option<IteratorKind> {
        match value {
            Value::Window(..) => Some(IteratorKind::Watching { window: value.clone(), at: 0, size: Self::window_extent(value) }),
            Value::Shared(cell) => match &*cell.borrow() {
                Value::Vector(_) => Some(IteratorKind::Living(cell.clone(), 0)),
                inner @ (Value::Mutable(..) | Value::Window(..)) => Self::live_walk(inner),
                _ => None,
            },
            Value::Mutable(cell, _) => matches!(&*cell.borrow(), Value::Vector(_)).then(|| IteratorKind::Living(cell.clone(), 0)),
            _ => None,
        }
    }

    /// A thing with no walk of its own but a place-reading method is
    /// walked through its places from nought.
    fn placed_walk(&self, subject: &Value) -> Option<Value> {
        (matches!(subject, Value::Thing(_)) && self.appointed(subject, 11).is_some()).then(|| Self::cursor_value(IteratorKind::Placed(subject.clone(), BigInt::from(0))))
    }

    fn iterator_has_more(&mut self, iterator: &Value) -> Result<bool, String> {
        let Value::Iterator(cell) = iterator else { return Ok(false); };
        if cell.borrow().peek.is_some() { return Ok(true); }
        let next = self.next_value(iterator)?;
        let found = next.is_some();
        cell.borrow_mut().peek = next;
        Ok(found)
    }

    fn core_collect(&mut self, value: &Value) -> Result<Vec<Value>, String> {
        if let Value::Iterator(_) = value {
            let mut all = Vec::new();
            loop { match self.next_value(value)? { Some(item) => all.push(item), None => return Ok(all) } }
        }
        self.gathered_members(value).map_err(|_| self.core_complaint("core.uniterable", &value.kind_word()))
    }

    fn core_run(&mut self, callable: &Value, values: Vec<Value>) -> Result<Value, String> {
        match callable {
            Value::Member(receiver, name) => self.value_member(receiver, name, values, Vec::new()).map_err(|fault| self.suspension_fault(fault)),
            Value::Intrinsic(word) => {
                let op = *self.table.prims.get(word.as_ref()).ok_or_else(|| self.core_complaint("core.uncallable", &callable.kind_word()))?;
                self.prim(op, word, &values)
            }
            Value::Bound(program, frame) => match self.invoke(program.clone(), frame.clone(), values) {
                Ok(answer) => Ok(answer),
                Err(escape) => { self.got_away = Some(escape); Err(self.core_complaint("core.unready", &program.ident)) }
            },
            // A blueprint called makes a thing of it; a thing called
            // answers through its own calling method.
            Value::Blueprint(class) => match self.make_instance(class.clone(), values) {
                Ok(made) => Ok(made),
                Err(escape) => { self.got_away = Some(escape); Err(self.core_complaint("core.unready", &class.name)) }
            },
            Value::Thing(_) => self.ask_special(callable, 17, &values)?.ok_or_else(|| self.core_complaint("core.uncallable", &callable.kind_word())),
            other => Err(self.core_complaint("core.uncallable", &other.kind_word())),
        }
    }

    fn core_belongs(&self, item: &Value, expected: &Value) -> Result<bool, String> {
        match expected {
            Value::Tuple(kinds) => {
                for kind in kinds.iter() { if self.core_belongs(item, kind)? { return Ok(true); } }
                Ok(false)
            }
            Value::Blueprint(class) => Ok(matches!(item, Value::Thing(t) if t.of.goes_by(&class.name, false))),
            Value::KindOf(Kind::Nothing) => Ok(matches!(item, Value::Nil)),
            Value::Intrinsic(word) => {
                // A thing of a blueprint standing on the native kind is of that kind.
                if let Value::Thing(t) = item { if let Some(kind) = Self::native_beneath(&t.of) { return Ok(kind == word.as_ref()); } }
                let op = self.table.prims.get(word.as_ref());
                let answer = match op {
                    Some(Prim::ComplexMade) => matches!(item, Value::Complex(_)),
                    Some(Prim::AsInt) => matches!(item, Value::Small(_) | Value::Huge(_) | Value::Flag(_)),
                    Some(Prim::AsReal) => matches!(item, Value::Frac(r) if r.places.is_some()),
                    Some(Prim::AsText) => matches!(item, Value::Text(_)),
                    Some(Prim::SortOf) => matches!(item, Value::Blueprint(_) | Value::Intrinsic(_) | Value::OctetKind { .. }),
                    Some(Prim::Truthful) => matches!(item, Value::Flag(_)),
                    Some(Prim::Listed) => matches!(item, Value::Vector(_)),
                    Some(Prim::Tupling) => matches!(item, Value::Tuple(_)),
                    Some(Prim::Uniques) => matches!(item, Value::Set(_)),
                    Some(Prim::Dictionary) => matches!(item, Value::Dict(_)),
                    _ => return Err(self.core_complaint("core.isinstance.amiss", "")),
                };
                Ok(answer)
            }
            _ => Err(self.core_complaint("core.isinstance.amiss", "")),
        }
    }

    fn core_primitive(&mut self, op: Prim, name: &str, mut input: Vec<Value>, keywords: Vec<(String, Value)>) -> Result<Value, String> {
        // A value's identity is the cell it is kept in, so that one
        // primitive alone is handed the cell as it stands.
        // A list or a dictionary's window is walked as it stands, so what
        // iter is handed is looked at before it is settled into a copy.
        let live = if op == Prim::Iterator && input.len() == 1 { Self::live_walk(&input[0]) } else { None };
        let portion = match (op, input.first()) { (Prim::Quoted, Some(Value::Window(_, portion))) => Some(*portion), _ => None };
        if op != Prim::IdentityOf { for item in &mut input { *item = item.settled(); } }
        if keywords.is_empty() {
            if op == Prim::Hashed && matches!(input.first(), Some(Value::Octets { .. })) { return self.octet_routine(17, &input); }
            if op == Prim::Belongs && matches!(input.get(1), Some(Value::OctetKind { .. })) { return self.octet_routine(16, &input); }
            if op == Prim::Quoted && matches!(input.first(), Some(Value::Text(_))) { return crate::text::apply(self.table, crate::text::Work::REPR, name, &input, self.wording()); }
            if self.has_class_order() && input.first().map_or(false, |v| matches!(v, Value::Blueprint(_) | Value::Thing(_) | Value::Routine(_) | Value::Bound(..) | Value::Wrapped(..))) {
                let job = match op { Prim::CallableValue=>Some(2), Prim::GetMember=>Some(3), Prim::SetMember=>Some(4), Prim::DropMember=>Some(5), Prim::HasAttribute=>Some(6), Prim::MembersOf=>Some(7), _=>None };
                if let Some(job) = job { return self.work_on_class(job, input).map_err(|e| self.suspension_fault(e)); }
            }
        }
        if keywords.is_empty() { if let Some(value) = self.user_operation(op, &input)? { return Ok(value); } }
        if op == Prim::Dictionary && input.len() > 1 { return Err(self.table.single("ext.builtin.map.arguments.amiss").unwrap_or_default().to_owned()); }
        use num_traits::{Signed, Zero};
        use num_integer::Integer;
        use Prim::*;
        let mut ordering = None;
        let mut descending = false;
        let mut fallback = None;
        let mut exact = false;
        let mut additions = Vec::new();
        for (label, value) in keywords {
            let is = |tail: &str| self.table.spells(&format!("ext.builtin.{}", tail), &label);
            match op {
                Dictionary => { additions.push((Value::text(&label), value)); continue; }
                Ordered | Least | Greatest if is("key") => { ordering = Some(value); continue; }
                Ordered if is("reverse") => {
                    if !matches!(value, Value::Flag(_) | Value::Huge(_) | Value::Small(_)) { return Err(self.core_complaint("core.integer", &value.kind_word())); }
                    descending = self.stands_true(&value); continue;
                }
                Least | Greatest if is("default") => { fallback = Some(value); continue; }
                Zipped if is("zip.strict") => { exact = self.stands_true(&value); continue; }
                _ => (),
            }
            let slot = match (op, ()) {
                (Numbered, _) if is("start") => 1,
                (Rounded, _) if is("round.ndigits") => 1,
                (Rounded, _) if is("round.number") => 0,
                (Powered, _) if is("pow.base") => 0,
                (Powered, _) if is("pow.exp") => 1,
                (Powered, _) if is("pow.mod") => 2,
                _ => return Err(self.argument_fault("ext.syntax.call.amiss.unknown", Some(&label))),
            };
            if input.get(slot).map_or(false, |v| !matches!(v, Value::Unset)) { return Err(self.argument_fault("ext.syntax.call.amiss.duplicate", Some(&label))); }
            if input.len() <= slot { input.resize(slot + 1, Value::Unset); }
            input[slot] = value;
        }
        let require = |lower, upper| -> Result<(), String> {
            if input.len() < lower || input.len() > upper || input.iter().any(|v| matches!(v, Value::Unset)) {
                let which = if lower == 1 && upper == 1 { "core.arity.one" } else if lower == upper { "core.arity.exact" } else { "core.arity" };
                let parts = self.table.strings(&format!("ext.builtin.{}",which));
                let complaint = match which {
                    "core.arity.one" => format!("{}{}{}{}{}",parts[0],name,parts[1],input.len(),parts[2]),
                    "core.arity.exact" => format!("{}{}{}{}{}{}",parts[0],name,parts[1],lower,parts[2],input.len()),
                    _ => self.core_complaint(which,name),
                };
                Err(complaint)
            } else { Ok(()) }
        };
        let as_number = |v: &Value| if let Value::Flag(b) = v { Value::Small(*b as i64) } else { v.clone() };
        let whole = |v: &Value| -> Result<BigInt, String> {
            if matches!(v, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { v.as_big() } else { Err(self.core_complaint("core.integer", &v.kind_word())) }
        };
        let cursor = |values: Vec<Value>| Self::cursor_value(IteratorKind::Stored(values.into_iter().collect()));
        match op {
            Belongs => { require(2, 2)?; Ok(Value::Flag(self.core_belongs(&input[0], &input[1])?)) }
            Quoted => {
                require(1, 1)?;
                // An iterator is quoted by its kind and its identity, and
                // the quoting does not advance it.
                if matches!(input[0], Value::Iterator(_)) {
                    let Value::Small(mark) = self.core_primitive(Prim::IdentityOf, name, vec![input[0].clone()], Vec::new())? else { return Err(self.core_complaint("core.unready", name)) };
                    return Ok(Value::text(&format!("<{} object at 0x{:x}>", input[0].kind_word(), mark)));
                }
                let quoted = input[0].quoted(self.table.lone("system.real.render") == Some("shortest"));
                // A window upon a dictionary is quoted under its own name.
                Ok(Value::text(&match portion {
                    Some(letter) => format!("dict_{}({})", match letter { 'k' => "keys", 'v' => "values", _ => "items" }, quoted),
                    None => quoted,
                }))
            }
            Truthful => { require(0, 1)?; Ok(Value::Flag(input.first().map_or(false, |v| self.stands_true(v)))) }
            CallableValue => { require(1, 1)?; Ok(Value::Flag(matches!(input[0], Value::Intrinsic(_) | Value::Bound(..) | Value::Routine(_) | Value::Blueprint(_) | Value::Member(..) | Value::Method(..)))) }
            Hashed => {
                require(1, 1)?;
                input[0].hash_number().map(Value::Small).ok_or_else(|| self.core_complaint("core.unhashable", &input[0].kind_word()))
            }
            IdentityOf => {
                require(1, 1)?;
                // A collection a name keeps in a shared cell is known by the
                // cell, which stays put however the collection changes.
                let held = match &input[0] {
                    Value::Shared(cell) => match &*cell.borrow() {
                        Value::Vector(_) | Value::Dict(_) | Value::Set(_) => return Ok(Value::Small(Rc::as_ptr(cell) as usize as i64)),
                        Value::Mutable(inner, _) => return Ok(Value::Small(Rc::as_ptr(inner) as usize as i64)),
                        inner => inner.clone(),
                    },
                    Value::Mutable(cell, _) => return Ok(Value::Small(Rc::as_ptr(cell) as usize as i64)),
                    other => other.settled(),
                };
                let address: u64 = match &held {
                    Value::Nil => 0, Value::Flag(false) => 1, Value::Flag(true) => 2,
                    Value::Small(n) => (*n as u64).wrapping_mul(16).wrapping_add(3),
                    Value::Vector(p) | Value::Tuple(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Intrinsic(word) => {
                        let mut words: Vec<_> = self.table.prims.keys().collect(); words.sort();
                        words.iter().position(|w| w.as_str() == word.as_ref()).unwrap_or(0) as u64 + 16
                    }
                    Value::Text(chars) => chars.as_ptr() as usize as u64,
                    Value::Set(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Dict(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Thing(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Blueprint(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Iterator(p) => Rc::as_ptr(p) as usize as u64,
                    // A routine bound where it was defined is that
                    // definition reached that time: two reachings of the
                    // one definition are two routines.
                    Value::Bound(p, env) => (Rc::as_ptr(p) as usize as u64) ^ (Rc::as_ptr(env) as usize as u64).rotate_left(21),
                    Value::Routine(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Huge(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Frac(p) => Rc::as_ptr(p) as usize as u64,
                    _ => return Err(self.core_complaint("core.unready", name)),
                };
                let stamp = (input[0].kind_word(), address);
                let found = self.identities.iter().position(|old| *old == stamp).unwrap_or_else(|| { self.identities.push(stamp); self.identities.len()-1 });
                Ok(Value::Small(found as i64 + 1))
            }
            Tupling | Uniques => {
                require(0, 1)?;
                let entries = match input.first() { Some(v) => self.core_collect(v)?, None => Vec::new() };
                if op == Tupling { return Ok(Value::Tuple(Rc::new(entries))); }
                // A thing among the entries goes in as the set literal puts
                // it in, under its own hash and its own equality; any other
                // entry with no hash is refused.
                let mut store = crate::data::SetStore::new(self.table.single("ext.builtin.set").unwrap_or(""));
                for entry in entries {
                    if matches!(entry, Value::Thing(_)) {
                        let key = self.hash_key(&entry)?;
                        let previous = store.values();
                        let mut duplicate = false;
                        for old in previous { if self.keys_agree(&old, &key)? { duplicate = true; break; } }
                        if !duplicate { let address = format!("instance:{}", store.entries.len()); store.put(address, key); }
                    } else {
                        if entry.hash_number().is_none() { return Err(self.core_complaint("core.unhashable", &entry.kind_word())); }
                        store.put(self.hash_for_set(&entry)?, entry);
                    }
                }
                return Ok(Value::Set(Rc::new(RefCell::new(store))));
            }
            Dictionary => {
                require(0, 1)?;
                let mut incoming = Vec::new();
                if let Some(source) = input.first() {
                    if let Value::Dict(pairs) = source { incoming.extend(pairs.iter().cloned()); }
                    else {
                        for (position,row) in self.core_collect(source)?.into_iter().enumerate() {
                            let fields = self.core_collect(&row).map_err(|_| self.core_complaint("core.dict.sequence", &position.to_string()))?;
                            if fields.len() != 2 {
                                let parts = self.table.strings("ext.builtin.core.dict.pair");
                                return Err(format!("{}{}{}{}{}",parts[0],position,parts[1],fields.len(),parts[2]));
                            }
                            incoming.push((fields[0].clone(), fields[1].clone()));
                        }
                    }
                }
                incoming.extend(additions);
                let mut entries: Vec<(Value, Value)> = Vec::new();
                for (key, item) in incoming {
                    // A thing as a key carries its own hash, as it does in
                    // a dictionary literal.
                    let key = if matches!(key, Value::Thing(_)) { self.hash_key(&key)? } else { key };
                    if !matches!(key, Value::Keyed(..)) && key.hash_number().is_none() && !matches!(key, Value::Nil | Value::Frac(_)) { return Err(self.core_complaint("core.unhashable", &key.kind_word())); }
                    let mut place = None;
                    for (index, (old, _)) in entries.iter().enumerate() {
                        let same = if matches!(old, Value::Keyed(..)) || matches!(key, Value::Keyed(..)) { self.keys_agree(old, &key)? } else { as_number(old).equals(&as_number(&key)) };
                        if same { place = Some(index); break; }
                    }
                    match place { Some(index) => entries[index].1 = item, None => entries.push((key, item)) }
                }
                Ok(Value::Dict(Rc::new(entries)))
            }
            Iterator => {
                require(1, 2)?;
                if input.len() == 2 { return Ok(Self::cursor_value(IteratorKind::Summoned { work: input[0].clone(), stop: input[1].clone() })); }
                match live { Some(kind) => Ok(Self::cursor_value(kind)), None => self.iterated_value(&input[0]) }
            }
            NextItem => {
                require(1, 2)?;
                self.next_value(&input[0])?.or_else(|| input.get(1).cloned()).ok_or_else(|| self.core_complaint("core.exhausted", ""))
            }
            Backwards => {
                require(1, 1)?;
                if !matches!(input[0], Value::Vector(_) | Value::Tuple(_) | Value::Text(_) | Value::Progression(_) | Value::Dict(_)) { return Err(self.core_complaint("core.unready", name)); }
                // A progression runs backwards as a progression, last
                // place first, never gathered into the row it stands for.
                if let Value::Progression(walk) = &input[0] {
                    let last = &walk.first + (walk.count() - 1) * &walk.stride;
                    let backwards = crate::data::Progression { first: last, limit: &walk.first - &walk.stride, stride: -&walk.stride, word: walk.word.clone() };
                    return self.core_primitive(Prim::Iterator, name, vec![Value::Progression(Rc::new(backwards))], Vec::new());
                }
                let walked = self.core_collect(&input[0])?;
                Ok(cursor(walked.into_iter().rev().collect()))
            }
            Numbered => {
                require(1, 2)?;
                let first = input.get(1).map(whole).transpose()?.unwrap_or_default();
                let source = self.iterated_value(&input[0])?;
                Ok(Self::cursor_value(IteratorKind::Count(source, first)))
            }
            Zipped | Mapped => {
                require(if op == Mapped { 2 } else { 0 }, usize::MAX)?;
                let mut sources = Vec::new();
                for source in input.iter().skip(usize::from(op == Mapped)) { sources.push(self.iterated_value(source)?); }
                let mapper = (op == Mapped).then(|| input[0].clone());
                Ok(Self::cursor_value(IteratorKind::Parallel { inputs: sources, mapper, exact }))
            }
            Filtered => {
                require(2, 2)?;
                let source = self.iterated_value(&input[1])?;
                Ok(Self::cursor_value(IteratorKind::Select(source, input[0].clone())))
            }
            EveryTrue => {
                require(1, 1)?;
                let iterator = self.iterated_value(&input[0])?;
                loop {
                    match self.next_value(&iterator)? {
                        Some(item) if !self.stands_true(&item) => return Ok(Value::Flag(false)),
                        None => return Ok(Value::Flag(true)),
                        _ => (),
                    }
                }
            }
            Ordered => {
                require(1, 1)?;
                let key = ordering.unwrap_or(Value::Nil);
                let entries = self.core_collect(&input[0])?;
                let row = self.arranged(entries, &key, descending)?;
                Ok(Value::Vector(Rc::new(row)).keep(true))
            }
            Least | Greatest => {
                require(1, usize::MAX)?;
                if input.len() > 1 && fallback.is_some() { return Err(self.core_complaint("core.default.many", "")); }
                let key = ordering.unwrap_or(Value::Nil);
                // The members come one at a time and each is weighed the
                // moment it arrives, so a walk is drawn on no further
                // than it need be and the key is asked of the members in
                // the order they appear. A member weighing the same as
                // the one standing does not displace it, so the first of
                // several alike is the one answered with.
                // A generator is walked where it stands, for gathering it
                // first would run it to its end before a single member
                // had been weighed.
                let walk = if input.len() > 1 { Self::cursor_value(IteratorKind::Stored(input.clone().into_iter().collect())) }
                    else if matches!(input[0], Value::Generator(_)) { input[0].clone() }
                    else { self.iterated_value(&input[0])? };
                let Some(mut choice) = self.next_value(&walk)? else { return fallback.ok_or_else(|| self.core_complaint("core.empty", name)) };
                let mut standing = match &key { Value::Nil => choice.clone(), work => self.core_run(work, vec![choice.clone()])? };
                let wanted = if op == Least { Prim::Lt } else { Prim::Gt };
                while let Some(further) = self.next_value(&walk)? {
                    let mark = match &key { Value::Nil => further.clone(), work => self.core_run(work, vec![further.clone()])? };
                    let answer = self.prim(wanted, "", &[mark.clone(), standing.clone()])?;
                    if self.object_truth(&answer)? { choice = further; standing = mark; }
                }
                Ok(choice)
            }
            Magnitude => {
                require(1, 1)?;
                if let Value::Complex(pair) = &input[0] {
                    let norm = pair.0.hypot(pair.1);
                    if pair.0.is_finite() && pair.1.is_finite() && !norm.is_finite() { return Err(self.core_complaint("core.power.overflow", "")); }
                    return Ok(crate::complex::decimal_value(norm));
                }
                let parts = math::ratio_of(&as_number(&input[0])).ok_or_else(|| self.core_complaint("core.unready", name))?;
                Ok(math::make_number(parts.above.abs(), parts.beneath, parts.places))
            }
            Hexadecimal | Octal | Binary => {
                require(1, 1)?;
                let integer = whole(&input[0])?;
                let base = if op == Binary { 2 } else if op == Octal { 8 } else { 16 };
                let prefix = if op == Binary { "0b" } else if op == Octal { "0o" } else { "0x" };
                let digits = integer.abs().to_str_radix(base);
                Ok(Value::text(&format!("{}{}{}", if integer.is_negative() { "-" } else { "" }, prefix, digits)))
            }
            QuotRem => {
                require(2, 2)?;
                if input.iter().any(|x| matches!(x, Value::Complex(_))) { return Err(crate::complex::floor(self.table, &input[0], &input[1])); }
                let one = as_number(&input[0]); let two = as_number(&input[1]);
                let integral = |v: &Value| matches!(v, Value::Huge(_) | Value::Small(_));
                if integral(&one) && integral(&two) {
                    let divisor = two.as_big()?;
                    if divisor.is_zero() { return Err(self.core_complaint("core.zero", "")); }
                    let dividend = one.as_big()?;
                    return Ok(Value::Tuple(Rc::new(vec![Value::from_big(dividend.div_floor(&divisor)),Value::from_big(dividend.mod_floor(&divisor))])));
                }
                let left = math::ratio_of(&one).ok_or_else(|| self.core_complaint("core.unready", name))?;
                let right = math::ratio_of(&two).ok_or_else(|| self.core_complaint("core.unready", name))?;
                let x = crate::data::nearest_binary(&left.above,&left.beneath);
                let y = crate::data::nearest_binary(&right.above,&right.beneath);
                if y == 0.0 { return Err(self.core_complaint("core.zero", "")); }
                let residue = x % y;
                let corrected = residue != 0.0 && residue.is_sign_negative() != y.is_sign_negative();
                let remain = if residue == 0.0 { 0.0f64.copysign(y) } else if corrected { residue+y } else { residue };
                let quotient = (x-residue)/y - if corrected { 1.0 } else { 0.0 };
                let trunc = quotient.floor();
                let floor = if quotient == 0.0 { 0.0f64.copysign(x/y) } else if quotient-trunc > 0.5 { trunc+1.0 } else { trunc };
                let pair = [floor,remain].into_iter().map(|n| crate::data::worth_of_binary(n,math::DEFAULT_PLACES)).collect();
                Ok(Value::Tuple(Rc::new(pair)))
            }
            Powered => {
                require(2, 3)?;
                if input.iter().any(|x| matches!(x, Value::Complex(_))) {
                    if input.len() != 2 { return Err(crate::complex::complaint(self.table, "unready")); }
                    return crate::complex::reckon(self.table, Prim::Power, &input);
                }
                if input.len() < 3 || matches!(input[2], Value::Nil) {
                    let base = as_number(&input[0]); let exponent = as_number(&input[1]);
                    let e = math::ratio_of(&exponent).ok_or_else(|| self.core_complaint("core.unready", name))?;
                    let b = math::ratio_of(&base).ok_or_else(|| self.core_complaint("core.unready", name))?;
                    if e.above.is_negative() || e.beneath != BigInt::from(1) || e.places.is_some() || b.places.is_some() {
                        let n = crate::data::nearest_binary(&b.above,&b.beneath);
                        let power = crate::data::nearest_binary(&e.above,&e.beneath);
                        if n == 0.0 && power < 0.0 { return Err(self.core_complaint("core.power.zero", "")); }
                        let made = n.powf(power);
                        if made.is_nan() { return Err(self.core_complaint("core.unready", name)); }
                        if made.is_infinite() { return Err(self.core_complaint("core.power.overflow", "")); }
                        return Ok(crate::data::worth_of_binary(made, math::DEFAULT_PLACES));
                    }
                    return math::compute(Calc::Power, &base, &exponent).ok_or_else(|| self.core_complaint("core.unready", name))?;
                }
                if input.iter().any(|v| !matches!(v, Value::Flag(_) | Value::Small(_) | Value::Huge(_))) { return Err(self.core_complaint("core.power.integer", "")); }
                let modulus = whole(&input[2])?;
                if modulus.is_zero() { return Err(self.core_complaint("core.mod.zero", "")); }
                let m = modulus.abs();
                let exponent = whole(&input[1])?;
                let original = whole(&input[0])?;
                let base = if exponent.is_negative() {
                    let inverse = original.extended_gcd(&m);
                    if inverse.gcd != BigInt::from(1) { return Err(self.core_complaint("core.inverse", "")); }
                    inverse.x.mod_floor(&m)
                } else { original };
                let residue = base.modpow(&exponent.abs(), &m);
                let signed = if modulus.is_negative() && !residue.is_zero() { residue - m } else { residue };
                Ok(Value::from_big(signed))
            }
            Rounded => {
                require(1, 2)?;
                let places = match input.get(1) { None | Some(Value::Nil) => 0, Some(v) => whole(v)?.to_i64().ok_or_else(|| self.core_complaint("core.unready", name))? };
                // A whole number rounded to places after the point is itself;
                // to places before it, a half goes to the even neighbour where
                // the table says so.
                if self.table.flag("ext.builtin.round.whole.even") && matches!(input[0], Value::Small(_) | Value::Huge(_) | Value::Flag(_)) {
                    let number = input[0].as_big()?;
                    if places >= 0 { return Ok(Value::from_big(number)); }
                    let unit = BigInt::from(10).pow(u32::try_from(-places).ok().filter(|n| *n <= 100000).ok_or_else(|| self.core_complaint("core.unready", name))?);
                    let (mut quotient, remainder) = number.div_mod_floor(&unit);
                    let twice = &remainder * 2;
                    if twice > unit || (twice == unit && quotient.is_odd()) { quotient += 1; }
                    return Ok(Value::from_big(quotient * unit));
                }
                let exponent = u32::try_from(places.max(0)).ok().filter(|n| *n <= 100000).ok_or_else(|| self.core_complaint("core.unready", name))?;
                let value = as_number(&input[0]);
                let fraction = math::ratio_of(&value).filter(|r| !r.beneath.is_zero()).ok_or_else(|| self.core_complaint("core.unready", name))?;
                if self.table.lone("system.real.render") == Some("shortest") && fraction.places.is_some() {
                    let factor = BigInt::from(10).pow(places.unsigned_abs().min(100000) as u32);
                    let negative = fraction.above.is_negative();
                    let mut numerator = fraction.above.abs();
                    let mut denominator = fraction.beneath;
                    if places < 0 { denominator *= &factor; } else { numerator *= &factor; }
                    let mut rounded = &numerator / &denominator;
                    if (&numerator % &denominator) * 2 >= denominator { rounded += 1; }
                    if negative { rounded = -rounded; }
                    if input.len() < 2 || matches!(input[1], Value::Nil) { return Ok(Value::from_big(rounded)); }
                    let worth = if places < 0 { crate::data::nearest_binary(&(rounded * factor), &BigInt::from(1)) }
                        else { crate::data::nearest_binary(&rounded, &factor) };
                    return Ok(crate::data::worth_of_binary(if negative && worth == 0.0 { -0.0 } else { worth }, math::DEFAULT_PLACES));
                }
                // Follow the arithmetic of the shared library at each step.
                let factor = Value::from_big(BigInt::from(10).pow(exponent));
                let scaled = self.prim(Times, name, &[value, factor.clone()])?;
                let doubled = self.prim(Times, name, &[scaled, Value::Small(2)])?;
                let adjustment = if fraction.above.is_negative() { Minus } else { Plus };
                let adjusted = self.prim(adjustment, name, &[doubled, Value::Small(1)])?;
                let integral = self.prim(IntDiv, name, &[adjusted, Value::Small(2)])?;
                match input.get(1) {
                    None | Some(Value::Nil) => {
                        let r = math::ratio_of(&integral).ok_or_else(|| self.core_complaint("core.unready", name))?;
                        Ok(Value::from_big(r.above / r.beneath))
                    }
                    _ => self.prim(OverReal, name, &[integral, factor]),
                }
            }
            HasAttribute | GetMember | SetMember | DropMember | MembersOf => {
                let count = if op == MembersOf { 1 } else { 2 };
                require(count, if matches!(op, GetMember | SetMember) { 3 } else { count })?;
                if op != MembersOf && !matches!(input[1], Value::Text(_)) { return Err(self.core_complaint("core.attribute.name", &input[1].kind_word())); }
                let Value::Thing(thing) = &input[0] else {
                    if op == HasAttribute { return Ok(Value::Flag(false)); }
                    if op == GetMember && input.len() == 3 { return Ok(input[2].clone()); }
                    return Err(self.core_complaint(if op == MembersOf { "core.vars" } else { "core.unready" }, if op == MembersOf { "" } else { name }));
                };
                let mut members = thing.holds.borrow_mut();
                if op == MembersOf { return Err(self.core_complaint("core.unready", name)); }
                let Value::Text(word) = &input[1] else { return Err(self.core_complaint("core.attribute.name", &input[1].kind_word())); };
                let position = members.iter().position(|(n,_)| n == word.as_ref());
                if op == GetMember && position.is_none() && thing.of.program(word).is_some() { return Err(self.core_complaint("core.unready", name)); }
                if op == HasAttribute { return Ok(Value::Flag(position.is_some() || thing.of.program(word).is_some())); }
                if op == SetMember {
                    if input.len() != 3 { return Err(self.core_complaint("core.arity", name)); }
                    match position { Some(p) => members[p].1 = input[2].clone(), None => members.push((word.to_string(), input[2].clone())) }
                    return Ok(Value::Nil);
                }
                if let Some(position) = position { return Ok(if op == DropMember { members.remove(position); Value::Nil } else { members[position].1.clone() }); }
                if op == GetMember && input.len() == 3 { return Ok(input[2].clone()); }
                let words = self.table.strings("ext.builtin.core.attribute");
                Err(format!("{}{}{}{}{}", words[0], thing.of.name, words[1], word, words[2]))
            }
            _ => unreachable!(),
        }
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
#[path = "classes.rs"]
mod classes;
