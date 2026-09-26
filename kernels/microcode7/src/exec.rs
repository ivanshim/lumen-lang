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
use crate::form::{Input, Traps, Form, Prim, Routine, Address, Callee, Clause};
use crate::data::{Adornment, Among, Found, IteratorKind, IteratorState, Blueprint, Env, Kind, MapStore, Names, Reach, Thing, Value};

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

/// What a gatherer of faults is divided by: kinds its members may
/// stand under, or a routine asked of each member.
enum Sieve {
    Kinds(Vec<Rc<Blueprint>>),
    Asked(Value),
}

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
    /// The same, save that the first argument is the place written
    /// into and stays the form that reads it, so the primitive is
    /// still handed the name and not what it holds.
    Into(Callee, Form, usize),
    Call(usize),
    Select(Form, Form),
    Test(Rc<Form>),
    Decide(Rc<Form>),
    Turn(Rc<Form>, usize),
    HandOut,
    From,
    Finish,
    Truth(Prim, Form),
    /// A try whose body is under way: what may take what the body
    /// raises, how deep the found values stood when it began and how
    /// many faults were held then.
    Warding(Rc<Warded>, usize, usize),
    /// The same try once nothing more can take anything: its last part
    /// runs whichever way the body goes.
    Lastly(Rc<Warded>, usize, usize),
    /// What the try came to, put back once the last part has run.
    Restore(Box<Result<Value, Escape>>, usize),
    /// Let go of a held fault, and of the place a clause held it in.
    Unhold(usize, Option<Address>),
    /// Nothing more is owed: the body is over.
    Stop,
}

/// The parts of a try a suspended body stands inside.
#[derive(Debug)]
struct Warded {
    context: Option<Address>,
    clauses: Vec<Clause>,
    last: Option<Form>,
    otherwise: Option<Form>,
}

/// What one step of the owed work came to.
enum Stepped {
    Going,
    Handed(Value),
    Over,
}

pub struct Suspension {
    trace_state: Option<Rc<Thing>>,
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
    overseen: Option<(Rc<RefCell<Value>>, (usize, u64))>,
    /// The routine the body belongs to, where the walk is a routine's
    /// body and not a row of members already in hand. A step back into
    /// it stands inside that routine, so it is named while the step
    /// lasts and anything asking what names are in reach is answered
    /// about the right one.
    of: Option<Rc<Routine>>,
    /// The faults the body itself is handling, kept while it sleeps so
    /// that what is raised next stands behind them.
    holding: Vec<Value>,
    /// The word the reference gives a walk of the very thing this one
    /// was made from, where that walk is not a program's own: a map
    /// walked backwards, say. Nothing for a generator the program wrote.
    pub(crate) walked: Option<&'static str>,
}

impl Suspension {
    fn body(program: &Rc<Routine>, frame: Rc<Env>) -> Self {
        Self { trace_state: None, frame, owed: vec![Owed::Find(program.body.clone())], found: Vec::new(),
            begun: false, ended: false, receiving: false, result: Value::Nil,
            inner: None, members: None, ready: None, overseen: None, of: Some(program.clone()),
            holding: Vec::new(), walked: None }
    }
}

pub struct Machine<'a> {
    active_trace: Option<Rc<Thing>>,
    pub library_sources: HashMap<String, String>,
    imported: HashMap<String, Value>,
    /// Names now under construction: a module reading its own name back
    /// out of the loader before its top level has finished running --
    /// `builtins` asks for itself this way -- is handed the instance
    /// already standing, not read as stale for want of a place in
    /// `sys.modules` that only the finished import will write.
    importing: std::collections::HashSet<String>,
    /// Set while the namespace that answers for unbound names is being
    /// read, so that a name missing inside it stops there instead of
    /// asking for the same namespace over again.
    within_spare: bool,
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
    /// The blueprint every metaclass is built on, made when first asked for.
    builder_kind: Option<Rc<Blueprint>>,
    /// The blueprints standing for native kinds, one for each word a class has stood on.
    native_kinds: Vec<(String, Rc<Blueprint>)>,
    routine_members: Vec<(Value, Rc<Thing>)>,
    /// Routines whose spare arguments or code the program wrote over,
    /// each under the program and frame it was bound as: what the
    /// routine was, kept so the pair stays its own, what calls of it now
    /// run, and the program whose code it now runs.
    written_over: HashMap<(usize, usize), (Rc<Routine>, Rc<Env>, Rc<Routine>, Rc<Env>, Rc<Routine>)>,
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
    /// How many of the held faults belong to whoever asked a sleeping
    /// body for its next value, so that what the body puts away while it
    /// sleeps is found again at the same remove.
    holding_below: usize,
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
    /// How many compound writes are working out their plain working
    /// here. A refusal raised under one names the sign as the program
    /// wrote it, compound and all. Counted, since such a write may
    /// reach another.
    landed: usize,
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
    /// The member last sought and not found, with what it was sought
    /// on, for the attribute fault that will tell of it.
    sought_in_vain: Option<(String, Value)>,
    /// The key last found absent, where the words of the fault cannot
    /// carry it: the complaint for a key names the key itself, and a
    /// key of any kind but a text or a whole number is kept here whole
    /// rather than written into those words and read back out.
    key_in_vain: RefCell<Option<Value>>,
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
    /// The names and the frame set aside for a reading of text that
    /// stands inside a routine and was handed no dictionaries of its
    /// own. The text is read as a piece of that routine, so the
    /// routine's names are its own to read.
    text_within: Option<(Vec<String>, Rc<Env>)>,
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
    /// Whether a call fits its arguments to names. The rule is settled
    /// once for the whole run, so a name being read or written need not
    /// go looking for the word again at every turn.
    names_in_calls: bool,
    /// The words the language shows its worths in, gathered once.
    words: Names<'a>,
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

/// A path resolved against the working directory and cleared of `.`
/// and `..`, so it reads the same way wherever the run is later asked
/// about it from, as CPython's own `__file__` always does. Where the
/// host cannot resolve it (a `..` reaching above a filesystem root
/// under an unusual mount, say), the path as given is kept rather than
/// losing `__file__` altogether.
fn made_absolute(path: &str) -> String {
    std::fs::canonicalize(path).map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| path.to_string())
}

/// Letters this run has a fair chance of never having spelled before,
/// for naming a fresh temporary directory: the moment down to the
/// nanosecond, mixed with a counter this process alone advances, both
/// folded into base 36 to keep the name short.
fn unique_directory_name() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let moment = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let mut mixed = (moment as u64) ^ (std::process::id() as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ count.wrapping_mul(2654435761);
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if mixed == 0 { return "0".to_string(); }
    let mut out = Vec::new();
    while mixed > 0 {
        out.push(DIGITS[(mixed % 36) as usize]);
        mixed /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap()
}

fn ascend(frame: &Rc<Env>, depth: usize) -> &Rc<Env> {
    let mut f = frame;
    for _ in 0..depth {
        f = f.outer.as_ref().expect("a frame above");
    }
    f
}

/// The words a language shows its worths in. Not one of them changes
/// while a program runs, so they are gathered once at the outset and
/// handed out from there; gathering them afresh meant a dozen lookups
/// by name for every working of arithmetic.
fn words_of(table: &Table) -> Names<'_> {
    Names {
        truth: table.single("literal.true").unwrap_or("true"),
        falsity: table.single("literal.false").unwrap_or("false"),
        flag_counted: table.flag("system.flag.counts"),
        brief_reals: table.lone("system.real.render") == Some("shortest"),
        // A language may show nothing as no text at all, as PHP does,
        // rather than as the word a program writes for it.
        real_figures: table.count("ext.system.real.bits").and(table.count("ext.system.real.digits")),
        bit_reals: table.count("ext.system.real.bits").is_some(),
        nil: match table.flag("literal.null.silent") {
            true => "",
            false => table.single("literal.null").unwrap_or("null"),
        },
        within_word: table.single("ext.stmt.class.guarded"),
        alone_word: table.single("ext.stmt.class.hidden"),
        kept_as_bytes: table.flag("ext.system.text.bytes"),
        keys_by_worth: table.flag("ext.syntax.map.value_keys"),
    }
}

impl<'a> Machine<'a> {
    fn given_faults(table: &Table) -> HashMap<String, Value> {
        let mut chain: Vec<Rc<Blueprint>> = Vec::new();
        for (number, word) in table.strings("ext.builtin.exceptions").iter().enumerate() {
            let parent = match number {
                0 => None, 1 | 17 | 18 | 37 | 39 => Some(0), 3 | 4 => Some(2),
                6 | 7 => Some(5), 11 => Some(10), 14 | 21 => Some(13),
                22 => Some(9), 25..=35 => Some(24), 38 => Some(37), 40 | 41 => Some(20), 42 => Some(19),
                43 | 44 | 45 => Some(22), 46 => Some(36), 47 => Some(46), _ => Some(1),
            };
            let mut seed = Vec::new();
            match number {
                0 => seed.push(("\0fault-kind".to_string(), Value::Flag(true))),
                7 => seed.push(("\0key-fault".to_string(), Value::Flag(true))),
                // The exit ends the run when nobody takes it; the two
                // gatherers hold rows of faults, the ordinary one of
                // ordinary faults only, and it also stands under them.
                17 => seed.push(("\0leaves-run".to_string(), Value::Flag(true))),
                37 => seed.push(("\0gathers".to_string(), Value::Flag(false))),
                38 => {
                    seed.push(("\0gathers".to_string(), Value::Flag(true)));
                    if let Some(ordinary) = chain.get(1) { seed.push(("\0also-under".to_string(), Value::Blueprint(ordinary.clone()))); }
                }
                43 => seed.push(("\0unicode-encode".to_string(), Value::Flag(true))),
                44 => seed.push(("\0unicode-decode".to_string(), Value::Flag(true))),
                45 => seed.push(("\0unicode-translate".to_string(), Value::Flag(true))),
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

    /// The furnished fault kind at a place in the roster, and whether a
    /// kind stands under it.
    fn furnished_kind(&self, place: usize) -> Option<Rc<Blueprint>> {
        let word = self.table.strings("ext.builtin.exceptions").get(place)?;
        match self.fault_kinds.get(word) { Some(Value::Blueprint(kind)) => Some(kind.clone()), _ => None }
    }

    fn stands_under(&self, kind: &Rc<Blueprint>, place: usize) -> bool {
        self.furnished_kind(place).map_or(false, |ancestor| Self::fault_descends(kind, &ancestor))
    }

    fn make_fault(&mut self, kind: Rc<Blueprint>, row: Vec<Value>, because: Value) -> Value {
        self.made += 1;
        let mut holds = kind.every_field();
        // The rest of what a fault holds: a traceback that is nil, the
        // name and object of a name or attribute fault, and the number
        // and words of a system fault.
        if let Some(key) = self.table.single("ext.builtin.exceptions.traceback.member") { holds.push((key.to_string(), Value::Nil)); }
        // A name fault, an attribute fault and an import fault (19 is
        // ImportError) each carry the name that was absent.
        let names_absent = self.stands_under(&kind, 10) || self.stands_under(&kind, 12) || self.stands_under(&kind, 19);
        if names_absent { if let Some(key) = self.table.single("ext.builtin.exceptions.name") { holds.push((key.to_string(), Value::Nil)); } }
        if self.stands_under(&kind, 12) { if let Some(key) = self.table.single("ext.builtin.exceptions.object") { holds.push((key.to_string(), Value::Nil)); } }
        // The exhaustion carries what a generator returned, which is
        // the first of the arguments it was made with.
        if self.is_stop_kind(&kind) {
            if let Some(key) = self.table.single("ext.builtin.exceptions.value") {
                holds.push((key.to_string(), row.first().cloned().unwrap_or(Value::Nil)));
            }
        }
        if self.stands_under(&kind, 20) {
            let numbered = row.len() >= 2;
            for (at, key) in self.table.strings("ext.builtin.exceptions.os").iter().enumerate() {
                holds.push((key.clone(), if numbered { row.get(at).cloned().unwrap_or(Value::Nil) } else { Value::Nil }));
            }
            if let ([opening, middle, colon, quote], true) = (self.table.strings("ext.builtin.exceptions.os.message"), numbered) {
                let mut told = format!("{opening}{}{middle}{}", row[0].render(self.wording()), row[1].render(self.wording()));
                if let Some(named) = row.get(2) {
                    if !matches!(named, Value::Nil) { told = format!("{told}{colon}{}{quote}", named.render(self.wording())); }
                }
                holds.push(("\0told-as".to_string(), Value::text(&told)));
            }
        }
        // A Unicode codec fault carries its own account of what it
        // stood on, rather than an args tuple alone, and a decoding
        // fault holds it as octets even where a changeable row of them
        // was given, so a copy taken after cannot change what it shows.
        let decoding = self.stands_under(&kind, 44);
        let members = self.table.strings("ext.builtin.exceptions.unicode");
        if (self.stands_under(&kind, 43) || decoding) && row.len() == 5 {
            let object = match &row[1] {
                Value::Octets { cell, .. } if decoding => self.octets(cell.borrow().clone(), false),
                other => other.clone(),
            };
            let values = [row[0].clone(), object, row[2].clone(), row[3].clone(), row[4].clone()];
            for (key, value) in members.iter().zip(values) { holds.push((key.clone(), value)); }
        } else if self.stands_under(&kind, 45) && row.len() == 4 {
            let values = [row[0].clone(), row[1].clone(), row[2].clone(), row[3].clone()];
            for (key, value) in members.iter().skip(1).zip(values) { holds.push((key.clone(), value)); }
        }
        if self.stands_under(&kind, 36) {
            let keys = self.table.strings("ext.builtin.exceptions.syntax");
            let mut locations = Vec::new();
            if let Some(value) = row.get(1).filter(|_| row.len() == 2) {
                match value.settled() {
                    Value::Tuple(items) | Value::Vector(items) => locations.extend(items.iter().cloned()),
                    _ => {}
                }
            }
            locations.insert(0, row.first().cloned().unwrap_or(Value::Nil));
            locations.resize(keys.len(), Value::Nil);
            holds.extend(keys.iter().cloned().zip(locations));
            let layout: Vec<Value> = keys.iter().map(|k| Value::text(k)).collect();
            holds.push(("\0syntax-layout".to_string(), Value::Tuple(Rc::new(layout))));
        }
        let values = Value::Arguments(Rc::new(row));
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
        // A chain already standing behind `handled` may loop back on
        // itself without ever passing through the value being raised
        // (a program may set the context member to whatever it likes).
        // A second walker, moved every other step, meets the first
        // again if that is so, so the search still ends.
        let mut runner = handled.clone();
        let mut alternate = false;
        while let Some(Value::Thing(older)) = context_of(&step) {
            if Rc::ptr_eq(&older, &thing) {
                if let Some((_, slot)) = step.holds.borrow_mut().iter_mut().find(|(k, _)| k == key) { *slot = Value::Nil; }
                break;
            }
            step = older;
            if Rc::ptr_eq(&step, &runner) { break; }
            if alternate {
                if let Some(Value::Thing(ahead)) = context_of(&runner) { runner = ahead; }
            }
            alternate = !alternate;
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
        if Rc::ptr_eq(kind, ancestor) { return true; }
        // A kind standing under two stands under the second as well.
        let second = kind.fields.iter().find_map(|(key, held)| match held { Value::Blueprint(other) if key == "\0also-under" => Some(other), _ => None });
        second.map_or(false, |other| Self::fault_descends(other, ancestor))
            || kind.under.as_ref().map_or(false, |parent| Self::fault_descends(parent, ancestor))
    }

    /// A fault made by a call: what was given by position is its row,
    /// a gatherer's two are its heading and its members, and by name a
    /// call may give only the absent name and the object it was sought on.
    fn fault_from_call(&mut self, kind: Rc<Blueprint>, supplied: Vec<Value>) -> Res<Value> {
        let (row, named) = self.open_arguments(supplied)?;
        let locations = if self.stands_under(&kind, 36) { self.syntax_detail_count(&row)? } else { Vec::new() };
        let unicode_arity = if self.stands_under(&kind, 43) || self.stands_under(&kind, 44) { Some(5) }
            else if self.stands_under(&kind, 45) { Some(4) } else { None };
        if let Some(wanted) = unicode_arity {
            if row.len() != wanted { return Err(format!("TypeError: {}() takes exactly {} arguments ({} given)", kind.name, wanted, row.len()).into()); }
        }
        let made = if kind.every_field().iter().any(|(key, _)| key == "\0gathers") {
            if row.len() != 2 { return Err(self.argument_fault("ext.builtin.exceptions.group.invalid", None).into()); }
            let members = match row[1].settled() { Value::Vector(items) | Value::Tuple(items) => items.to_vec(), _ => Vec::new() };
            self.gather_faults(kind, row[0].clone(), members)?
        } else { self.make_fault(kind, row, Value::Nil) };
        if let Value::Thing(thing) = &made {
            self.locate_syntax_fault(thing, &locations);
            let mut holds = thing.holds.borrow_mut();
            for (key, value) in named {
                let allowed = ["ext.builtin.exceptions.name", "ext.builtin.exceptions.object"].iter().any(|label| self.table.single(label) == Some(key.as_str()));
                match holds.iter_mut().find(|(k, _)| *k == key) {
                    Some(entry) if allowed => entry.1 = value,
                    _ => return Err(self.argument_fault("ext.builtin.exceptions.unready", None).into()),
                }
            }
        }
        Ok(made)
    }

    /// A gatherer made of a heading and a row of faults, or refused.
    /// The base gatherer given ordinary faults alone comes out as the
    /// ordinary gatherer, as the reference has it.
    fn gather_faults(&mut self, kind: Rc<Blueprint>, heading: Value, members: Vec<Value>) -> Res<Value> {
        let refused = self.argument_fault("ext.builtin.exceptions.group.invalid", None);
        if !matches!(heading, Value::Text(_)) || members.is_empty() { return Err(refused.into()); }
        if !members.iter().all(|m| matches!(m, Value::Thing(t) if self.is_fault_kind(&t.of))) { return Err(refused.into()); }
        let ordinary_only = members.iter().all(|m| matches!(m, Value::Thing(t) if self.stands_under(&t.of, 1)));
        let strict = kind.every_field().iter().any(|(key, held)| key == "\0gathers" && matches!(held, Value::Flag(true)));
        if strict && !ordinary_only { return Err(refused.into()); }
        let kind = match (ordinary_only, self.furnished_kind(37), self.furnished_kind(38)) {
            (true, Some(base), Some(ordinary)) if Rc::ptr_eq(&kind, &base) => ordinary,
            _ => kind,
        };
        let how_many = members.len();
        let members = Value::Tuple(Rc::new(members));
        let made = self.make_fault(kind, vec![heading.clone(), members.clone()], Value::Nil);
        if let Value::Thing(thing) = &made {
            let mut holds = thing.holds.borrow_mut();
            holds.push(("\0heading".to_string(), heading.clone()));
            holds.push(("\0gathered".to_string(), members.clone()));
            if let [opening, one, several] = self.table.strings("ext.builtin.exceptions.group.summary") {
                let tail = if how_many == 1 { one } else { several };
                holds.push(("\0told-as".to_string(), Value::text(&format!("{}{opening}{how_many}{tail}", heading.render(self.wording())))));
            }
            if let Some(key) = self.table.single("ext.builtin.exceptions.group.message") { holds.push((key.to_string(), heading)); }
            if let Some(key) = self.table.single("ext.builtin.exceptions.group.members") { holds.push((key.to_string(), members)); }
        }
        Ok(made)
    }

    /// A gatherer's kind, heading and members; nothing for a fault that
    /// gathers nothing.
    fn gathered(value: &Value) -> Option<(Rc<Blueprint>, Value, Vec<Value>)> {
        let Value::Thing(thing) = value else { return None };
        let holds = thing.holds.borrow();
        let heading = holds.iter().find(|(key, _)| key == "\0heading")?.1.clone();
        let (_, Value::Tuple(members)) = holds.iter().find(|(key, _)| key == "\0gathered")? else { return None };
        Some((thing.of.clone(), heading, members.to_vec()))
    }

    /// Whether a sieve lets a fault through whole: a kind it stands
    /// under, or a routine that says yes to it.
    fn sieve_takes(&mut self, value: &Value, sieve: &Sieve) -> Res<bool> {
        match sieve {
            Sieve::Kinds(kinds) => Ok(matches!(value, Value::Thing(t) if kinds.iter().any(|kind| Self::fault_descends(&t.of, kind)))),
            Sieve::Asked(routine) => {
                let answer = self.apply_class_member(routine.clone(), vec![value.clone()])?;
                self.object_truth(&answer).map_err(Escape::from)
            }
        }
    }

    /// Divide a fault by a sieve into what passes and what stays, each
    /// a gatherer shaped like the whole, or nothing. What a gatherer
    /// holds besides its members is written onto both.
    fn sieve_faults(&mut self, value: Value, sieve: &Sieve) -> Res<(Option<Value>, Option<Value>)> {
        if self.sieve_takes(&value, sieve)? { return Ok((Some(value), None)); }
        let Some((kind, heading, members)) = Self::gathered(&value) else { return Ok((None, Some(value))) };
        let mut passed = Vec::new();
        let mut stayed = Vec::new();
        for member in members {
            let (through, held) = self.sieve_faults(member, sieve)?;
            passed.extend(through);
            stayed.extend(held);
        }
        let mut sides = [None, None];
        for (row, side) in [passed, stayed].into_iter().zip(sides.iter_mut()) {
            if row.is_empty() { continue; }
            let made = self.gather_faults(kind.clone(), heading.clone(), row)?;
            self.write_across(&value, &made);
            *side = Some(made);
        }
        let [passed, stayed] = sides;
        Ok((passed, stayed))
    }

    /// A gatherer's notes, cause, hushing flag and traceback go onto a
    /// gatherer made out of part of it.
    fn write_across(&self, from: &Value, onto: &Value) {
        let (Value::Thing(source), Value::Thing(target)) = (from, onto) else { return };
        let labels = ["ext.builtin.exceptions.notes", "ext.builtin.exceptions.cause", "ext.builtin.exceptions.suppress", "ext.builtin.exceptions.traceback.member"];
        let source = source.holds.borrow();
        let mut target = target.holds.borrow_mut();
        for key in labels.iter().filter_map(|label| self.table.single(label)) {
            let Some((_, held)) = source.iter().find(|(k, _)| k == key) else { continue };
            let held = match held { Value::Vector(items) => Value::Vector(Rc::new(items.to_vec())), other => other.clone() };
            match target.iter_mut().find(|(k, _)| k == key) {
                Some(entry) => entry.1 = held,
                None => target.push((key.to_string(), held)),
            }
        }
    }

    fn syntax_detail_count(&mut self, values: &[Value]) -> Result<Vec<Value>, String> {
        let [_, detail] = values else { return Ok(Vec::new()) };
        let parts = self.core_collect(detail)?;
        match parts.len() {
            4 | 6 => Ok(parts),
            n => {
                let description = if n < 4 { "at least 4" } else if n < 6 { "at least 6" } else { "at most 6" };
                Err(format!("TypeError: function takes {description} arguments ({n} given)"))
            },
        }
    }

    fn locate_syntax_fault(&self, thing: &Thing, parts: &[Value]) {
        if parts.is_empty() { return; }
        let mut fields = thing.holds.borrow_mut();
        for (at, name) in self.table.strings("ext.builtin.exceptions.syntax").iter().skip(1).take(6).enumerate() {
            let value = parts.get(at).cloned().unwrap_or(Value::Nil);
            match fields.iter_mut().find(|(key, _)| key == name) {
                Some(entry) => entry.1 = value,
                None => fields.push((name.clone(), value)),
            }
        }
    }

    /// Whether a word names one of the few methods a fault answers itself.
    fn fault_method_word(&self, word: &str) -> bool {
        ["ext.builtin.exceptions.note", "ext.builtin.exceptions.traceback.with", "ext.builtin.exceptions.group.split", "ext.builtin.exceptions.group.subgroup", "ext.builtin.exceptions.group.derive"]
            .iter().any(|label| self.table.single(label) == Some(word))
    }

    /// The methods a fault answers itself: a note added, a traceback
    /// set, and a gatherer divided three ways.
    fn fault_method(&mut self, thing: Rc<Thing>, word: &str, given: &[Value]) -> Res<Value> {
        if self.stands_under(&thing.of, 36) && self.table.single("ext.stmt.class.constructor") == Some(word) {
            let parts = self.syntax_detail_count(given)?;
            let replacement = self.make_fault(thing.of.clone(), given.to_vec(), Value::Nil);
            if let Value::Thing(new) = replacement {
                self.locate_syntax_fault(&new, &parts);
                let keys = self.table.strings("ext.builtin.exceptions.syntax");
                let args_key = self.table.single("ext.builtin.exceptions.args");
                let mut target = thing.holds.borrow_mut();
                for (key, value) in new.holds.borrow().iter().filter(|(k, _)| (keys.contains(k) && (given.len() == 2 || keys.first() == Some(k))) || k == "\0raised-values" || args_key == Some(k.as_str())) {
                    match target.iter_mut().find(|(k, _)| k == key) {
                        Some(entry) => entry.1 = value.clone(),
                        None => target.push((key.clone(), value.clone())),
                    }
                }
            }
            return Ok(Value::Nil);
        }
        let unready = self.argument_fault("ext.builtin.exceptions.unready", None);
        if self.table.single("ext.builtin.exceptions.note") == Some(word) {
            let [Value::Text(_)] = given else { return Err(self.argument_fault("ext.builtin.exceptions.note.invalid", None).into()) };
            let key = self.table.single("ext.builtin.exceptions.notes").unwrap_or_default().to_string();
            let mut holds = thing.holds.borrow_mut();
            match holds.iter_mut().find(|(k, _)| *k == key) {
                Some((_, Value::Vector(items))) => Rc::make_mut(items).push(given[0].clone()),
                Some(_) => return Err(unready.into()),
                None => holds.push((key, Value::Vector(Rc::new(given.to_vec())))),
            }
            return Ok(Value::Nil);
        }
        if self.table.single("ext.builtin.exceptions.traceback.with") == Some(word) {
            match given {
                [value @ (Value::Nil | Value::Backtrace(_))] => {
                    if let Some(key) = self.table.single("ext.builtin.exceptions.traceback.member") {
                        for (field, target) in thing.holds.borrow_mut().iter_mut() { if field == key { *target = value.clone(); break; } }
                    }
                    return Ok(Value::Thing(thing));
                }
                _ => return Err(String::from("TypeError: __traceback__ must be a traceback or None").into()),
            }
        }
        let whole = Value::Thing(thing);
        let Some((kind, heading, _)) = Self::gathered(&whole) else { return Err(unready.into()) };
        let [chooser] = given else { return Err(self.argument_fault("ext.builtin.exceptions.group.invalid", None).into()) };
        if self.table.single("ext.builtin.exceptions.group.derive") == Some(word) {
            let members = match chooser.settled() { Value::Vector(items) | Value::Tuple(items) => items.to_vec(), _ => Vec::new() };
            return self.gather_faults(kind, heading, members);
        }
        let sieve = match chooser {
            Value::Blueprint(kind) if self.is_fault_kind(kind) => Sieve::Kinds(vec![kind.clone()]),
            Value::Tuple(items) if items.iter().all(|k| matches!(k, Value::Blueprint(kind) if self.is_fault_kind(kind))) => {
                Sieve::Kinds(items.iter().filter_map(|k| match k { Value::Blueprint(kind) => Some(kind.clone()), _ => None }).collect())
            }
            Value::Routine(_) | Value::Bound(..) | Value::Method(..) => Sieve::Asked(chooser.clone()),
            _ => return Err(unready.into()),
        };
        let (passed, stayed) = self.sieve_faults(whole, &sieve)?;
        let both = self.table.single("ext.builtin.exceptions.group.split") == Some(word);
        Ok(if both { Value::Tuple(Rc::new(vec![passed.unwrap_or(Value::Nil), stayed.unwrap_or(Value::Nil)])) } else { passed.unwrap_or(Value::Nil) })
    }

    /// Writing a cause onto a fault, nil or not, hushes its context.
    fn context_hushed_by(&self, holder: &Value, key: &str) {
        let Value::Thing(thing) = holder else { return };
        if !self.is_fault_kind(&thing.of) || self.table.single("ext.builtin.exceptions.cause") != Some(key) { return };
        let mut holds = thing.holds.borrow_mut();
        if let Some(entry) = self.table.single("ext.builtin.exceptions.suppress").and_then(|flag| holds.iter_mut().find(|(k, _)| k == flag)) { entry.1 = Value::Flag(true); }
    }

    /// The name a complaint says was not there, read out of its words.
    fn name_not_there(&self, told: &str) -> Option<String> {
        let told = told.trim_start_matches('\0');
        if let Some(rest) = told.strip_prefix("Undefined variable") { return Some(rest.trim_start_matches(':').trim().trim_matches('\'').to_string()); }
        let [head, tail] = self.table.strings("ext.system.fault.name") else { return None };
        let from = told.find(head.as_str())? + head.len();
        told[from..].strip_suffix(tail.as_str()).map(str::to_string)
    }

    /// The namespace a "cannot import name" complaint named, read out
    /// of its own words: what `ImportError.name` is CPython's own
    /// module.
    fn import_source_not_there(&self, told: &str) -> Option<String> {
        let told = told.trim_start_matches('\0');
        let [head, mid, close] = self.table.strings("ext.stmt.import.member.missing") else { return None };
        let after_head = told.strip_prefix(head.as_str())?;
        let at = after_head.find(mid.as_str())?;
        let after_mid = &after_head[at + mid.len()..];
        let close_at = after_mid.find(close.as_str())?;
        Some(after_mid[..close_at].to_string())
    }

    /// Where a fault nobody took is an exit, the status the run is to
    /// end with, after saying whatever the exit carried that is not a
    /// number: nil for nought, a number as itself, and anything else
    /// told as a complaint with a status of one.
    fn exit_status(&self, thing: &Thing) -> Option<i32> {
        if !thing.of.every_field().iter().any(|(key, _)| key == "\0leaves-run") { return None; }
        let holds = thing.holds.borrow();
        let carried = match holds.iter().find(|(key, _)| key == "\0raised-values") {
            Some((_, Value::Arguments(row))) if row.len() == 1 => row[0].clone(),
            Some((_, Value::Arguments(row))) if row.is_empty() => Value::Nil,
            Some((_, other)) => other.clone(),
            None => Value::Nil,
        };
        Some(match carried {
            Value::Nil => 0,
            Value::Small(n) => n as i32,
            Value::Flag(yes) => i32::from(yes),
            other => { eprintln!("{}", other.render(self.wording())); 1 }
        })
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
            Value::Blueprint(_) => Err(self.table.single("ext.stmt.throw.invalid").or(self.table.single("ext.stmt.catch.invalid")).unwrap_or_default().to_string().into()),
            other => {
                let Some(invalid) = self.table.single("ext.stmt.throw.invalid") else { return Ok(other) };
                match &other {
                    Value::Thing(thing) if self.is_fault_kind(&thing.of) => Ok(other),
                    // Text opening with a furnished kind's name and a
                    // colon raises that kind with the words after, the
                    // way the library has always raised them.
                    Value::Text(words) if self.class_of_fault(words).map_or(false, |named| self.fault_kinds.contains_key(&named)) => {
                        self.as_raised(words).ok_or_else(|| invalid.to_string().into())
                    }
                    _ => Err(invalid.to_string().into()),
                }
            }
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
            importing: std::collections::HashSet::new(),
            imported: HashMap::new(),
            within_spare: false,
            world_book: None,
            readings: Vec::new(),
            reading_now: None,
            natives_book: None,
            code_kind: None,
            ancestor: None, property_kind: None, builder_kind: None, native_kinds: Vec::new(), routine_members: Vec::new(), written_over: HashMap::new(),
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
            active_trace: None,
            holding_fault: Vec::new(),
            holding_below: 0,
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
            landed: 0,
            inside: Vec::new(),
            holding: RefCell::new(Vec::new()),
            afterward: RefCell::new(Vec::new()),
            things: RefCell::new(Vec::new()),
            read_before: RefCell::new(std::collections::HashSet::new()),
            got_away: None,
            sought_in_vain: None,
            key_in_vain: RefCell::new(None),
            standing: 0,
            would_not_read: None,
            knows_cells: (HashMap::new(), HashMap::new(), std::collections::HashSet::new()),
            frames_named: Vec::new(),
            text_within: None,
            hearer: RefCell::new(None),
            untaken: RefCell::new(None),
            written_out: std::cell::Cell::new(false),
            unheard: RefCell::new(Vec::new()),
            any_unheard: std::cell::Cell::new(false),
            builds_places: table.flag("ext.op.index.makes"),
            names_in_calls: table.flag("ext.syntax.call.bind_names"),
            words: words_of(table),
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
            Value::Dict(pairs) => pairs.to_vec(),
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
        if self.names_in_calls && v.iter().any(|x| matches!(x, Value::Shared(_))) {
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

    /// The cell a class keeps for a value of its own. The value is made
    /// a cell where it still lies bare, so that the class and every name
    /// reaching it stand for the one holding; the class along the line
    /// that keeps the name answers for it, and a line keeping none takes
    /// the name as the class's own.
    fn own_cell(&self, class: &Rc<Blueprint>, called: &str) -> Rc<RefCell<Value>> {
        let keeper = class.keeper(called).unwrap_or(class);
        let mut shared = keeper.shared.borrow_mut();
        let at = match shared.iter().position(|(k, _)| k.as_str() == called) {
            Some(at) => at,
            None => {
                shared.push((called.to_string(), Value::Nil));
                shared.len() - 1
            }
        };
        if let Value::Shared(cell) = &shared[at].1 {
            return cell.clone();
        }
        let was = std::mem::replace(&mut shared[at].1, Value::Nil);
        let cell = Rc::new(RefCell::new(was));
        shared[at].1 = Value::Shared(cell.clone());
        cell
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

    fn text_remainder(&mut self, pattern: &str, rhs: &Value) -> Result<String, String> {
        if self.table.has_any("ext.builtin.format") {
            let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
            return layout.remainder(pattern, rhs, self, false);
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
        self.words
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
            // A key of any other kind was kept whole when it was found
            // absent, and stands as the fault's one argument.
            else if told == "\0absent-value=" { self.key_in_vain.borrow_mut().take() }
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
            // A name fault says which name; an attribute fault says
            // which and on what.
            if let Value::Thing(object) = &raised {
                let mut written: Vec<(Option<&str>, Value)> = Vec::new();
                if self.table.single("ext.system.fault.class.name") == Some(named.as_str()) {
                    if let Some(word) = self.name_not_there(told) { written.push((self.table.single("ext.builtin.exceptions.name"), Value::text(&word))); }
                }
                if self.table.single("ext.system.fault.class.attribute") == Some(named.as_str()) {
                    if let Some((word, holder)) = self.sought_in_vain.take() {
                        written.push((self.table.single("ext.builtin.exceptions.name"), Value::text(&word)));
                        written.push((self.table.single("ext.builtin.exceptions.object"), holder));
                    }
                }
                // An import fault names the namespace the wanted name
                // was sought in, read straight out of its own words
                // the way a name fault's own name is.
                if self.table.strings("ext.stmt.import.member.missing").first().map_or(false, |head| told.trim_start_matches('\0').starts_with(head.as_str())) {
                    if let Some(source) = self.import_source_not_there(told) { written.push((self.table.single("ext.builtin.exceptions.name"), Value::text(&source))); }
                }
                let mut holds = object.holds.borrow_mut();
                for (key, value) in written {
                    if let Some(entry) = key.and_then(|key| holds.iter_mut().find(|(k, _)| k == key)) { entry.1 = value; }
                }
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

    /// A frame of the routine's own names set aside for text read where
    /// it stands: the names it goes by, and a frame holding what each
    /// holds now. A name kept in a cell is given a cell of its own, and
    /// a name the routine reads from the scope around it is brought in
    /// beside its own, so the text reads it as the reference has it and
    /// a write the text makes is the text's alone.
    fn frame_aside(&self, frame: &Rc<Env>) -> (Vec<String>, Rc<Env>) {
        // The frame set aside stands directly under the outermost one,
        // since the text is read knowing one frame of names and reaches
        // the globals from just above them.
        let under = Some(self.outermost.clone());
        let Some(program) = self.frames_named.last() else {
            return (Vec::new(), Env::make(0, under));
        };
        let apart = |held: &Value| match held {
            Value::Shared(cell) => Value::Shared(Rc::new(RefCell::new(cell.borrow().clone()))),
            other => other.clone(),
        };
        let mut names = program.idents.clone();
        let mut cells: Vec<Value> = frame.cells.borrow().iter().map(apart).collect();
        cells.resize(names.len(), Value::Unset);
        for slot in &program.reaching {
            if names.iter().any(|word| word == slot.ident.as_ref()) { continue; }
            let held = match ascend(frame, slot.up).cells.borrow().get(slot.at) {
                Some(held) => apart(held),
                None => continue,
            };
            names.push(slot.ident.to_string());
            cells.push(held);
        }
        (names, Rc::new(Env { cells: RefCell::new(cells), outer: under }))
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
        let built = match crate::build::build_within_at(&tokens, self.table, &self.idents, &held, knows, 0, within, false, None) {
            Ok(built) => built,
            Err((said, row, _)) => return Err(Escape::Error(self.text_would_not_read(said, row))),
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
        let surrounding = self.active_trace.take();
        self.frames_named.push(built.program.clone());
        let ran = self.value_of(&built.program.body, &top);
        self.frames_named.pop();
        self.active_trace = surrounding;
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
                // An exit ends the run with the status it asks, once
                // what was to run afterward has run.
                if let Some(status) = self.exit_status(&thing) {
                    let _ = self.run_afterward();
                    self.let_things_go();
                    self.let_go_all();
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    std::process::exit(status);
                }
                if let Some(report) = thing.holds.borrow().iter().find_map(|(key, held)| (key == "\0report").then(|| held.bare())) {
                    return Err(report);
                }
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

    /// The same load as `fetch`, but the last one a binding gets before
    /// something else is written there: the value moves out of the
    /// cell, an unset value left in its place, rather than cloned out
    /// of it. Used only where the binding is about to be overwritten
    /// and nothing else can see it meanwhile, so a row grown one member
    /// at a time by its own gathering place costs what growing it
    /// costs, not a copy of everything gathered so far on every step.
    fn take(&mut self, slot: &Address, frame: &Rc<Env>) -> Result<Value, String> {
        let f = ascend(frame, slot.up);
        if Rc::ptr_eq(f, &self.outermost) {
            if let Some(found) = self.booked_read(slot.at, &slot.ident) { return found; }
        }
        // A cell handed out for names shared across calls is read
        // through exactly as `fetch` reads it, since some other
        // binding may hold the very same cell; only a plain local's
        // own value, or the value a closed-over one keeps, is this
        // read's alone to move out.
        if let Value::Shared(cell) = &f.cells.borrow()[slot.at] {
            if self.names_in_calls && !(Rc::ptr_eq(f, &self.outermost) && self.idents[slot.at].starts_with("\0import/")) {
                return Ok(Value::Shared(cell.clone()));
            }
            if !self.table.flag("ext.stmt.function.closes_over") { return Ok(cell.borrow().clone()); }
            let taken = cell.replace(Value::Unset);
            return if matches!(taken, Value::Unset) { self.fetch(slot, frame) } else { Ok(taken) };
        }
        let held = std::mem::replace(&mut f.cells.borrow_mut()[slot.at], Value::Unset);
        if matches!(held, Value::Unset) { return self.fetch(slot, frame); }
        Ok(held)
    }

    fn fetch(&mut self, slot: &Address, frame: &Rc<Env>) -> Result<Value, String> {
        let f = ascend(frame, slot.up);
        if Rc::ptr_eq(f, &self.outermost) {
            if let Some(found) = self.booked_read(slot.at, &slot.ident) { return found; }
        }
        let mut v = f.cells.borrow()[slot.at].clone();
        if let Value::Shared(cell) = &v {
            if self.names_in_calls && !(Rc::ptr_eq(f, &self.outermost) && self.idents[slot.at].starts_with("\0import/")) { return Ok(v); }
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
        if let Some(spare) = self.spare_name(&slot.ident) { return Ok(spare); }
        Err(format!("Undefined variable: {}", slot.ident))
    }

    /// A language may keep a namespace whose members answer for the
    /// names a program never bound: Python writes len without importing
    /// the namespace its len lives in. The namespace named by
    /// ext.system.names.module is read in on the first name that misses
    /// and consulted from then on; when it holds nothing under the name,
    /// or cannot be read at all, the name stays missing.
    fn spare_name(&mut self, wanted: &str) -> Option<Value> {
        if self.within_spare { return None; }
        let named = self.table.strings("ext.system.names.module").first()?.clone();
        if !self.imported.contains_key(&named) {
            self.within_spare = true;
            let outcome = self.load_namespace(&named);
            self.within_spare = false;
            outcome.ok()?;
        }
        let Some(Value::Thing(space)) = self.imported.get(&named) else { return None };
        for (word, cell) in space.holds.borrow().iter() {
            if word != wanted { continue; }
            let held = match cell { Value::Shared(link) => link.borrow().clone(), worth => worth.clone() };
            return if matches!(held, Value::Unset) { None } else { Some(held) };
        }
        None
    }

    fn store(&self, slot: &Address, frame: &Rc<Env>, value: Value) -> Result<(), String> {
        if self.names_in_calls {
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
    fn locate(&mut self, slot: &Address, frame: &Rc<Env>) -> Result<(Rc<Env>, usize), String> {
        let f = ascend(frame, slot.up);
        if !matches!(f.cells.borrow()[slot.at], Value::Unset) {
            return Ok((f.clone(), slot.at));
        }
        // A name that lives only in a dictionary handed over for a text
        // to run in, as exec is handed one, has no cell of its own to
        // find, though a read of it finds the dictionary's entry. The
        // write goes through what that entry holds, or lands in the
        // dictionary, so the cell the name would have had is answered.
        let booked = |run: &mut Self, at: usize| matches!(run.booked_read(at, &slot.ident), Some(Ok(_)));
        if Rc::ptr_eq(f, &self.outermost) && booked(self, slot.at) {
            return Ok((f.clone(), slot.at));
        }
        match slot.fallback {
            Some(g) if !matches!(self.outermost.cells.borrow()[g], Value::Unset) => Ok((self.outermost.clone(), g)),
            Some(g) if booked(self, g) => Ok((self.outermost.clone(), g)),
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

    fn enter_fault_handler(&mut self, raised: Value) {
        // The program now owns this error and may rewrite its arguments.
        if let Value::Thing(item) = &raised {
            let mut members = item.holds.borrow_mut();
            if let Some(at) = members.iter().position(|(key, _)| key == "\0report") { members.remove(at); }
        }
        self.holding_fault.push(raised);
    }

    /// What a piece of the program raised, carried through a place that
    /// holds only words. A furnished kind goes as its name and its words
    /// behind the marker, so that the clause around the call and the
    /// ending of the run alike read the thing raised back out of it; a
    /// walk over a generator is such a place, and what the generator's
    /// body raised reaches the arms round the walk this way.
    fn suspension_fault(&self, fault: Escape) -> String {
        if let Escape::Thrown(Value::Thing(thing)) = &fault {
            for (name, held) in thing.holds.borrow().iter() {
                if name == "\0report" { return held.bare(); }
            }
            let carried = thing.holds.borrow().iter().any(|(key, _)| key == "\0raised-values");
            if let Some(words) = Value::Thing(thing.clone()).raised_words(self.wording()).filter(|_| carried) {
                return match words.is_empty() {
                    true => format!("\0{}", thing.of.name),
                    false => format!("\0{}: {}", thing.of.name, words),
                };
            }
        }
        match fault {
            Escape::Error(words) | Escape::Stopped(words) => words,
            Escape::Thrown(value) => format!("Uncaught {}", value.render(self.wording())),
            _ => self.generator_words("unsupported"),
        }
    }

    fn generator_words(&self, suffix: &str) -> String {
        self.table.single(&format!("ext.stmt.yield.{}", suffix)).unwrap_or_default().to_string()
    }

    /// The members a walk hands over for taking apart: no more than the
    /// places call for, and one beyond them so that too many may be told
    /// from enough, except where a starred place takes all that is left.
    fn apart_members(&mut self, walk: &Value, wanted: usize, starred: bool) -> Result<Vec<Value>, String> {
        let mut taken = Vec::new();
        while starred || taken.len() <= wanted {
            match self.next_value(walk)? { Some(item) => taken.push(item), None => break }
        }
        Ok(taken)
    }

    fn make_iterator(&mut self, source: Value) -> Res {
        if let Value::Generator(_) = source { return Ok(source); }
        let members = self.gathered_members(&source)?;
        Ok(self.walk_over(&source, members))
    }

    /// The walk a delegated suspension hands members over from. A thing
    /// of the program's own, and a walk already under way, are kept as
    /// they stand rather than gathered, since such a walk may have no
    /// end at all and a suspension asks it for one member at a time.
    fn delegated_walk(&mut self, source: Value) -> Res {
        if matches!(source, Value::Thing(_) | Value::Iterator(_) | Value::Cursor(_)) { return Ok(self.iterated_value(&source)?); }
        self.make_iterator(source)
    }

    /// A step of the walk a suspension delegates to: another sleeping
    /// body is handed what was sent in, and anything else is asked for
    /// its next member, which is all such a walk knows how to be asked.
    fn delegated_step(&mut self, walk: &Value, sent: Value) -> Res<Option<Value>> {
        if let Value::Generator(inner) = walk { return self.resume(inner, sent); }
        match self.next_value(walk) {
            Ok(item) => Ok(item),
            Err(words) => Err(match self.got_away.take() { Some(escape) => escape, None => Escape::Error(words) }),
        }
    }

    /// What a walk gives back where it ends: a sleeping body gives what
    /// it returned, and a plain walk gives nothing.
    fn delegated_result(walk: &Value) -> Value {
        match walk { Value::Generator(inner) => inner.borrow().result.clone(), _ => Value::Nil }
    }

    /// The thing of the program's own a walk steps through, where the
    /// walk is one taken from such a thing.
    fn walked_thing(walk: &Value) -> Option<Value> {
        let Value::Iterator(cell) = walk else { return None };
        match &cell.borrow().kind { IteratorKind::Handed(thing) => Some(thing.clone()), _ => None }
    }

    /// A walk told to hand over nothing more, so that the suspension
    /// waiting on it steps past the delegation when it goes on.
    fn finish_walk(walk: &Value) {
        if let Value::Iterator(cell) = walk { let mut held = cell.borrow_mut(); held.peek = None; held.done = true; }
    }

    /// The method a thing of the program's own answers a plain name
    /// with, and the frame it runs in, looked for as a special name is:
    /// in the blueprint's namespace, then among its methods, then in the
    /// blueprints it stands upon.
    fn named_within(&self, subject: &Value, names: &[String]) -> Option<(Rc<Routine>, Rc<Env>)> {
        let Value::Thing(thing) = subject else { return None };
        let named = |key: &String| names.iter().any(|word| word == key);
        let mut blueprint = &thing.of;
        loop {
            let found = blueprint.shared.borrow().iter().find(|(key, _)| named(key)).map(|(_, value)| value.clone())
                .or_else(|| blueprint.methods.iter().find(|(key, _)| named(key)).map(|(_, body)| Value::Routine(body.clone())));
            match found {
                Some(Value::Bound(body, frame)) => return Some((body, frame)),
                Some(Value::Routine(body)) => return Some((body, self.outermost.clone())),
                Some(_) => return None,
                None => blueprint = blueprint.under.as_ref()?,
            }
        }
    }

    /// A walk let go of: another sleeping body is ended the way a walk
    /// is ended, and a walk of the program's own is told to end where it
    /// knows how, since it may have a last part of its own.
    fn end_delegate(&mut self, walk: &Value) -> Res<()> {
        if let Value::Generator(inner) = walk { return self.end_generator(inner); }
        let thing = Self::walked_thing(walk);
        let shutting = thing.as_ref().and_then(|held| self.named_within(held, &self.table.strings("ext.stmt.yield.close").to_vec()));
        if let (Some(thing), Some((body, scope))) = (thing, shutting) {
            self.invoke(body, scope, vec![thing])?;
        }
        Self::finish_walk(walk);
        Ok(())
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
            trace_state: None,
            holding: Vec::new(),
            frame: self.outermost.clone(), owed: Vec::new(), found: Vec::new(),
            begun: false, ended: false, receiving: false, result: Value::Nil,
            inner: None, members: Some(members.into_iter()), ready: None, overseen, of: None,
            walked: None,
        })))
    }

    /// A walk over a map's keys, values or pairs, taken backwards, named
    /// the way the reference names such a walk.
    fn walk_over_backwards(&self, source: &Value, members: Vec<Value>) -> Value {
        let walk = self.walk_over(source, members);
        let word = match source { Value::Window(_, portion) => crate::data::reversed_window_kind(*portion), _ => "dict_reversekeyiterator" };
        if let Value::Generator(state) = &walk { state.borrow_mut().walked = Some(word); }
        walk
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
    fn dict_extent(cell: &Rc<RefCell<Value>>) -> (usize, u64) {
        match &*cell.borrow() {
            Value::Dict(entries) => (entries.len(), entries.serial),
            Value::Mutable(deeper, _) | Value::Shared(deeper) => Self::dict_extent(deeper),
            _ => (0, 0),
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

    /// The words refusing a key no map can hold, naming the kind of
    /// what was offered: a list, a dict or a set that may be altered,
    /// at any depth inside a tuple. A sealed set keys a map as its own
    /// entries let it, so it is no offence. Nothing where the key will
    /// do, or the table has no such words.
    fn cannot_key(&self, key: &Value) -> Option<String> {
        let words = self.table.strings("ext.syntax.map.unhashable");
        let [head, tail] = words else { return None };
        fn culprit(value: &Value) -> Option<String> {
            match value {
                Value::Mutable(cell, _) | Value::Shared(cell) => culprit(&cell.borrow()),
                Value::Set(_) if value.set_sealed() => None,
                Value::Octets { changeable: true, .. } => Some("bytearray".into()),
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
            // A key of any other kind is kept whole beside the words,
            // since writing it into them would not give back the key
            // that was asked after.
            held => { *self.key_in_vain.borrow_mut() = Some(held); String::from("\0absent-value=") }
        }
    }

    /// The pairs a value offers a map: its own where it is a map, else
    /// one for each two-item member. Those before an ill-shaped member
    /// come back beside the words about it, to be written first.
    fn pairs_offered(&self, source: &Value) -> (Vec<(Value, Value)>, Option<String>) {
        let mut pairs = Vec::new();
        // A thing over a native worth offers the pairs that worth
        // offers, standing for it as it does everywhere its blueprint
        // appointed nothing of its own.
        let source = Self::underlying(&source.settled()).unwrap_or_else(|| source.clone());
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
        if let Some(walk) = state.inner.take() { self.end_delegate(&walk)?; }
        state.ended = true;
        state.ready = None;
        state.owed.clear();
        state.found.clear();
        state.holding.clear();
        state.members = None;
        Ok(())
    }

    /// The exhaustion a walk that is over raises: what the body returned
    /// stands as the value it was made with, and where it returned
    /// nothing the plain words serve.
    fn exhausted_of(&mut self, generator: &Rc<RefCell<Suspension>>) -> Escape {
        let words = self.generator_words("exhausted");
        let result = match generator.try_borrow() { Ok(state) => state.result.clone(), Err(_) => Value::Nil };
        if matches!(result, Value::Nil) { return Escape::Error(words); }
        let named = self.table.single("ext.stmt.class.special.stop").map(str::to_owned);
        match named.and_then(|word| self.fault_kinds.get(&word).cloned()) {
            Some(Value::Blueprint(kind)) => Escape::Thrown(self.make_fault(kind, vec![result], Value::Nil)),
            _ => Escape::Error(words),
        }
    }

    /// What a throw hands a walk: a kind is made into one of its own,
    /// and a kind given with a value of that kind raises the value.
    fn thrown_into(&mut self, mut values: Vec<Value>, frame: &Rc<Env>) -> Res {
        let given = match values.len() {
            1 => values.remove(0),
            _ => match values.remove(1) {
                Value::Nil => values.remove(0),
                held @ Value::Thing(_) => held,
                other => other,
            },
        };
        // What is no exception at all is refused by its kind, which is
        // not quite the refusal a raise of the same thing meets.
        match self.raise_class(given.clone(), frame) {
            Err(Escape::Error(words)) => match self.table.single("ext.stmt.yield.throw.invalid") {
                Some(before) => Err(format!("{before}{}", given.kind_word()).into()),
                None => Err(Escape::Error(words)),
            },
            other => other,
        }
    }

    /// The kind raised at a suspension when a walk is ended.
    fn exit_value(&mut self) -> Option<Value> {
        let word = self.table.single("ext.stmt.yield.exit")?.to_owned();
        let Some(Value::Blueprint(kind)) = self.fault_kinds.get(&word).cloned() else { return None };
        Some(self.make_fault(kind, Vec::new(), Value::Nil))
    }

    fn is_exit(&self, value: &Value) -> bool {
        let Value::Thing(thing) = value else { return false };
        self.table.single("ext.stmt.yield.exit").map_or(false, |word| thing.of.goes_by(word, false))
    }

    /// Ending a walk raises the ending kind where its body left off, so
    /// that last parts run and a clause may take it. A body that takes
    /// it and hands out another value is refused; one that lets it by,
    /// or returns, ends quietly, and what it returned is given back.
    fn shut_generator(&mut self, generator: &Rc<RefCell<Suspension>>) -> Res {
        let sleeping = {
            let state = generator.try_borrow().map_err(|_| self.generator_words("busy"))?;
            state.begun && !state.ended && state.of.is_some()
        };
        let Some(exit) = self.exit_value().filter(|_| sleeping) else {
            self.end_generator(generator)?;
            return Ok(Value::Nil);
        };
        match self.step_into(generator, Value::Nil, Some(exit), &[]) {
            Ok(Some(_)) => {
                self.end_generator(generator)?;
                Err(self.generator_words("close.ignored").into())
            }
            Ok(None) => {
                let result = generator.try_borrow().map_err(|_| self.generator_words("busy"))?.result.clone();
                self.end_generator(generator)?;
                Ok(result)
            }
            Err(Escape::Thrown(value)) if self.is_exit(&value) || matches!(&value, Value::Thing(thing) if self.is_stop_kind(&thing.of)) => {
                self.end_generator(generator)?;
                Ok(Value::Nil)
            }
            Err(other) => { self.end_generator(generator)?; Err(other) }
        }
    }

    fn resume(&mut self, generator: &Rc<RefCell<Suspension>>, sent: Value) -> Res<Option<Value>> {
        self.step_into(generator, sent, None, &[])
    }

    /// A step back into a sleeping body, either handing it a value or
    /// raising one where it left off. A body never begun and one already
    /// over take nothing in: what is thrown at them is raised on the spot.
    fn step_into(&mut self, generator: &Rc<RefCell<Suspension>>, sent: Value, mut hurled: Option<Value>, given: &[Value]) -> Res<Option<Value>> {
        // A body waiting on a delegated walk is not where the throw
        // lands: the walk it waits on is shown the value first, and only
        // what comes back out of that reaches the body itself.
        if let Some(value) = hurled.clone() {
            let waited = {
                let state = generator.try_borrow().map_err(|_| self.generator_words("busy"))?;
                match (&state.inner, state.begun && !state.ended) {
                    (Some(walk), true) => Some(walk.clone()),
                    _ => None,
                }
            };
            match waited {
                Some(Value::Generator(inner)) => {
                    let stepped = self.step_into(&inner, Value::Nil, Some(value), given);
                    let mut state = generator.try_borrow_mut().map_err(|_| self.generator_words("busy"))?;
                    match stepped {
                        Ok(Some(item)) => return Ok(Some(item)),
                        Ok(None) => { hurled = None; }
                        Err(Escape::Thrown(raised)) => { state.inner = None; hurled = Some(raised); }
                        Err(other) => { state.inner = None; return Err(other); }
                    }
                }
                // A walk of the program's own that knows how to be
                // thrown into is shown the value, and what it hands back
                // is what the throw came to, the delegation standing.
                // One that does not know is ended, and the value is
                // raised where the delegation stands instead.
                Some(walk) if !self.is_exit(&value) => {
                    let thing = Self::walked_thing(&walk);
                    let throwing = thing.as_ref().and_then(|held| self.named_within(held, &self.table.strings("ext.stmt.yield.throw").to_vec()));
                    match (thing, throwing) {
                        // The walk is shown what the throw was given,
                        // not the value the kernel made of it, since
                        // that is what the reference hands it.
                        (Some(thing), Some((body, scope))) => {
                            let arguments = std::iter::once(thing).chain(if given.is_empty() { vec![value] } else { given.to_vec() }).collect();
                            match self.invoke(body, scope, arguments) {
                                Ok(handed) => return Ok(Some(handed)),
                                Err(Escape::Thrown(raised)) if matches!(&raised, Value::Thing(thing) if self.is_stop_kind(&thing.of)) => {
                                    Self::finish_walk(&walk);
                                    hurled = None;
                                }
                                Err(other) => { Self::finish_walk(&walk); return Err(other); }
                            }
                        }
                        _ => {
                            self.end_delegate(&walk)?;
                            generator.try_borrow_mut().map_err(|_| self.generator_words("busy"))?.inner = None;
                        }
                    }
                }
                // A walk ended rather than thrown into is told to end
                // where it knows how, and the ending is raised where the
                // delegation stands.
                Some(walk) => {
                    self.end_delegate(&walk)?;
                    generator.try_borrow_mut().map_err(|_| self.generator_words("busy"))?.inner = None;
                }
                None => {}
            }
        }
        let mut state = generator.try_borrow_mut().map_err(|_| self.generator_words("busy"))?;
        if let Some(value) = hurled.clone() {
            if state.ended || !state.begun || state.of.is_none() {
                state.ended = true;
                state.result = Value::Nil;
                state.owed.clear();
                state.found.clear();
                state.holding.clear();
                return Err(Escape::Thrown(value));
            }
        }
        if state.ended { state.result = Value::Nil; return Ok(None); }
        if !state.begun && !matches!(sent, Value::Nil) { return Err(self.generator_words("unstarted").into()); }
        state.begun = true;
        if state.ready.is_some() { return Ok(state.ready.take()); }
        // A map that changed size under the walk stops the step.
        if let (Some(_), Some((cell, size))) = (&state.members, &state.overseen) {
            let current = Self::dict_extent(cell);
            if current != *size {
                let which = usize::from(current.0 == size.0);
                let complaint = self.table.strings("ext.builtin.core.dict.changed")[which].clone();
                state.ended = true;
                return Err(format!("\0{complaint}").into());
            }
        }
        if let Some(members) = &mut state.members {
            if !matches!(sent, Value::Nil) { return Err(self.generator_words("unsupported").into()); }
            let next = members.next();
            state.ended = next.is_none();
            return Ok(next);
        }
        let row = self.row;
        // The body picks up where it left off, so while it runs the run
        // stands in the routine it was written in and not in whichever
        // one asked for the next value.
        let named = state.of.clone();
        if let Some(program) = &named { self.frames_named.push(program.clone()); }
        // The faults the body itself is handling stand above the
        // caller's, so that it sees its own first and the caller's
        // behind them, as a body called from a clause does.
        let held_below = std::mem::replace(&mut self.holding_below, self.holding_fault.len());
        let mut mine = std::mem::take(&mut state.holding);
        self.holding_fault.append(&mut mine);
        let caller_trace = std::mem::replace(&mut self.active_trace, state.trace_state.take());
        let caller_source = self.written_in.clone();
        if let Some(place) = named.as_ref().and_then(|p| p.written_in.as_ref()) { self.written_in = place.clone(); }
        let outcome = self.unfold(&mut state, sent, hurled);
        let outcome = self.traced_result(outcome);
        state.trace_state = std::mem::replace(&mut self.active_trace, caller_trace);
        self.written_in = caller_source;
        let mark = self.holding_below.min(self.holding_fault.len());
        state.holding = self.holding_fault.split_off(mark);
        self.holding_below = held_below;
        if named.is_some() { self.frames_named.pop(); }
        self.row = row;
        // The stop kind raised in the body is the generator's fault,
        // not the end of its walk.
        let outcome = match outcome {
            Err(Escape::Thrown(Value::Thing(t))) if self.table.has_any("ext.stmt.yield.escaped") && self.is_stop_kind(&t.of) => Err(self.stop_got_out()),
            Err(Escape::Error(said)) if self.table.has_any("ext.stmt.yield.escaped") && self.table.single("ext.builtin.core.exhausted") == Some(said.as_str()) => Err(self.stop_got_out()),
            other => other,
        };
        if outcome.is_err() || matches!(outcome, Ok(None)) {
            state.ended = true;
            state.owed.clear();
            state.found.clear();
            state.holding.clear();
        }
        outcome
    }

    /// Whether a kind is the stop kind or stands under it.
    fn is_stop_kind(&self, kind: &Rc<Blueprint>) -> bool {
        let Some(word) = self.table.single("ext.stmt.class.special.stop") else { return false };
        matches!(self.fault_kinds.get(word), Some(Value::Blueprint(stop)) if Self::fault_descends(kind, stop))
    }

    /// The fault a generator raises when its body raised the stop kind.
    fn stop_got_out(&mut self) -> Escape {
        let said = self.generator_words("escaped");
        self.as_raised(&said).map_or(Escape::Error(said), Escape::Thrown)
    }

    /// One step of the owed work. The body is carried forward a piece at
    /// a time so that a watch standing round a suspension is kept with
    /// the body rather than on the machine's own stack.
    fn unfold_step(&mut self, state: &mut Suspension, sent: &mut Value) -> Res<Stepped> {
        let Some(work) = state.owed.pop() else { return Ok(Stepped::Over) };
        let frame = state.frame.clone();
        {
            match work {
                Owed::Find(node) => match node {
                    Form::OnLine(row, body) => {
                        self.row = row;
                        if let Some(active) = &self.active_trace { active.holds.borrow_mut()[0].1 = Value::Small(row as i64); }
                        state.owed.push(Owed::Find(*body));
                    }
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
                        return Err(if continuing { Escape::Resume(0) } else { Escape::Leave(0) });
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
                    // A call that writes into a place is handed the
                    // place by the form that reads it, not by what that
                    // form holds: the name has to reach the primitive
                    // for it to write where the name lives. So the
                    // place is kept whole while the rest are worked out
                    // one at a time, and a suspension among them still
                    // finds its way out.
                    Form::Apply(Callee::Prim(op @ (Prim::Append | Prim::Replace | Prim::Restore | Prim::Front), called), mut args)
                        if matches!(args.first(), Some(Form::Read(_))) =>
                    {
                        let place = args.remove(0);
                        state.owed.push(Owed::Into(Callee::Prim(op, called), place, args.len()));
                        for arg in args.into_iter().rev() { state.owed.push(Owed::Find(arg)); }
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
                    Form::Attempt { context, body, clauses, last, otherwise } => {
                        // A try with no suspension anywhere inside it, and
                        // a gathered clause, are left to the plain
                        // reckoning; the rest is taken on piece by piece.
                        let plain = clauses.iter().any(|clause| clause.grouped)
                            || !(suspension_within(&body) || clauses.iter().any(|clause| suspension_within(&clause.body))
                                || last.as_deref().map_or(false, suspension_within)
                                || otherwise.as_deref().map_or(false, suspension_within));
                        if plain {
                            let whole = Form::Attempt { context, body, clauses, last, otherwise };
                            state.found.push(self.value_of(&whole, &frame)?);
                        } else {
                            let plan = Rc::new(Warded { context, clauses, last: last.map(|form| *form), otherwise: otherwise.map(|form| *form) });
                            let floor = state.found.len();
                            // Counted from the caller's own held faults,
                            // since a later step back in may stand at
                            // another depth altogether.
                            let held = self.holding_fault.len() - self.holding_below;
                            state.owed.push(Owed::Lastly(plan.clone(), floor, held));
                            state.owed.push(Owed::Warding(plan, floor, held));
                            state.owed.push(Owed::Find(*body));
                        }
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
                Owed::Into(callee, place, count) => {
                    let mut args = vec![place];
                    args.extend(state.found.split_off(state.found.len() - count).into_iter().map(Form::Const));
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
                    let whole = self.table.flag("ext.op.logical.operand");
                    let left = state.found.pop().unwrap_or(Value::Nil);
                    let holds = self.stands_true(&left);
                    if (op == Prim::Both && !holds) || (op == Prim::Either && holds) {
                        state.found.push(if whole { left } else { Value::Flag(holds) });
                    } else {
                        if !whole { state.owed.push(Owed::Apply(Callee::Prim(Prim::AsTruth, Rc::from("")), 1)); }
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
                    return Ok(Stepped::Handed(state.found.pop().unwrap_or(Value::Nil)));
                }
                Owed::From => {
                    if state.inner.is_none() {
                        let source = state.found.pop().unwrap_or(Value::Nil);
                        state.inner = Some(self.delegated_walk(source)?);
                        *sent = Value::Nil;
                    }
                    let inner = state.inner.clone().expect("the delegated walk");
                    if let Some(item) = self.delegated_step(&inner, std::mem::replace(sent, Value::Nil))? {
                        state.owed.push(Owed::From);
                        return Ok(Stepped::Handed(item));
                    }
                    state.found.push(Self::delegated_result(&inner));
                    state.inner = None;
                }
                Owed::Finish => {
                    state.result = state.found.pop().unwrap_or(Value::Nil);
                    // A return leaves by way of every last part still
                    // owed, innermost first, before the body is over.
                    let mut parts = Vec::new();
                    while let Some(owed) = state.owed.pop() {
                        if let Owed::Lastly(plan, _, held) = owed {
                            self.holding_fault.truncate(self.holding_below + held);
                            if let Some(last) = plan.last.clone() { parts.push(last); }
                        }
                    }
                    if parts.is_empty() { return Ok(Stepped::Over); }
                    state.found.clear();
                    state.owed.push(Owed::Stop);
                    for last in parts.into_iter().rev() {
                        state.owed.push(Owed::Drop);
                        state.owed.push(Owed::Find(last));
                    }
                }
                Owed::Stop => return Ok(Stepped::Over),
                Owed::Warding(plan, floor, held) => {
                    // Nothing was raised: a manager is told the body is
                    // done with, and an else part runs in the body's place.
                    if let Some(address) = &plan.context {
                        self.leaving(&frame, address, None)?;
                    } else if let Some(otherwise) = plan.otherwise.clone() {
                        state.owed.push(Owed::Find(otherwise));
                        state.owed.push(Owed::Drop);
                    }
                    let _ = (floor, held);
                }
                Owed::Lastly(plan, floor, held) => {
                    let value = state.found.pop().unwrap_or(Value::Nil);
                    self.holding_fault.truncate(self.holding_below + held);
                    match plan.last.clone() {
                        Some(last) => {
                            state.owed.push(Owed::Restore(Box::new(Ok(value)), floor));
                            state.owed.push(Owed::Find(last));
                        }
                        None => { state.found.truncate(floor); state.found.push(value); }
                    }
                }
                Owed::Restore(left, floor) => {
                    state.found.truncate(floor);
                    match *left {
                        Ok(value) => state.found.push(value),
                        Err(escape) => return Err(escape),
                    }
                }
                Owed::Unhold(held, place) => {
                    self.holding_fault.truncate(self.holding_below + held);
                    if let Some(place) = place { self.store(&place, &frame, Value::Unset)?; }
                }
            }
        }
        Ok(Stepped::Going)
    }

    /// What a body raises is shown to the tries it stands inside, from
    /// the innermost outwards: a clause that takes it runs in its place,
    /// and every last part passed on the way runs before the value goes
    /// any further. What nothing takes leaves the body.
    fn unwind(&mut self, state: &mut Suspension, escape: Escape) -> Res<()> {
        let mut escape = escape;
        if matches!(escape, Escape::Stopped(_) | Escape::Done) { return Err(escape); }
        let frame = state.frame.clone();
        loop {
            let Some(owed) = state.owed.pop() else {
                return Err(match escape {
                    Escape::Leave(_) | Escape::Resume(_) => Escape::Error(self.generator_words("unsupported")),
                    other => other,
                });
            };
            match owed {
                Owed::Turn(cycle, floor) if matches!(escape, Escape::Leave(_) | Escape::Resume(_)) => {
                    state.found.truncate(floor);
                    if matches!(escape, Escape::Resume(_)) { state.owed.push(Owed::Turn(cycle, floor)); }
                    state.found.push(Value::Nil);
                    return Ok(());
                }
                Owed::Lastly(plan, floor, held) => {
                    let Some(last) = plan.last.clone() else { continue };
                    state.found.truncate(floor);
                    self.holding_fault.truncate(self.holding_below + held);
                    if let Escape::Thrown(value) = &escape { self.enter_fault_handler(value.clone()); }
                    state.owed.push(Owed::Unhold(held, None));
                    state.owed.push(Owed::Restore(Box::new(Err(escape)), floor));
                    state.owed.push(Owed::Find(last));
                    return Ok(());
                }
                Owed::Warding(plan, floor, held) => {
                    state.found.truncate(floor);
                    // Words stand in for a value raised where the run
                    // could only read; here, where a clause may take it,
                    // is where the value itself is wanted.
                    escape = match escape {
                        Escape::Error(_) if self.got_away.is_some() => self.got_away.take().expect("what got away"),
                        Escape::Error(told) if self.table.has_any("ext.builtin.exceptions") => match self.as_raised(&told) {
                            Some(value) => Escape::Thrown(value),
                            None => Escape::Error(told),
                        },
                        other => other,
                    };
                    let Escape::Thrown(raised) = &escape else { continue };
                    let raised = raised.clone();
                    self.save_traceback(&raised, false);
                    if let Some(address) = &plan.context {
                        match self.leaving(&frame, address, Some(&raised)) {
                            Ok(true) => { state.found.push(Value::Nil); return Ok(()); }
                            Ok(false) => continue,
                            Err(met) => { escape = met; continue; }
                        }
                    }
                    self.holding_fault.truncate(self.holding_below + held);
                    self.enter_fault_handler(raised.clone());
                    match self.taking_clause(&plan, &raised, &frame) {
                        Ok(Some((body, place, clears))) => {
                            self.under = None;
                            self.entering = None;
                            if let Some(place) = &place { self.store(place, &frame, raised.clone())?; }
                            state.owed.push(Owed::Unhold(held, place.filter(|_| clears)));
                            state.owed.push(Owed::Find(body));
                            return Ok(());
                        }
                        Ok(None) => { self.holding_fault.truncate(self.holding_below + held); continue; }
                        Err(met) => { self.holding_fault.truncate(self.holding_below + held); escape = met; continue; }
                    }
                }
                _ => continue,
            }
        }
    }

    /// The manager of a watched body is told the body is done with. It
    /// answers whether a value raised there is to be let go.
    fn leaving(&mut self, frame: &Rc<Env>, address: &Address, raised: Option<&Value>) -> Result<bool, Escape> {
        let manager = self.fetch(address, frame)?;
        if !matches!(&manager, Value::Thing(_)) { return Ok(false); }
        let unready = self.table.single("ext.stmt.class.special.unready").unwrap_or_default().to_owned();
        let arguments = match raised {
            None => vec![Value::Nil; 3],
            Some(value @ Value::Thing(thing)) => vec![Value::Blueprint(thing.of.clone()), value.clone(), self.traceback_of(value)],
            Some(_) => return Err(unready.into()),
        };
        let preceding = self.holding_fault.len();
        if let Some(value) = raised { self.enter_fault_handler(value.clone()); }
        // What got away from an earlier, unrelated call must not be
        // mistaken for what this one raises: only a fault this very
        // call sets belongs to it.
        self.got_away = None;
        let asked = self.ask_special(&manager, 34, &arguments).map_err(|told| self.got_away.take().unwrap_or(Escape::Error(told)));
        let asked = self.raised_if_error(asked);
        self.holding_fault.truncate(preceding);
        let answer = asked?.ok_or_else(|| self.bad_answer())?;
        Ok(raised.is_some() && self.object_truth(&answer)?)
    }

    /// The clause that takes what was raised, with the place it is to be
    /// held in while that clause runs.
    fn taking_clause(&mut self, plan: &Warded, raised: &Value, frame: &Rc<Env>) -> Result<Option<(Form, Option<Address>, bool)>, Escape> {
        for clause in &plan.clauses {
            let accepts = match &clause.choices {
                None => match raised {
                    Value::Thing(value) => clause.classes.iter().any(|name| value.of.goes_by(name, self.classes_either_way)),
                    _ => false,
                },
                Some(choices) => {
                    let mut fits = clause.takes_all;
                    for choice in choices {
                        let class = self.value_of(choice, frame)?.settled();
                        let named = match &class {
                            Value::Tuple(members) => members.iter().map(Value::settled).collect(),
                            _ => vec![class],
                        };
                        for class in named {
                            if !matches!(class, Value::Unset | Value::Blueprint(_)) {
                                return Err(self.table.single("ext.stmt.catch.invalid").unwrap_or("A catch needs a class").to_string().into());
                            }
                            if let Value::Blueprint(kind) = class {
                                if self.table.has_any("ext.builtin.exceptions") && !self.is_fault_kind(&kind) { return Err(self.table.single("ext.stmt.catch.invalid").unwrap_or_default().to_string().into()); }
                                if let Value::Thing(value) = raised {
                                    fits |= match self.table.has_any("ext.builtin.exceptions") {
                                        true => Self::fault_descends(&value.of, &kind),
                                        false => value.of.goes_by(&kind.name, self.classes_either_way),
                                    };
                                }
                            }
                        }
                        if fits { break; }
                    }
                    fits
                }
            };
            if accepts {
                return Ok(Some((clause.body.clone(), clause.held.clone(), clause.choices.is_some())));
            }
        }
        Ok(None)
    }

    fn unfold(&mut self, state: &mut Suspension, mut sent: Value, hurled: Option<Value>) -> Res<Option<Value>> {
        let taking = state.receiving;
        state.receiving = false;
        match hurled {
            Some(value) => {
                // Raised where the body left off, so what the body
                // itself was handling stands behind it.
                self.keep_context(&value);
                self.unwind(state, Escape::Thrown(value))?;
            }
            None => if taking { state.found.push(sent.clone()); },
        }
        loop {
            match self.unfold_step(state, &mut sent) {
                Ok(Stepped::Going) => {}
                Ok(Stepped::Handed(item)) => return Ok(Some(item)),
                Ok(Stepped::Over) => return Ok(None),
                Err(met) => self.unwind(state, met)?,
            }
        }
    }

    fn traceback_of(&self, value: &Value) -> Value {
        let Value::Thing(thing) = value else { return Value::Nil };
        let Some(key) = self.table.single("ext.builtin.exceptions.traceback.member") else { return Value::Nil };
        thing.holds.borrow().iter().find_map(|(name, held)| (name == key).then(|| held.clone())).unwrap_or(Value::Nil)
    }

    fn traced_result<T>(&mut self, result: Result<T, Escape>) -> Result<T, Escape> {
        if result.is_ok() { return result; }
        if self.table.strings("ext.builtin.exceptions.traceback").len() < 11 { return result; }
        let outcome = match result {
            Err(Escape::Error(text)) => Err(match self.got_away.take() {
                Some(escape) => escape,
                None => match self.as_raised(&text) {
                    None => Escape::Error(text),
                    Some(raised) => {
                        // Keep the outer reporter's words while the raised
                        // value acquires the frames a handler can inspect.
                        if let Value::Thing(item) = &raised {
                            if text.as_bytes().first() != Some(&0) && self.holding_fault.is_empty() {
                                let entry = (String::from("\0report"), Value::text(&text));
                                item.holds.borrow_mut().push(entry);
                            }
                        }
                        Escape::Thrown(raised)
                    }
                },
            }),
            rest => rest,
        };
        if let Err(Escape::Thrown(value)) = &outcome { self.save_traceback(value, false); }
        outcome
    }

    fn save_traceback(&mut self, value: &Value, repeat: bool) {
        let Value::Thing(raised) = value else { return };
        let keys = self.table.strings("ext.builtin.exceptions.traceback").to_vec();
        if keys.len() < 11 { return; }
        let Some(slot) = self.table.single("ext.builtin.exceptions.traceback.member").map(str::to_owned) else { return };
        if !raised.holds.borrow().iter().any(|(key, _)| key == &slot) { return; }
        if self.active_trace.is_none() {
            let program = self.frames_named.last();
            let name = program.filter(|p| p.ident != "<program>").map_or(keys[10].as_str(), |p| p.ident.as_str());
            let first = program.map_or(1, |p| p.declared_on.max(1));
            let code_members = vec![(keys[6].clone(), Value::text(name)), (keys[7].clone(), Value::Text(self.written_in.clone())), (keys[8].clone(), Value::Small(first as i64))];
            let of = self.code_blueprint();
            self.made += 1;
            let code = Value::Thing(Rc::new(Thing { of, holds: RefCell::new(code_members), turn: self.made }));
            let frame_type = Rc::new(Blueprint { name: keys[9].clone(), under: None, presentation: None,
                parents: Vec::new(), ancestry: Vec::new(), fields: Vec::new(), reaches: Vec::new(), methods: Vec::new(), answers: Vec::new(), constants: Vec::new(), shared: RefCell::new(Vec::new()) });
            self.made += 1;
            self.active_trace = Some(Rc::new(Thing { of: frame_type, turn: self.made, holds: RefCell::new(vec![
                (keys[4].clone(), Value::Small(self.row as i64)), (keys[5].clone(), code),
            ]) }));
        }
        let activation = self.active_trace.as_ref().unwrap().clone();
        let following = self.traceback_of(value);
        if let Value::Backtrace(link) = &following {
            if !repeat && Rc::ptr_eq(&link.activation, &activation) { return; }
        }
        let link = crate::data::TraceLink { location: self.row, activation, following };
        let mut fields = raised.holds.borrow_mut();
        if let Some((_, field)) = fields.iter_mut().find(|(key, _)| key == &slot) { *field = Value::Backtrace(Rc::new(link)); }
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
            Form::Take(slot) => Ok(self.take(slot, frame)?),
            Form::Glance(slot) => {
                let f = ascend(frame, slot.up);
                let held = f.cells.borrow()[slot.at].clone();
                let held = match (&held, slot.fallback) {
                    (Value::Unset, Some(g)) => self.outermost.cells.borrow()[g].clone(),
                    _ => held,
                };
                Ok(match held {
                    Value::Shared(cell) if !self.names_in_calls => cell.borrow().clone(),
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
                if let Some(active) = &self.active_trace { active.holds.borrow_mut()[0].1 = Value::Small(*row as i64); }
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
                // A class named outright holds its own values where a
                // thing holds its properties, so a write within one goes
                // through the cell the class keeps, which is the very
                // holding that reading the name gives out.
                if let (true, Value::Blueprint(class)) = (self.has_class_order(), &thing) {
                    return Ok(Value::Shared(self.own_cell(class, called)));
                }
                let Value::Thing(thing) = thing else {
                    if matches!(thing, Value::Routine(_) | Value::Bound(..) | Value::Method(..)) && self.table.has_any("ext.system.scope.unready") {
                        return Err(self.table.single("ext.system.scope.unready").unwrap_or_default().to_string().into());
                    }
                    return Err(format!("Cannot share property '{}' of {}", called, thing.bare()).into());
                };
                let mut holds = thing.holds.borrow_mut();
                let found = self.member_place(&holds, called);
                // A thing holding nothing of that name reads the class's
                // own value, and a write within it is a write within
                // that shared holding rather than a property made on the
                // thing, which a write of the name itself would make.
                if found.is_none() && self.has_class_order() && thing.of.keeper(called).is_some() {
                    drop(holds);
                    return Ok(Value::Shared(self.own_cell(&thing.of, called)));
                }
                let at = match found {
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
                return Ok(Value::Shared(self.own_cell(&class, called)));
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
                    Value::Shared(cell) if self.table.flag("ext.stmt.function.closes_over") && !self.names_in_calls => *cell.borrow_mut() = Value::Unset,
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
                // A thing whose blueprint appoints no such method parts
                // with no place, and the kind that will not is named, as
                // a tuple's and a text's are. A thing keeping a native
                // worth parts with the place out of that worth, which is
                // what the thing holds.
                if matches!(&target, Value::Thing(_)) && self.table.has_any("ext.op.sequence.delete")
                    && !matches!(Self::underlying(&target), Some(Value::Mutable(..))) {
                    return Err(self.deletion_refused(&target).into());
                }
                // A thing standing for a whole number is that number
                // where a row or a text is shortened at a place.
                let named = if matches!(target.settled(), Value::Vector(_) | Value::Text(_)) && matches!(named.settled(), Value::Thing(_)) {
                    let asked = self.stood_for_whole(&named.settled());
                    if let Some(away) = self.got_away.take() { return Err(away); }
                    asked?.unwrap_or(named)
                } else { named };
                let named = match &named {
                    Value::Span(bounds) if self.table.has_any("ext.builtin.slice") => {
                        let settled = self.span_settled(bounds);
                        if let Some(away) = self.got_away.take() { return Err(away); }
                        Value::Span(Rc::new(settled?))
                    }
                    _ => named,
                };
                let at = self.as_key_spoken(&named);
                // A key no map can hold is refused before the cell is
                // taken for the deletion, since the key may be that very
                // cell and must still be looked into to be named.
                if matches!(&target, Value::Dict(_)) {
                    if let Some(words) = self.cannot_key(&at) { return Err(words.into()); }
                }
                let Value::Shared(cell) = holder else {
                    return Err("Cannot take a place out of something that is not an array".to_string().into());
                };
                // A key that could be no key at all is refused before
                // the map is taken up for writing, since the key may be
                // the very map the place is taken out of.
                // What the cell holds may be the collection wrapped once
                // or more, a mutable standing for it or another shared
                // cell, so each wrapping is stepped through until the
                // collection itself is in hand.
                let mut cell = cell;
                loop {
                    let within = {
                        let inside = cell.borrow();
                        match &*inside {
                            Value::Mutable(inner, _) | Value::Shared(inner) => Some(inner.clone()),
                            // A thing of a blueprint standing on a
                            // native kind is stepped into as its worth,
                            // which is where its places live.
                            thing @ Value::Thing(_) => match Self::underlying(thing) {
                                Some(Value::Mutable(inner, _)) => Some(inner),
                                _ => None,
                            },
                            _ => None,
                        }
                    };
                    match within { Some(inner) => cell = inner, None => break }
                }
                let keyless = matches!(&*cell.borrow(), Value::Dict(_)).then(|| self.cannot_key(&at)).flatten();
                if let Some(words) = keyless { return Err(words.into()); }
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
                    // A tuple and text hold their places for good: a
                    // language of sequences refuses to take one out of
                    // either, and names the kind it was asked of.
                    held @ (Value::Tuple(_) | Value::Text(_)) if self.works_sequences() => {
                        return Err(self.deletion_refused(held).into());
                    }
                    // A row of such a language counts a place from the
                    // end as well as from the start, refuses a key of
                    // the wrong kind by that kind, and tells of a place
                    // it does not hold in the words for one.
                    held @ Value::Vector(_) if self.works_sequences() => {
                        let Value::Vector(items) = held else { unreachable!() };
                        let Some(offset) = (match &at { Value::Flag(b) => Some(i64::from(*b)), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None })
                            else { return Err(self.key_refused(held, &at).into()) };
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.place_written_beyond(held).into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, x)| x.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    Value::Vector(items) if self.table.has_any("ext.stmt.del") => {
                        let offset = (match &at { Value::Flag(b) => Some(if *b { 1 } else { 0 }), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None }).ok_or_else(|| self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string())?;
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string().into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, v)| v.clone()).collect();
                        Value::Vector(Rc::new(retained))
                    }
                    // A changeable row of bytes parts with a place, or
                    // with a run of them, where it stands; a fixed row
                    // parts with none and is named in the words for a
                    // sequence that cannot be shortened.
                    row @ Value::Octets { .. } => self.octets_shortened(row, &at)?,
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
                if let Some(active) = &self.active_trace { active.holds.borrow_mut()[0].1 = Value::Small(*row as i64); }
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
                let outcome = self.value_of(inner, frame);
                self.traced_result(outcome)
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
                        let unsaid = matches!(held, Value::Unset);
                        self.make_fault(base, if unsaid { Vec::new() } else { vec![held] }, Value::Nil)
                    }
                    _ => {
                        let held = if matches!(held, Value::Unset) { Value::text("") } else { held };
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
                let context_line = self.row;
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
                let body_result = self.traced_result(body_result);
                if let Some(address) = context {
                    let manager = self.fetch(address, frame)?;
                    if !matches!(&manager, Value::Thing(_)) { return body_result; }
                    if matches!(&body_result, Err(Escape::Error(_))) || matches!(&body_result, Err(Escape::Thrown(value)) if !matches!(value, Value::Thing(_))) {
                        return Err(self.table.single("ext.stmt.class.special.unready").unwrap_or_default().to_owned().into());
                    }
                    let arguments = if let Err(Escape::Thrown(v)) = &body_result {
                        let kind = match v { Value::Thing(t) => Value::Blueprint(t.of.clone()), _ => Value::Nil };
                        vec![kind, v.clone(), self.traceback_of(v)]
                    } else { vec![Value::Nil; 3] };
                    // The raised value stays held while the manager lets
                    // the body go, so whatever the leaving raises keeps it
                    // as context; and what the leaving raises comes back
                    // as raised rather than as a wrong answer.
                    if let Err(Escape::Thrown(value)) = &body_result { self.enter_fault_handler(value.clone()); }
                    // What got away from an earlier, unrelated call must
                    // not be mistaken for what this one raises: only a
                    // fault this very call sets belongs to it.
                    self.got_away = None;
                    let body_line = self.row;
                    if self.table.has_any("ext.builtin.exceptions.traceback") {
                        self.row = context_line;
                        if let Some(active) = &self.active_trace { active.holds.borrow_mut()[0].1 = Value::Small(self.row as i64); }
                    }
                    let asked = self.ask_special(&manager, 34, &arguments).map_err(|told| self.got_away.take().unwrap_or(Escape::Error(told)));
                    if asked.is_ok() && self.table.has_any("ext.builtin.exceptions.traceback") {
                        self.row = body_line;
                        if let Some(active) = &self.active_trace { active.holds.borrow_mut()[0].1 = Value::Small(self.row as i64); }
                    }
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
                    Err(Escape::Thrown(raised)) if clauses.iter().any(|clause| clause.grouped) => {
                        self.enter_fault_handler(raised.clone());
                        let outcome = self.grouped_clauses(clauses, raised, frame);
                        self.holding_fault.truncate(preceding);
                        outcome
                    }
                    Err(Escape::Thrown(raised)) => {
                        self.enter_fault_handler(raised.clone());
                        let chosen = (|| {
                            for clause in clauses {
                                if self.table.has_any("ext.builtin.exceptions.traceback") {
                                    self.row = clause.source_line;
                                    if let Some(active) = &self.active_trace { active.holds.borrow_mut()[0].1 = Value::Small(self.row as i64); }
                                }
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
                                            // A tuple names the classes within
                                            // it, an empty one naming none. All
                                            // of them are weighed before the
                                            // clause takes anything, so a member
                                            // that is no class is refused even
                                            // where an earlier member fits.
                                            let named = match &class {
                                                Value::Tuple(members) => members.iter().map(Value::settled).collect(),
                                                _ => vec![class],
                                            };
                                            for class in named {
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
                    if let Err(Escape::Thrown(value)) = &ending { self.enter_fault_handler(value.clone()); }
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
                        // The kind primitive, built on: what is being
                        // made is a metaclass, and the things it makes
                        // are classes rather than objects.
                        Some(Value::Intrinsic(_,word)) if self.has_class_order() && self.table.prims.get(word.as_ref())==Some(&Prim::SortOf) => Some(self.builder_blueprint()),
                        // A native kind the table lets a class stand on.
                        Some(Value::Intrinsic(_,word)) if self.table.spells("ext.stmt.class.builtin", &word) => Some(self.native_kind(&word)),
                        // The byte kinds are values in their own right,
                        // so each is looked up by the word spelling it.
                        Some(Value::OctetKind { changeable, .. }) if self.table.spells("ext.stmt.class.builtin", self.octet_kind_word(changeable)) => {
                            let word = self.octet_kind_word(changeable).to_owned();
                            Some(self.native_kind(&word))
                        }
                        // The property builtin, stood on as a class.
                        Some(named) if self.has_class_order() && self.spells_property_kind(&named) => Some(self.property_blueprint()),
                        Some(Value::Intrinsic(_,word)) if self.table.spells("ext.builtin.bool", &word) && self.table.has_any("ext.builtin.bool.base") => {
                            return Err(self.table.single("ext.builtin.bool.base").unwrap_or_default().to_owned().into());
                        }
                        _ => return Err(if self.has_class_order(){self.detail("unready").to_owned()}else{format!("Class {} cannot be built on that", plan.name)}.into()),
                    },
                };
                let mut answers = Vec::with_capacity(plan.answers);
                for _ in 0..plan.answers {
                    match given.next() {
                        Some(Value::Blueprint(b)) => answers.push(b),
                        Some(Value::Intrinsic(_,word)) if self.has_class_order() && self.table.prims.get(word.as_ref())==Some(&Prim::SortOf) => { let kind = self.builder_blueprint(); answers.push(kind); }
                        Some(Value::Intrinsic(_,word)) if self.table.spells("ext.stmt.class.builtin", &word) => { let kind = self.native_kind(&word); answers.push(kind); }
                        Some(Value::OctetKind { changeable, .. }) if self.table.spells("ext.stmt.class.builtin", self.octet_kind_word(changeable)) => {
                            let word = self.octet_kind_word(changeable).to_owned();
                            let kind = self.native_kind(&word); answers.push(kind);
                        }
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
                    // The body's own live namespace, where it made one,
                    // stands last and settles every member it governs:
                    // its current pairs replace whatever a place of the
                    // same name still holds, and a member `del` took out
                    // of it -- the name itself, or through `locals()` --
                    // never lands among the class's own at all.
                    if plan.has_book {
                        if let Some(book) = given.next() {
                            if let Value::Dict(pairs) = book.settled() {
                                for (key, value) in pairs.iter() {
                                    if matches!(key, Value::Text(_)) {
                                        let named = key.bare();
                                        entries.retain(|(old, _)| old != &named);
                                        entries.push((named, value.clone()));
                                    }
                                }
                            }
                        }
                    }
                    // Written in the order of the writing, the methods
                    // behind the rest; the class is to hold them in the
                    // order in which the body named them.
                    let rank=|key:&String|plan.ranking.iter().position(|name|name==key).unwrap_or(usize::MAX);
                    entries.sort_by_key(|(key,_)|rank(key));
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
                if self.has_class_order() && matches!(&stands,Value::Wrapped(..)|Value::Thing(_)|Value::Routine(_)) {
                    let mut values=self.value_list(args,frame)?;
                    if matches!(&stands,Value::Wrapped(..)) { values=self.opened_arguments(values)?; }
                    return self.apply_class_member(stands,values);
                }
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
                    self.method_of_value(receiver, &operation)
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
                    // A language may want the side that settled the pair
                    // handed back whole instead of a flag for its truth.
                    let whole = self.table.flag("ext.op.logical.operand");
                    let seen = self.value_of(&args[0], frame)?;
                    let left = self.object_truth(&seen)?;
                    if (*op == Prim::Both && !left) || (*op == Prim::Either && left) {
                        return Ok(if whole { seen } else { Value::Flag(left) });
                    }
                    // The right side is read in place and evaluated only
                    // here. Where a language holds it back as an arm of
                    // its own, that arm is run now; a routine the
                    // program merely named is a value like any other and
                    // is handed back unrun, since `x or f` answers with
                    // f and does not call it.
                    let right = match self.value_of(&args[1], frame)? {
                        Value::Bound(p, env) if p.frameless => self.invoke(p, env, Vec::new())?,
                        v => v,
                    };
                    if whole { return Ok(right); }
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
                        // A cause of nil is allowed, and hushes the context.
                        let cause = if matches!(cause, Value::Nil) { cause } else { self.raise_class(cause, frame)? };
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
                    self.save_traceback(&raised, true);
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
                        return self.fault_from_call(class, values);
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
                            return self.shut_generator(&generator);
                        }
                        if self.table.spells("ext.stmt.yield.send", &called) && values.len() == 1 {
                            return match self.resume(&generator, values.remove(0))? {
                                Some(item) => Ok(item),
                                None => Err(self.exhausted_of(&generator)),
                            };
                        }
                        if self.table.spells("ext.stmt.yield.throw", &called) && (1..=3).contains(&values.len()) {
                            let given = values.clone();
                            let value = self.thrown_into(values, frame)?;
                            return match self.step_into(&generator, Value::Nil, Some(value), &given)? {
                                Some(item) => Ok(item),
                                None => Err(self.exhausted_of(&generator)),
                            };
                        }
                        return Err(self.generator_words("unsupported").into());
                    }
                    if self.table.spells("ext.text.format", &called) {
                        if let Value::Text(pattern) = &subject {
                            let (positions, keywords) = self.open_arguments(values)?;
                            let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
                            let filled = layout.interpolate(pattern, &positions, &keywords, self)?;
                            return Ok(Value::text(&filled));
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
                Prim::Append | Prim::Replace | Prim::Restore => {
                    let Some(Form::Read(slot)) = args.first() else {
                        return Err(format!("First argument to {}() must be an array variable name", name).into());
                    };
                    let mut values = self.value_list(&args[1..], frame)?;
                    if self.names_in_calls {
                        let (plain, named) = self.open_arguments(values)?;
                        if !named.is_empty() { return Err(self.builtin_keyword_fault(name).into()); }
                        values = plain;
                    }
                    let want = if *op == Prim::Append { 1 } else { 2 };
                    if values.len() != want {
                        return Err(format!("{}() expects {} arguments, got {}", name, want + 1, values.len() + 1).into());
                    }
                    let (f, i) = self.locate(slot, frame)?;
                    // A name that lives only in a dictionary handed
                    // over for a text to run in, as exec is handed
                    // one, has no cell of its own: `locate` answers
                    // the cell it would have had, empty. What it
                    // stands for is fetched in and given that cell
                    // for the write below to reach, and once the
                    // write is done the dictionary is told what the
                    // cell came to hold, since that dictionary and
                    // not the empty cell is where a read of the name
                    // looks first.
                    let booked_here = Rc::ptr_eq(&f, &self.outermost) && matches!(f.cells.borrow()[i], Value::Unset);
                    if booked_here {
                        if let Some(Ok(found)) = self.booked_read(i, &slot.ident) {
                            f.cells.borrow_mut()[i] = found;
                        }
                    }
                    let mut values = values;
                    let value = values.pop().unwrap();
                    let mut key = values.pop().map(|k| self.as_key_spoken(&k));
                    if self.table.has_any("ext.stmt.class.special") {
                        let original = self.fetch(slot, frame)?;
                        // A thing over a native worth is written into
                        // through that worth, where its blueprint has no
                        // method of its own for the writing.
                        let (target, worth_cell) = match self.underlying_unless(&original.settled(), &[12]) {
                            Some(Value::Mutable(cell, _)) => { let held = cell.borrow().clone(); (held, Some(cell)) }
                            _ => (original.settled(), None),
                        };
                        // A thing standing for a whole number is that
                        // number where a row or a text is written into.
                        if let (Some(index @ Value::Thing(_)), Value::Vector(_) | Value::Text(_)) = (key.clone(), &target) {
                            let asked = self.stood_for_whole(&index);
                            if let Some(away) = self.got_away.take() { return Err(away); }
                            if let Some(whole) = asked? { key = Some(whole); }
                        }
                        if let (Some(key), Value::Dict(entries)) = (&key, &target) {
                            if let Some(words) = self.cannot_key(key) { return Err(words.into()); }
                            let key = self.hash_key(key)?;
                            // A map living alone in the cell every name for
                            // it shares is grown there in place, rather
                            // than copied whole for every key: the
                            // ordinary way one is built up key by key.
                            // Only a key whose address needs no help from
                            // the program's own code takes this road,
                            // since the cell is held open while it runs
                            // and a call back into the program could
                            // reach the very map being written.
                            let cell_here = match (&worth_cell, &original) {
                                (Some(cell), _) => Some(cell.clone()),
                                (None, Value::Shared(cell) | Value::Mutable(cell, _)) => Some(cell.clone()),
                                _ => None,
                            };
                            let direct = cell_here.as_ref().map_or(false, |cell| matches!(&*cell.borrow(), Value::Dict(_)));
                            if let (Some(cell), true, Ok(address)) = (&cell_here, direct, key.hash_address()) {
                                // A pair's own key carrying no address of
                                // its own (kept off the place entirely)
                                // means a miss in the place proves
                                // nothing: such a pair might still be
                                // this key by the program's own equality,
                                // which only the walk below can answer,
                                // so the road taken here is left for
                                // that walk rather than risked.
                                let outcome = { let held = cell.borrow(); let Value::Dict(rc) = &*held else { unreachable!("checked just above") }; rc.locate(&address) };
                                match outcome {
                                    Found::Found(at) => {
                                        // The clone above is let go before
                                        // the cell is opened, so the
                                        // entries the cell holds are this
                                        // call's only claim on them and
                                        // are grown without copying.
                                        drop(target);
                                        let mut held = cell.borrow_mut();
                                        let Value::Dict(rc) = &mut *held else { unreachable!("checked just above") };
                                        Rc::make_mut(rc).overwrite_at(at, value);
                                        return Ok(Value::Nil);
                                    }
                                    Found::Absent => {
                                        drop(target);
                                        let mut held = cell.borrow_mut();
                                        let Value::Dict(rc) = &mut *held else { unreachable!("checked just above") };
                                        Rc::make_mut(rc).insert_known_absent(key, address, value);
                                        return Ok(Value::Nil);
                                    }
                                    Found::Unknown => {}
                                }
                            }
                            let mut entries = entries.to_vec();
                            let mut position = 0;
                            while position < entries.len() && !self.keys_agree(&entries[position].0, &key)? { position += 1; }
                            if position < entries.len() { entries[position].1 = value; } else { entries.push((key, value)); }
                            let changed = Value::Dict(Rc::new(entries.into()));
                            match (worth_cell, original) {
                                (Some(cell), _) => { cell.replace(changed); }
                                (None, Value::Shared(cell) | Value::Mutable(cell, _)) => { cell.replace(changed); }
                                _ => self.store(slot, frame, changed)?,
                            }
                            return Ok(Value::Nil);
                        }
                        if let (Some(index), Some(cell)) = (&key, &worth_cell) {
                            // The worth beneath a thing is written into
                            // by the rules of its own kind: a row takes
                            // no key but a whole number or a run of
                            // places, and refuses any other by the two
                            // kinds rather than becoming a map.
                            if self.works_sequences() && !matches!(index, Value::Span(_) | Value::Small(_) | Value::Huge(_) | Value::Flag(_)) {
                                let beneath = cell.borrow().settled();
                                if matches!(beneath, Value::Vector(_)) { return Err(self.key_refused(&beneath, index).into()); }
                            }
                            let letter = self.letter_places.then(|| value.render(self.wording()));
                            written_into(&mut cell.borrow_mut(), Some(index.clone()), value, &self.no_places(), self.builds_places, letter, !self.names_in_calls)?;
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
                    // Writing a place back into what held it, once
                    // something inside it changed: where the very thing
                    // being written already stands there, nothing about
                    // the holder is to change and it is left alone.
                    //
                    // Only a language of sequences asks for this, and
                    // only it may pay for it: telling whether the thing
                    // already stands there means reading the place, and
                    // in a language where reading a place that is not
                    // there is worth a word of warning, the reading
                    // would be heard. A language that spells nothing of
                    // sequences writes the holder back as it always
                    // did, and says nothing.
                    if *op == Prim::Restore && self.works_sequences() {
                        let standing = { let cells = f.cells.borrow(); cells[i].clone() };
                        if let Some(key) = &key {
                            if Self::one_cell(&value, &self.element(&standing, key, Reading::Plain).unwrap_or(Value::Nil)) {
                                return Ok(Value::Nil);
                            }
                        }
                    }
                    // A tuple and text hold their places for good where
                    // a language of sequences says so, whether one place
                    // is written into or a whole run of them.
                    if self.works_sequences() && key.is_some() {
                        let standing = { let cells = f.cells.borrow(); cells[i].settled() };
                        if matches!(standing, Value::Tuple(_) | Value::Text(_)) {
                            return Err(self.writing_refused(&standing).into());
                        }
                    }
                    // A row of such a language reckons a place from the
                    // end as readily as from the start, so the place
                    // counted back is the place written into; beyond the
                    // row it holds no place at all, and the words for a
                    // place written into say so.
                    if self.works_sequences() {
                        let whole = |named: &Value| match named {
                            Value::Flag(b) => Some(i64::from(*b)),
                            Value::Small(n) => Some(*n),
                            Value::Huge(n) => n.to_i64(),
                            _ => None,
                        };
                        let standing = { let cells = f.cells.borrow(); cells[i].settled() };
                        if let (Value::Vector(items), Some(offset)) = (&standing, key.as_ref().and_then(whole)) {
                            let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                            if !(0..items.len() as i64).contains(&position) { return Err(self.place_written_beyond(&standing).into()); }
                            key = Some(Value::Small(position));
                        } else if let (Value::Vector(_), Some(named)) = (&standing, key.as_ref()) {
                            // A key of some other kind names no place in
                            // a row. Writing at one is refused by the two
                            // kinds, as reading at one is, and the row
                            // stays a row rather than becoming a map
                            // whose keys are its places.
                            if !matches!(named, Value::Span(_)) { return Err(self.key_refused(&standing, named).into()); }
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
                        if booked_here { self.booked_write(i, &slot.ident, Some(f.cells.borrow()[i].clone())); }
                        return Ok(Value::Nil);
                    }
                    let held = f.cells.borrow()[i].clone();
                    let held = if let Value::Shared(cell) = held { cell.borrow().clone() } else { held };
                    if let Value::Octets { cell, changeable, .. } = held {
                        if !changeable { return Err(self.octet_error("immutable").into()); }
                        let byte = self.octet_item(&value, false)?;
                        if let Some(index) = key {
                            let index = self.octet_at(&index, cell.borrow().len(), changeable)?;
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
                            written_into(&mut held, key, value, &self.no_places(), self.builds_places, letter, !self.names_in_calls)?
                        }
                        None => {
                            let mut slots = f.cells.borrow_mut();
                            written_into(&mut slots[i], key, value, &self.no_places(), self.builds_places, letter, !self.names_in_calls)?
                        }
                    };
                    // More letters handed to a place in text than it has
                    // room for: the first went in and the language says so.
                    if over {
                        if let Some(said) = self.table.single("ext.op.index.text.first") {
                            self.grumble("warning", said);
                        }
                    }
                    if booked_here { self.booked_write(i, &slot.ident, Some(f.cells.borrow()[i].clone())); }
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
                    // A word of a native kind spelled with the entry
                    // after a dot, such as `str.upper`, calls that
                    // kind's entry unbound: its first argument is the
                    // receiver, so a thing of a class standing on the
                    // very kind the word names works here through what
                    // it keeps of it, exactly as the loose entry read
                    // off the kind itself does.
                    if let Some((word, _)) = name.split_once('.') {
                        if let Some(Value::Thing(t)) = values.first() {
                            if Self::native_beneath(&t.of).as_deref() == Some(word) {
                                if let Some(worth) = Self::underlying(&values[0]) { values[0] = worth; }
                            }
                        }
                    }
                    // A list that a lazy walk is to begin on is not gathered
                    // into a copy: it goes on in its own cell, so the walk
                    // reaches what the body adds and misses what it takes.
                    if *op == Prim::Iterated && self.table.flag("ext.stmt.yield.suspends") {
                        if let Some(living @ IteratorKind::Living(..)) = values.first().and_then(Self::live_walk) { return Ok(Self::cursor_value(living)); }
                    }
                    // The property builtin takes its accessors by name; making the property sorts them out.
                    if *op == Prim::ClassWork(11) && self.has_class_order() && !self.detail("descriptor.get").is_empty() { return self.work_on_class(11, values); }
                    if self.names_in_calls && self.table.prims.contains_key(name.as_ref()) {
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
                            Prim::Of if values.len()==2 && (matches!(&values[0], Value::Thing(t) if t.of.presentation.is_some()) || matches!(&values[0], Value::Blueprint(c) if c.presentation.is_some()) || matches!(&values[0], Value::Routine(_) | Value::Method(..) | Value::Bound(..) | Value::Wrapped(..)) || matches!(&values[0], Value::Intrinsic(_, word) if self.table.spells("ext.stmt.class.builtin", word))) =>return self.read_class_member(values[0].clone(),&values[1].bare(),false),
                            Prim::Onto if values.len()==3=>{ self.context_hushed_by(&values[0],&values[1].bare()); return self.alter_class_member(values[0].clone(),&values[1].bare(),Some(values[2].clone()),false) },
                            Prim::Pluck if values.len()==2=>return self.alter_class_member(values[0].clone(),&values[1].bare(),None,false),
                            _=>{}
                        }
                    }
                    if let Some(done) = self.stands_for_property(*op, &values)? {
                        return Ok(done);
                    }
                    // Text read while the run goes is read where it
                    // stands: inside a routine it knows that routine's
                    // names, as the reference has it. They are set
                    // aside here, where the frame is at hand, for the
                    // reading past here to find, and only where it was
                    // handed no dictionaries of its own does it look.
                    let aside = match self.reads_manners() && matches!(op, Prim::Weigh | Prim::Perform) && !Rc::ptr_eq(frame, &self.outermost) {
                        true => {
                            let mine = self.frame_aside(frame);
                            Some(std::mem::replace(&mut self.text_within, Some(mine)))
                        }
                        false => None,
                    };
                    // A row grown one item at a time by its own literal
                    // or comprehension keeps the same buffer throughout
                    // when nothing else holds it, rather than a clone of
                    // everything gathered so far paid on every item
                    // added to it. A bound name hands its row back
                    // wrapped in the very cell it came from, so a write
                    // through it still reaches every other name sharing
                    // that cell; a plain row is grown and handed back
                    // the same way `prim` would have built it.
                    if let Prim::ExtendLiteral(false, expanded) = *op {
                        let grown = match values.first() {
                            Some(Value::Vector(_)) => true,
                            Some(Value::Shared(cell)) => matches!(&*cell.borrow(), Value::Vector(_)),
                            _ => false,
                        };
                        if grown && values.len() == 2 {
                            let item = values.pop().expect("literal item");
                            let source = values.pop().expect("growing literal");
                            let extra = if expanded { Some(self.gathered_members(&item)?) } else { None };
                            match source {
                                Value::Vector(mut prior) => {
                                    match extra { Some(more) => Rc::make_mut(&mut prior).extend(more), None => Rc::make_mut(&mut prior).push(item) }
                                    return Ok(Value::Vector(prior));
                                }
                                Value::Shared(cell) => {
                                    if let Value::Vector(prior) = &mut *cell.borrow_mut() {
                                        match extra { Some(more) => Rc::make_mut(prior).extend(more), None => Rc::make_mut(prior).push(item) }
                                    }
                                    return Ok(Value::Shared(cell));
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                    // A map grown one key at a time by its own literal or
                    // comprehension keeps the same store throughout when
                    // nothing else holds it, so a key written into it
                    // finds its own place through the store's own
                    // text-keyed index rather than a scan of every key
                    // already there, and the store grows in step with the
                    // pairs instead of being thrown away and rebuilt for
                    // the next key. A bound name hands its map back
                    // wrapped in the very cell it came from, so a write
                    // through it still reaches every other name sharing
                    // that cell.
                    if let Prim::ExtendLiteral(true, expanded) = *op {
                        let grown = match values.first() {
                            Some(Value::Dict(_)) => true,
                            Some(Value::Shared(cell)) => matches!(&*cell.borrow(), Value::Dict(_)),
                            _ => false,
                        };
                        if grown && values.len() == 2 {
                            let item = values.pop().expect("literal item");
                            let source = values.pop().expect("growing literal");
                            match source {
                                Value::Dict(mut prior) => {
                                    self.extend_dict_grown(&mut prior, &item, expanded)?;
                                    return Ok(Value::Dict(prior));
                                }
                                Value::Shared(cell) => {
                                    let mut held = cell.borrow_mut();
                                    if let Value::Dict(prior) = &mut *held {
                                        self.extend_dict_grown(prior, &item, expanded)?;
                                    }
                                    drop(held);
                                    return Ok(Value::Shared(cell));
                                }
                                _ => unreachable!(),
                            }
                        }
                    }
                    let made = self.prim(*op, name, &values);
                    if let Some(held) = aside { self.text_within = held; }
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

    /// The arguments of a call upon a bound member of a value, with
    /// whatever was spread opened out: a row spread hands over its
    /// members and a map spread its pairs, each keyed as it was
    /// written, which is what every other callee is handed.
    fn opened_arguments(&mut self, values: Vec<Value>) -> Res<Vec<Value>> {
        let (mut given, keywords) = self.open_arguments(values)?;
        given.extend(keywords.into_iter().map(|(word, value)| Value::Couple(Rc::new((Value::text(&word), value)))));
        Ok(given)
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
            if *work == crate::text::Work::FORMATMAP {
                if positional.len() != 2 { return Err(crate::text::complaint(self.table, "arguments").into()); }
                return Ok(Value::text(&self.mapping_format(subject, &positional[1], 0)?));
            }
            Ok(crate::text::apply(self.table, *work, name, &positional, self.wording())?)
        })())
    }

    fn word_it_spells(&mut self, stands: &Value, args: &[Form], frame: &Rc<Env>) -> Option<Res<Value>> {
        let word = match stands { Value::Text(word) | Value::Intrinsic(_, word) => word, _ => return None };
        let op = self.table.prims.get(word.as_ref()).copied()?;
        let name = word.to_string();
        Some((|| {
            let mut values = self.value_list(args, frame)?;
            if op == Prim::ClassWork(11) && self.has_class_order() && !self.detail("descriptor.get").is_empty() { return self.work_on_class(11, values); }
            if matches!(stands, Value::Intrinsic(..)) && self.names_in_calls {
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
            Value::Bound(p, env) => Ok(self.as_now_written(p, env)),
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

    /// What a value of a native kind answers to by name, gathered for
    /// the kind read as a class: a sample of the kind is asked, so what
    /// the kind names and what a value of it answers stay one thing.
    pub(super) fn kind_member_names(&self, word: &str) -> Vec<String> {
        let Some(sample) = self.kind_stand_in(word) else { return Vec::new() };
        let mut gathered = Vec::new();
        for name in self.table.strings("ext.stmt.class.special") {
            if self.native_member(&sample, name) { gathered.push(name.clone()); }
        }
        gathered
    }

    /// An empty value of the native kind that word names, standing in
    /// for the kind itself wherever the kind is asked what its values
    /// can do. Nothing where the word names no native kind.
    pub(super) fn kind_stand_in(&self, word: &str) -> Option<Value> {
        let walking = ["generator", "reversed", "filter", "map", "zip", "enumerate", "callable_iterator", "bytearray_iterator", "bytes_iterator", "dict_reverseitemiterator", "dict_reversevalueiterator", "dict_reversekeyiterator", "dict_itemiterator", "dict_valueiterator", "dict_keyiterator", "set_iterator", "longrange_iterator", "range_iterator", "str_iterator", "str_ascii_iterator", "tuple_iterator", "list_reverseiterator", "list_iterator", "iterator"];
        if walking.contains(&word) {
            let empty = IteratorKind::Stored(std::collections::VecDeque::new());
            return Some(Self::cursor_value_walked(empty, Some(Rc::from(word))));
        }
        let portion = match word {
            "dict_items" => Some('i'), "dict_values" => Some('v'), "dict_keys" => Some('k'), "mappingproxy" => Some('m'), _ => None,
        };
        if let Some(part) = portion {
            let dictionary = Value::Dict(Rc::new(Vec::new().into()));
            return Some(Value::Window(Rc::new(dictionary), part));
        }
        Some(match self.table.prims.get(word)? {
            Prim::AsText => Value::text(""),
            Prim::AsInt => Value::Small(0),
            Prim::Truthful => Value::Flag(false),
            Prim::AsReal => crate::data::worth_of_binary(0.0, crate::math::DEFAULT_PLACES),
            Prim::ComplexMade => crate::complex::pair(self.table, 0.0, 0.0),
            Prim::Span => Value::Progression(Rc::new(crate::data::Progression {
                first: BigInt::from(0), limit: BigInt::from(0), stride: BigInt::from(1), word: word.to_owned(),
            })),
            Prim::Listed => Value::Vector(Rc::new(Vec::new())),
            Prim::Tupling => Value::Tuple(Rc::new(Vec::new())),
            Prim::Dictionary => Value::Dict(Rc::new(Vec::new().into())),
            kind @ (Prim::Uniques | Prim::Unchanging) => Value::Set(Rc::new(RefCell::new(crate::data::SetStore::new(word, *kind == Prim::Unchanging)))),
            Prim::Octets(kind) => Value::Octets { cell: Rc::new(RefCell::new(Vec::new())), changeable: *kind == 1, lead: Rc::from(word) },
            Prim::SpanOf => Value::Span(Rc::new(vec![Value::Nil, Value::Nil, Value::Nil])),
            _ => return None,
        })
    }

    /// Every member a value of a native kind answers to by name: the
    /// special names its mark answers, and the methods of its kind,
    /// spelled as the table spells them. A spelling that names the kind
    /// before the method leads to the kind's own maker rather than to a
    /// member of a value, so it is passed over.
    pub(super) fn native_directory(&self, sample: &Value) -> Vec<String> {
        // A span keeps its bounds and answers to nothing else the mark
        // mechanism below reckons, its own mark standing for no working
        // a program writes in the plain way.
        if let Value::Span(_) = sample { return vec!["start".to_string(), "step".to_string(), "stop".to_string()]; }
        let Some(mark) = Self::native_mark(sample) else { return Vec::new() };
        let mut gathered = Vec::new();
        for name in self.table.strings("ext.stmt.class.special") {
            if self.native_member(sample, name) { gathered.push(name.clone()); }
        }
        for (label, operation) in crate::table::BUILTIN_LABELS {
            let wanted = match operation {
                Prim::ValueMethod => match label.strip_prefix("ext.builtin.method.") {
                    Some(working) => crate::members::answers_to(sample, working),
                    None => false,
                },
                // Text, a set and a run of bytes answer further methods
                // through primitives of their own. Neither the extent of
                // text nor how it shows is a method of it, and of the
                // bytes primitives only those a run of bytes answers to.
                Prim::Textual(work) => mark == 's' && !matches!(work, crate::text::Work::LENGTH | crate::text::Work::REPR),
                Prim::SetCall(code @ 1..=17) => mark == 'e' || mark == 'E' && !Self::set_alters(code),
                Prim::Octets(3 | 4 | 6..=13) => "bB".contains(mark),
                _ => false,
            };
            if !wanted { continue; }
            gathered.extend(self.table.strings(label).iter().filter(|word| !word.contains('.')).cloned());
        }
        // A count of a row keeps its bounds beside the places it answers
        // through the methods above.
        if mark == 'p' { gathered.extend(["start", "stop", "step"].iter().map(|s| s.to_string())); }
        gathered.sort();
        gathered.dedup();
        gathered
    }

    /// The letter that stands for what a native value is, read through
    /// whatever cells lie between a name and the value itself. The
    /// letter settles both which special members the value answers to
    /// and which values those members will work with. A window upon a
    /// map is marked before it settles, since settling leaves a plain
    /// row with none of a window's ways, and the window upon the values
    /// is marked apart from the other two.
    fn native_mark(value: &Value) -> Option<char> {
        let mut held = value.clone();
        while let Value::Mutable(cell, _) | Value::Shared(cell) = held { let inner = cell.borrow().clone(); held = inner; }
        Some(match held {
            Value::Iterator(_) | Value::Generator(_) => 'w',
            Value::Window(_, portion) => if portion == 'v' { 'V' } else { 'W' },
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) => 'n',
            Value::Frac(number) => if number.float_style { 'r' } else { 'q' },
            Value::Complex(_) => 'c',
            Value::Text(_) => 's',
            Value::Octets { changeable, .. } => if changeable { 'B' } else { 'b' },
            Value::Vector(_) => 'l',
            Value::Tuple(_) | Value::Row(_) => 't',
            Value::Dict(_) => 'd',
            Value::Set(ref store) => if store.try_borrow().map_or(false, |held| held.sealed) { 'E' } else { 'e' },
            Value::Progression(_) => 'p',
            _ => return None,
        })
    }

    /// Whether a value marked so answers the special name standing at
    /// that place of the table. A walk hands over its next member and
    /// itself; whatever is walked over hands over a walk of it; the
    /// holders answer for their extent, for a place read of them and
    /// for membership; a row, a map and a changeable run of bytes
    /// answer for a place written into and one taken out; and every
    /// kind that stands by itself answers for how it reads, how it
    /// compares, and the signs its own kind is written with.
    fn mark_answers(mark: char, at: usize) -> bool {
        let counts = "nrqc".contains(mark);
        let lined = "sbBltp".contains(mark);
        let holds = lined || "deE".contains(mark);
        let joins = "sbBlt".contains(mark);
        // Both set kinds answer for the workings a set is written with;
        // only the alterable one answers for a write through them, and
        // only the sealed one for a hash of its own.
        let uniques = "eE".contains(mark);
        // A walk and a window both stand for something else, which is
        // what reads and compares, so neither answers for itself.
        let alone = !"wWV".contains(mark);
        match at {
            0..=7 => alone,
            8 => alone && !"ldeB".contains(mark),
            // A sealed set has a hash of its own; the eighth place
            // above leaves the alterable set out and lets this stand.

            9 => counts,
            10 => holds || "WV".contains(mark),
            11 => holds && !uniques,
            // A run of bytes is written into place by place and gives
            // a place up again, as a row does.
            12 => "ldB".contains(mark),
            13 => "ldB".contains(mark),
            14 => holds || mark == 'W',
            15 => holds || "wWV".contains(mark),
            16 => mark == 'w',
            42 => "ldpWV".contains(mark),
            18 | 20 | 28 => counts || joins,
            19 => counts || uniques,
            21 | 24 | 25 | 26 | 27 | 29 | 32 | 38 | 39 | 40 | 41 => counts,
            22 | 30 | 60 | 61 => counts && mark != 'c',
            23 | 31 => counts && mark != 'c' || "sbB".contains(mark),
            43 | 44 | 62 | 63 | 67 | 68 => mark == 'n',
            47 | 49 => "lB".contains(mark),
            48 | 57 | 59 => mark == 'e',
            58 => "ed".contains(mark),
            64 | 66 | 69 | 71 => mark == 'n' || uniques,
            65 | 70 => "nd".contains(mark) || uniques,
            // Every worth is laid out to a specification, the layout
            // having marks of its own for each kind or words against
            // the kinds that take none.
            74 | 79 => mark == 'c',
            72 => true,
            // A walk answers a guess at how many members it has left,
            // where the table keeps one for a walk of its own kind.
            78 => mark == 'w',
            _ => false,
        }
    }

    /// Whether a numbered set working alters the set it is handed. A
    /// sealed set answers to none of these: the words name no member of
    /// it whatever, as the reference has it.
    fn set_alters(code: u8) -> bool {
        matches!(code, 1..=5 | 7 | 15..=17)
    }

    /// Whether a working of the kind marked first takes a value of the
    /// kind marked second, or hands it back undone so that the other
    /// side of the pair may answer. A whole number works with whole
    /// numbers alone, a real with those and reals, a run of bytes with
    /// either run, and every holder with its own kind.
    fn mark_accepts(mark: char, other: Option<char>) -> bool {
        let Some(other) = other else { return false };
        match mark {
            'n' => other == 'n',
            'r' => "nr".contains(other),
            'q' => "nrq".contains(other),
            'c' => "nrc".contains(other),
            'b' | 'B' => "bB".contains(other),
            // Either set kind works with either, so that a set and a
            // sealed one meet under the one sign.
            'e' | 'E' => "eE".contains(other),
            _ => mark == other,
        }
    }

    /// Whether the member at that place hands back what it cannot take
    /// rather than refusing it outright. The comparisons and the
    /// workings a number is written with do; the joining and the
    /// repeating a row is written with refuse, as their plain signs do.
    fn mark_defers(mark: char, at: usize) -> bool {
        if "sbBlt".contains(mark) && matches!(at, 18 | 20 | 23 | 28 | 31) { return false; }
        matches!(at, 2..=7 | 18..=24 | 26..=32 | 60..=71)
    }

    /// Where among the table's special names this one stands, when a
    /// value of a native kind answers to it as a member of its own.
    pub(super) fn native_place(&self, value: &Value, name: &str) -> Option<usize> {
        let mark = Self::native_mark(value)?;
        let at = self.table.strings("ext.stmt.class.special").iter().position(|word| word == name)?;
        Self::mark_answers(mark, at).then_some(at)
    }

    pub(super) fn native_member(&self, value: &Value, name: &str) -> bool {
        self.native_place(value, name).is_some()
    }

    /// The cell a native holder's names share, followed through however
    /// many cells stand between the name and the holder itself.
    fn native_cell(value: &Value) -> Option<Rc<RefCell<Value>>> {
        let (Value::Mutable(cell, _) | Value::Shared(cell)) = value else { return None };
        let deeper = match &*cell.borrow() {
            held @ (Value::Mutable(..) | Value::Shared(_)) => Some(held.clone()),
            _ => None,
        };
        match deeper { Some(held) => Self::native_cell(&held), None => Some(cell.clone()) }
    }

    /// A working run for a special member, by whichever door the
    /// primitive is reached through.
    fn native_working(&mut self, work: Prim, name: &str, operands: Vec<Value>) -> Res<Value> {
        if Self::is_core_primitive(work) { return self.core_primitive(work, name, operands, Vec::new()).map_err(Escape::from); }
        self.prim(work, name, &operands).map_err(Escape::from)
    }

    /// A special member of a native value, run by the place its name
    /// stands at. Nothing is reckoned afresh here: the reading, the
    /// comparing, the signs, the extent, the walk, the place written
    /// into and the place taken out are the ones the plain forms run,
    /// so the answers and the refusals are the plain forms' own.
    fn native_member_run(&mut self, receiver: &Value, name: &str, at: usize, arguments: Vec<Value>, keywords: Vec<(String, Value)>) -> Res<Value> {
        let wanted = match at {
            12 => 2,
            2..=7 | 11 | 13 | 14 | 18..=24 | 26..=32 | 47..=59 | 60..=72 => 1,
            _ => 0,
        };
        if !keywords.is_empty() || arguments.len() != wanted { return Err(self.method_fault("arguments").into()); }
        let mark = Self::native_mark(receiver).ok_or_else(|| self.bad_answer())?;
        match (mark, at) {
            ('c', 74) => return Ok(receiver.settled()),
            ('c', 79) => {
                let z = crate::complex::coordinates(&receiver.settled()).expect("complex coordinates");
                let row = [z.0,z.1].into_iter().map(crate::complex::decimal_value).collect();
                return Ok(Value::Tuple(Rc::new(row)));
            }
            ('c', 4..=7) => return Ok(Value::Refusal(Rc::from(self.table.single("ext.stmt.class.special.declined").unwrap_or_default()))),
            _ => {}
        }
        // Handed a value of a kind it does not take, the working hands
        // it back undone and the other side of the pair may answer.
        if wanted == 1 && Self::mark_defers(mark, at) && !Self::mark_accepts(mark, if mark == 'c' && matches!(arguments[0].settled(), Value::Frac(ref number) if number.places.is_some()) { Some('r') } else { Self::native_mark(&arguments[0]) }) {
            let word = self.table.strings("ext.stmt.class.special.declined").first().map_or(String::new(), String::clone);
            return Ok(Value::Refusal(Rc::from(word.as_str())));
        }
        // The members a native value answers with a working of one.
        if let Some(work) = match at {
            0 => Some(Prim::AsText), 1 => Some(Prim::Quoted), 8 => Some(Prim::Hashed), 9 => Some(Prim::Truthful),
            10 => Some(Prim::Length), 15 => Some(Prim::Iterator), 16 => Some(Prim::NextItem),
            25 => Some(Prim::Negate), 38 | 43 => Some(Prim::AsInt), 39 => Some(Prim::AsReal),
            40 => Some(Prim::Magnitude), 41 => Some(Prim::Positive), 44 => Some(Prim::BitsOver),
            42 => Some(Prim::Backwards),
            _ => None,
        } { return self.native_working(work, name, vec![receiver.clone()]); }
        // A guess at how many members a walk has left, for the kinds
        // of walk this kernel can answer that of without asking
        // anything further of what it walks: a stored row and a
        // progression read the guess straight from the place they
        // stand at, and a walk taken by place from a thing's own
        // `__getitem__` from how far its last place still stands
        // above nought.
        if at == 78 {
            let Value::Iterator(cell) = receiver else { return Ok(Value::Small(0)); };
            let placed_back = { let held = cell.borrow(); match &held.kind {
                IteratorKind::Stored(entries) => return Ok(Value::Small(entries.len() as i64)),
                IteratorKind::Stepping(walk, at) => {
                    let left = walk.count() - at;
                    return Ok(Value::from_big(if left > BigInt::from(0) { left } else { BigInt::from(0) }));
                }
                IteratorKind::PlacedBack(thing, at) => {
                    if held.done || *at < BigInt::from(0) { return Ok(Value::Small(0)); }
                    Some((thing.clone(), at.clone()))
                }
                _ => None,
            }};
            // A walk taken by place from a thing's own `__getitem__`
            // asks that thing's `__len__` afresh each time, exactly as
            // the reference does, rather than trusting the length the
            // walk itself was given when it began.
            if let Some((thing, at)) = placed_back {
                let length = self.prim(Prim::Length, "len", &[thing]).map_err(Escape::Error)?;
                let size = match &length { Value::Small(_) | Value::Huge(_) | Value::Flag(_) => length.as_big(), _ => Err(self.core_complaint("core.integer", &length.kind_word())) }?;
                let hint = at + BigInt::from(1);
                return Ok(Value::from_big(if size < hint { size } else { hint }));
            }
            return Ok(Value::Small(0));
        }
        // The specification a worth is laid out to, asked for under the
        // name the protocol gives it. The layout is the one a field of
        // a template is given, and so are the refusals.
        if at == 72 {
            let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
            let given = arguments[0].settled();
            let Value::Text(spec) = &given else { return Err(layout.complain("ext.text.format.spec.type", &[layout.typename(&given)]).into()) };
            return Ok(Value::text(&layout.present(&receiver.settled(), spec, "")?));
        }
        // A whole and a remainder answered together, either way about.
        if matches!(at, 60 | 61) {
            let pair = if at == 60 { [receiver.clone(), arguments[0].clone()] } else { [arguments[0].clone(), receiver.clone()] };
            return self.native_working(Prim::QuotRem, name, pair.to_vec());
        }
        // A compound sign written as a member is the landing that sign
        // makes, which grows a row or a set where it stands.
        if (47..=59).contains(&at) {
            return self.native_working(Prim::Landing((at - 47) as u8), name, vec![receiver.clone(), arguments[0].clone()]);
        }
        // A span names a run of places at once, and a run written into
        // is not one place written into: the row takes what comes where
        // it stands, as it does for a span written to by its sign.
        if at == 12 {
            if let Value::Span(bounds) = arguments[0].clone() {
                match Self::native_cell(receiver) {
                    Some(cell) => self.span_written(&mut cell.borrow_mut(), &bounds, &arguments[1])?,
                    None => self.span_written(&mut receiver.settled(), &bounds, &arguments[1])?,
                }
                return Ok(Value::Nil);
            }
        }
        // A place written into and a place taken out. Whichever of the
        // two does not write where the holder stands is answered afresh,
        // and what came back goes into the cell every name for the
        // holder shares. Neither is worth anything, as the table has it.
        if matches!(at, 12 | 13) {
            let mut operands = vec![receiver.settled()];
            operands.extend(arguments.iter().cloned());
            if at == 12 && matches!(operands[0], Value::Vector(_)) {
                if let Some(index) = self.stood_for_whole(&operands[1])? { operands[1] = index; }
            }
            // What goes into a place goes in as it stands: a worth
            // read out of its cell first would be a copy, and a name
            // for it would no longer be a name for what the holder
            // keeps. The working is therefore asked for straight.
            let changed = match at {
                12 => self.prim_values(Prim::Placed, name, &operands).map_err(Escape::from)?,
                _ => self.prim(Prim::Erase, name, &operands)?,
            };
            if let Some(cell) = Self::native_cell(receiver) { cell.replace(changed.settled()); }
            return Ok(Value::Nil);
        }
        // The signs of a pair. A member read the other way about is the
        // same sign with the value it was asked of on the right, and
        // membership is asked with the member before the holder.
        let (work, about) = match at {
            2 => (Prim::Eq, false), 3 => (Prim::Ne, false), 4 => (Prim::Lt, false), 5 => (Prim::Le, false),
            6 => (Prim::Gt, false), 7 => (Prim::Ge, false), 11 => (Prim::At, false), 14 => (Prim::Contains, true),
            18 => (Prim::Plus, false), 26 => (Prim::Plus, true),
            19 => (Prim::Minus, false), 27 => (Prim::Minus, true),
            20 => (Prim::Times, false), 28 => (Prim::Times, true),
            21 => (Prim::OverReal, false), 29 => (Prim::OverReal, true),
            22 => (Prim::IntDiv, false), 30 => (Prim::IntDiv, true),
            23 => (Prim::Mod, false), 31 => (Prim::Mod, true),
            24 => (Prim::Power, false), 32 => (Prim::Power, true),
            62 => (Prim::BitsUp, false), 67 => (Prim::BitsUp, true),
            63 => (Prim::BitsDown, false), 68 => (Prim::BitsDown, true),
            64 => (Prim::BitsBoth, false), 69 => (Prim::BitsBoth, true),
            65 => (Prim::BitsEither, false), 70 => (Prim::BitsEither, true),
            66 => (Prim::BitsOne, false), 71 => (Prim::BitsOne, true),
            _ => return Err(self.method_fault("arguments").into()),
        };
        let pair = if about { [arguments[0].clone(), receiver.clone()] } else { [receiver.clone(), arguments[0].clone()] };
        self.native_working(work, name, pair.to_vec())
    }

    /// The working a value's method of this name stands for, where the
    /// table gives one a builtin kind answers to.
    fn value_method_named(&self, name: &str) -> Option<String> {
        crate::table::BUILTIN_LABELS.iter()
            .find(|(label, prim)| *prim == Prim::ValueMethod && self.table.spells(label, name))
            .map(|(label, _)| label.strip_prefix("ext.builtin.method.").unwrap_or(label).to_string())
    }

    /// A pair of a thing and a method's name, standing where a routine
    /// would: that method of that thing, the thing handed over first.
    /// A class in the first place names a method of the class itself.
    /// What a native kind carries under a name, asked of the kind's own
    /// word rather than of a value of it. Such an entry stands loose:
    /// the value it works upon is the first it is handed when it is
    /// called, as an unbound method is handed one. Nothing where the
    /// word names no native kind, or where that kind carries nothing
    /// under the name.
    pub(super) fn carried_by_kind(&self, value: &Value, name: &str) -> Option<Value> {
        let word: String = match value {
            Value::Blueprint(b) => Self::native_word(b)?,
            Value::Intrinsic(_, word) => word.to_string(),
            Value::OctetKind { changeable, .. } => self.octet_kind_word(*changeable).to_string(),
            _ => return None,
        };
        let stand_in = self.kind_stand_in(&word)?;
        if self.native_directory(&stand_in).binary_search(&name.to_string()).is_err() { return None; }
        Some(Value::Wrapped(60, Rc::new(vec![Value::text(&word), Value::text(name)])))
    }

    pub(super) fn attribute(&self, value: &Value, name: &str) -> Option<Value> {
        if let Some(carried) = self.carried_by_kind(value, name) { return Some(carried); }
        // A walk over a routine's own body answers whether it is on the
        // way through the machine at this very moment: exactly when the
        // cell that holds it cannot be borrowed a second time.
        if let Value::Generator(state) = value {
            if self.table.strings("ext.stmt.yield.running").first().map_or(false, |w| w == name) { return Some(Value::Flag(state.try_borrow().is_err())); }
        }
        let names = self.table.strings("ext.stmt.class.special");
        if let Value::Span(bounds) = value {
            return self.span_bound_named(name).map(|i| bounds[i].clone());
        }
        // A stepped walk holds three numbers and no places; each of the
        // three answers to its own word.
        if let Value::Progression(walk) = value {
            if let Some(which) = self.walk_member_named(name) {
                return Some(Value::from_big(match which { 0 => walk.first.clone(), 1 => walk.limit.clone(), _ => walk.stride.clone() }));
            }
        }
        match value {
            Value::Thing(t) if names.get(35).map_or(false, |s| s == name) => return Some(Value::Blueprint(t.of.clone())),
            Value::Thing(t) if names.get(36).map_or(false, |s| s == name) => return Some(Value::Attributes(t.clone())),
            Value::Blueprint(c) if names.get(37).map_or(false, |s| s == name) => return Some(Value::text(&c.name)),
            _ => (),
        }
        if self.table.single("ext.builtin.class.name") == Some(name) {
            if let Value::Blueprint(kind) = value { return Some(Value::text(&kind.name)); }
            if let Value::Intrinsic(_, word) = value { return Some(Value::text(word)); }
            if let Value::KindOf(kind) = value { return Some(Value::text(Value::word_for_kind(*kind))); }
            // Either octet kind is a worth of its own rather than an
            // intrinsic word, so it answers for its name here.
            if let Value::OctetKind { changeable, .. } = value { return Some(Value::text(self.octet_kind_word(*changeable))); }
        }
        if name == self.detail("doc") {
            if let Value::Intrinsic(_, word) = value {
                if let Some(doc) = Self::builtin_kind_doc(word) { return Some(Value::text(doc)); }
            }
        }
        if let (Value::Text(subject), Some(Prim::Textual(work))) = (value, self.table.prims.get(name)) {
            return Some(Value::TextCall { subject: subject.clone(), work: *work, name: Rc::from(name) });
        }
        // A walk, a row, a map or a set hands over the walking pair and
        // the holder's members as a method bound to it.
        if self.native_member(value, name) { return Some(Value::Member(Rc::new(value.clone()), name.to_string())); }
        // So does a value of a builtin kind with the methods its kind
        // gives it, and a set or a row of bytes with the builtins that
        // take the receiver first.
        if let Some(operation) = self.value_method_named(name) {
            if crate::members::answers_to(value, &operation) {
                return Some(Value::Member(Rc::new(value.clone()), operation));
            }
        }
        // A row of bytes answers to the methods its kind keeps, whose
        // words are the ones text goes by, each of them being a text
        // working read byte by byte.
        if let Value::Octets { changeable, .. } = value.settled() {
            if let Some(working) = self.octet_member(name, changeable) {
                return Some(Value::Member(Rc::new(value.clone()), working.to_string()));
            }
        }
        // A set answers some of its methods with primitives that take
        // the receiver first, so the member is the word itself with the
        // set standing behind it. A sealed set holds no altering working
        // of its own, so no member of that name is to be found upon it.
        let setted = match self.table.prims.get(name) {
            Some(Prim::SetCall(code @ 1..=17)) => matches!(value.settled(), Value::Set(_)) && !(Self::set_alters(*code) && value.set_sealed()),
            _ => false,
        };
        if setted {
            return Some(Value::Wrapped(3, Rc::new(vec![Value::text(name), value.settled()])));
        }
        let class = match value {
            Value::Thing(thing) => {
                let fields = thing.holds.borrow();
                if self.is_fault_kind(&thing.of) && (self.fault_method_word(name) || (self.stands_under(&thing.of, 36) && self.table.single("ext.stmt.class.constructor") == Some(name))) {
                    return Some(Value::Member(Rc::new(Value::Thing(thing.clone())), name.to_string()));
                }
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
            return self.fault_from_call(class, args);
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
                if self.has_class_order() && matches!(&stands,Value::Wrapped(..)|Value::Thing(_)|Value::Routine(_)) {
                    let mut given=self.value_list(args,frame)?;
                    if matches!(&stands,Value::Wrapped(..)) { given=self.opened_arguments(given)?; }
                    return Ok(Next::Value(self.apply_class_member(stands,given)?));
                }
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

    /// Three words about a member and what it was sought on: what that
    /// goes by stands between the first two, the member's name between
    /// the last two. Nothing where the table gives no such words.
    fn member_worded(parts: &[String], called: &str, name: &str) -> String {
        if parts.len() != 3 { return String::new(); }
        format!("{}{}{}{}{}", parts[0], called, parts[1], name, parts[2])
    }

    /// The name a namespace was read in under, where this value is that
    /// very namespace and nothing else.
    pub(super) fn namespace_holding(&self, value: &Value) -> Option<String> {
        let Value::Thing(thing) = value else { return None };
        self.imported.iter().find(|(_, held)| matches!(held, Value::Thing(other) if Rc::ptr_eq(other, thing))).map(|(path, _)| path.clone())
    }

    /// The word a value goes by as a kind: a blueprint its own name, a
    /// builtin kind the word spelling it. Nothing where the value is no
    /// kind at all.
    fn kind_word_of(&self, value: &Value) -> Option<String> {
        match value {
            Value::Blueprint(class) => Some(class.name.clone()),
            Value::KindOf(kind) => Some(Value::word_for_kind(*kind).to_string()),
            Value::OctetKind { changeable, .. } => Some(self.octet_kind_word(*changeable).to_string()),
            Value::Intrinsic(op, word) if op.names_a_kind() => Some(word.to_string()),
            other => self.kind_spelling(other).map(|word| word.to_string()),
        }
    }

    /// The complaint for a member sought on a namespace or on a kind.
    /// CPython names each by its own name rather than by the kind it is
    /// of, and words the two differently. Nothing for anything else.
    pub(super) fn member_named_missing(&self, value: &Value, name: &str) -> String {
        let settled = value.settled();
        if let Some(path) = self.namespace_holding(&settled) {
            return Self::member_worded(self.table.strings("ext.builtin.member.absent.module"), &path, name);
        }
        match self.kind_word_of(&settled) {
            Some(word) => Self::member_worded(self.table.strings("ext.builtin.member.absent.class"), &word, name),
            None => String::new(),
        }
    }

    /// The complaint for a member sought on a value of a builtin kind
    /// that answers to no such name.
    pub(super) fn member_missing(&self, value: &Value, name: &str) -> String {
        let told = self.member_named_missing(value, name);
        if !told.is_empty() { return told; }
        Self::member_worded(self.table.strings("ext.builtin.method.error.attribute"), &value.settled().kind_word(), name)
    }

    /// The complaint for a member written on such a value, or taken
    /// away: it keeps no namespace of its own to hold one.
    pub(super) fn member_unwritable(&self, value: &Value, name: &str) -> String {
        let told = Self::member_worded(self.table.strings("ext.builtin.member.unwritable"), &value.settled().kind_word(), name);
        if told.is_empty() { return self.member_missing(value, name); }
        told
    }

    /// What a member standing for a value's method comes to: the method
    /// bound to the value, save that the parts of a number are members
    /// read rather than methods left standing to be called.
    fn method_of_value(&mut self, receiver: Value, operation: &str) -> Result<Value, Escape> {
        // A view of a map's keys, values or pairs keeps a reading of
        // the map itself under this name: a fresh view of its own,
        // read-only, and equal to the map for as long as it stands.
        if operation == "mapping" {
            let raw = match &receiver {
                Value::Window(owner, _) => Some(owner.clone()),
                Value::Mutable(cell, _) | Value::Shared(cell) => match &*cell.borrow() { Value::Window(owner, _) => Some(owner.clone()), _ => None },
                _ => None,
            };
            if let Some(owner) = raw { return Ok(Value::Window(owner, 'm')); }
        }
        if ["numerator", "denominator", "real", "imag"].contains(&operation) {
            match receiver.settled() {
                Value::Complex(pair) => return Ok(crate::complex::decimal_value(if operation == "real" { pair.0 } else { pair.1 })),
                Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Flag(_) => return self.value_member(&receiver, operation, Vec::new(), Vec::new()),
                _ => {}
            }
        }
        Ok(Value::Member(Rc::new(receiver), operation.to_string()))
    }

    /// How the table spells a value's method, so that a complaint names
    /// the member the way a program writes it.
    fn member_spelling(&self, operation: &str) -> String {
        self.table.single(&format!("ext.builtin.method.{operation}")).unwrap_or(operation).to_string()
    }

    /// The starred clauses of a try. Each takes from the raised gatherer
    /// the members under its kinds, a lone fault standing as a gatherer
    /// of one with an empty heading. Whatever the clauses leave goes up
    /// again as it came, a lone fault on its own; whatever they raise
    /// anew goes up with it in a gatherer of empty heading, or alone
    /// when it is the only thing there is to raise.
    fn grouped_clauses(&mut self, clauses: &[Clause], raised: Value, frame: &Rc<Env>) -> Res<Value> {
        let unready = self.argument_fault("ext.builtin.exceptions.unready", None);
        let lone = Self::gathered(&raised).is_none();
        let whole = if lone {
            let Some(base) = self.furnished_kind(37) else { return Err(unready.into()) };
            self.gather_faults(base, Value::text(""), vec![raised.clone()])?
        } else { raised.clone() };
        let mut left = Some(whole.clone());
        let mut anew = Vec::new();
        let mut sent_back = Vec::new();
        let mut something_taken = false;
        for clause in clauses {
            let Some(pending) = left.take() else { break };
            let mut kinds = Vec::new();
            for choice in clause.choices.iter().flatten() {
                match self.value_of(choice, frame)?.settled() {
                    Value::Blueprint(kind) if self.is_fault_kind(&kind) => kinds.push(kind),
                    _ => return Err(self.table.single("ext.stmt.catch.invalid").unwrap_or_default().to_string().into()),
                }
            }
            let (taken, rest) = self.sieve_faults(pending, &Sieve::Kinds(kinds))?;
            left = rest;
            let Some(taken) = taken else { continue };
            something_taken = true;
            if let Some(holding) = self.holding_fault.last_mut() { *holding = taken.clone(); }
            self.under = None;
            self.entering = None;
            if let Some(place) = &clause.held { self.store(place, frame, taken.clone())?; }
            let answer = self.value_of(&clause.body, frame);
            if let Some(place) = &clause.held { self.store(place, frame, Value::Unset)?; }
            match answer {
                Ok(_) => {}
                Err(Escape::Thrown(value)) if matches!((&value, &taken), (Value::Thing(a), Value::Thing(b)) if Rc::ptr_eq(a, b)) => sent_back.push(value),
                Err(Escape::Thrown(value)) => anew.push(value),
                Err(Escape::Error(said)) => match self.as_raised(&said) { Some(value) => anew.push(value), None => return Err(Escape::Error(said)) },
                Err(other) => return Err(other),
            }
        }
        // What went back up and what no clause took stand together
        // again under the heading they came with.
        let remaining: Vec<Value> = sent_back.into_iter().chain(left).collect();
        let remaining = match remaining.len() {
            0 => None,
            1 => remaining.into_iter().next(),
            _ => {
                let Some((kind, heading, _)) = Self::gathered(&whole) else { return Err(unready.into()) };
                let members = remaining.iter().flat_map(|part| Self::gathered(part).map_or_else(|| vec![part.clone()], |(_, _, items)| items)).collect();
                let joined = self.gather_faults(kind, heading, members)?;
                self.write_across(&whole, &joined);
                Some(joined)
            }
        };
        if anew.is_empty() {
            return match remaining {
                None => Ok(Value::Nil),
                Some(_) if lone && !something_taken => Err(Escape::Thrown(raised)),
                Some(rest) => Err(Escape::Thrown(rest)),
            };
        }
        if anew.len() == 1 && remaining.is_none() { return Err(Escape::Thrown(anew.remove(0))); }
        let Some(base) = self.furnished_kind(37) else { return Err(unready.into()) };
        anew.extend(remaining);
        let gathered = self.gather_faults(base, Value::text(""), anew)?;
        Err(Escape::Thrown(gathered))
    }

    pub(super) fn value_member(&mut self, receiver: &Value, name: &str, arguments: Vec<Value>, keywords: Vec<(String, Value)>) -> Res<Value> {
        // One more member set on the end of a list, in the place the
        // list already occupies. This comes first of all: further down
        // the contents are read out into a worth of their own, and a
        // list two worths hold must be copied before either may write
        // to it. The member coming in is looked at for a way home to
        // the cell; the ones already there were looked at on the way in.
        if name == "append" && keywords.is_empty() && arguments.len() == 1 {
            if let Value::Mutable(cell, _) = receiver {
                if matches!(&*cell.borrow(), Value::Vector(_)) {
                    if crate::members::circular(&arguments[0], cell, 1) { return Err(self.method_fault("unready").into()); }
                    if let Value::Vector(members) = &mut *cell.borrow_mut() { Rc::make_mut(members).push(arguments[0].clone()); }
                    return Ok(Value::Nil);
                }
            }
        }
        // A list grown by whatever can be walked: the walk is drawn out
        // first, the list's own working knowing only the kinds it names.
        let mut arguments = arguments;
        if name == "extend" && arguments.len() == 1 && matches!(receiver.settled(), Value::Vector(_)) {
            let plain = matches!(arguments[0].settled(), Value::Vector(_) | Value::Tuple(_) | Value::Row(_) | Value::Set(_) | Value::Dict(_) | Value::Text(_) | Value::Progression(_));
            if !plain { let drawn = self.gathered_members(&arguments[0])?; arguments[0] = Value::Vector(Rc::new(drawn)); }
        }
        // A special member asked for by name on a value of a native
        // kind. Each hands its work to the primitive that already does
        // it, so that the answer and the refusal are the ones the plain
        // form gives.
        if let Some(at) = self.native_place(receiver, name) {
            return self.native_member_run(receiver, name, at, arguments, keywords);
        }
        if let Value::Thing(thing) = receiver.settled() {
            if self.is_fault_kind(&thing.of) {
                if !keywords.is_empty() { return Err(self.argument_fault("ext.builtin.exceptions.unready", None).into()); }
                return self.fault_method(thing, name, &arguments);
            }
        }
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
        // A tuple answers to the two methods that only look through it,
        // and a row searched for a value it does not hold names that
        // value; the words for both are the table's.
        if self.works_sequences() && matches!(name, "index" | "count" | "remove") && matches!(&actual, Value::Tuple(_) | Value::Vector(_)) && (name != "remove" || matches!(&actual, Value::Vector(_))) {
            let (Value::Tuple(items) | Value::Vector(items)) = &actual else { unreachable!() };
            let most = if name == "index" { 3 } else { 1 };
            if !keywords.is_empty() || arguments.is_empty() || arguments.len() > most {
                return Err(self.method_fault("arguments").into());
            }
            let counted = |x: &Value| -> Result<i64, String> { match x.settled() {
                Value::Small(n) => Ok(n), Value::Flag(t) => Ok(i64::from(t)),
                Value::Huge(n) => Ok(n.to_i64().unwrap_or(if *n < BigInt::from(0) { i64::MIN } else { i64::MAX })),
                _ => Err(self.method_fault("arguments")),
            }};
            let inside = |n: i64| -> usize {
                let place = if n < 0 { n.saturating_add(items.len() as i64) } else { n };
                place.max(0) as usize
            };
            let from = match arguments.get(1) { Some(x) => inside(counted(x)?), None => 0 };
            let upto = match arguments.get(2) { Some(x) => inside(counted(x)?), None => items.len() };
            let end = arguments.get(2).map_or(usize::MAX, |_| upto);
            let mut total = 0;
            let mut position = from;
            while position < end {
                let contents = receiver.settled();
                let (Value::Vector(row) | Value::Tuple(row)) = contents else { break };
                let Some(value) = row.get(position) else { break };
                let agrees = self.member_agrees(value, &arguments[0])
                    .map_err(|words| self.got_away.take().unwrap_or(Escape::Error(words)))?;
                if agrees {
                    match name {
                        "index" => return Ok(Value::Small(position as i64)),
                        "remove" => {
                            if let Value::Mutable(storage, _) = receiver {
                                if let Value::Vector(values) = &mut *storage.borrow_mut() {
                                    if values.len() > position { Rc::make_mut(values).remove(position); }
                                }
                            }
                            return Ok(Value::Nil);
                        }
                        _ => total += 1,
                    }
                }
                position += 1;
            }
            if name == "count" { return Ok(Value::Small(total)); }
            if name == "remove" { return Err(self.method_fault("remove").into()); }
            let missing = match &actual {
                Value::Tuple(_) => self.sequence_piece("missing", 2).to_owned(),
                _ => format!("{}{}{}", self.sequence_piece("missing", 0),
                    arguments[0].quoted(false), self.sequence_piece("missing", 1)),
            };
            return Err(missing.into());
        }

        if name == "encode" && matches!(&actual, Value::Text(_) | Value::Unpaired(_)) {
            let mut options = arguments;
            for (key, value) in keywords {
                let place = match key.as_str() { "encoding"=>0, "errors"=>1, _=>return Err(self.octet_error("unready").into()) };
                if options.len() > place { return Err(self.octet_error("arguments").into()); }
                while options.len() < place { options.push(Value::text("utf-8")); }
                options.push(value);
            }
            if options.len() > 2 { return Err(self.octet_error("arguments").into()); }
            options.insert(0, actual);
            return self.octet_routine(2, &options).map_err(|words| self.got_away.take().unwrap_or(Escape::Error(words)));
        }
        if matches!(&actual, Value::Octets { .. }) || name == "encode" && matches!(&actual, Value::Text(_) | Value::Unpaired(_)) {
            // A working that writes where the row lies is asked of a
            // changeable row alone; a fixed row has no such member.
            let changeable = matches!(&actual, Value::Octets { changeable: true, .. });
            let operation = if name == "encode" { Some(2) } else { self.octet_member(name, changeable).and_then(Self::octet_operation) };
            if let Some(operation) = operation {
                let mut values = vec![actual]; values.extend(arguments);
                for (key, value) in keywords {
                    let slot = match (operation, key.as_str()) { (3, "encoding") => 1, (3, "errors") => 2, _ => return Err(self.octet_error("arguments").into()) };
                    if values.len() > slot { return Err(self.octet_error("arguments").into()); }
                    while values.len() < slot { values.push(Value::text("utf-8")); }
                    values.push(value);
                }
                // A row lengthened by a walk takes the walk's members
                // for its bytes, so the walk is drawn out into a row
                // first, as the maker of a row of bytes draws one out.
                if operation == 53 && values.len() == 2
                    && !matches!(values[1].settled(), Value::Octets { .. } | Value::Vector(_) | Value::Tuple(_) | Value::Row(_) | Value::Text(_)) {
                    let source = values[1].settled();
                    if let Ok(members) = self.core_collect(&source) { values[1] = Value::Vector(Rc::new(members)); }
                }
                return self.octet_routine(operation, &values).map_err(|words| self.got_away.take().unwrap_or(Escape::Error(words)));
            }
        }
        if matches!(&actual, Value::Text(_)) {
            let key = format!("ext.builtin.text.{}", name);
            if let Some((_, Prim::Textual(work))) = crate::table::BUILTIN_LABELS.iter().find(|(label, _)| *label == key) {
                let mut given = vec![actual]; given.extend(arguments.into_iter().map(|v| match v.settled() { Value::Tuple(row)=>Value::Vector(row), other=>other }));
                if *work == crate::text::Work::JOIN && given.len() == 2 { given[1] = Value::Vector(Rc::new(self.gathered_members(&given[1])?)); }
                crate::text::fit_names(self.table, *work, &mut given, keywords)?;
                if *work == crate::text::Work::FORMATMAP {
                    if given.len() != 2 { return Err(crate::text::complaint(self.table, "arguments").into()); }
                    let Value::Text(s) = &given[0] else { return Err(crate::text::complaint(self.table, "receiver").into()); };
                    let s = s.to_string();
                    return Ok(Value::text(&self.mapping_format(&s, &given[1], 0)?));
                }
                return crate::text::apply(self.table, *work, name, &given, self.wording()).map_err(Escape::from);
            }
        }
        if let Value::Progression(walk) = &actual {
            if matches!(name, "index" | "count") {
                if !keywords.is_empty() || arguments.len() != 1 { return Err(self.method_fault("arguments").into()); }
                let along = Self::walk_position(walk, &arguments[0]);
                if name == "count" { return Ok(Value::Small(if along.is_some() { 1 } else { 0 })); }
                return match along {
                    Some(position) => Ok(Value::from_big(position)),
                    None => Err(self.walk_fault("ext.builtin.range.missing", Some(&arguments[0].quoted(false))).into()),
                };
            }
        }
        if matches!(receiver.settled(), Value::Set(_)) {
            let code = match name { "remove" => Some(2), "pop" => Some(4), "clear" => Some(5), "copy" => Some(6), "update" => Some(7), _ => None }
                .filter(|code| !(Self::set_alters(*code) && receiver.set_sealed()));
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

        // Text asked to fill a template is laid out by the layout the
        // table gives, whether the member was named on the text where
        // it stands or taken from it and called afterwards, so that
        // both ways reach a thing's own methods alike.
        if name == "format" && self.table.has_any("ext.builtin.format") {
            if let Value::Text(pattern) = receiver.settled() {
                let pattern = pattern.to_string();
                let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
                let filled = layout.interpolate(&pattern, &arguments, &keywords, self)?;
                return Ok(Value::text(&filled));
            }
        }

        let keywords = if name == "split" || name == "rsplit" {
            keywords.into_iter().map(|(written, value)| {
                let purpose = if self.table.spells("ext.builtin.method.split.sep", &written) { "sep" }
                    else if self.table.spells("ext.builtin.method.split.maxsplit", &written) { "maxsplit" } else { "" };
                (purpose.to_string(), value)
            }).collect()
        } else { keywords };
        // A map's own methods that look up a key take the very road the
        // subscript takes rather than `Request`'s free `same_item`,
        // which cannot call `__eq__`: the store's own place first, and
        // the program's own equality only where the place cannot say.
        // `fromkeys` looks up no place of an existing map, but still
        // needs that same equality to dedupe the keys it is given, so
        // it is answered here beside the rest, before a member call
        // with no receiver of its own reaches `Request` at all.
        if name == "fromkeys" {
            if arguments.len() > 1 { return Err(self.method_fault("arguments").into()); }
            let filling = arguments.into_iter().next().unwrap_or(Value::Nil);
            return self.dict_fromkeys(receiver, filling).map_err(Escape::from);
        }
        if matches!(receiver.settled(), Value::Dict(_)) {
            if matches!(name, "get" | "setdefault" | "pop") {
                return self.dict_key_method(receiver, name, arguments).map_err(Escape::from);
            }
            if name == "update" {
                return self.dict_update(receiver, arguments, &keywords).map_err(Escape::from);
            }
        }
        if name != "sort" {
            let says = |kind: &str| self.method_fault(kind);
            let unanswered = |value: &Value, word: &str| self.member_missing(value, &self.member_spelling(word));
            return crate::members::Request { target: receiver, operation: name, given: arguments, named: &keywords, names: self.wording(), complaint: &says, unanswered: &unanswered }.answer().map_err(Escape::from);
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

    /// `str.format_map` fills named fields from a mapping read key by
    /// key, each key taken the very way a subscript takes it: a plain
    /// mapping's own pairs, a program's own class through whatever
    /// `__getitem__` it carries, and a dict subclass's `__missing__`
    /// standing in for a key the mapping does not hold. A path after
    /// the key reads an attribute or a further place the same way a
    /// plain field of `.format` does.
    fn mapping_format(&mut self, s: &str, mapping: &Value, depth: usize) -> Result<String, Escape> {
        if depth > 2 { return Err(crate::text::complaint(self.table, "format").into()); }
        let mut out = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if (c == '{' || c == '}') && chars.peek() == Some(&c) { chars.next(); out.push(c); continue; }
            if c == '}' { return Err(crate::text::complaint(self.table, "format.brace").into()); }
            if c != '{' { out.push(c); continue; }
            let mut field = String::new();
            let mut nested = 0;
            let mut closed = false;
            for c in chars.by_ref() {
                if c == '}' && nested == 0 { closed = true; break; }
                if c == '{' { nested += 1; }
                if c == '}' { nested -= 1; }
                field.push(c);
            }
            if !closed { return Err(crate::text::complaint(self.table, "format.brace").into()); }
            let (head, spec) = field.split_once(':').unwrap_or((&field, ""));
            let (path, conversion) = head.split_once('!').unwrap_or((head, ""));
            let first_end = path.find(['.', '[']).unwrap_or(path.len());
            let key = &path[..first_end];
            if key.is_empty() || key.chars().next().unwrap().is_ascii_digit() {
                return Err(crate::text::complaint(self.table, "format.positional").into());
            }
            let mut value = self.prim(Prim::At, "", &[mapping.clone(), Value::text(key)])?;
            let mut rest = &path[first_end..];
            while !rest.is_empty() {
                value = value.settled();
                if let Some(tail) = rest.strip_prefix('.') {
                    let end = tail.find(['.', '[']).unwrap_or(tail.len());
                    let member = &tail[..end];
                    value = match &value {
                        Value::Thing(t) => t.holds.borrow().iter().find(|(k, _)| k == member).map(|(_, v)| v.clone()),
                        _ => None,
                    }.ok_or_else(|| crate::text::complaint(self.table, "format"))?;
                    rest = &tail[end..];
                } else if let Some(tail) = rest.strip_prefix('[') {
                    let end = tail.find(']').ok_or_else(|| crate::text::complaint(self.table, "format"))?;
                    let asked = &tail[..end];
                    let index = asked.parse::<i64>().map_or_else(|_| Value::text(asked), Value::Small);
                    value = self.prim(Prim::At, "", &[value, index])?;
                    rest = &tail[end + 1..];
                } else { return Err(crate::text::complaint(self.table, "format").into()); }
            }
            let spec = self.mapping_format(spec, mapping, depth + 1)?;
            let names = self.wording();
            let shown = if conversion == "r" || conversion == "a" {
                let mut shown = crate::text::expression(&value, names);
                if conversion == "a" {
                    shown = shown.chars().map(|c| if c.is_ascii() { c.to_string() }
                        else if c as u32 <= 255 { format!("\\x{:02x}", c as u32) }
                        else if c as u32 <= 65535 { format!("\\u{:04x}", c as u32) }
                        else { format!("\\U{:08x}", c as u32) }).collect();
                }
                Value::text(&shown).in_field(names, &spec, "")
            } else { value.in_field(names, &spec, conversion) };
            out.push_str(&shown.ok_or_else(|| crate::text::complaint(self.table, "format"))?);
        }
        Ok(out)
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

    /// Which of a stepped walk's three numbers this word asks for,
    /// where the table has words for them at all.
    fn walk_member_named(&self, name: &str) -> Option<usize> {
        self.table.strings("ext.builtin.range.members").iter().position(|word| !word.is_empty() && word == name)
    }

    /// How far along a stepped walk a worth stands, or nowhere at all.
    /// A worth that is no whole number lies nowhere, and so does one
    /// the stride passes over.
    fn walk_position(walk: &crate::data::Progression, worth: &Value) -> Option<BigInt> {
        let whole = worth.as_big().ok().filter(|n| worth.equals(&Value::from_big(n.clone())))?;
        let forward = walk.stride > BigInt::from(0);
        let reached = if forward { whole >= walk.first && whole < walk.limit } else { whole <= walk.first && whole > walk.limit };
        let along = &whole - &walk.first;
        if !reached || &along % &walk.stride != BigInt::from(0) { return None; }
        Some(along / &walk.stride)
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
                let writer = self.attribute(&sink, &route[2])
                    .ok_or_else(|| self.member_missing(&sink, &route[2]))?;
                let mut first = true;
                for item in positional {
                    if !first { self.apply_held(writer.clone(), vec![Value::text(&join)])?; }
                    first = false;
                    let rendered = self.object_words(&item, false)?;
                    self.apply_held(writer.clone(), vec![Value::text(&rendered)])?;
                }
                self.apply_held(writer, vec![Value::text(&tail)])?;
                if drained {
                    if let Some(word) = table.single("ext.builtin.print.flush") {
                        let flush = self.attribute(&sink, word)
                            .ok_or_else(|| self.member_missing(&sink, word))?;
                        self.apply_held(flush, Vec::new())?;
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
                return Ok(Some(self.walk_over_backwards(source, keys)));
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
                Prim::AsText if table.spells("ext.builtin.to_string.encoding", &key) => 1,
                Prim::AsText if table.spells("ext.builtin.to_string.errors", &key) => {
                    // The error policy may be named on its own, and the
                    // wide encoding is then the one meant.
                    if positional.len() == 1 { positional.push(Value::text(&self.octet_codec_name(Self::WIDE))); }
                    2
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
            if let Some(limit) = self.figure_limit() {
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
    fn figure_limit(&self) -> Option<usize> {
        let names = self.table.strings("ext.builtin.to_int.digits.state");
        let changed = (|| {
            let Value::Thing(module) = self.imported.get(names.first()?)? else { return None };
            let name = names.get(1)?;
            let fields = module.holds.borrow();
            let (_, slot) = fields.iter().find(|(key, _)| key == name)?;
            match slot.settled() {
                Value::Small(number) => usize::try_from(number).ok(),
                _ => None,
            }
        })();
        changed.or_else(|| self.table.count("ext.builtin.to_int.digits"))
    }

    fn figures_allowed(&self, value: &Value) -> Result<(), String> {
        let limit = self.figure_limit().unwrap_or(0);
        if limit == 0 { return Ok(()) }
        let Value::Huge(number) = value.settled() else { return Ok(()) };
        // log10(2) exceeds 30102/100000. Only numbers near the
        // boundary need to be rendered to settle their digit count.
        let bits = number.bits().saturating_sub(1) as u128;
        let definitely_large = bits * 30102 >= (limit as u128) * 100000;
        if definitely_large || number.to_str_radix(10).trim_start_matches('-').len() > limit {
            Err(self.too_many_figures(limit))
        } else { Ok(()) }
    }

    /// The words refusing a whole number of more figures than the table
    /// allows in text, the limit set in the middle of them.
    fn too_many_figures(&self, limit: usize) -> String {
        let (head, tail) = self.table.around("ext.builtin.to_int.digits.amiss").unwrap_or(("", ""));
        format!("{head}{limit}{tail}")
    }

    /// What a stepped walk complains of. The table's words already open
    /// with the class the complaint answers to, so the reply is marked
    /// as told in full and nothing further is written over it.
    fn walk_fault(&self, key: &str, named: Option<&str>) -> String {
        let said = self.argument_fault(key, named);
        match said.is_empty() || !self.table.has_any("ext.builtin.exceptions") {
            true => said,
            false => format!("\0{said}"),
        }
    }

    /// The words a taking-apart says when it goes wrong: the pieces the
    /// table holds under the label, with the counts and kind names the
    /// kernel writes between them. Where the language furnishes
    /// exceptions those pieces open with the class the complaint belongs
    /// to, so it goes back marked as told in full and nothing further is
    /// put over it. A table holding too few pieces leaves nothing to
    /// write and the caller says its own plain words instead.
    fn apart_words(&self, key: &str, written: &[String]) -> Option<String> {
        let pieces = self.table.strings(key);
        if pieces.len() <= written.len() { return None; }
        let mut said: String = pieces.iter().zip(written).map(|(piece, part)| format!("{piece}{part}")).collect();
        said.push_str(&pieces[written.len()]);
        match self.table.has_any("ext.builtin.exceptions") {
            true => Some(format!("\0{said}")),
            false => Some(said),
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
    pub(super) fn open_arguments(&mut self, values: Vec<Value>) -> Res<(Vec<Value>, Vec<(String, Value)>)> {
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
                        Value::Dict(entries) => positions.extend(entries.iter().map(|entry| match &entry.0 { Value::Keyed(v, _) => v.as_ref().clone(), key => key.clone() })),
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

    /// The name the outermost names go by, where the language gives
    /// them one: the namespace a routine written there belongs to.
    fn namespace_named(&self) -> Option<String> {
        let word = self.table.strings("ext.system.module.name").first()?;
        if let Some(book) = &self.world_book {
            if let Some(held) = looked_up(book, word) { return Some(held.bare()); }
        }
        let at = self.idents.iter().position(|name| name == word)?;
        match self.outermost.cells.borrow().get(at)?.settled() {
            Value::Text(said) => Some(said.to_string()),
            _ => None,
        }
    }

    /// The words for a call handed the same keyword twice, which only a
    /// spread of pairs can bring about since two written out are
    /// refused as the text is read. The reference names the routine by
    /// the namespace it belongs to and the name it goes by there, so
    /// the two are written together here.
    fn keyword_twice(&self, program: &Routine, key: &str) -> String {
        let words = self.table.strings("ext.syntax.call.amiss.keyword");
        if words.len() < 3 { return self.argument_fault("ext.syntax.call.amiss.duplicate", Some(key)); }
        let called = match self.namespace_named() {
            Some(space) if !program.qualification.is_empty() => format!("{}.{}", space, program.qualification),
            _ => program.qualification.clone(),
        };
        self.whole_complaint([words[0].as_str(), &called, &words[1], key, &words[2]].concat())
    }

    /// A complaint whose words already open with the kind of fault they
    /// are is handed on as it stands where the language has exceptions.
    fn whole_complaint(&self, words: String) -> String {
        match self.table.has_any("ext.builtin.exceptions") {
            true => format!("\0{words}"),
            false => words,
        }
    }

    /// What a routine is called in the words about a call made to it.
    fn routine_called(program: &Routine) -> String {
        match program.qualification.as_str() {
            "" => program.ident.clone(),
            full => full.to_string(),
        }
    }

    /// The places a call left unfilled, named one by one between their
    /// quotes: two stand joined by their own word, and three or more are
    /// run together with a different joining before the final one.
    fn unfilled_complaint(&self, program: &Routine, unfilled: &[String], only_named: bool) -> String {
        let pieces = self.table.strings("ext.syntax.call.amiss.absent");
        if pieces.len() != 7 { return self.argument_fault("ext.syntax.call.amiss.missing", unfilled.first().map(String::as_str)); }
        let joins = self.table.strings("ext.syntax.call.amiss.absent.names");
        let join = |n: usize| joins.get(n).cloned().unwrap_or_default();
        let mut listing = String::new();
        let last = unfilled.len() - 1;
        for (n, name) in unfilled.iter().enumerate() {
            if n > 0 {
                listing += &match (n == last, unfilled.len()) {
                    (true, 2) => join(3),
                    (true, _) => join(4),
                    _ => join(2),
                };
            }
            listing += &(join(0) + name + &join(1));
        }
        let manner = &pieces[if only_named { 6 } else { 5 }];
        let noun = &pieces[if unfilled.len() > 1 { 4 } else { 3 }];
        self.whole_complaint(format!("{}{}{}{}{}{manner}{noun}{listing}", pieces[0], Self::routine_called(program), pieces[1], unfilled.len(), pieces[2]))
    }

    /// Too many worths handed over in order: how many the routine takes
    /// (a range, where some of its places have defaults), how many came,
    /// and how many places filled only by name came beside them.
    fn overfull_complaint(&self, program: &Routine, manners: &[char], fitted: &[Value], handed: usize) -> String {
        let pieces = self.table.strings("ext.syntax.call.amiss.excess");
        if pieces.len() != 12 { return self.argument_fault("ext.syntax.call.amiss", None); }
        let mut takes = 0;
        let mut optional = 0;
        let mut beside = 0;
        for (at, how) in manners.iter().enumerate() {
            let slot = program.formal_slots[at];
            let has_default = program.carried.contains(&slot) || program.local_defaults.contains(&slot);
            match how {
                'b' | 'p' => { takes += 1; if has_default { optional += 1; } }
                'n' if !matches!(fitted[at], Value::Unset) => beside += 1,
                _ => {}
            }
        }
        let plural = |count: usize, single: usize| pieces[if count == 1 { single } else { single + 1 }].as_str();
        let span = if optional > 0 { format!("{}{}{}", pieces[2], takes - optional, pieces[3]) } else { String::new() };
        let taken = if optional > 0 { pieces[5].as_str() } else { plural(takes, 4) };
        let aside = match beside {
            0 => String::new(),
            _ => format!("{}{}{}{}", plural(handed, 4), pieces[9], beside, plural(beside, 10)),
        };
        let verb = if handed == 1 && beside == 0 { &pieces[7] } else { &pieces[8] };
        self.whole_complaint(format!("{}{}{}{span}{takes}{taken}{}{handed}{aside}{verb}", pieces[0], Self::routine_called(program), pieces[1], pieces[6]))
    }

    /// A keyword that meets no place. If the call named any place the
    /// routine takes in order alone, it is those places that are told,
    /// as the routine lists them; otherwise the keyword itself.
    fn unplaced_keyword(&self, program: &Routine, manners: &[char], keys: &[String], key: &str) -> String {
        let mut in_order_only = Vec::new();
        for (at, how) in manners.iter().enumerate() {
            if *how == 'p' && keys.iter().any(|given| *given == program.formals[at]) { in_order_only.push(program.formals[at].clone()); }
        }
        let routine = Self::routine_called(program);
        let ordered = self.table.strings("ext.syntax.call.amiss.ordered");
        if ordered.len() == 4 && !in_order_only.is_empty() {
            return self.whole_complaint(ordered[0].clone() + &routine + &ordered[1] + &in_order_only.join(&ordered[2]) + &ordered[3]);
        }
        match self.table.strings("ext.syntax.call.amiss.unexpected") {
            [opening, middle, end] => self.whole_complaint(opening.clone() + &routine + middle + key + end),
            _ => self.argument_fault("ext.syntax.call.amiss.unknown", Some(key)),
        }
    }

    fn fit_arguments(&mut self, program: &Routine, manners: &[char], values: Vec<Value>) -> Res<Vec<Value>> {
        // Where every place is an ordinary one, none of the worths given
        // carries a name or a scattering, and there are exactly as many
        // of them as there are places, the row given is already the row
        // wanted; the sorting below would only build it again.
        if manners.len() == values.len()
            && manners.iter().all(|how| matches!(how, 'b' | 'p'))
            && values.iter().all(|worth| !matches!(worth, Value::Couple(_) | Value::Unset)) {
            return Ok(values);
        }
        let (positional, named) = self.open_arguments(values)?;
        let mut fitted = vec![Value::Unset; manners.len()];
        let ordinary: Vec<usize> = manners.iter().enumerate()
            .filter_map(|(slot, how)| matches!(how, 'b' | 'p').then_some(slot)).collect();
        let gather = manners.iter().position(|how| *how == 'v');
        let gather_names = manners.iter().position(|how| *how == 'k');
        // Worths beyond the ordinary places are not refused until the
        // names are placed: the reference speaks of a place given twice
        // before it counts the worths given in order.
        let handed = positional.len();
        let mut remaining = Vec::new();
        for (n, held) in positional.into_iter().enumerate() {
            match ordinary.get(n) {
                Some(slot) => fitted[*slot] = held,
                None => remaining.push(held),
            }
        }
        // The spare worths a gathering place takes stand as a tuple, so
        // that they read, weigh and compare as the language says.
        if let Some(slot) = gather { fitted[slot] = Value::Tuple(Rc::new(remaining)); }
        let keys: Vec<String> = match manners.contains(&'p') {
            true => named.iter().map(|(key, _)| key.clone()).collect(),
            false => Vec::new(),
        };
        let mut spare_names = Vec::new();
        let mut already = std::collections::HashSet::new();
        for (key, worth) in named {
            // A place given twice over, once in order and once by name,
            // is worded with the routine the call was meant for, under
            // the name it goes by where it was written.
            let duplicate = || match self.table.strings("ext.syntax.call.amiss.positional") {
                [opening, between, closing] => self.whole_complaint(opening.clone() + &Self::routine_called(program) + between + &key + closing),
                _ => self.argument_fault("ext.syntax.call.amiss.duplicate", Some(&key)),
            };
            if !already.insert(key.clone()) { return Err(self.keyword_twice(program, &key).into()); }
            let found = program.formals.iter().enumerate()
                .find(|(at, name)| **name == key && matches!(manners[*at], 'b' | 'n'));
            if let Some((at, _)) = found {
                if !matches!(fitted[at], Value::Unset) { return Err(duplicate().into()); }
                fitted[at] = worth;
            } else if gather_names.is_some() {
                spare_names.push((Value::text(&key), worth));
            } else {
                return Err(self.unplaced_keyword(program, manners, &keys, &key).into());
            }
        }
        if handed > ordinary.len() && gather.is_none() {
            return Err(self.overfull_complaint(program, manners, &fitted, handed).into());
        }
        if let Some(slot) = gather_names { fitted[slot] = Value::Dict(Rc::new(spare_names.into())); }
        // Every unfilled place is told at once: first those taken in
        // order, and the ones taken by name only when none of those is.
        let unfilled = |wanted: &dyn Fn(char) -> bool| -> Vec<String> {
            (0..fitted.len()).filter(|at| wanted(manners[*at]) && matches!(fitted[*at], Value::Unset)
                && !program.carried.contains(&program.formal_slots[*at])
                && !program.local_defaults.contains(&program.formal_slots[*at]))
                .map(|at| program.formals[at].clone()).collect()
        };
        let in_order = unfilled(&|how| how != 'n');
        if !in_order.is_empty() { return Err(self.unfilled_complaint(program, &in_order, false).into()); }
        let by_name = unfilled(&|how| how == 'n');
        if !by_name.is_empty() { return Err(self.unfilled_complaint(program, &by_name, true).into()); }
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

    /// A routine as calls of it now run: as it was bound, or as the
    /// program wrote its spare arguments or its code over since.
    pub(super) fn as_now_written(&self, program: Rc<Routine>, env: Rc<Env>) -> (Rc<Routine>, Rc<Env>) {
        if self.written_over.is_empty() { return (program, env); }
        match self.written_over.get(&(Rc::as_ptr(&program) as usize, Rc::as_ptr(&env) as usize)) {
            Some((_, _, now, room, _)) => (now.clone(), room.clone()),
            None => (program, env),
        }
    }

    pub fn invoke(&mut self, program: Rc<Routine>, env: Rc<Env>, args: Vec<Value>) -> Res {
        let (program, env) = self.as_now_written(program, env);
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
        let caller_trace = if mine { self.active_trace.take() } else { None };
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
        let outcome = self.traced_result(outcome);
        if mine { self.active_trace = caller_trace; }
        if self.table.has_any("ext.builtin.exceptions.traceback") { self.row = was_on_row; }
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
        self.octet_worded(reason, 0)
    }

    /// One of the complaints a bytes label spells, by its place among
    /// them: a label wording the same fault of a whole row and of one
    /// byte, or of a place read and a place taken out, keeps each
    /// wording in its own place.
    /// A changeable row of bytes parts with a place, or with a run of
    /// them, where it stands; a fixed row parts with none and is named
    /// in the words for a sequence that cannot be shortened. The row is
    /// handed back as it stands, since it is written where it lies.
    fn octets_shortened(&self, row: &Value, at: &Value) -> Result<Value, String> {
        let Value::Octets { cell, changeable, .. } = row else { return Err(self.octet_error("unready")); };
        if !*changeable { return Err(self.deletion_refused(row)); }
        let mut numbers = cell.borrow_mut();
        match at {
            Value::Span(bounds) if self.table.has_any("ext.builtin.slice") => {
                let (_, picked, _) = self.span_selection(bounds, numbers.len())?;
                let kept = numbers.iter().enumerate().filter(|(j, _)| !picked.contains(j)).map(|(_, n)| *n).collect();
                *numbers = kept;
            }
            key => { let place = self.octet_at(key, numbers.len(), true)?; numbers.remove(place); }
        }
        drop(numbers);
        Ok(row.clone())
    }

    fn octet_worded(&self, reason: &str, at: usize) -> String {
        self.table.strings(&format!("ext.system.bytes.{reason}")).get(at).map_or_else(String::new, String::clone)
    }

    /// The workings a row of bytes answers to by name, each beside the
    /// bytes primitive that carries it out. They are text's workings
    /// read byte by byte and go by text's words; making bytes out of
    /// text belongs to text alone and stands nowhere in this table.
    const OCTET_MEMBERS: &'static [(&'static str, u8)] = &[
        ("capitalize", 34), ("center", 29), ("count", 18), ("decode", 3), ("endswith", 22),
        ("expandtabs", 33), ("find", 13), ("fromhex", 50), ("hex", 4), ("index", 19),
        ("isalnum", 41), ("isalpha", 42), ("isascii", 43), ("isdigit", 44), ("islower", 45),
        ("isspace", 46), ("istitle", 47), ("isupper", 48), ("join", 9), ("ljust", 30),
        ("lower", 7), ("lstrip", 24), ("maketrans", 51), ("partition", 26), ("removeprefix", 37),
        ("removesuffix", 38), ("replace", 11), ("rfind", 20), ("rindex", 21), ("rjust", 31),
        ("rpartition", 27), ("rsplit", 23), ("rstrip", 25), ("split", 8), ("splitlines", 28),
        ("startswith", 10), ("strip", 12), ("swapcase", 36), ("title", 35), ("translate", 39),
        ("upper", 6), ("zfill", 32),
    ];

    /// The workings a changeable row of bytes answers to besides those,
    /// each beside its own bytes primitive. Each writes where the row
    /// lies, so a fixed row answers to none of them; their words are
    /// the ones a row's own methods go by.
    const OCTET_CHANGERS: &'static [(&'static str, u8)] = &[
        ("append", 52), ("clear", 57), ("copy", 59), ("extend", 53), ("insert", 54),
        ("pop", 55), ("remove", 56), ("reverse", 58),
    ];

    /// The working a row of bytes answers to under this spelling, where
    /// the table spells one for it. The words are sought in the bytes
    /// family, then among text's, whose words a row of bytes shares,
    /// and last among the value methods. A word written with its kind
    /// ahead of it answers by its bare tail as well, which is the name
    /// a row of bytes keeps the method under.
    pub(super) fn octet_member(&self, spelling: &str, changeable: bool) -> Option<&'static str> {
        let carries = |label: String| self.table.loose_strings(&label).iter()
            .any(|word| word == spelling || word.rsplit('.').next() == Some(spelling));
        let among = self.value_method_named(spelling);
        let changers: &[(&str, u8)] = if changeable { Self::OCTET_CHANGERS } else { &[] };
        Self::OCTET_MEMBERS.iter().chain(changers).map(|(word, _)| *word).find(|word| {
            carries(format!("ext.builtin.bytes.{word}")) || carries(format!("ext.builtin.text.{word}"))
                || among.as_deref() == Some(*word)
        })
    }

    /// The primitive that carries out a working of that name.
    fn octet_operation(word: &str) -> Option<u8> {
        Self::OCTET_MEMBERS.iter().chain(Self::OCTET_CHANGERS)
            .find(|(named, _)| *named == word).map(|(_, code)| *code)
    }

    /// The refusal of a key of a kind a row of bytes cannot be read at.
    /// CPython words this apart from every other sequence's: the fixed
    /// kind is spoken of in the singular and the changeable one by its
    /// own name, and the key's kind closes the words.
    fn octet_key_refused(&self, changeable: bool, key: &Value) -> String {
        let pieces = self.table.strings("ext.system.bytes.subscript");
        format!("{}{}", pieces.get(usize::from(changeable)).map_or("", String::as_str), key.kind_word())
    }

    /// The refusal of a joining onto a row of bytes: what was handed to
    /// it is named, and then the row's own kind.
    fn octet_joining_refused(&self, left: &Value, right: &Value) -> String {
        let pieces = self.table.strings("ext.system.bytes.concat");
        let at = |i: usize| pieces.get(i).map_or("", String::as_str);
        format!("{}{}{}{}", at(0), right.kind_word(), at(1), left.kind_word())
    }

    /// A row of bytes filled mark by mark.
    fn octet_filled(&self, pattern: &str, supplied: &Value) -> Result<Vec<u8>, String> {
        let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
        let mut marks = OctetMarks {
            layout: crate::formatting::Layout { table: self.table, names: self.wording() },
            refusal: self.table.strings("ext.op.rem.format.byte"),
        };
        let filled = layout.remainder(pattern, supplied, &mut marks, true)?;
        filled.chars().map(|letter| u8::try_from(u32::from(letter)).map_err(|_| self.octet_error("unready"))).collect()
    }

    /// The bytes that written hexadecimal stands for: two figures to a
    /// byte, with blanks allowed to stand between them.
    fn octets_from_hex(&self, spelling: &str) -> Result<Vec<u8>, String> {
        let mut waiting: Option<(u8, usize)> = None;
        let mut content = Vec::new();
        for (position, ch) in spelling.chars().enumerate() {
            if waiting.is_none() && (ch.is_ascii_whitespace() || ch == '\x0b') { continue; }
            let figure = ch.to_digit(16).filter(|_| ch.is_ascii()).ok_or_else(|| format!("{}{}", self.octet_error("hex"), position))? as u8;
            match waiting.take() {
                Some((high, _)) => content.push(high * 16 + figure),
                None => waiting = Some((figure, position)),
            }
        }
        if waiting.is_some() { return Err(format!("{}{}", self.octet_error("hex"), spelling.chars().count())); }
        Ok(content)
    }

    /// The row spelled out in hexadecimal. A mark may be set between
    /// the bytes every so many of them: counted back from the end where
    /// the number stands above nought, forward from the start where it
    /// stands below, and nowhere at all where it is nought.
    fn octets_in_hex(&self, content: &[u8], arguments: &[Value]) -> Result<Value, String> {
        let (mark, every) = match arguments.first() {
            None => (String::new(), 0i64),
            Some(Value::Text(between)) => {
                if between.chars().count() != 1 { return Err(self.octet_error("arguments")); }
                let every = match arguments.get(1) { None => 1, Some(n) => self.octet_whole(n)?.to_i64().unwrap_or(1) };
                (between.to_string(), every)
            }
            _ => return Err(self.octet_error("arguments")),
        };
        let run = every.unsigned_abs() as usize;
        let mut spelled = String::with_capacity(content.len() * 2);
        for (position, number) in content.iter().enumerate() {
            let parts = position > 0 && run > 0
                && if every > 0 { (content.len() - position) % run == 0 } else { position % run == 0 };
            if parts { spelled.push_str(&mark); }
            spelled.push_str(&format!("{number:02x}"));
        }
        Ok(Value::text(&spelled))
    }

    /// A table of two hundred and fifty six bytes: each stands for
    /// itself save the ones the first row names, which stand for the
    /// bytes standing in the same places of the second.
    fn octet_mapping(&self, from: &[u8], onto: &[u8]) -> Result<Value, String> {
        if from.len() != onto.len() { return Err(self.octet_error("arguments")); }
        let mut mapping: Vec<u8> = (0..=u8::MAX).collect();
        for (place, byte) in from.iter().zip(onto.iter()) { mapping[usize::from(*place)] = *byte; }
        Ok(self.octets(mapping, false))
    }

    /// Whether every byte of the row answers to a question about the
    /// seven-bit letters, and whether the row is cased one way alone.
    /// None of these holds of an empty row save the one asking whether
    /// the whole row is seven-bit, which an empty row satisfies.
    fn octets_all(&self, content: &[u8], question: u8) -> bool {
        let one_case = |upper: bool| {
            let named = |n: &u8| if upper { n.is_ascii_uppercase() } else { n.is_ascii_lowercase() };
            let other = |n: &u8| if upper { n.is_ascii_lowercase() } else { n.is_ascii_uppercase() };
            content.iter().any(named) && !content.iter().any(other)
        };
        match question {
            43 => content.iter().all(u8::is_ascii),
            45 => one_case(false),
            48 => one_case(true),
            // Titled: a letter opening a word stands upper and one
            // within a word stands lower, and a row of no letters at
            // all is titled by nothing.
            47 => {
                let mut inside = false;
                let mut letters = 0;
                for number in content {
                    if !number.is_ascii_alphabetic() { inside = false; continue; }
                    letters += 1;
                    if number.is_ascii_uppercase() == inside { return false; }
                    inside = true;
                }
                letters > 0
            }
            _ if content.is_empty() => false,
            41 => content.iter().all(u8::is_ascii_alphanumeric),
            42 => content.iter().all(u8::is_ascii_alphabetic),
            44 => content.iter().all(u8::is_ascii_digit),
            _ => content.iter().all(|n| n.is_ascii_whitespace() || *n == 11),
        }
    }

    fn octets(&self, values: Vec<u8>, changeable: bool) -> Value {
        let lead = self.table.strings("ext.system.bytes.repr")[if changeable { 1 } else { 0 }].clone();
        Value::Octets { cell: Rc::new(RefCell::new(values)), changeable, lead: lead.into() }
    }

    /// The word the table spells one of the two byte kinds with. Those
    /// kinds are values in their own right and not intrinsic words, so
    /// anything that has to work from the word asks for it here.
    pub(super) fn octet_kind_word(&self, changeable: bool) -> &str {
        self.table.single(if changeable { "ext.builtin.bytearray" } else { "ext.builtin.bytes" }).unwrap_or("")
    }

    fn octet_type(&self, changeable: bool) -> Value {
        let parts = self.table.strings("ext.system.bytes.type");
        Value::OctetKind { changeable, shown: format!("{}{}{}", parts[0], self.octet_kind_word(changeable), parts[1]).into() }
    }

    fn octet_whole(&self, value: &Value) -> Result<BigInt, String> {
        match value.kind() {
            Some(Kind::Whole | Kind::Truth) => value.as_big(),
            _ => Err(self.octet_error("arguments")),
        }
    }

    /// One byte read off a value. A value of a kind no whole number
    /// can be read from is named by the words every builtin names it
    /// by; a whole number outside a byte's bounds is refused in the
    /// words for a whole row where the row is being built, and in those
    /// for one byte everywhere else.
    fn octet_item(&self, value: &Value, whole_row: bool) -> Result<u8, String> {
        if !matches!(value.kind(), Some(Kind::Whole | Kind::Truth)) {
            return Err(self.core_complaint("core.integer", &value.kind_word()));
        }
        value.as_big()?.to_u8().ok_or_else(|| self.octet_worded("range", usize::from(!whole_row)))
    }

    /// A whole number a row of bytes is handed as a place. CPython
    /// names the kind that cannot stand for one.
    fn octet_place(&self, value: &Value) -> Result<i64, String> {
        if !matches!(value.kind(), Some(Kind::Whole | Kind::Truth)) {
            return Err(self.core_complaint("core.integer", &value.kind_word()));
        }
        let number = value.as_big()?;
        Ok(number.to_i64().unwrap_or(if number < BigInt::zero() { i64::MIN } else { i64::MAX }))
    }

    /// The bytes a value gives up to a row being lengthened: another
    /// row hands over its own, and a row of numbers or a text hands
    /// over one byte for each of its members.
    fn octet_lengthening(&self, source: &Value) -> Result<Vec<u8>, String> {
        match &source.settled() {
            Value::Octets { cell, .. } => Ok(cell.borrow().to_vec()),
            Value::Vector(items) | Value::Tuple(items) | Value::Row(items) =>
                items.iter().map(|item| self.octet_item(item, false)).collect(),
            Value::Text(word) => word.chars().map(|letter| self.octet_item(&Value::text(&letter.to_string()), false)).collect(),
            other => Err(self.core_complaint("core.uniterable", &other.kind_word())),
        }
    }

    fn octet_contents(&self, source: &Value, iterable: bool) -> Result<Vec<u8>, String> {
        self.octet_gathered(source, iterable, false)
    }

    fn octet_gathered(&self, source: &Value, iterable: bool, whole_row: bool) -> Result<Vec<u8>, String> {
        if let Value::Octets { cell, .. } = source { return Ok(cell.borrow().to_vec()); }
        if iterable {
            if let Value::Vector(items) = source {
                let mut result = Vec::with_capacity(items.len());
                for item in items.iter() { result.push(self.octet_item(item, whole_row)?); }
                return Ok(result);
            }
        }
        Err(self.octet_error("arguments"))
    }

    fn octet_at(&self, index: &Value, length: usize, changeable: bool) -> Result<usize, String> {
        if !matches!(index.kind(), Some(Kind::Whole | Kind::Truth)) { return Err(self.octet_key_refused(changeable, index)); }
        let raw = self.octet_whole(index)?;
        let adjusted = if raw < BigInt::zero() { raw + length } else { raw };
        match adjusted.to_usize() {
            Some(i) if i < length => Ok(i),
            _ => Err(self.octet_error("index")),
        }
    }

    // The codecs stand in the encodings label one to an entry. An entry
    // begins with the name the codec complains under and goes on with
    // the spellings which reach it, which need not include the first.
    // Their order is the order this kernel knows them by: the wide
    // encoding, the seven-bit one, the byte-for-byte one, and the one
    // that spells a far character as an escape.
    const WIDE: usize = 0;
    const SEVEN_BIT: usize = 1;
    const BYTE_FOR_BYTE: usize = 2;
    const ESCAPED: usize = 3;

    fn octet_encoding(&self, argument: Option<&Value>) -> Result<usize, String> {
        match argument {
            None => Ok(Self::WIDE),
            Some(Value::Text(encoding)) => {
                let wanted = encoding.to_lowercase().replace('_', "-");
                let entries = self.table.strings("ext.system.bytes.encodings");
                entries.iter().position(|entry| entry.split_whitespace().skip(1).any(|name| name == wanted))
                    .ok_or_else(|| self.octet_error("unready"))
            }
            _ => Err(self.octet_error("arguments")),
        }
    }

    /// The proper name of a codec, which is the one its complaints carry.
    fn octet_codec_name(&self, alphabet: usize) -> String {
        self.table.strings("ext.system.bytes.encodings").get(alphabet)
            .and_then(|entry| entry.split_whitespace().next()).unwrap_or("").to_owned()
    }

    fn octets_from_text(&self, text: &str, alphabet: usize) -> Result<Vec<u8>, String> {
        if alphabet == Self::ESCAPED {
            let mut result = Vec::new();
            for letter in text.chars() {
                let code = letter as u32;
                match code {
                    0..=255 => result.push(code as u8),
                    256..=65535 => result.extend_from_slice(format!("\\u{code:04x}").as_bytes()),
                    _ => result.extend_from_slice(format!("\\U{code:08x}").as_bytes()),
                }
            }
            return Ok(result);
        }
        if alphabet != Self::WIDE {
            let ceiling = if alphabet == Self::SEVEN_BIT { 127 } else { 255 };
            let chars = text.chars().collect::<Vec<_>>();
            if let Some(first) = chars.iter().position(|c| *c as u32 > ceiling) {
                let last = chars[first..].iter().position(|c| *c as u32 <= ceiling).map_or(chars.len(), |i| first + i);
                let location = if last - first > 1 { format!("characters in position {}-{}", first, last - 1) }
                    else {
                        let code = chars[first] as u32;
                        let escape = match code { 0..=255 => format!("\\x{code:02x}"), 256..=65535 => format!("\\u{code:04x}"), _ => format!("\\U{code:08x}") };
                        format!("character '{}' in position {}", escape, first)
                    };
                let name = self.octet_codec_name(alphabet);
                return Err(format!("{}'{}' codec can't encode {}: ordinal not in range({})", self.octet_error("encode"), name, location, ceiling + 1));
            }
            if alphabet == Self::BYTE_FOR_BYTE { return Ok(text.chars().map(|c| c as u8).collect()); }
        }
        Ok(Vec::from(text.as_bytes()))
    }

    /// What is said of bytes a codec cannot read. The places run from the
    /// first byte at fault to the last, both counted in.
    fn octet_decode_fault(&self, alphabet: usize, first: usize, last: usize, numbers: &[u8], why: &str) -> String {
        let subject = match last - first {
            0 => format!("byte 0x{:02x} in position {}", numbers[first], first),
            _ => format!("bytes in position {}-{}", first, last),
        };
        format!("{}'{}' codec can't decode {}: {}", self.octet_error("decode"), self.octet_codec_name(alphabet), subject, why)
    }

    /// Bytes read back as text where a far character was written as an
    /// escape. A run of backslashes hides the escape unless the run is
    /// odd, and the last backslash of an odd run begins it.
    fn octets_unescaped(&self, numbers: &[u8]) -> Result<Value, String> {
        let mut text = String::new();
        let mut at = 0;
        while at < numbers.len() {
            if numbers[at] != b'\\' { text.push(numbers[at] as char); at += 1; continue; }
            let run = numbers[at..].iter().take_while(|b| **b == b'\\').count();
            let marker = numbers.get(at + run).copied();
            if run % 2 == 0 || !matches!(marker, Some(b'u') | Some(b'U')) {
                for _ in 0..run { text.push('\\'); }
                at += run;
                continue;
            }
            for _ in 0..run - 1 { text.push('\\'); }
            let start = at + run - 1;
            let wide = marker == Some(b'U');
            let wanted = if wide { 8 } else { 4 };
            let shape = if wide { "\\UXXXXXXXX" } else { "\\uXXXX" };
            let figures = numbers[start + 2..].iter().take(wanted).take_while(|b| b.is_ascii_hexdigit()).count();
            if figures < wanted {
                return Err(self.octet_decode_fault(Self::ESCAPED, start, start + 1 + figures, numbers, &format!("truncated {shape} escape")));
            }
            let spelled = std::str::from_utf8(&numbers[start + 2..start + 2 + wanted]).map_err(|_| self.octet_error("unready"))?;
            let code = u32::from_str_radix(spelled, 16).map_err(|_| self.octet_error("unready"))?;
            match char::from_u32(code) {
                Some(letter) => text.push(letter),
                // A code above the last character is out of range; one
                // that names half a pair is a character this kernel
                // cannot hold, and it says so rather than guess.
                None if code > 0x10ffff => return Err(self.octet_decode_fault(Self::ESCAPED, start, start + 1 + wanted, numbers, "\\Uxxxxxxxx out of range")),
                None => return Err(self.octet_error("unready")),
            }
            at = start + 2 + wanted;
        }
        Ok(Value::text(&text))
    }

    fn octets_to_text(&self, numbers: &[u8], alphabet: usize) -> Result<Value, String> {
        if alphabet == Self::ESCAPED { return self.octets_unescaped(numbers); }
        if alphabet == Self::BYTE_FOR_BYTE { return Ok(Value::text(&numbers.iter().map(|b| *b as char).collect::<String>())); }
        let mut fault = None;
        if alphabet == Self::SEVEN_BIT {
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
            Some((first, last, why)) => Err(self.octet_decode_fault(alphabet, first, last - 1, numbers, why)),
            None => Ok(Value::text(std::str::from_utf8(numbers).map_err(|_| self.octet_error("unready"))?)),
        }
    }

    /// Text asked for from a row of bytes, with the encoding named and
    /// the error policy named after it. Nothing but a row of bytes can
    /// be read this way; text asked of text is refused outright, as is
    /// anything else.
    fn text_decoded(&mut self, values: &[Value]) -> Result<Value, String> {
        if values.len() > 3 { return Err(self.argument_fault("ext.builtin.to_string.unready", None)); }
        let words = self.table.strings("ext.builtin.to_string.undecodable");
        let said = |at: usize| words.get(at).cloned().unwrap_or_default();
        let content = match &values[0] {
            Value::Octets { cell, .. } => cell.borrow().clone(),
            Value::Text(_) => return Err(said(0)),
            other => return Err(format!("{}{}{}", said(1), other.kind_word(), said(2))),
        };
        self.convert_text(false, &content, values)
    }

    /// The workings a row of bytes shares with text: looking through
    /// it, cutting it, padding it, asking what its bytes are, and
    /// mapping one byte onto another. Each counts in bytes and holds
    /// only the seven-bit letters to be letters, as CPython does. The
    /// content has been taken out of its cell already, and `changeable`
    /// settles which of the two kinds comes back out.
    fn octets_textual(&self, operation: u8, content: &[u8], changeable: bool, arguments: &[Value]) -> Result<Value, String> {
        let wrong = || self.octet_error("arguments");
        let refusal = || self.octet_error("unready");
        let built = |made: Vec<u8>| self.octets(made, changeable);
        let given = |at: usize| arguments.get(at).filter(|v| !matches!(v, Value::Nil));
        // What is looked for may be written as one byte's number as
        // readily as a row of bytes.
        let looked_for = |value: &Value| -> Result<Vec<u8>, String> {
            match value.kind() {
                Some(Kind::Whole | Kind::Truth) => Ok(vec![self.octet_item(value, false)?]),
                _ => self.octet_contents(value, false),
            }
        };
        let counted = |value: &Value| -> Result<i64, String> { Ok(self.octet_whole(value)?.to_i64().unwrap_or(i64::MAX)) };
        // Where a search opens and where it closes: a place below
        // nought counts back from the end, and both are held inside.
        let inside = |number: i64| -> usize {
            let offset = if number < 0 { number.saturating_add(content.len() as i64) } else { number };
            offset.clamp(0, content.len() as i64) as usize
        };
        let bounds = |first: Option<&Value>, second: Option<&Value>| -> Result<(usize, usize), String> {
            let opens = match first { Some(v) => inside(counted(v)?), None => 0 };
            let closes = match second { Some(v) => inside(counted(v)?), None => content.len() };
            Ok((opens, closes.max(opens)))
        };
        let scan = |haystack: &[u8], part: &[u8], backward: bool| -> Option<usize> {
            let last = haystack.len().checked_sub(part.len())?;
            let places = 0..=last;
            if backward { places.rev().find(|&i| haystack[i..].starts_with(part)) }
            else { places.into_iter().find(|&i| haystack[i..].starts_with(part)) }
        };
        let blank = |n: &u8| n.is_ascii_whitespace() || *n == 11;
        match operation {
            41..=48 if arguments.is_empty() => Ok(Value::Flag(self.octets_all(content, operation))),
            // Whether the row opens with, or closes with, any of the
            // parts named, between the bounds handed over.
            10 | 22 if (1..=3).contains(&arguments.len()) => {
                let (opens, closes) = bounds(given(1), given(2))?;
                let middle = &content[opens..closes];
                let each = match &arguments[0] { Value::Tuple(row) => row.as_ref().clone(), only => vec![only.clone()] };
                for one in each {
                    let end = self.octet_contents(&one, false)?;
                    let held = if operation == 10 { middle.starts_with(&end) } else { middle.ends_with(&end) };
                    if held { return Ok(Value::Flag(true)); }
                }
                Ok(Value::Flag(false))
            }
            // Where a part stands in the row, looked for from either
            // end, and how many times over it stands there. The two
            // that give no place below nought refuse instead.
            13 | 18 | 19 | 20 | 21 if (1..=3).contains(&arguments.len()) => {
                let part = looked_for(&arguments[0])?;
                let (opens, closes) = bounds(given(1), given(2))?;
                let middle = &content[opens..closes];
                if operation == 18 {
                    if part.is_empty() { return Ok(Value::Small(middle.len() as i64 + 1)); }
                    let mut tally = 0i64;
                    let mut cursor = 0usize;
                    while let Some(step) = scan(&middle[cursor..], &part, false) {
                        tally += 1;
                        cursor += step + part.len();
                    }
                    return Ok(Value::Small(tally));
                }
                match scan(middle, &part, matches!(operation, 20 | 21)) {
                    Some(place) => Ok(Value::Small((place + opens) as i64)),
                    None if matches!(operation, 19 | 21) => Err(self.octet_error("missing")),
                    None => Ok(Value::Small(-1)),
                }
            }
            // Cut apart from the right, the pieces given back standing
            // in the order they stood in the row.
            23 if arguments.len() <= 2 => {
                let ceiling = match given(1) {
                    Some(v) => { let asked = counted(v)?; if asked < 0 { usize::MAX } else { asked as usize } }
                    None => usize::MAX,
                };
                let mut pieces: Vec<Vec<u8>> = Vec::new();
                let mut left = content;
                match given(0) {
                    Some(value) => {
                        let separator = self.octet_contents(value, false)?;
                        if separator.is_empty() { return Err(self.octet_error("separator")); }
                        while pieces.len() < ceiling {
                            let Some(place) = scan(left, &separator, true) else { break };
                            pieces.push(left[place + separator.len()..].to_vec());
                            left = &left[..place];
                        }
                        pieces.push(left.to_vec());
                    }
                    None => loop {
                        while left.last().map_or(false, blank) { left = &left[..left.len() - 1]; }
                        if left.is_empty() { break; }
                        if pieces.len() == ceiling { pieces.push(left.to_vec()); break; }
                        let edge = left.iter().rposition(blank).map_or(0, |i| i + 1);
                        pieces.push(left[edge..].to_vec());
                        left = &left[..edge];
                    },
                }
                pieces.reverse();
                Ok(Value::Vector(Rc::new(pieces.into_iter().map(built).collect())))
            }
            // The blank, or a set of bytes chosen instead, taken off
            // one end of the row.
            24 | 25 if arguments.len() <= 1 => {
                let away = match given(0) { Some(v) => self.octet_contents(v, false)?, None => vec![32, 9, 10, 13, 11, 12] };
                let mut left = content;
                if operation == 24 { while left.first().map_or(false, |n| away.contains(n)) { left = &left[1..]; } }
                else { while left.last().map_or(false, |n| away.contains(n)) { left = &left[..left.len() - 1]; } }
                Ok(built(left.to_vec()))
            }
            // The row cut in three about the first or the last standing
            // of a part, with the part itself in the middle.
            26 | 27 if arguments.len() == 1 => {
                let separator = self.octet_contents(&arguments[0], false)?;
                if separator.is_empty() { return Err(self.octet_error("separator")); }
                let empty = Vec::new();
                let cut = match scan(content, &separator, operation == 27) {
                    Some(place) => [content[..place].to_vec(), separator.clone(), content[place + separator.len()..].to_vec()],
                    None if operation == 27 => [empty.clone(), empty, content.to_vec()],
                    None => [content.to_vec(), empty.clone(), empty],
                };
                Ok(Value::Tuple(Rc::new(cut.into_iter().map(built).collect())))
            }
            // The lines of the row. A line closes at a line feed, at a
            // return, or at a return and a feed together, and nowhere
            // else at all.
            28 if arguments.len() <= 1 => {
                let holding = !matches!(arguments.first(), None | Some(Value::Nil) | Some(Value::Flag(false)) | Some(Value::Small(0)));
                let mut lines = Vec::new();
                let mut opened = 0usize;
                let mut cursor = 0usize;
                while cursor < content.len() {
                    if !matches!(content[cursor], 10 | 13) { cursor += 1; continue; }
                    let mut closed = cursor + 1;
                    if content[cursor] == 13 && content.get(closed) == Some(&10) { closed += 1; }
                    lines.push(content[opened..if holding { closed } else { cursor }].to_vec());
                    cursor = closed;
                    opened = closed;
                }
                if opened < content.len() { lines.push(content[opened..].to_vec()); }
                Ok(Value::Vector(Rc::new(lines.into_iter().map(built).collect())))
            }
            // Filled out to a width with a byte of choice, or with
            // noughts written behind whatever sign leads the figures.
            29 | 30 | 31 | 32 if (1..=if operation == 32 { 1 } else { 2 }).contains(&arguments.len()) => {
                let width = counted(&arguments[0])?.max(0) as usize;
                if width > 1_000_000 { return Err(refusal()); }
                let spare = width.saturating_sub(content.len());
                if operation == 32 {
                    let lead = usize::from(matches!(content.first(), Some(b'+' | b'-')));
                    let mut filled = content[..lead].to_vec();
                    filled.resize(lead + spare, b'0');
                    filled.extend_from_slice(&content[lead..]);
                    return Ok(built(filled));
                }
                let padding = match arguments.get(1) {
                    Some(v) => { let asked = self.octet_contents(v, false)?; if asked.len() != 1 { return Err(wrong()); } asked[0] }
                    None => b' ',
                };
                let ahead = match operation { 31 => spare, 29 => spare / 2 + (spare % 2) * (width % 2), _ => 0 };
                let mut filled = vec![padding; ahead];
                filled.extend_from_slice(content);
                filled.resize(width.max(content.len()), padding);
                Ok(built(filled))
            }
            // Every tab opened to the next stop, counted from the last
            // line break in the row.
            33 if arguments.len() <= 1 => {
                let stop = match given(0) { Some(v) => counted(v)?.max(0) as usize, None => 8 };
                let mut opened = Vec::new();
                let mut column = 0usize;
                for number in content {
                    if *number == 9 {
                        let spare = if stop == 0 { 0 } else { stop - column % stop };
                        opened.resize(opened.len() + spare, b' ');
                        column += spare;
                        continue;
                    }
                    opened.push(*number);
                    column = if matches!(number, 10 | 13) { 0 } else { column + 1 };
                }
                Ok(built(opened))
            }
            // The letters cased afresh: the leading one by itself, the
            // leading one of every word, or every one turned about.
            34 | 35 | 36 if arguments.is_empty() => {
                let mut inside = false;
                let mut recased = Vec::with_capacity(content.len());
                for (place, number) in content.iter().enumerate() {
                    recased.push(match operation {
                        34 if place > 0 => number.to_ascii_lowercase(),
                        34 => number.to_ascii_uppercase(),
                        35 if inside => number.to_ascii_lowercase(),
                        35 => number.to_ascii_uppercase(),
                        _ if number.is_ascii_uppercase() => number.to_ascii_lowercase(),
                        _ => number.to_ascii_uppercase(),
                    });
                    inside = number.is_ascii_alphabetic();
                }
                Ok(built(recased))
            }
            // A part dropped from one end, where it stands there.
            37 | 38 if arguments.len() == 1 => {
                let end = self.octet_contents(&arguments[0], false)?;
                let kept = if operation == 37 { content.strip_prefix(end.as_slice()) } else { content.strip_suffix(end.as_slice()) };
                Ok(built(kept.unwrap_or(content).to_vec()))
            }
            // Every byte read through a table of two hundred and fifty
            // six, with the ones named for dropping left out first.
            39 if (1..=2).contains(&arguments.len()) => {
                let mapping = match &arguments[0] {
                    Value::Nil => None,
                    value => { let table = self.octet_contents(value, false)?; if table.len() != 256 { return Err(wrong()); } Some(table) }
                };
                let away = match arguments.get(1) { Some(v) => self.octet_contents(v, false)?, None => Vec::new() };
                let mut turned = Vec::with_capacity(content.len());
                for number in content.iter().filter(|n| !away.contains(n)) {
                    turned.push(match &mapping { Some(table) => table[usize::from(*number)], None => *number });
                }
                Ok(built(turned))
            }
            51 if arguments.len() == 2 => {
                let from = self.octet_contents(&arguments[0], false)?;
                let onto = self.octet_contents(&arguments[1], false)?;
                self.octet_mapping(&from, &onto)
            }
            50 if arguments.len() == 1 => {
                let Value::Text(spelling) = &arguments[0] else { return Err(wrong()) };
                Ok(built(self.octets_from_hex(spelling)?))
            }
            _ => Err(refusal()),
        }
    }

    fn codec_function(&mut self, name: &str, arguments: Vec<Value>) -> Result<Value, String> {
        let namespace = self.namespace_for("codecs")?;
        let function = self.attribute(&namespace, name).ok_or_else(|| self.octet_error("unready"))?;
        self.apply_within(function, arguments)
    }

    fn convert_text(&mut self, writing: bool, input: &[u8], values: &[Value]) -> Result<Value, String> {
        if values.len() > 3 || values.is_empty() { return Err(self.octet_error("arguments")); }
        let handling = match values.get(2) {
            Some(Value::Text(word)) => word.to_string(),
            None => String::from("strict"),
            _ => return Err(self.octet_error("arguments")),
        };
        let alphabet = self.octet_encoding(values.get(1));
        if !matches!(alphabet, Ok(0..=2)) {
            return self.codec_function(if writing { "_encode" } else { "_decode" }, values.to_vec());
        }
        if writing && matches!(values[0], Value::Unpaired(_)) { return self.codec_function("_encode_surrogates", values.to_vec()); }
        let alphabet = alphabet?;
        let encoding = self.octet_codec_name(alphabet);
        if writing {
            let Value::Text(word) = &values[0] else { return Err(self.octet_error("arguments")); };
            if alphabet == Self::WIDE { return Ok(self.octets(word.as_bytes().to_vec(), false)); }
            let units = word.chars().collect::<Vec<_>>();
            let ceiling = if alphabet == Self::SEVEN_BIT { 127 } else { 255 };
            let mut result = Vec::new();
            let mut cursor = 0;
            while let Some(&letter) = units.get(cursor) {
                if letter as u32 <= ceiling { result.push(letter as u8); cursor += 1; continue; }
                let mut stop = cursor + 1;
                while stop < units.len() && units[stop] as u32 > ceiling { stop += 1; }
                match handling.as_str() {
                    "ignore" => cursor = stop,
                    "replace" => { result.extend(std::iter::repeat(b'?').take(stop - cursor)); cursor = stop; }
                    _ => {
                        let call = vec![Value::text(&encoding), values[0].clone(), Value::Small(cursor as i64), Value::Small(stop as i64), Value::text(&format!("ordinal not in range({})", ceiling + 1)), Value::text(&handling)];
                        let replaced = self.codec_function("_encode_error", call)?;
                        if let Value::Tuple(parts) = replaced {
                            let extra = match &parts[0] {
                                Value::Text(s) => self.octets_from_text(s, alphabet)?,
                                Value::Octets { cell, .. } => cell.borrow().to_vec(),
                                _ => return Err(self.octet_error("arguments")),
                            };
                            result.extend(extra);
                            cursor = parts[1].as_big()?.to_usize().ok_or_else(|| self.octet_error("arguments"))?;
                        } else { return Err(self.octet_error("arguments")); }
                    }
                }
            }
            return Ok(self.octets(result, false));
        }
        if alphabet == Self::BYTE_FOR_BYTE { return self.octets_to_text(input, alphabet); }
        let mut result = Vec::<u32>::new();
        let mut cursor = 0;
        while cursor < input.len() {
            let remaining = &input[cursor..];
            let bad = match alphabet {
                Self::SEVEN_BIT => remaining.iter().position(|b| *b > 127).map(|n| (n, n + 1, "ordinal not in range(128)")),
                _ => match std::str::from_utf8(remaining) {
                    Ok(_) => None,
                    Err(e) => {
                        let first = e.valid_up_to();
                        let last = e.error_len().map_or(remaining.len(), |n| first + n);
                        let cause = match e.error_len() {
                            None => "unexpected end of data",
                            Some(_) if remaining[first] < 194 || remaining[first] > 244 => "invalid start byte",
                            _ => "invalid continuation byte",
                        };
                        Some((first, last, cause))
                    }
                },
            };
            match bad {
                None => { result.extend(std::str::from_utf8(remaining).map_err(|_| self.octet_error("unready"))?.chars().map(|ch| ch as u32)); break; }
                Some((first, last, cause)) => {
                    result.extend(std::str::from_utf8(&remaining[..first]).map_err(|_| self.octet_error("unready"))?.chars().map(|ch| ch as u32));
                    if handling == "ignore" { cursor += last; continue; }
                    if handling == "replace" { result.push(65533); cursor += last; continue; }
                    let call = vec![Value::text(&encoding), self.octets(input.to_vec(), false), Value::Small((cursor + first) as i64), Value::Small((cursor + last) as i64), Value::text(cause), Value::text(&handling)];
                    match self.codec_function("_decode_error", call)? {
                        Value::Tuple(parts) => {
                            result.extend(parts[0].character_numbers().ok_or_else(|| self.octet_error("arguments"))?);
                            cursor = parts[1].as_big()?.to_usize().ok_or_else(|| self.octet_error("arguments"))?;
                        }
                        _ => return Err(self.octet_error("arguments")),
                    }
                }
            }
        }
        Ok(Value::characters(result))
    }

    fn octet_routine(&mut self, operation: u8, values: &[Value]) -> Result<Value, String> {
        let normalized: Vec<Value> = values.iter().map(Value::settled).collect();
        let values = normalized.as_slice();
        self.octet_work(operation, values, false)
    }

    fn octet_work(&mut self, operation: u8, values: &[Value], negative_allowed: bool) -> Result<Value, String> {
        if !values.is_empty() && operation < 4 && (operation > 1 || values.len() > 1) {
            let input = if operation == 3 { self.octet_contents(&values[0], false)? } else { Vec::new() };
            let answer = self.convert_text(operation != 3, &input, values)?;
            return match (operation, answer) {
                (1, Value::Octets { cell, .. }) => Ok(self.octets(cell.borrow().to_vec(), true)),
                (_, answer) => Ok(answer),
            };
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
                    1 => self.octet_gathered(&values[0], true, operation == 0)?,
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
            // Written hexadecimal read into a row of the kind asked
            // for, and the mapping table: both are asked of the kinds
            // themselves and take no row of bytes ahead of them.
            5 | 49 => {
                let [Value::Text(source)] = values else { return Err(wrong()); };
                return Ok(self.octets(self.octets_from_hex(source)?, operation == 49));
            }
            40 => {
                let [from, onto] = values else { return Err(wrong()); };
                return self.octet_mapping(&self.octet_contents(from, false)?, &self.octet_contents(onto, false)?);
            }
            // The workings that change a changeable row where it lies.
            // The row itself stands first, so the cell the row lives in
            // takes the change and every name for the row sees it. None
            // of them is worth anything but the one handing a byte back
            // and the one handing a fresh row back.
            52..=59 => {
                let Some(Value::Octets { cell, changeable: true, .. }) = values.first() else { return Err(refusal()); };
                let rest = &values[1..];
                let counted = |wanted: usize| if rest.len() == wanted { Ok(()) } else { Err(wrong()) };
                match operation {
                    52 => { counted(1)?; let byte = self.octet_item(&rest[0], false)?; cell.borrow_mut().push(byte); }
                    53 => { counted(1)?; let more = self.octet_lengthening(&rest[0])?; cell.borrow_mut().extend(more); }
                    54 => {
                        counted(2)?;
                        let asked = self.octet_place(&rest[0])?;
                        let byte = self.octet_item(&rest[1], false)?;
                        let mut held = cell.borrow_mut();
                        let index = if asked < 0 { asked.saturating_add(held.len() as i64).max(0) as usize } else { (asked as usize).min(held.len()) };
                        held.insert(index, byte);
                    }
                    55 => {
                        if rest.len() > 1 { return Err(wrong()); }
                        let asked = match rest.first() { Some(value) => self.octet_place(value)?, None => -1 };
                        let mut held = cell.borrow_mut();
                        if held.is_empty() { return Err(self.octet_worded("index", 2)); }
                        let index = if asked < 0 { asked.saturating_add(held.len() as i64) } else { asked };
                        if index < 0 || index as usize >= held.len() { return Err(self.octet_worded("index", 1)); }
                        return Ok(Value::Small(i64::from(held.remove(index as usize))));
                    }
                    56 => {
                        counted(1)?;
                        let byte = self.octet_item(&rest[0], false)?;
                        let mut held = cell.borrow_mut();
                        let Some(index) = held.iter().position(|kept| *kept == byte) else { return Err(self.octet_worded("missing", 1)); };
                        held.remove(index);
                    }
                    57 => { counted(0)?; cell.borrow_mut().clear(); }
                    58 => { counted(0)?; cell.borrow_mut().reverse(); }
                    _ => { counted(0)?; let copy = cell.borrow().clone(); return Ok(self.octets(copy, true)); }
                }
                return Ok(Value::Nil);
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
                    let mut content = self.octet_gathered(&values[0], true, true)?;
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
                // A thing of a blueprint built on a byte kind is of it.
                if let Value::Thing(t) = object {
                    return Ok(Value::Flag(Self::native_beneath(&t.of).as_deref() == Some(self.octet_kind_word(*wanted))));
                }
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
            4 if args.len() <= 2 => return self.octets_in_hex(&content, args),
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
            // Whatever else a row of bytes answers to it shares with
            // text, and every one of those is worked over the bytes.
            _ => return self.octets_textual(operation, &content, *changeable, args),
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
        let near = |r: &crate::data::Ratio| -> Result<f64, String> {
            if r.under && r.above == BigInt::from(0) { return Ok(-0.0); }
            let bound = crate::data::nearest_binary(&r.above, &r.beneath);
            if r.places.is_none() && !r.above.is_zero() && bound.is_infinite() {
                return Err("OverflowError: int too large to convert to float".to_string());
            }
            Ok(bound)
        };
        let b = near(&base)?;
        let e = near(&exponent)?;
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

    /// Where a key given to one of a map's own methods stands among
    /// its pairs, answered exactly as the subscript road answers it:
    /// the store's own place first, and the program's own equality —
    /// which `same_item`, a free function, cannot call — only where
    /// the place itself cannot say. The keyed form of the key comes
    /// back beside the place, ready to store should the key be new.
    fn map_locate(&mut self, entries: &[(Value, Value)], store: Option<&crate::data::MapStore>, key: &Value) -> Result<(Option<usize>, Value), String> {
        let keyed = self.hash_key(key)?;
        if let (Some(store), Ok(address)) = (store, keyed.hash_address()) {
            match store.locate(&address) {
                crate::data::Found::Found(at) => return Ok((Some(at), keyed)),
                crate::data::Found::Absent => return Ok((None, keyed)),
                crate::data::Found::Unknown => {}
            }
        }
        for (position, (stored, _)) in entries.iter().enumerate() {
            if self.keys_agree(stored, &keyed)? { return Ok((Some(position), keyed)); }
        }
        Ok((None, keyed))
    }

    /// A key written into a store-backed map, in place: `map_locate`'s
    /// own answer, grown into the pairs on a miss, or written over the
    /// pair on a hit — the very road a single `d[k] = v` already takes
    /// for a map alone in its own cell, taken here for a map alone in
    /// the working stack's care instead, so the store's place grows
    /// one pair at a time rather than being rebuilt from every pair
    /// for every key.
    fn store_write(&mut self, pairs: &mut Rc<MapStore>, key: Value, value: Value) -> Result<(), String> {
        let store: &MapStore = pairs.as_ref();
        let (found, keyed) = self.map_locate(store, Some(store), &key)?;
        match found {
            Some(at) => Rc::make_mut(pairs).overwrite_at(at, value),
            None => store_insert_new(pairs, keyed, value),
        }
        Ok(())
    }

    /// A literal or comprehension map grown by one more item: a spread
    /// map's every pair, or the one pair a plain item couples, each
    /// written into the store-backed map in its turn through
    /// `store_write`, so a key already there is kept in its first
    /// place with its value overwritten, and a new key is added last.
    fn extend_dict_grown(&mut self, prior: &mut Rc<MapStore>, item: &Value, expanded: bool) -> Result<(), String> {
        let incoming = match (expanded, item.settled()) {
            (false, Value::Couple(pair)) => vec![pair.as_ref().clone()],
            (true, Value::Dict(entries)) => entries.to_vec(),
            _ => return Err(self.table.single("ext.syntax.map.spread.unmapped").unwrap_or("A map spread needs a map").into()),
        };
        for (key, value) in incoming {
            if let Some(words) = self.cannot_key(&key) { return Err(words); }
            self.store_write(prior, key, value)?;
        }
        Ok(())
    }

    /// A pair written over the value its key already holds, or added
    /// last: `enter` in its own words, but able to call `__eq__` for
    /// a key that needs it, which the free function it replaces here
    /// could not.
    fn map_enter(&mut self, entries: &mut Vec<(Value, Value)>, key: Value, value: Value) -> Result<(), String> {
        let keyed = self.hash_key(&key)?;
        for entry in entries.iter_mut() {
            if self.keys_agree(&entry.0, &keyed)? { entry.1 = value; return Ok(()); }
        }
        entries.push((keyed, value));
        Ok(())
    }

    /// The cell a map method's receiver stands in written over with a
    /// new set of pairs, exactly as `Request::replace` writes it.
    fn replace_dict(&mut self, receiver: &Value, pairs: Vec<(Value, Value)>) -> Result<(), String> {
        let Value::Mutable(cell, _) = receiver else { return Err(self.method_fault("unready")); };
        let new_value = Value::Dict(Rc::new(pairs.into()));
        if crate::members::circular(&new_value, cell, 0) { return Err(self.method_fault("unready")); }
        cell.replace(new_value);
        Ok(())
    }

    /// `get`, `setdefault` and `pop`: the one key each is asked about
    /// is looked for by the road `map_locate` takes, so a Thing with
    /// its own `__eq__` is found by it and an int subclass by the
    /// plain int it is worth, exactly as `d[key]` finds them.
    fn dict_key_method(&mut self, receiver: &Value, name: &str, arguments: Vec<Value>) -> Result<Value, String> {
        if arguments.is_empty() || arguments.len() > 2 { return Err(self.method_fault("arguments")); }
        if matches!(arguments[0].settled(), Value::Vector(_) | Value::Dict(_)) { return Err(self.method_fault("arguments")); }
        let Value::Dict(store) = receiver.settled() else { return Err(self.method_fault("unready")); };
        let pairs = store.to_vec();
        let (found, keyed) = self.map_locate(&pairs, Some(store.as_ref()), &arguments[0])?;
        if let Some(index) = found {
            let answer = pairs[index].1.clone();
            if name == "pop" {
                let mut remaining = pairs;
                remaining.remove(index);
                self.replace_dict(receiver, remaining)?;
            }
            return Ok(answer);
        }
        if name == "pop" && arguments.len() == 1 {
            return Err(self.method_fault("key") + &arguments[0].repr(&self.wording()));
        }
        let answer = arguments.get(1).cloned().unwrap_or(Value::Nil);
        if name == "setdefault" {
            let mut grown = pairs;
            grown.push((keyed, answer.clone()));
            self.replace_dict(receiver, grown)?;
        }
        Ok(answer)
    }

    /// `update`: each pair goes in as it is met, through `map_enter`,
    /// so that the pairs read before an ill-shaped one are kept when
    /// the call stops on it, as they were before, and a key that
    /// needs `__eq__` to find its place is found by it.
    fn dict_update(&mut self, receiver: &Value, arguments: Vec<Value>, keywords: &[(String, Value)]) -> Result<Value, String> {
        if arguments.len() > 1 { return Err(self.method_fault("arguments")); }
        let Value::Dict(store) = receiver.settled() else { return Err(self.method_fault("unready")); };
        let mut entries = store.to_vec();
        let mut stopped = None;
        if let Some(source) = arguments.first() {
            match source.settled() {
                Value::Dict(d) => {
                    let cell = Self::dict_cell(source);
                    let initial = cell.as_ref().map(Self::dict_extent);
                    for (key, value) in d.iter() {
                        self.map_enter(&mut entries, key.clone(), value.clone())?;
                        if cell.as_ref().map(Self::dict_extent) != initial {
                            self.replace_dict(receiver, entries)?;
                            return Err(format!("\0{}", self.table.strings("ext.builtin.core.dict.changed")[2]));
                        }
                    }
                }
                other => {
                    let says = |kind: &str| self.method_fault(kind);
                    for item in crate::members::gather(&other, &says)? {
                        let says = |kind: &str| self.method_fault(kind);
                        let values = crate::members::gather(&item, &says)?;
                        if values.len() != 2 { stopped = Some(self.method_fault("arguments")); break; }
                        self.map_enter(&mut entries, values[0].clone(), values[1].clone())?;
                    }
                }
            }
        }
        if stopped.is_none() { for (key, value) in keywords { self.map_enter(&mut entries, Value::text(key), value.clone())?; } }
        let previous_turn = (entries.len() == store.len()).then_some(store.serial);
        self.replace_dict(receiver, entries)?;
        if let (Some(serial), Some(owner)) = (previous_turn, Self::dict_cell(receiver)) {
            let mut held = owner.borrow_mut();
            match &mut *held {
                Value::Dict(current) => Rc::make_mut(current).serial = serial,
                _ => unreachable!("dictionary receiver"),
            }
        }
        match stopped { Some(words) => Err(words), None => Ok(Value::Nil) }
    }

    /// `dict.fromkeys`: a new map with a key for each member its
    /// iterable target gives, every one holding the fill value it was
    /// given, or nothing — deduplicated through `map_locate`, the
    /// store's own place first and `keys_agree` (rather than the free
    /// function `same_item`) only where that leaves things unknown, so
    /// a Thing with its own `__eq__` still collapses to the one key
    /// CPython gives it, and a target of many keys is not scanned
    /// again for every one it grows by.
    fn dict_fromkeys(&mut self, target: &Value, filling: Value) -> Result<Value, String> {
        // `crate::members::gather` only knows the handful of container
        // kinds it lists by name and has no interpreter to run a
        // generator or a thing's own walk with; `gathered_members` is
        // the general road every other iterable-taking builtin walks,
        // so a generator expression handed to `fromkeys` is read out
        // here exactly as a `for` loop over it would read it.
        let members = self.gathered_members(target)?;
        let mut entries: Rc<MapStore> = Rc::new(Vec::new().into());
        for key in members {
            let store: &MapStore = entries.as_ref();
            let (found, keyed) = self.map_locate(store, Some(store), &key)?;
            if found.is_none() { store_insert_new(&mut entries, keyed, filling.clone()); }
        }
        Ok(Value::Dict(entries).keep(false))
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
        self.appointed_within(subject, index).map(|(body, _)| body)
    }

    /// The special method and the frame it runs in. A method handed
    /// over from its address keeps the frame its class was formed in,
    /// which a class inside a function needs beneath it to reach any
    /// name outside its own; one the plan carries runs from the
    /// outermost, having no other.
    fn appointed_within(&self, subject: &Value, index: usize) -> Option<(Rc<Routine>, Rc<Env>)> {
        match self.appointment(subject, index) {
            Some(Value::Bound(body, frame)) => Some((body, frame)),
            Some(Value::Routine(body)) => Some((body, self.outermost.clone())),
            _ => None,
        }
    }

    /// A membership complaint the table words about a kind: the words
    /// before it, the kind, and the words after it.
    fn membership_words(&self, label: &str, kind: &str) -> String {
        let words = self.table.strings(label);
        format!("{}{kind}{}", words.first().map_or("", |w| w.as_str()), words.get(1).map_or("", |w| w.as_str()))
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
            Some(Value::Routine(_) | Value::Bound(..)) => {
                let (body, scope) = self.appointed_within(subject, index).expect("the method just found");
                let arguments = std::iter::once(subject.clone()).chain(tail.iter().cloned()).collect();
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
        Self::carries_instance_past(value, &mut Vec::new(), true)
    }

    /// Whether an operand must go the thing-aware road of a working.
    /// A set holding things is written out through each thing's own
    /// words, but its workings (`&`, `|`, `<=` and the rest) already
    /// ask each thing for its hash and its equality on the plain road,
    /// the only one that also takes a set's subclass by its worth; so
    /// a working does not look into a set's members.
    fn operand_carries_instance(value: &Value) -> bool {
        Self::carries_instance_past(value, &mut Vec::new(), false)
    }

    /// The scan proper, remembering the cells passed through so that a
    /// collection reaching itself is not looked into without end.
    fn carries_instance_past(value: &Value, passed: &mut Vec<usize>, into_sets: bool) -> bool {
        match value {
            Value::Backtrace(_) | Value::Keyed(..) | Value::Attributes(_) | Value::Thing(_) | Value::Method(..) => true,
            Value::Shared(cell) | Value::Mutable(cell, _) => {
                let address = Rc::as_ptr(cell) as usize;
                if passed.contains(&address) { return false; }
                passed.push(address);
                Self::carries_instance_past(&cell.borrow(), passed, into_sets)
            }
            Value::Row(v) | Value::Tuple(v) | Value::Vector(v) => v.iter().any(|item| Self::carries_instance_past(item, passed, into_sets)),
            Value::Dict(d) => d.iter().flat_map(|(k, v)| [k, v]).any(|item| Self::carries_instance_past(item, passed, into_sets)),
            // A set cannot hold itself (nothing that may be altered is
            // hashable), but one set can stand inside many others, a
            // frozen set of frozen sets most of all; one already looked
            // into held no thing (the walk would have stopped there), so
            // it is passed over the next time rather than walked again.
            Value::Set(store) => {
                if !into_sets { return false; }
                let address = Rc::as_ptr(store) as *const () as usize;
                if passed.contains(&address) { return false; }
                passed.push(address);
                store.try_borrow().map_or(false, |held| held.entries.iter().any(|(_, item)| Self::carries_instance_past(item, passed, into_sets)))
            }
            _ => false,
        }
    }

    /// Whether the table asks that a collection written as text read as
    /// its representation does, every member inside it written as a
    /// representation, so text keeps its quotes and a map its braces.
    fn collections_read_alike(&self) -> bool {
        self.table.lone("system.collection.render") == Some("representation")
    }

    /// Whether a thing over this worth puts its own blueprint's name
    /// before the worth when it is written. A set and a row of bytes
    /// open to change both do; every other native kind is written as
    /// the worth by itself.
    fn worth_leads_with_name(worth: &Value) -> bool {
        matches!(worth.settled(), Value::Set(_) | Value::Octets { changeable: true, .. })
    }

    /// A thing over a native worth, written. Where the kind leads with
    /// its own name the name to lead with is the thing's blueprint, so
    /// a class built on a set or on a changeable row of bytes is named
    /// where the builtin would have named itself.
    fn underlying_words(&mut self, name: &str, worth: &Value, quoted: bool) -> Result<String, String> {
        if !Self::worth_leads_with_name(worth) { return self.object_words(worth, quoted); }
        // The worth with nothing leading it: a changeable row of bytes
        // reads as the fixed row of the same bytes, a set as its
        // members between braces.
        let bare = match worth.settled() {
            Value::Octets { cell, changeable: true, .. } => { let copy = cell.borrow().clone(); self.octets(copy, false) }
            held => held,
        };
        Ok(format!("{name}({})", self.object_words(&bare, true)?))
    }

    fn object_words(&mut self, subject: &Value, quoted: bool) -> Result<String, String> {
        if !matches!(subject, Value::Dict(_) | Value::Vector(_) | Value::Tuple(_) | Value::Set(_)) {
            return self.object_words_inner(subject, quoted);
        }
        let ceiling = self.table.count("ext.system.recursion.limit");
        if ceiling.is_some_and(|limit| self.standing >= limit) {
            if let Some(told) = self.table.single("ext.system.recursion.exceeded") {
                return Err(format!("\0{told}"));
            }
        }
        self.standing += 1;
        let text = self.object_words_inner(subject, quoted);
        self.standing -= 1;
        text
    }

    fn object_words_inner(&mut self, subject: &Value, quoted: bool) -> Result<String, String> {
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
                let whole = Value::Mutable(place, false);
                if self.collections_read_alike() { return Ok(whole.repr(&self.wording())); }
                return Ok(self.show(std::slice::from_ref(&whole)));
            }
            return self.object_words(&inner, quoted || represented);
        }
        if matches!(subject, Value::Backtrace(_)) { return Ok(String::from("<traceback object>")); }
        if self.table.strings("ext.stmt.class.special").is_empty() || (!quoted && !Self::carries_instance(subject)) {
            // A collection standing in no cell of its own is shown the
            // same way, by the walk that stops where it comes round.
            if self.collections_read_alike() && matches!(subject, Value::Vector(_) | Value::Dict(_) | Value::Tuple(_) | Value::Row(_)) {
                return Ok(subject.repr(&self.wording()));
            }
            return Ok(self.show(std::slice::from_ref(subject)));
        }
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
                    if !own { let name = t.of.name.clone(); return self.underlying_words(&name, &under, quoted); }
                }
                let chosen = usize::from(quoted || self.appointment(subject, 0).is_none());
                match self.ask_special(subject, chosen, &[])? {
                    None => {
                        let module = self.detail("main");
                        Ok(if module.is_empty() { format!("<{} object>", t.of.name) }
                            else { format!("<{module}.{} object at 0x1>", t.of.name) })
                    }
                    Some(Value::Text(s)) => Ok(s.to_string()),
                    _ => Err(self.bad_answer()),
                }
            }
            // A bound method is written by the routine's own full name
            // and the thing it is bound to, as CPython writes it; a
            // language with no word for the running module keeps the
            // old writing.
            Value::Method(routine, receiver) if !self.detail("main").is_empty() => {
                let of = self.object_words(&Value::Thing(receiver.clone()), true)?;
                let named = if routine.qualification.is_empty() { routine.ident.as_str() } else { routine.qualification.as_str() };
                Ok(format!("<bound method {named} of {of}>"))
            }
            // This walk holds a collection's members and not the cell
            // about them, so it leaves its own note on the members it
            // is within. A collection reached from inside itself is
            // shown as the marks it would have stood between, while one
            // reached twice by two roads is shown whole on each.
            Value::Vector(v) => {
                let among = Among::members(subject);
                if let Some(marks) = among.instead { return Ok(marks.to_string()); }
                let pieces = v.iter().map(|x| self.object_words(x, true)).collect::<Result<Vec<_>, _>>()?;
                Ok(String::from("[") + &pieces.join(", ") + "]")
            }
            // A fixed row is shown the same way, save that a row of one
            // member keeps the comma marking it a row and not a bracket.
            Value::Tuple(v) | Value::Row(v) => {
                let among = Among::members(subject);
                if let Some(marks) = among.instead { return Ok(marks.to_string()); }
                let pieces = v.iter().map(|x| self.object_words(x, true)).collect::<Result<Vec<_>, _>>()?;
                Ok(String::from("(") + &pieces.join(", ") + if v.len() == 1 { "," } else { "" } + ")")
            }
            Value::Dict(d) => {
                let among = Among::members(subject);
                if let Some(marks) = among.instead { return Ok(marks.to_string()); }
                let pieces = d.iter().map(|(k, v)| {
                    Ok(self.object_words(k, true)? + ": " + &self.object_words(v, true)?)
                }).collect::<Result<Vec<_>, String>>()?;
                Ok(String::from("{") + &pieces.join(", ") + "}")
            }
            // A set cannot reach itself, so its members need no note
            // left on them the way a list's or a map's do; each is
            // asked for its own representation, an instance's own
            // `__repr__` among them, and not the plain address every
            // other reader of a set's members is given.
            Value::Set(store) => {
                let members = store.borrow().values();
                if members.is_empty() { return Ok(store.borrow().spelling.clone() + "()"); }
                let mut pieces = Vec::with_capacity(members.len());
                for item in &members { pieces.push(self.object_words(item, true)?); }
                let inner = String::from("{") + &pieces.join(", ") + "}";
                Ok(if store.borrow().sealed { store.borrow().spelling.clone() + "(" + &inner + ")" } else { inner })
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
                let (routine, scope) = self.appointed_within(source, 16).ok_or_else(|| self.bad_answer())?;
                // A walk the method itself steps on through another
                // thing's own method parks what that one raised and says
                // so in words; the parked value is what the method raised.
                let stepped = match self.invoke(routine, scope, vec![source.clone()]) {
                    Err(Escape::Error(_)) if self.got_away.is_some() => Err(self.got_away.take().expect("what got away")),
                    other => other,
                };
                match stepped {
                    Ok(v) => Ok(Some(v)),
                    Err(Escape::Thrown(Value::Thing(t))) if self.table.strings("ext.stmt.class.special.stop").iter().any(|name| t.of.goes_by(name, false)) => Ok(None),
                    // The kernel says a walk is over in words of its own,
                    // with no value raised behind them. A method handing
                    // over the members of another walk meets those words
                    // when that walk ends, and ends there too.
                    Err(Escape::Error(s)) if self.table.strings("ext.stmt.class.special.stop").iter().any(|name| s == *name || s.starts_with(&format!("{name}:"))) => Ok(None),
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

    /// The whole number a thing stands for by its index method: nothing
    /// where it has none, the number where it answered one, and a
    /// complaint naming the kind where it answered anything else.
    fn stood_for_whole(&mut self, item: &Value) -> Result<Option<Value>, String> {
        let Some(answer) = self.ask_special(item, 43, &[])? else { return Ok(None) };
        match answer {
            Value::Flag(truth) => Ok(Some(Value::Small(i64::from(truth)))),
            Value::Small(_) | Value::Huge(_) => Ok(Some(answer)),
            other => {
                let pieces = self.table.strings("ext.stmt.class.index.amiss");
                Err(format!("{}{}{}", pieces.first().map_or("", String::as_str), other.kind_word(), pieces.get(1).map_or("", String::as_str)))
            }
        }
    }

    /// The complaint for a working neither operand's methods would
    /// take: its sign and the two kinds set among the four pieces the
    /// table gives.
    fn operands_refused(&self, sign: &str, left: &Value, right: &Value) -> String {
        let pieces = self.table.strings("ext.stmt.class.binary.amiss");
        if pieces.len() != 4 { return self.bad_answer(); }
        format!("{}{sign}{}{}{}{}{}", pieces[0], pieces[1], left.kind_word(), pieces[2], right.kind_word(), pieces[3])
    }

    /// Whether a value is one the machine has words of its own for: a
    /// thing built from a blueprint, or a collection carrying one, in a
    /// language that names the special methods at all.
    pub(super) fn speaks_for(&self, item: &Value) -> bool {
        Self::carries_instance(item) && !self.table.strings("ext.stmt.class.special").is_empty()
    }

    /// A thing written to a specification: by its own method, by the
    /// worth beneath it, or as its text where nothing was specified.
    pub(super) fn thing_in_spec(&mut self, item: &Value, spec: &str) -> Result<String, String> {
        match self.ask_special(item, 72, &[Value::text(spec)])? {
            Some(Value::Text(shown)) => return Ok(shown.to_string()),
            Some(_) => return Err(self.bad_answer()),
            None => (),
        }
        if let Some(worth) = Self::underlying(item) {
            let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
            return layout.present(&worth.settled(), spec, "");
        }
        if spec.is_empty() { return self.object_words(item, false); }
        let pieces = self.table.strings("ext.stmt.class.format.amiss");
        Err(format!("{}{}{}", pieces.first().map_or("", String::as_str), item.kind_word(), pieces.get(1).map_or("", String::as_str)))
    }

    fn user_operation(&mut self, operation: Prim, operands: &[Value]) -> Result<Option<Value>, String> {
        if let (Prim::Hashed, [method @ (Value::Method(..) | Value::Wrapped(3, _))]) = (operation, operands) {
            return Ok(method.hash_number().map(Value::Small));
        }
        if self.table.strings("ext.stmt.class.special").is_empty() { return Ok(None); }
        // A compound write asks the thing it lands on for its in-place
        // answer first; declined or absent, the plain working runs.
        if let (Prim::Landing(place), [held, by]) = (operation, operands) {
            let thing = held.settled();
            if matches!(thing, Value::Thing(_)) {
                if let Some(answer) = self.ask_special(&thing, 47 + usize::from(place), std::slice::from_ref(by))? {
                    if !matches!(answer, Value::Refusal(_)) { return Ok(Some(answer)); }
                }
            }
            return Ok(None);
        }
        // Writing a value out looks into a set's members for a thing; a
        // working on values does not (see operand_carries_instance).
        let writes = matches!(operation, Prim::Quoted | Prim::Asciied | Prim::AsText);
        if Self::is_core_primitive(operation) && !operands.iter().any(|v| (if writes { Self::carries_instance(v) } else { Self::operand_carries_instance(v) }) || matches!(v, Value::Cursor(_))) { return Ok(None); }
        if operation == Prim::Belongs { return Ok(None); }
        // Text before the remainder sign lays its own marks out, which
        // is the working the left side's own method names. The value on
        // the right gives the marks its words and is never asked for the
        // turned-about answer, which it has no business giving.
        if let (Prim::Mod, [Value::Text(pattern), right], true) = (operation, operands, self.table.flag("ext.op.rem.formats_text")) {
            // A thing built over text stands below the left side's own
            // kind, and the language asks its turned-about method first,
            // so such a thing keeps that road.
            let over_text = matches!(Self::underlying(right).map(|worth| worth.settled()), Some(Value::Text(_)));
            if !(over_text && self.appointed(right, 31).is_some()) {
                let (pattern, right) = (pattern.clone(), right.clone());
                return self.text_remainder(&pattern, &right).map(|filled| Some(Value::text(&filled)));
            }
        }
        let pair = match operation {
            Prim::Plus => Some((18, 26)), Prim::Minus => Some((19, 27)), Prim::Times => Some((20, 28)),
            Prim::Over | Prim::OverReal => Some((21, 29)), Prim::IntDiv => Some((22, 30)),
            Prim::Mod => Some((23, 31)), Prim::Power => Some((24, 32)),
            Prim::MatrixProduct => Some((45, 46)),
            Prim::BitsUp => Some((62, 67)), Prim::BitsDown => Some((63, 68)),
            Prim::BitsBoth => Some((64, 69)), Prim::BitsEither => Some((65, 70)), Prim::BitsOne => Some((66, 71)),
            Prim::Lt => Some((4, 6)), Prim::Gt => Some((6, 4)), Prim::Le => Some((5, 7)), Prim::Ge => Some((7, 5)),
            Prim::Eq => Some((2, 2)), Prim::Ne => Some((3, 3)),
            Prim::At | Prim::Toward | Prim::Apart => Some((11, usize::MAX)),
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
            if operation == Prim::Times {
                let row_side = [left, right].iter().position(|value|
                    matches!(value, Value::Vector(_) | Value::Tuple(_) | Value::Text(_) | Value::Octets { .. }));
                if let Some(at) = row_side {
                    let count = if at == 0 { right } else { left };
                    if matches!(count, Value::Thing(_)) && Self::underlying(count).is_none() {
                        if let Some(whole) = self.stood_for_whole(count)? {
                            let mut normalized = operands.to_vec();
                            normalized[1 - at] = whole;
                            return self.prim(operation, "", &normalized).map(Some);
                        }
                    }
                }
            }
            // An arithmetic, matrix or bit working with a thing of no
            // native worth on either side, which no method took, is
            // refused with its sign and both kinds named.
            let bare = |v: &Value| matches!(v, Value::Thing(_)) && Self::underlying(v).is_none();
            if forward >= 18 && (bare(left) || bare(right)) {
                let sign = self.written_as(&operation);
                return Err(self.operands_refused(&sign, left, right));
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
        // A thing sought among a set's members is sought as a key is,
        // by its own hash and its own equality, before the worth
        // beneath it is let to stand in its place: the worth would be
        // asked for an address of its own kind and have none.
        if let (Prim::Contains | Prim::Absent, [needle @ (Value::Thing(_) | Value::Keyed(..)), haystack]) = (operation, operands) {
            if let Value::Set(store) = haystack.settled() {
                if let Value::Set(candidate) = self.set_search_item(needle) {
                    let present = store.borrow().keys.contains(&candidate.borrow().whole_address());
                    return Ok(Some(Value::Flag(if operation == Prim::Absent { !present } else { present })));
                }
                let keyed = self.hash_key(needle)?;
                let members = store.borrow().values();
                let mut found = false;
                for held in members { if self.keys_agree(&held, &keyed)? { found = true; break; } }
                return Ok(Some(Value::Flag(found != (operation == Prim::Absent))));
            }
        }
        let free_at: &[usize] = match operation {
            Prim::Length => &[10], Prim::Hashed => &[8], Prim::Truthful | Prim::AsTruth => &[9, 10], Prim::NextItem => &[16],
            Prim::AsText => &[0, 1], Prim::Quoted | Prim::Asciied => &[1], Prim::AsInt => &[38], Prim::AsReal => &[39], Prim::Magnitude => &[40],
            Prim::Contains | Prim::Absent => &[14],
            Prim::Iterator | Prim::Listed | Prim::Ordered | Prim::Tupling | Prim::Uniques | Prim::Backwards | Prim::Numbered | Prim::Zipped | Prim::Mapped | Prim::Filtered | Prim::EveryTrue | Prim::SomeTrue | Prim::Least | Prim::Greatest => &[15],
            Prim::Rounded | Prim::QuotRem | Prim::Powered | Prim::Hexadecimal | Prim::Octal | Prim::Binary => &[],
            Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power | Prim::At | Prim::Fetch => &[],
            // The bit workings reach the worth beneath a thing as the
            // arithmetic ones do, the blueprint's own methods for them
            // having been asked above: a thing over a whole number is
            // that number to them, and one over a set is that set.
            Prim::BitsUp | Prim::BitsDown | Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne => &[],
            _ => &[usize::MAX],
        };
        if free_at != [usize::MAX] {
            let mut settled = Vec::with_capacity(operands.len());
            let mut changed = false;
            for operand in operands {
                match self.underlying_unless(operand, free_at) {
                    // Where a kind leads with its own name the thing is
                    // written by its blueprint, so there the worth is
                    // not allowed to stand in for the thing.
                    Some(worth) if matches!(operation, Prim::Quoted | Prim::Asciied | Prim::AsText) && Self::worth_leads_with_name(&worth) => settled.push(operand.clone()),
                    Some(worth) => { settled.push(worth); changed = true; }
                    None => settled.push(operand.clone()),
                }
            }
            if changed && !settled.iter().any(|v| (if writes { Self::carries_instance(v) } else { Self::operand_carries_instance(v) }) || matches!(v, Value::Cursor(_))) {
                let word = self.table.prims.iter().find(|(_, p)| **p == operation).map(|(w, _)| w.clone()).unwrap_or_default();
                let outcome = self.prim(operation, &word, &settled);
                if let Ok(Value::Tuple(values)) = &outcome {
                    let inherited_tuple = operation == Prim::Times && operands.iter().any(|value| {
                        match Self::underlying(value).map(|worth| worth.settled()) {
                            Some(Value::Tuple(source)) => Rc::ptr_eq(&source, values),
                            _ => false,
                        }
                    });
                    if operation == Prim::Tupling || inherited_tuple {
                        let copied = values.iter().cloned().collect();
                        return Ok(Some(Value::Tuple(Rc::new(copied))));
                    }
                }
                // A thing built on a native kind is named by its own
                // blueprint where the refusal names the kinds it was
                // handed, and not by the worth standing beneath it: an
                // arithmetic or bit working refused outright, or two
                // kinds with no order between them.
                if let (Err(told), [one, other], [x, y]) = (&outcome, operands, settled.as_slice()) {
                    let sign = self.sign_named(operation);
                    if *told == self.operands_refused(&sign, x, y) {
                        return Err(self.operands_refused(&sign, one, other).into());
                    }
                    if matches!(operation, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) && *told == self.unordered_complaint(operation, x, y) {
                        return Err(self.unordered_complaint(operation, one, other).into());
                    }
                }
                return Ok(Some(outcome?));
            }
        }
        let value = match (operation, operands) {
            (Prim::Of | Prim::HasMember, [Value::Backtrace(link), Value::Text(member)]) => {
                let roster = self.table.strings("ext.builtin.exceptions.traceback");
                let at = roster.iter().position(|key| key == member.as_ref());
                if operation == Prim::HasMember { return Ok(Some(Value::Flag(matches!(at, Some(1..=3))))); }
                return match at {
                    Some(1) => Ok(Some(Value::Small(link.location as i64))),
                    Some(2) => Ok(Some(link.following.clone())),
                    Some(3) => Ok(Some(Value::Thing(link.activation.clone()))),
                    _ => Err(format!("AttributeError: 'traceback' object has no attribute '{}'", member)),
                };
            },
            // What whole number, real, magnitude, or positive form a thing
            // stands for, by its own methods where it has them.
            (Prim::AsInt | Prim::AsReal | Prim::Magnitude | Prim::Positive | Prim::NumberAlone, [subject @ Value::Thing(_)]) => {
                let index = match operation { Prim::AsInt => 38, Prim::AsReal => 39, Prim::Magnitude => 40, _ => 41 };
                match self.ask_special(subject, index, &[])? {
                    Some(answer) => answer,
                    None if index == 41 => match Self::underlying(subject) {
                        Some(number @ Value::Complex(_)) => number,
                        _ => return Ok(None),
                    },
                    None => return Ok(None),
                }
            }
            // Inversion is the thing's own where it has the method, and
            // its worth's where it stands on a native kind.
            (Prim::BitsOver, [subject @ Value::Thing(_)]) => match self.ask_special(subject, 44, &[])? {
                Some(answer) => answer,
                None => match Self::underlying(subject) {
                    Some(worth) => self.prim(operation, "", &[worth.settled()])?,
                    None => return Ok(None),
                },
            },
            // A thing standing for a whole number is that number where a
            // row, a text, a tuple or a progression is read, or a row or
            // a text shortened, at a place.
            (Prim::At | Prim::Fetch | Prim::Toward | Prim::Apart | Prim::Erase, [row, key @ Value::Thing(_)]) if matches!(row.settled(), Value::Vector(_) | Value::Tuple(_) | Value::Text(_) | Value::Progression(_) | Value::Octets { .. }) => {
                match self.stood_for_whole(key)? {
                    Some(whole) => self.prim(operation, "", &[row.clone(), whole])?,
                    None => return Ok(None),
                }
            }
            // A range wants whole numbers: each thing among its bounds
            // is asked for the one it stands for.
            (Prim::Span, _) if operands.iter().any(|v| matches!(v, Value::Thing(_))) => {
                let mut told = Vec::with_capacity(operands.len());
                for bound in operands {
                    told.push(match self.stood_for_whole(bound)? { Some(whole) => whole, None => bound.clone() });
                }
                let word = self.table.prims.iter().find(|(_, p)| **p == operation).map(|(w, _)| w.clone()).unwrap_or_default();
                self.prim(operation, &word, &told)?
            }
            // So does a radix rendering.
            (Prim::Hexadecimal | Prim::Octal | Prim::Binary, [item @ Value::Thing(_)]) => {
                let Some(whole) = self.stood_for_whole(item)? else { return Ok(None) };
                let word = self.table.prims.iter().find(|(_, p)| **p == operation).map(|(w, _)| w.clone()).unwrap_or_default();
                self.prim(operation, &word, &[whole])?
            }
            // Rounding is the thing's own where it has the method, given
            // the places if any were asked.
            (Prim::Rounded, [item @ Value::Thing(_), places @ ..]) => match self.ask_special(item, 73, places)? {
                Some(answer) => answer,
                None => return Ok(None),
            },
            // Division with remainder asks the left thing, then the right
            // one reflected; refused by both, it is refused by name.
            (Prim::QuotRem, [left, right]) if operands.iter().any(|v| matches!(v, Value::Thing(_))) => {
                for (subject, place, other) in [(left, 60, right), (right, 61, left)] {
                    if let Some(answer) = self.ask_special(subject, place, std::slice::from_ref(other))? {
                        if !matches!(answer, Value::Refusal(_)) { return Ok(Some(answer)); }
                    }
                }
                let word = self.table.prims.iter().find(|(_, p)| **p == operation).map(|(w, _)| w.clone()).unwrap_or_default();
                return Err(self.operands_refused(&format!("{word}()"), left, right));
            }
            // Power without a modulus is the ordinary dyad; with one, the
            // modulus goes along to the power method and its reflection.
            (Prim::Powered, [_, _]) if operands.iter().any(|v| matches!(v, Value::Thing(_))) => self.prim(Prim::Power, "", operands)?,
            (Prim::Powered, [base, exponent, modulus]) if operands[..2].iter().any(|v| matches!(v, Value::Thing(_))) => {
                for (subject, place, other) in [(base, 24, exponent), (exponent, 32, base)] {
                    if let Some(answer) = self.ask_special(subject, place, &[other.clone(), modulus.clone()])? {
                        if !matches!(answer, Value::Refusal(_)) { return Ok(Some(answer)); }
                    }
                }
                let word = self.table.prims.iter().find(|(_, p)| **p == operation).map(|(w, _)| w.clone()).unwrap_or_default();
                return Err(self.operands_refused(&format!("{word}()"), base, exponent));
            }
            // A thing may say what complex number it stands for, and must
            // answer with one.
            (Prim::ComplexMade, [item @ Value::Thing(_)]) => match self.ask_special(item, 74, &[])? {
                Some(answer @ Value::Complex(_)) => answer,
                Some(_) => return Err(self.bad_answer()),
                None => match Self::underlying(item) {
                    Some(number @ Value::Complex(_)) => number,
                    _ => return Ok(None),
                },
            },
            // Formatting, by the builtin or by a field of a formatted
            // string; a conversion asked in the field shows the thing as
            // text first. A collection carrying a thing among its
            // members, a set among them, takes the same road: asking
            // for its own `__format__` answers nothing for anything
            // that is not itself a thing, so the road falls through to
            // the thing-aware text below rather than the plain one.
            (Prim::FormatValue, [item]) if Self::carries_instance(item) => Value::text(&self.thing_in_spec(item, "")?),
            (Prim::FormatValue, [item, Value::Text(spec)]) if Self::carries_instance(item) => {
                let spec = spec.to_string();
                Value::text(&self.thing_in_spec(item, &spec)?)
            }
            (Prim::RenderField, [item, spec, conversion]) if Self::carries_instance(item) => {
                let (spec, conversion) = (spec.bare(), conversion.bare());
                let shown = if conversion.is_empty() { self.thing_in_spec(item, &spec)? } else {
                    let text = Value::text(&self.object_words(item, conversion != "s")?);
                    crate::formatting::Layout { table: self.table, names: self.wording() }.present(&text, &spec, "")?
                };
                Value::text(&shown)
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
                // A value that could be no key of a map is in no map,
                // and the language says so rather than answering no.
                if let Some(words) = self.cannot_key(needle) { return Err(words); }
                let key = self.hash_key(needle)?;
                // A key needing no help from the program's own code is
                // sought through the store's own place, in one look
                // rather than a walk of every entry — unless the place
                // itself cannot say (some other pair carries no address
                // of its own), when only the walk can answer.
                let placed = match key.hash_address() {
                    Ok(address) => match entries.locate(&address) {
                        Found::Found(_) => Some(true),
                        Found::Absent => Some(false),
                        Found::Unknown => None,
                    },
                    Err(_) => None,
                };
                let found = match placed {
                    Some(found) => found,
                    None => {
                        let mut found = false;
                        for (stored, _) in entries.iter() { if self.keys_agree(stored, &key)? { found = true; break; } }
                        found
                    }
                };
                Value::Flag(found != (operation == Prim::Absent))
            }
            (Prim::Placed, [Value::Dict(entries), key, value]) => {
                if let Some(words) = self.cannot_key(key) { return Err(words); }
                let keyed = self.hash_key(key)?;
                let mut result = entries.to_vec();
                let placed = match keyed.hash_address() {
                    Ok(address) => match entries.locate(&address) {
                        Found::Found(pos) => Some(Some(pos)),
                        Found::Absent => Some(None),
                        Found::Unknown => None,
                    },
                    Err(_) => None,
                };
                let at = match placed {
                    Some(at) => at,
                    None => {
                        let mut at = None;
                        for (position, (stored, _)) in result.iter().enumerate() {
                            if self.keys_agree(stored, &keyed)? { at = Some(position); break; }
                        }
                        at
                    }
                };
                match at { Some(at) => result[at].1 = value.clone(), None => result.push((keyed, value.clone())) };
                Value::Dict(Rc::new(result.into()))
            }
            (Prim::Erase, [Value::Dict(entries), key]) => {
                if let Some(words) = self.cannot_key(key) { return Err(words); }
                let hashed = self.hash_key(key)?;
                let placed = match hashed.hash_address() {
                    Ok(address) => match entries.locate(&address) {
                        Found::Found(pos) => Some(Some(pos)),
                        Found::Absent => Some(None),
                        Found::Unknown => None,
                    },
                    Err(_) => None,
                };
                let at = match placed {
                    Some(at) => at,
                    None => {
                        let mut at = None;
                        for (position, (stored, _)) in entries.iter().enumerate() {
                            if self.keys_agree(stored, &hashed)? { at = Some(position); break; }
                        }
                        at
                    }
                };
                let Some(at) = at else {
                    return Err(if self.table.has_any("ext.builtin.exceptions") { self.absent_key(key) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() });
                };
                let remaining: Vec<(Value, Value)> = entries.iter().enumerate().filter(|(position, _)| *position != at).map(|(_, pair)| pair.clone()).collect();
                Value::Dict(Rc::new(remaining.into()))
            }
            // A class names the method for membership, or names nothing
            // for it: a class that sets the name to nothing has said
            // there is no membership in its things, and the asking
            // fails rather than falling back upon a walk.
            // A thing sought among a set's members is sought as a key
            // is: by its own hash and its own equality, the members
            // standing in the set as the gathering put them there.
            (Prim::Contains | Prim::Absent, [_, haystack])
                if matches!(self.appointment(haystack, 14), Some(Value::Nil)) && self.table.strings("ext.op.in.declined").len() == 2 => {
                return Err(self.membership_words("ext.op.in.declined", &haystack.kind_word()));
            }
            (Prim::Contains | Prim::Absent, [needle, haystack]) if self.appointment(haystack, 14).is_some() => {
                let found = self.ask_special(haystack, 14, std::slice::from_ref(needle))?.unwrap();
                Value::Flag(self.object_truth(&found)? != (operation == Prim::Absent))
            }
            // A thing that says nothing of membership but says how it is
            // walked is searched by walking it, and the walk stops at
            // the first member equal to the one sought.
            (Prim::Contains | Prim::Absent, [needle, haystack @ Value::Thing(_)])
                if self.appointment(haystack, 15).is_some() || self.placed_walk(haystack).is_some() => {
                let walk = self.iterated_value(haystack)?;
                let mut found = false;
                while let Some(item) = self.next_value(&walk)? {
                    // A member is the one sought where it is the very
                    // same value, before anything is asked of it; where
                    // it is another, the member is asked first whether
                    // it equals the one sought, as a member of a row is.
                    let alike = if item.one_place(needle) { true } else {
                        match self.user_operation(Prim::Eq, &[item.clone(), needle.clone()])? {
                            Some(said) => self.object_truth(&said)?,
                            None => contained_equal(&item, needle),
                        }
                    };
                    if alike { found = true; break; }
                }
                Value::Flag(found != (operation == Prim::Absent))
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
                if let Ok(address) = hashed.hash_address() {
                    match entries.locate(&address) {
                        Found::Found(position) => return Ok(Some(entries[position].1.clone())),
                        Found::Absent => return Ok(None),
                        Found::Unknown => {}
                    }
                }
                for (candidate, value) in entries.iter() {
                    if self.keys_agree(candidate, &hashed)? { return Ok(Some(value.clone())); }
                }
                return Ok(None);
            }
            (Prim::Invert, [one]) => Value::Flag(!self.object_truth(one)?),
            (Prim::Negate, [one]) if self.appointed(one, 25).is_some() => self.ask_special(one, 25, &[])?.unwrap(),
            (Prim::Quoted, [one]) => Value::text(&self.object_words(one, true)?),
            // The ascii builtin writes what the quoting builtin writes
            // and then puts every letter outside ASCII into the escape
            // that stands for it.
            (Prim::Asciied, [one]) => {
                let said = self.object_words(one, true)?;
                Value::text(&crate::text::ascii_escaped(&said))
            }
            (Prim::AsText, [one]) => match one {
                Value::Unpaired(_) => one.clone(),
                _ => { self.figures_allowed(one)?; Value::text(&self.object_words(one, false)?) }
            },
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
            (Prim::Quoted | Prim::Asciied | Prim::Truthful | Prim::Hashed | Prim::Ordered | Prim::Iterator | Prim::NextItem | Prim::Belongs, _) => return Err(self.bad_answer()),
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
    /// Two rows, or two tuples, one of which holds a thing somewhere
    /// among its members: equal where every pair is, a member being
    /// equal to itself before anything is asked and any other pair asked
    /// as the program asks it, so a thing on either side answers for
    /// itself. Nothing where neither holds a thing.
    fn alike_with_things(&mut self, one: &Value, two: &Value) -> Result<Option<bool>, String> {
        fn holds_thing(value: &Value) -> bool {
            match value {
                Value::Thing(_) => true,
                Value::Vector(held) | Value::Tuple(held) => held.iter().any(holds_thing),
                Value::Shared(cell) | Value::Mutable(cell, _) => holds_thing(&cell.borrow()),
                _ => false,
            }
        }
        let ((Value::Vector(left), Value::Vector(right)) | (Value::Tuple(left), Value::Tuple(right))) = (one, two) else { return Ok(None) };
        if !left.iter().chain(right.iter()).any(holds_thing) { return Ok(None); }
        if left.len() != right.len() { return Ok(Some(false)); }
        for (here, there) in left.iter().zip(right.iter()) {
            if contained_equal(here, there) { continue; }
            let said = self.prim(Prim::Eq, "", &[here.clone(), there.clone()])?;
            if !self.object_truth(&said)? { return Ok(Some(false)); }
        }
        Ok(Some(true))
    }

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

    /// The kind a value is held to when it is asked whether it stands in
    /// order beside another. It is the kind's own word, save that a row
    /// of bytes and one that can be written into are one family here, as
    /// they are for equality: either stands in order beside the other,
    /// though a refusal still names each of them apart.
    fn order_family(value: &Value) -> String {
        let word = value.kind_word();
        match word.as_str() {
            "bytearray" => "bytes".to_owned(),
            // A view of a map's keys or its pairs orders itself beside
            // a set the way a set does; a view of its values takes no
            // order at all, as CPython leaves it.
            "frozenset" | "dict_keys" | "dict_items" => "set".to_owned(),
            _ => word,
        }
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

    /// Whether two maps hold the same pairs, key for key, by the very
    /// road the subscript takes rather than `equal_contents`'s
    /// `one_place`/`equals`, which cannot call a key's own `__eq__`:
    /// same length, and every key of the one found among the other's
    /// through `keys_agree`, its value equal to the one paired with it
    /// there. A value that is itself a Thing keeps to `equal_contents`;
    /// only a map's own keys need the interpreter to compare them.
    fn dicts_equal(&mut self, one: &crate::data::MapStore, other: &crate::data::MapStore) -> Result<bool, String> {
        if one.len() != other.len() { return Ok(false); }
        // Each key of the one is sought among the other's through
        // `map_locate`, which is the store's own place first and a
        // walk only where the place cannot say — so a map of plain
        // keys stays the linear comparison it always was, and only a
        // map with a Thing among its keys pays for the walk.
        for (key, value) in one.iter() {
            let (found, _) = self.map_locate(other, Some(other), key)?;
            match found {
                None => return Ok(false),
                Some(index) => {
                    let rhs = &other[index].1;
                    if value.one_place(rhs) { continue; }
                    let verdict = self.prim(Prim::Eq, "", &[value.clone(), rhs.clone()])?;
                    if !self.object_truth(&verdict)? { return Ok(false); }
                }
            }
        }
        Ok(true)
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
            self.map_enter(&mut result, key, value)?;
        }
        Ok(Value::Dict(Rc::new(result.into())))
    }

    fn collection_cell(&self, value: Value) -> Value {
        match value {
            Value::Vector(_) | Value::Dict(_) if self.names_in_calls =>
                Value::Shared(Rc::new(RefCell::new(value))),
            _ => value,
        }
    }

    /// One member added into `sum`'s running total: the exact reckoning
    /// where the two are numbers, and the plain working of `+`
    /// otherwise, so a member `+` itself refuses is refused in the
    /// very words `+` already gives, and one it joins (a row, a tuple)
    /// joins.
    fn sum_added(&mut self, total: &Value, item: &Value) -> Result<Value, String> {
        match math::compute(Calc::Plus, total, item) {
            Some(answer) => answer,
            None => self.prim(Prim::Plus, "+", &[total.clone(), item.clone()]),
        }
    }

    /// Literal members keep their cells. Other operations ask the
    /// contents of their arguments, leaving those cells where they were.
    fn sequence_shape(value: &Value) -> u8 {
        match value {
            Value::Vector(_) => 1,
            Value::Tuple(_) => 2,
            Value::Shared(storage) | Value::Mutable(storage, _) => Self::sequence_shape(&storage.borrow()),
            _ => 0,
        }
    }

    fn compare_sequences(&mut self, operation: Prim, first: &Value, second: &Value) -> Result<Value, String> {
        if self.table.count("ext.system.recursion.limit").is_some_and(|n| self.standing >= n) {
            if let Some(message) = self.table.single("ext.system.recursion.exceeded") { return Err(format!("\0{message}")); }
        }
        self.standing += 1;
        let compared = (|| {
            let extract = |value: &Value| match value.settled() {
                Value::Vector(row) | Value::Tuple(row) => row,
                _ => unreachable!(),
            };
            if matches!(first.settled(), Value::Vector(_)) && matches!(operation, Prim::Eq | Prim::Ne)
                && extract(first).len() != extract(second).len() {
                return Ok(Value::Flag(operation == Prim::Ne));
            }
            let mut position = 0;
            loop {
                let a = extract(first);
                let b = extract(second);
                if position >= a.len().min(b.len()) {
                    let length_order = a.len().cmp(&b.len());
                    let truth = match operation {
                        Prim::Eq => length_order.is_eq(), Prim::Ne => !length_order.is_eq(),
                        Prim::Lt => length_order.is_lt(), Prim::Gt => length_order.is_gt(),
                        Prim::Le => length_order.is_le(), _ => length_order.is_ge(),
                    };
                    return Ok(Value::Flag(truth));
                }
                if self.member_agrees(&a[position], &b[position])? { position += 1; continue; }
                if operation == Prim::Eq { return Ok(Value::Flag(false)); }
                if operation == Prim::Ne { return Ok(Value::Flag(true)); }
                return self.prim(operation, "", &[a[position].clone(), b[position].clone()]);
            }
        })();
        self.standing -= 1;
        compared
    }

    fn member_agrees(&mut self, item: &Value, sought: &Value) -> Result<bool, String> {
        if item.one_place(sought) { Ok(true) } else {
            let verdict = self.prim(Prim::Eq, "", &[item.clone(), sought.clone()])?;
            self.object_truth(&verdict)
        }
    }

    fn prim(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        if matches!(op, Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
            if let [first, second] = v {
                let shape = Self::sequence_shape(first);
                let comparable = shape != 0 && shape == Self::sequence_shape(second);
                if comparable && self.works_sequences() { return self.compare_sequences(op, first, second); }
            }
        }
        if matches!(op, Prim::Contains | Prim::Absent) && self.works_sequences() {
            if let [sought, sequence] = v {
                if Self::sequence_shape(sequence) != 0 {
                    let mut cursor = 0;
                    let mut present = false;
                    loop {
                        let values = sequence.settled();
                        let (Value::Vector(values) | Value::Tuple(values)) = values else { break };
                        let Some(value) = values.get(cursor) else { break };
                        if self.member_agrees(value, sought)? { present = true; break; }
                        cursor += 1;
                    }
                    return Ok(Value::Flag(present != (op == Prim::Absent)));
                }
            }
        }
        if op == Prim::Plus && v.len() == 2 && v.iter().any(|x| matches!(x, Value::Unpaired(_))) {
            if let (Some(first), Some(last)) = (v[0].character_numbers(), v[1].character_numbers()) {
                return Ok(Value::characters(first.into_iter().chain(last).collect()));
            }
        }
        if v.len() == 2 && v.iter().any(|x| matches!(x, Value::Unpaired(_))) {
            if let (Some(needle), Some(hay)) = (v[0].character_numbers(), v[1].character_numbers()) {
                match op {
                    Prim::Contains | Prim::Absent => {
                        let present = (0..=hay.len()).any(|i| hay[i..].starts_with(&needle));
                        return Ok(Value::Flag(if op == Prim::Contains { present } else { !present }));
                    }
                    Prim::Lt => return Ok(Value::Flag(needle < hay)),
                    Prim::Le => return Ok(Value::Flag(needle <= hay)),
                    Prim::Gt => return Ok(Value::Flag(needle > hay)),
                    Prim::Ge => return Ok(Value::Flag(needle >= hay)),
                    _ => {}
                }
            }
        }
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
        // Whether the word is spoken for at all is asked only once the
        // shape already says this could be a slice: most operations are
        // no `Prim::At` on a span of bounds, and the table has nothing
        // to say to them.
        if let (Prim::At, [target, Value::Span(bounds)]) = (op, v) {
            if self.table.has_any("ext.builtin.slice") {
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
        // The table is asked for those four pieces only where the
        // operation is one of the four that order at all: every other
        // operation reaching here has no use for them.
        if matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
            if let ([left, right], [before, between, and, after]) = (v, self.table.strings("ext.op.order.unsupported")) {
                // A view of a map's keys or its pairs is asked after by
                // its own kind, not the row its members would settle
                // to, so that it orders itself beside a set.
                let family_of = |value: &Value| if matches!(value, Value::Window(..)) { Self::order_family(value) } else { Self::order_family(&value.settled()) };
                // Two of one kind may still order themselves, as sets and
                // rows do; two of different kinds, or of a kind without any
                // order, cannot. The two set kinds count as one kind
                // here, either holding entries the other may hold too.
                let orderless = family_of(left) != family_of(right) || matches!(left.settled(), Value::Nil | Value::Dict(_) | Value::Progression(_));
                let (left, right) = (left.settled(), right.settled());
                let counts = |x: &Value| matches!(x, Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Flag(_));
                let texts = matches!((&left, &right), (Value::Text(_), Value::Text(_)));
                if orderless && !(counts(&left) && counts(&right)) && !texts && !matches!((&left, &right), (Value::Thing(_), _) | (_, Value::Thing(_))) && math::below(&left, &right).is_none() {
                    let sign = match op { Prim::Lt => "<", Prim::Le => "<=", Prim::Gt => ">", _ => ">=" };
                    return Err(format!("{before}{sign}{between}{}{and}{}{after}", left.kind_word(), right.kind_word()));
                }
            }
        }
        // A holder asked for one of its special members by name keeps
        // the cell its names share. Read out of that cell here, the
        // member would stand on a copy, and a write through it would
        // reach no other name for the holder.
        if let (Prim::Of, [subject, word]) = (op, v) {
            let called = word.bare();
            if self.native_member(subject, &called) { return Ok(Value::Member(Rc::new(subject.clone()), called)); }
        }
        // A window upon a map is asked whether it has a special member
        // before it settles. Settled, it is a plain row, and a row has
        // ways a window has not: the window would be credited with them.
        if let (Prim::HasMember, [subject @ Value::Window(..), word]) = (op, v) {
            let called = word.bare();
            if self.table.strings("ext.stmt.class.special").iter().any(|spelling| *spelling == called) {
                return Ok(Value::Flag(self.native_member(subject, &called)));
            }
        }
        if self.names_in_calls {
            let result = if matches!(op, Prim::ExtendLiteral(_, false)) {
                self.prim_values(op, name, &[collection_read(&v[0]), v[1].clone()])?
            } else if matches!(op, Prim::MakeArray | Prim::MakeMap | Prim::Couple | Prim::SpanOf | Prim::SliceBounds | Prim::IdentityOf | Prim::ValueMethod | Prim::Perform | Prim::Weigh | Prim::Prepare) {
                self.prim_values(op, name, v)?
            } else if (self.writes_a_row_over(op) || matches!(op, Prim::Pointed) && self.works_sequences())
                && matches!(v.first(), Some(Value::Shared(_) | Value::Mutable(..))) {
                // A row written over keeps its cell: the write is made
                // where the row stands, not on a copy of it, and the
                // cell must still be the cell when it is handed back to
                // the name, whatever is asked of it on the way.
                self.prim_values(op, name, v)?
            } else if (matches!(op, Prim::SetAssign(0) | Prim::Backwards | Prim::Iterated | Prim::Erase | Prim::Pointed)
                || matches!(op, Prim::Landing(place) if matches!(self.table.landing_working(place), Prim::SetAssign(0))))
                && v.first().map_or(false, |first| Self::dict_cell(first).is_some()) {
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
                cell.replace(Value::Dict(Rc::new(entries.into())));
                return match stopped { Some(words) => Err(words), None => Ok(left.clone()) };
            }
        }
        // A key taken out of a map held in a cell is taken out of the
        // map in that cell, so every name for the map sees it gone.
        if let (Prim::Erase, [target, key]) = (op, v) {
            if let Some(cell) = Self::dict_cell(target).filter(|_| self.names_in_calls) {
                let at = self.as_key(key);
                if let Some(words) = self.cannot_key(&at) { return Err(words); }
                let entries = match &*cell.borrow() { Value::Dict(held) => held.to_vec(), _ => Vec::new() };
                let wanted = self.hash_key(&at)?;
                if self.table.has_any("ext.stmt.del") && !self.key_held(&entries, &wanted)? {
                    return Err(if self.table.has_any("ext.builtin.exceptions") { self.absent_key(&at) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() });
                }
                let kept: Vec<(Value, Value)> = self.without_key(&entries, &wanted)?;
                cell.replace(Value::Dict(Rc::new(kept.into())));
                return Ok(target.clone());
            }
        }
        // A map walked from its cell, forwards for a loop or backwards
        // from its last key to its first, is watched for a change of
        // size on the way.
        if let (Prim::Iterated | Prim::Backwards, [source]) = (op, v) {
            if self.table.flag("ext.stmt.yield.suspends") && self.table.has_any("ext.syntax.map.resized") && Self::dict_cell(source).is_some() {
                let mut keys = self.gathered_members(source)?;
                if op == Prim::Backwards {
                    keys.reverse();
                    return Ok(self.walk_over_backwards(source, keys));
                }
                return Ok(self.walk_over(source, keys));
            }
        }
        // A view of a map's keys or pairs meets a set, or another view,
        // as a set would under the set signs, or ordered against one; the
        // reading of the map itself is no set of anything and takes
        // none of these signs.
        let not_mapping = |value: &Value| !matches!(value, Value::Window(_, 'm'));
        if matches!(op, Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne | Prim::Minus | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge)
            && v.iter().any(|value| matches!(value, Value::Window(..))) && v.iter().all(not_mapping) {
            let mut as_sets = Vec::new();
            for value in v {
                as_sets.push(match value {
                    Value::Window(..) => Value::Set(Rc::new(RefCell::new(self.gather_set(Some(&value.settled()))?))),
                    other => other.clone(),
                });
            }
            return self.prim(op, name, &as_sets);
        }
        // A view of a map's values reads equal to nothing but the very
        // view it is, the default the object kind gives a value with
        // no equality of its own, as CPython leaves it. A view of a
        // map's keys or its pairs is a set as far as equality goes,
        // and answers a set or another such view by what it holds;
        // anything else — a row, a walk, a value alone — it is no
        // equal of. The reading of the map itself answers `==` as the
        // map does: every key with its very value.
        if matches!(op, Prim::Eq | Prim::Ne) && v.iter().any(|value| matches!(value, Value::Window(..))) {
            if let [a, b] = v {
                let is_mapping = matches!(a, Value::Window(_, 'm')) || matches!(b, Value::Window(_, 'm'));
                if is_mapping {
                    let pairs_of = |value: &Value| -> Option<Vec<(Value, Value)>> {
                        match value {
                            Value::Dict(pairs) => Some(pairs.to_vec()),
                            Value::Window(owner, 'm') => match owner.settled() { Value::Dict(pairs) => Some(pairs.to_vec()), _ => None },
                            _ => None,
                        }
                    };
                    let equal = match (pairs_of(a), pairs_of(b)) {
                        (Some(one), Some(other)) => one.len() == other.len() && one.iter().all(|(k, val)| {
                            let bare = |k: &Value| match k { Value::Keyed(thing, _) => thing.as_ref().clone(), other => other.clone() };
                            let want = bare(k);
                            other.iter().any(|(k2, v2)| bare(k2).equals(&want) && val.equals(v2))
                        }),
                        _ => false,
                    };
                    return Ok(Value::Flag(if op == Prim::Ne { !equal } else { equal }));
                }
                let is_values = matches!(a, Value::Window(_, 'v')) || matches!(b, Value::Window(_, 'v'));
                let equal = if is_values {
                    match (a, b) { (Value::Window(x, xp), Value::Window(y, yp)) => Rc::ptr_eq(x, y) && xp == yp, _ => false }
                } else {
                    let set_like = |value: &Value| matches!(value, Value::Window(..)) || matches!(value, Value::Set(_));
                    if set_like(a) && set_like(b) {
                        let mut sets = Vec::new();
                        for value in [a, b] {
                            sets.push(match value {
                                Value::Set(_) => value.clone(),
                                _ => Value::Set(Rc::new(RefCell::new(self.gather_set(Some(&value.settled()))?))),
                            });
                        }
                        matches!(self.prim(Prim::Eq, name, &sets)?, Value::Flag(true))
                    } else { false }
                };
                return Ok(Value::Flag(if op == Prim::Ne { !equal } else { equal }));
            }
        }
        // `type` and `isinstance` ask after a view itself, keys or
        // values or pairs, not after the row its members would stand
        // as, so a view settles no further for either.
        // `isdisjoint`, which a view of a map's keys or its pairs
        // answers to as a set does, needs the view whole to tell that
        // apart from a view of its values, which answers to no set
        // working at all.
        let view_kept = matches!(op, Prim::SortOf | Prim::Belongs | Prim::SetCall(14));
        if v.iter().any(|value| matches!(value, Value::Mutable(..)) || matches!(value, Value::Window(..)) && !view_kept)
            && !matches!(op, Prim::Say | Prim::Out | Prim::Listed | Prim::MakeArray | Prim::MakeMap | Prim::Couple | Prim::ExtendLiteral(..) | Prim::Added | Prim::Placed | Prim::ValueMethod)
            && !(self.writes_a_row_over(op) || matches!(op, Prim::Pointed) && self.works_sequences()) {
            let settled: Vec<Value> = v.iter().map(|value| {
                if view_kept && matches!(value, Value::Window(..)) { value.clone() } else { value.settled() }
            }).collect();
            return self.prim(op, name, &settled);
        }
        if let Some(result) = self.user_operation(op, v)? { return Ok(result); }
        // Two things each standing for a kind, joined by `|`, make the
        // tuple of them: the very shape `isinstance` and `issubclass`
        // already read a union of kinds by, so no third shape is
        // needed to hold one. Either side may itself already be such
        // a tuple, so a union chains with a further kind, with `Nil`,
        // and with another union; but at least one side must itself
        // be a kind or an already-built union; `Nil` on both sides is
        // no union.
        if let (Prim::BitsEither, [a, b]) = (op, v) {
            if self.union_member(a) && self.union_member(b) && (self.union_anchor(a) || self.union_anchor(b)) {
                return Ok(Value::Tuple(Rc::new(vec![a.clone(), b.clone()])));
            }
        }
        // Rows, tuples and text as a language of sequences works them.
        // Anything the sequences have no say in falls through to the
        // readings below, as it would were there no such language.
        // A row of bytes joined to what is no row of bytes keeps words
        // of its own, which are asked for ahead of the sequence workings
        // so that a row standing on the left is named by them. A row
        // laid down again asks the same question there, so that the
        // count, and not the row, is the side those words name.
        if matches!(op, Prim::Plus | Prim::Times) && v.iter().any(|item| matches!(item, Value::Octets { .. })) {
            if let Some(words) = self.kinds_refused(op, v) { return Err(words); }
        }
        if self.works_sequences() {
            if let Some(answer) = self.sequence_working(op, v)? { return Ok(answer); }
        }
        if let (Prim::Eq | Prim::Ne, [one, two]) = (op, v) {
            if let Some(same) = self.alike_with_things(one, two)? { return Ok(Value::Flag(same == (op == Prim::Eq))); }
        }
        if let [Value::Wrapped(35, one), Value::Wrapped(35, two)] = v {
            if matches!(op, Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
                let (one, two) = (self.cell_contents(one), self.cell_contents(two));
                if let (Some(one), Some(two)) = (&one, &two) { return self.prim(op, name, &[one.clone(), two.clone()]); }
                let rank = one.is_some().cmp(&two.is_some());
                return Ok(Value::Flag(match op { Prim::Eq => rank.is_eq(), Prim::Ne => rank.is_ne(), Prim::Lt => rank.is_lt(), Prim::Le => rank.is_le(), Prim::Gt => rank.is_gt(), _ => rank.is_ge() }));
            }
        }
        if let ([one, two], true) = (v, matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) && self.table.strings("ext.op.order.unsupported").len() == 4) {
            let (one, two) = (one.clone(), two.clone());
            if let Some(answer) = self.weighed_member_wise(op, &one, &two)? { return Ok(answer); }
        }
        // A landing no thing answered for itself is the plain working,
        // given what it was handed: a map written into keeps its cell,
        // so that every name for it sees what was written.
        if let Prim::Landing(place) = op {
            let plain = self.table.landing_working(place);
            if let [held, by] = v {
                if let Some(kept) = self.native_written_over(plain, held, by)? { return Ok(kept); }
            }
            // Whatever the plain working refuses is refused under the
            // compound sign the program wrote.
            self.landed += 1;
            let done = self.prim_values(plain, name, v);
            self.landed -= 1;
            return done;
        }
        if Self::is_core_primitive(op) { return self.core_primitive(op, name, v.to_vec(), Vec::new()); }
        // Building octets asks for the numbers they are to keep. A
        // cursor gives those numbers up one at a time rather than all at
        // once, so draw it out into a row first; that is what lets a run
        // backwards over octets be built straight back into octets.
        if let Prim::Octets(which @ (0 | 1)) = op {
            if let [lone] = v {
                let plain = lone.settled();
                let already = matches!(plain, Value::Octets { .. } | Value::Vector(_) | Value::Text(_) | Value::Small(_) | Value::Huge(_) | Value::Flag(_));
                if !already {
                    if let Ok(numbers) = self.core_collect(&plain) { return self.octet_routine(which, &[Value::Vector(Rc::new(numbers))]); }
                }
            }
        }
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
                let quantity = self.repeat_count(times)?;
                let cell = cell.borrow();
                let mut result = Vec::new();
                let size = quantity.checked_mul(cell.len()).ok_or_else(|| self.octet_error("unready"))?;
                result.try_reserve_exact(size).map_err(|_| self.octet_error("unready"))?;
                if cell.len() > 0 { for _ in 0..quantity { result.extend_from_slice(&cell); } }
                return Ok(self.octets(result, *changeable));
            }
            // A row of bytes on the left of the remainder sign fills
            // its own marks, byte for byte: each byte of the pattern
            // stands for the character that carries it, and the filled
            // text is read back into a row of the pattern's own kind.
            if op == Prim::Mod && self.table.flag("ext.op.rem.formats_text") {
                if let Value::Octets { cell, changeable, .. } = &v[0] {
                    let (pattern, changeable) = (cell.borrow().iter().copied().map(char::from).collect::<String>(), *changeable);
                    let filled = self.octet_filled(&pattern, &v[1])?;
                    return Ok(self.octets(filled, changeable));
                }
            }
            if has_octets && matches!(op, Prim::Plus | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge | Prim::Mod | Prim::Join) {
                return Err(self.octet_error("unready"));
            }
        }
        // Where the table holds arithmetic to the kinds it means
        // something for, a working over kinds it means nothing for is
        // refused here: before a flag counts as a number, before text
        // is read for a number it spells, and before nothing counts as
        // nought.
        if let Some(words) = self.kinds_refused(op, v) { return Err(words); }
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
        if self.table.has_any("ext.op.div.zero") && !formatting && !v.iter().any(|item| matches!(item, Value::Complex(_))) && matches!(op, Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod) {
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
                let (Value::Set(x), Value::Set(y)) = (&v[0], &v[1]) else {
                    // A set sign or a set comparison handed something
                    // that is no set at all is refused as any other
                    // working of kinds it means nothing for is: its
                    // sign and both kinds named, or, for a comparison,
                    // the words CPython keeps for two kinds with no
                    // order between them.
                    return Err(if matches!(op, Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
                        self.unordered_complaint(op, &v[0], &v[1])
                    } else {
                        self.operands_refused(&self.sign_named(op), &v[0], &v[1])
                    });
                };
                let (x, y) = (x.borrow().clone(), y.borrow().clone());
                return Ok(match rule {
                    Some(rule) => Value::Set(Rc::new(RefCell::new(self.set_combine(&x, &y, rule)?))),
                    None => Value::Flag(match op {
                        Prim::Lt => x.keys.len() < y.keys.len() && self.set_beneath(&x, &y)?,
                        Prim::Le => self.set_beneath(&x, &y)?,
                        Prim::Gt => x.keys.len() > y.keys.len() && self.set_beneath(&y, &x)?,
                        _ => self.set_beneath(&y, &x)?,
                    }),
                });
            }
        }
        // The workings a number of two parts answers for. The bit
        // workings are not among them: a complex number has no bits to
        // work on, in Python as here, so `~2j` is refused in the same
        // words as `~2.0` rather than being carried to the complex
        // reckoning, which would have had to own it could not.
        if matches!(op, Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power
            | Prim::Positive | Prim::NumberAlone | Prim::Negate | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge) {
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
            // A landing was settled into its plain working above.
            Prim::Landing(_) => unreachable!(),
            Prim::OctetAssign(times) => {
                // A row written over with these signs is changed where
                // it stands, so that every name for it sees the change.
                if self.works_sequences() {
                    if let Some(kept) = self.sequence_written_over(times, &v[0], &v[1])? { return Ok(kept); }
                }
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
            Prim::ValueMethod | Prim::BindValueMethod | Prim::SortedValues => {
                let subject = v.first().cloned().unwrap_or(Value::Nil);
                return Err(self.member_missing(&subject, name));
            }
            Prim::SetAssign(operation) => {
                n(2)?;
                let ordinary = [Prim::BitsEither, Prim::BitsBoth, Prim::Minus, Prim::BitsOne][operation as usize];
                let answer = self.prim(ordinary, name, v)?;
                match (&v[0], &answer) {
                    // Nothing is written through a sealed set: the plain
                    // working stands and what it made is handed back, so
                    // that the name takes the new set, the reference
                    // finding no altering member of that name either.
                    (Value::Set(place), Value::Set(updated)) if !v[0].set_sealed() => {
                        place.replace(updated.borrow().clone());
                        v[0].clone()
                    }
                    _ => answer,
                }
            }
            Prim::SetCall(which) => self.work_set(which, v)?,
            Prim::EmptySet => Value::Set(Rc::new(RefCell::new(self.gather_set(None)?))),
            Prim::StartContext | Prim::DistinctObjects => return Err(self.bad_answer()),
            Prim::Textual(work) => {
                let values: Vec<Value> = v.iter().map(|x| match x.settled() { Value::Tuple(row)=>Value::Vector(row), other=>other }).collect();
                // A word of the text kind spelled with the method after
                // a dot, such as `str.upper`, is refused as CPython's
                // unbound method or descriptor is when it is handed no
                // receiver at all, or one that is no text. The one
                // static method of the kind, which builds a table and
                // takes no text of its own, is let through.
                if let (Some((word, entry)), false) = (name.split_once('.'), work == crate::text::Work::MAKETRANS) {
                    match values.first() {
                        None => {
                            let words = self.table.strings("ext.stmt.class.detail.descriptor.unbound");
                            return if words.len() == 3 { Err(format!("{}{word}{}{entry}{}",words[0],words[1],words[2])) } else { Err(crate::text::complaint(self.table, "receiver")) };
                        }
                        Some(Value::Text(_)) => {}
                        Some(other) => {
                            let words = self.table.strings("ext.stmt.class.detail.descriptor.foreign");
                            return if words.len() == 4 { Err(format!("{}{entry}{}{word}{}{}{}",words[0],words[1],words[2],other.kind_word(),words[3])) } else { Err(crate::text::complaint(self.table, "receiver")) };
                        }
                    }
                }
                crate::text::apply(self.table, work, name, &values, self.wording())?
            },
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
                    walk @ Value::Iterator(_) => self.apart_members(&walk.clone(), wanted, star.is_some())?,
                    Value::Set(set) => set.borrow().values(),
                    // A thing of the program's own is taken apart into
                    // the members its own walk hands over, asked for one
                    // at a time: such a walk may have no end at all, and
                    // the places call for no more than they call for.
                    thing @ Value::Thing(_) if self.appointment(thing, 15).is_some() || self.placed_walk(thing).is_some() => {
                        let walk = self.iterated_value(&thing.clone())?;
                        self.apart_members(&walk, wanted, star.is_some())?
                    }
                    Value::Tuple(items) | Value::Row(items) => items.to_vec(),
                    Value::TextRow(..) | Value::Octets { .. } | Value::Progression(_) => self.gathered_members(&v[0])?,
                    Value::Text(s) => s.chars().map(|letter| Value::text(&letter.to_string())).collect(),
                    Value::Dict(entries) => entries.iter().map(|entry| match &entry.0 { Value::Keyed(v, _) => v.as_ref().clone(), key => key.clone() }).collect(),
                    Value::Vector(v) => v.to_vec(),
                    _ => {
                        let said = self.apart_words("ext.stmt.unpack.unwalkable", &[v[0].kind_word()]);
                        return Err(said.unwrap_or_else(|| "Value cannot be taken apart".to_string()));
                    }
                };
                let minimum = if star.is_some() { wanted - 1 } else { wanted };
                if values.len() < minimum {
                    // A starred place takes home whatever is left over,
                    // so where one stands among the places the count
                    // asked for is a floor, and the table holds the
                    // words that say so before it.
                    let floor = match star {
                        Some(_) => self.table.strings("ext.stmt.unpack.short").get(3).cloned().unwrap_or_default(),
                        None => String::new(),
                    };
                    let said = self.apart_words("ext.stmt.unpack.short", &[format!("{floor}{minimum}"), values.len().to_string()]);
                    return Err(said.unwrap_or_else(|| "Wrong number of values".to_string()));
                }
                if star.is_none() && values.len() != wanted {
                    let count = match &v[0] {
                        Value::Vector(_) | Value::Tuple(_) | Value::Dict(_) => {
                            self.table.strings("ext.stmt.unpack.long").get(2)
                                .map(|between| format!("{wanted}{between}{}", values.len()))
                        }
                        _ => None,
                    }.unwrap_or_else(|| wanted.to_string());
                    let said = self.apart_words("ext.stmt.unpack.long", &[count]);
                    return Err(said.unwrap_or_else(|| "Wrong number of values".to_string()));
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
            // An iterator or a generator is not gathered: the loop steps
            // it a member at a time, so what its body said before a later
            // step raised stands said, and a loop broken off leaves the rest.
            Prim::Iterated => match self.begin_set_walk(&v[0]) {
                None if matches!(v[0], Value::Iterator(_) | Value::Generator(_)) => v[0].clone(),
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
                    let held = kept.clone();
                    for addition in additions { self.set_include(&held, addition)?; }
                    v[0].clone()
                } else if !dictionary {
                    let Value::Vector(prior) = &v[0] else { unreachable!() };
                    let mut next = prior.to_vec();
                    next.extend(if expanded { self.gathered_members(&v[1])? } else { vec![v[1].clone()] });
                    Value::Vector(Rc::new(next))
                } else {
                    // The fast pre-check above the general dispatch takes
                    // every ordinary literal or comprehension step; a
                    // call that reaches here instead is handed no more
                    // than the one map it grows, so paying to clone it
                    // once is the whole of the cost, not the shape of it.
                    let Value::Dict(prior) = &v[0] else { unreachable!() };
                    let incoming = match &v[1] {
                        Value::Couple(pair) if !expanded => vec![pair.as_ref().clone()],
                        Value::Dict(entries) if expanded => entries.to_vec(),
                        _ => return Err(self.table.single("ext.syntax.map.spread.unmapped").unwrap_or("A map spread needs a map").into()),
                    };
                    let mut combined = prior.to_vec();
                    for (key, value) in incoming {
                        if let Some(words) = self.cannot_key(&key) { return Err(words); }
                        self.map_enter(&mut combined, key, value)?;
                    }
                    Value::Dict(Rc::new(combined.into()))
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
                        else { walk.item(&BigInt::from(at)).ok_or_else(|| self.walk_fault("ext.builtin.range.index", None))? }
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
                if let Value::Generator(_) = &v[0] {
                    return Err("TypeError: cannot pickle 'generator' object".to_string());
                }
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
                    // The mark that says a complaint is told in full is
                    // the kernel's own note to itself, so it comes off
                    // before the words are handed to the program.
                    Err(Escape::Error(told)) => {
                        let told = told.trim_start_matches('\0');
                        items.extend([Value::Flag(false), Value::text(told), Value::text(told)]);
                    }
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
                Value::Dict(Rc::new(bindings.into()))
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
                if matches!(&v[0], Value::KindOf(_) | Value::Intrinsic(..) | Value::OctetKind { .. }) && self.table.spells("ext.builtin.class.name", &word) { return Ok(Value::Flag(true)); }
                // A native kind's word has a maker and a name, where a class may stand on it.
                if let Value::Intrinsic(_, kind) = &v[0] {
                    let lined = word == self.detail("mro") || word == self.detail("order");
                    if self.table.spells("ext.stmt.class.builtin", kind) && (lined || word == self.detail("allocate") || word == self.detail("name") || self.table.spells("ext.builtin.class.name", &word)) { return Ok(Value::Flag(true)); }
                }
                // A native kind the reference keeps a docstring for
                // answers to the member that reads it, whether or not
                // the kind is one a class may stand on.
                if word == self.detail("doc") && matches!(&v[0], Value::Intrinsic(_, kind) if Self::builtin_kind_doc(kind).is_some()) { return Ok(Value::Flag(true)); }
                let (class, own) = match &v[0] {
                    Value::Complex(_) => (None, self.table.spells("ext.builtin.complex.real", &word) || self.table.spells("ext.builtin.complex.imag", &word)),
                    Value::Span(_) => (None, self.span_bound_named(&word).is_some()),
                    Value::Progression(_) => (None, self.walk_member_named(&word).is_some()),
                    Value::Thing(o) => (Some(&o.of), self.member_place(&o.holds.borrow(), &word).is_some()),
                    Value::Blueprint(c) => (Some(c), false),
                    _ => (None, false),
                };
                // What a value of a native kind answers to is carried by the kind as well.
                if self.carried_by_kind(&v[0], &word).is_some() { return Ok(Value::Flag(true)); }
                let native = matches!(v[0], Value::Text(_)) && matches!(self.table.prims.get(&word), Some(Prim::Textual(work)) if *work != crate::text::Work::REPR);
                // A row of bytes answers to the methods its kind keeps.
                let of_octets = matches!(v[0].settled(), Value::Octets { changeable, .. } if self.octet_member(&word, changeable).is_some());
                // A walk over a routine's own body answers whether it is
                // running, where a language has a word for that.
                let generator_running = matches!(&v[0], Value::Generator(_)) && self.table.strings("ext.stmt.yield.running").first().map_or(false, |w| *w == word);
                Value::Flag(native || of_octets || generator_running || self.native_member(&v[0], &word) || (self.table.has_any("ext.builtin.exceptions") && matches!(&v[0], Value::Blueprint(_) | Value::Thing(_))) || matches!(v[0], Value::Member(..)) || own || (self.table.has_any("ext.stmt.class.special") && class.is_some()) || class.map_or(false, |c| c.keeper(&word).is_some() || c.program(&word).is_some() || c.constant(&word).is_some()))
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
                // The member that lays a template out is given back
                // bound to its text, like any other member of a text,
                // and lays the template out when it is called.
                if matches!(v[0], Value::Text(_)) && self.table.spells("ext.text.format", &called) {
                    return Ok(Value::Member(Rc::new(v[0].clone()), String::from("format")));
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
                            None => {
                                self.sought_in_vain = Some((called.clone(), Value::Thing(thing.clone())));
                                // A namespace names itself and the
                                // member it has not, where the table
                                // words that.
                                let told = self.member_named_missing(&Value::Thing(thing.clone()), &called);
                                if !told.is_empty() { return Err(told); }
                                return Err(format!("Undefined property: {}::${}", thing.of.name, called));
                            }
                        }
                    }
                    // A value of a builtin kind names its kind and the
                    // member it has not got, where the table words that.
                    other => {
                        self.sought_in_vain = Some((called.clone(), other.clone()));
                        let told = self.member_missing(other, &called);
                        if !told.is_empty() { return Err(told); }
                        return Err(format!("Cannot read property '{}' of {}", called, other.bare()));
                    }
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
                        self.context_hushed_by(&v[0], &called);
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
                // A tuple holds its places for good, and so does text
                // where a language of sequences says so; the place
                // written into is looked at through its cell, since a
                // name hands one over rather than what it holds.
                let standing = v[0].settled();
                if self.works_sequences() && matches!(standing, Value::Tuple(_) | Value::Text(_)) {
                    return Err(self.writing_refused(&standing));
                }
                if matches!(standing, Value::Tuple(_) | Value::Set(_)) {
                    return Err(self.core_complaint("core.immutable", &standing.kind_word()));
                }
                // A row of such a language reckons a place from the end
                // as readily as from the start, and holds no place at
                // all beyond itself: writing there is told of in the
                // words for a place written into.
                if let (true, Value::Vector(items)) = (self.works_sequences(), &v[0]) {
                    let offset = match &v[1] { Value::Flag(b) => Some(i64::from(*b)), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None };
                    // A key of some other kind names no place in a row.
                    // Writing at one is refused by the two kinds, just
                    // as reading at one is, and the row stays a row
                    // instead of becoming a map keyed by its places.
                    if offset.is_none() && !matches!(&v[1], Value::Span(_)) {
                        return Err(self.key_refused(&v[0], &v[1]));
                    }
                    if let Some(offset) = offset {
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.place_written_beyond(&v[0])); }
                        let mut all = items.as_ref().clone();
                        all[position as usize] = v[2].clone();
                        return Ok(Value::Vector(Rc::new(all)));
                    }
                }
                match &v[0] {
                    Value::Octets { cell, changeable, .. } => {
                        if !changeable { return Err(self.octet_error("immutable")); }
                        let index = self.octet_at(&v[1], cell.borrow().len(), *changeable)?;
                        let byte = self.octet_item(&v[2], false)?;
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
                        Value::Dict(Rc::new(all.into()))
                    }
                    Value::Dict(entries) => {
                        let mut all = entries.as_ref().clone();
                        set_key(&mut all, v[1].clone(), v[2].clone());
                        Value::Dict(Rc::new(all))
                    }
                    _ => return Err(self.no_places()),
                }
            }
            // A restoring write met as a plain value is a plain write:
            // the leniency belongs to the place it is written into.
            Prim::Restore => return self.prim_values(Prim::Placed, name, v),
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
            // A directory's own entries, sorted so a run answers the
            // same way twice: the bare name of each, nothing before it.
            Prim::DirEntries => {
                n(1)?;
                let w = self.wording();
                match std::fs::read_dir(v[0].render(w)) {
                    Ok(entries) => {
                        let mut names: Vec<Value> = entries.filter_map(|e| e.ok()).map(|e| Value::text(&e.file_name().to_string_lossy())).collect();
                        names.sort_by(|a, b| a.render(w).cmp(&b.render(w)));
                        Value::Vector(Rc::new(names))
                    }
                    Err(_) => Value::Flag(false),
                }
            }
            // A fresh, empty directory made under one already standing,
            // its name built from a prefix and a suffix around letters
            // this run has not used there before.
            Prim::DirFresh => {
                n(3)?;
                let w = self.wording();
                let (parent, prefix, suffix) = (v[0].render(w), v[1].render(w), v[2].render(w));
                let mut made = None;
                for _ in 0..100 {
                    let unique = unique_directory_name();
                    let candidate = std::path::Path::new(&parent).join(format!("{prefix}{unique}{suffix}"));
                    match std::fs::create_dir(&candidate) {
                        Ok(()) => { made = Some(candidate); break; }
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(_) => break,
                    }
                }
                match made {
                    Some(path) => Value::text(&path.to_string_lossy()),
                    None => Value::Flag(false),
                }
            }
            // A directory taken away along with everything under it.
            Prim::DirWhole => {
                n(1)?;
                let w = self.wording();
                Value::Flag(std::fs::remove_dir_all(v[0].render(w)).is_ok())
            }
            Prim::FaultWhole => {
                n(0)?;
                self.holding_fault.last().cloned().unwrap_or(Value::Nil)
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
                let row = vec![here, Value::text(std::env::consts::OS), Value::text(std::env::consts::ARCH), Value::Dict(Rc::new(surroundings.into()))];
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
                    told.push(Value::Dict(Rc::new(pairs.into())));
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
                #[cfg(target_os = "linux")]
                if v.len() == 2 && self.table.flag("ext.builtin.clock.parts") {
                    let clock_id = match (&v[0], &v[1]) {
                        (Value::Flag(false), Value::Flag(true)) => 0,
                        (Value::Flag(true), Value::Flag(true)) => 1,
                        _ => return Err(self.table.single("ext.builtin.module.helper.amiss").unwrap_or_default().to_string()),
                    };
                    #[repr(C)]
                    struct Tick { whole: i64, fraction: i64 }
                    extern "C" { fn clock_getres(which: i32, tick: *mut Tick) -> i32; }
                    let mut tick = Tick { whole: 0, fraction: 0 };
                    let status = unsafe { clock_getres(clock_id, &mut tick) };
                    if status != 0 { return Err(String::from("OSError: clock resolution is unavailable")); }
                    let seconds = tick.whole as f64 + (tick.fraction as f64 * 0.000000001);
                    return Ok(crate::data::worth_of_binary(seconds, self.table.count("ext.system.real.digits").unwrap_or(15)));
                }
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
                let width = |at: usize| -> Result<f64, String> {
                    let worth = self.worth_of(&v[at]);
                    // A nought under nought is a nought of its own at
                    // the width, and some of these workings answer
                    // differently for it, so the minus is put back.
                    if let Value::Frac(e) = &worth {
                        if e.under && num_traits::Zero::is_zero(&e.above) {
                            return Ok(-0.0);
                        }
                    }
                    match math::ratio_of(&worth) {
                        Some(r) => {
                            let bound = crate::data::nearest_binary(&r.above, &r.beneath);
                            // A whole number too great for any real of
                            // the width to hold cannot be carried to
                            // one here, and this is stopped rather than
                            // let the width's own past-every-number
                            // answer for a number that is not; a real
                            // already at the width answers as it is.
                            if r.places.is_none() && !r.above.is_zero() && bound.is_infinite() {
                                return Err("OverflowError: int too large to convert to float".to_string());
                            }
                            Ok(bound)
                        }
                        None => Ok(f64::NAN),
                    }
                };
                let two = match takes {
                    2 | 3 => width(2)?,
                    _ => 0.0,
                };
                if working == "fma" {
                    let one = width(1)?;
                    let three = width(3)?;
                    let got = math::fused(one, two, three)?;
                    let mut result = crate::data::worth_of_binary(got, self.real_figures());
                    if let Value::Frac(number) = &mut result { Rc::make_mut(number).float_style = self.table.flag("ext.builtin.math.floating"); }
                    return Ok(result);
                }
                let one = width(1)?;
                match math::worked(&working, one, two) {
                    Some(got) => {
                        // A working handed only reals of the width
                        // already, and answering past every number of
                        // it, has overflowed the width; a few workings
                        // stand outside numbers on their own account,
                        // and are left to answer as they answer.
                        let bounded = one.is_finite() && (takes < 2 || two.is_finite());
                        if got.is_infinite() && bounded && !matches!(working.as_str(), "fdiv" | "nextafter" | "ulp") {
                            return Err("OverflowError: math range error".to_string());
                        }
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
                    // A tuple and text hold their places for good: a
                    // language of sequences refuses to take one out of
                    // either, and names the kind it was asked of.
                    held @ (Value::Tuple(_) | Value::Text(_)) if self.works_sequences() => {
                        return Err(self.deletion_refused(held).into());
                    }
                    // A row of such a language counts a place from the
                    // end as well as from the start, refuses a key of
                    // the wrong kind by that kind, and tells of a place
                    // it does not hold in the words for one.
                    held @ Value::Vector(_) if self.works_sequences() => {
                        let Value::Vector(items) = held else { unreachable!() };
                        let Some(offset) = (match &v[1] { Value::Flag(b) => Some(i64::from(*b)), Value::Small(i) => Some(*i), Value::Huge(n) => n.to_i64(), _ => None })
                            else { return Err(self.key_refused(held, &v[1]).into()) };
                        let position = if offset >= 0 { offset } else { offset + items.len() as i64 };
                        if !(0..items.len() as i64).contains(&position) { return Err(self.place_written_beyond(held).into()); }
                        let retained = items.iter().enumerate().filter(|(j, _)| *j != position as usize).map(|(_, x)| x.clone()).collect();
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
                        Value::Dict(Rc::new(kept.into()))
                    }
                    Value::Dict(entries) => {
                        let at = self.as_key(&v[1]);
                        if let Some(words) = self.cannot_key(&at) { return Err(words); }
                        let wanted = self.hash_key(&at)?;
                        if self.table.has_any("ext.stmt.del") && !self.key_held(entries, &wanted)? {
                            return Err(if self.table.has_any("ext.builtin.exceptions") { self.absent_key(&at) } else { self.table.single("ext.stmt.del.unrun").unwrap_or_default().to_string() });
                        }
                        let kept: Vec<(Value, Value)> = self.without_key(entries, &wanted)?;
                        Value::Dict(Rc::new(kept.into()))
                    }
                    row @ Value::Octets { .. } => self.octets_shortened(row, &v[1])?,
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
            // Two sets set against each other by `==` go the road
            // membership itself takes: `equals`, reached for a plain
            // set from below, has no interpreter to call a member's
            // own `__eq__`, nor to tell two things that merely share
            // a hash apart.
            Prim::Eq | Prim::Ne if matches!((&v[0], &v[1]), (Value::Set(_), Value::Set(_))) => {
                let (Value::Set(one), Value::Set(other)) = (&v[0], &v[1]) else { unreachable!() };
                let (one, other) = (one.borrow().clone(), other.borrow().clone());
                let alike = self.sets_equal(&one, &other)?;
                Value::Flag(alike != (op == Prim::Ne))
            }
            Prim::Eq | Prim::Ne if self.table.flag("ext.op.eq.maps.unordered") => {
                let alike = match (&v[0], &v[1]) {
                    (Value::Dict(one), Value::Dict(other)) => self.dicts_equal(one, other)?,
                    _ => self.equal_contents(&v[0], &v[1]),
                };
                Value::Flag(alike != (op == Prim::Ne))
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
                    (key, Value::Dict(entries)) => {
                        let placed = match key.hash_address() {
                            Ok(address) => match entries.locate(&address) {
                                Found::Found(_) => Some(true),
                                Found::Absent => Some(false),
                                Found::Unknown => None,
                            },
                            Err(_) => None,
                        };
                        match placed {
                            Some(found) => found,
                            None => entries.iter().any(|(k, _)| contained_equal(key, k) || self.keys_match(key, k)),
                        }
                    },
                    // Sought through the one routine for the address a
                    // value takes among a set's members, so that a thing
                    // is sought by its own hash and its own equality
                    // here as it is wherever the set was grown.
                    (item, Value::Set(hay)) => {
                        let entries = hay.borrow().entries.clone();
                        let address = if let Value::Set(candidate) = self.set_search_item(item) {
                            candidate.borrow().whole_address()
                        } else { self.set_address(&entries, item)? };
                        hay.borrow().keys.contains(&address)
                    }
                    (item, Value::Octets { cell, .. }) => {
                        let numbers = cell.borrow();
                        if let Value::Octets { cell: needle, .. } = item {
                            let needle = needle.borrow();
                            (0..=numbers.len()).any(|i| numbers[i..].starts_with(&needle))
                        } else { numbers.contains(&self.octet_item(item, false)?) }
                    }
                    (needle, Value::TextRow(words, _)) => words.iter().any(|s| needle.equals(&Value::text(s))),
                    (Value::Text(part), Value::Text(text)) => text.contains(part.as_ref()),
                    // Searching text for what is not text is refused by
                    // the kind of the value sought, where the table
                    // words that refusal for itself.
                    (needle, Value::Text(_)) if self.table.has_any("ext.op.in.text") => {
                        return Err(format!("{}{}", self.table.single("ext.op.in.text").unwrap_or_default(), needle.kind_word()));
                    }
                    // An iterator gives up members until the one sought
                    // turns up, and stands after it thereafter.
                    (needle, walk @ Value::Iterator(_)) => {
                        let mut seen = false;
                        while let Some(item) = self.next_value(walk)? { if contained_equal(needle, &item) { seen = true; break; } }
                        seen
                    }
                    // Nothing else can be searched at all: it neither
                    // answers membership nor can be walked, and the
                    // table names its kind in saying so.
                    _ if self.table.strings("ext.op.in.uncontained").len() == 2 => {
                        return Err(self.membership_words("ext.op.in.uncontained", &v[1].kind_word()));
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
                    (Value::Intrinsic(_, a), Value::Intrinsic(_, b)) => a == b,
                    (Value::OctetKind { changeable: x, .. }, Value::OctetKind { changeable: y, .. }) => x == y,
                    // A bare kind is held as the kind and nothing more,
                    // so one kind read twice is the selfsame value.
                    (Value::KindOf(x), Value::KindOf(y)) => x == y,
                    (Value::Octets { cell: x, .. }, Value::Octets { cell: y, .. }) => Rc::ptr_eq(x, y),
                    (Value::Vector(a), Value::Vector(b)) => Rc::ptr_eq(a, b),
                    (Value::Set(a), Value::Set(b)) => Rc::ptr_eq(a, b),
                    (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
                    (Value::Routine(a), Value::Routine(b)) => Rc::ptr_eq(a,b),
                    (Value::Bound(a,here), Value::Bound(b,there)) => Rc::ptr_eq(a,b) && Rc::ptr_eq(here,there),
                    // Every read of a method ties it afresh: two reads are never one value.
                    (Value::Method(..), Value::Method(..)) => false,
                    (Value::Wrapped(k,a), Value::Wrapped(l,b)) => k==l && Rc::ptr_eq(a,b),
                    (Value::Backtrace(a), Value::Backtrace(b)) => Rc::ptr_eq(a, b),
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
            Prim::Join => Value::text(&format!("{}{}", self.told(&v[0], w), self.told(&v[1], w))),
            Prim::At if self.has_class_order() && matches!(&v[0],Value::Blueprint(_)) => {
                let Value::Blueprint(class)=&v[0] else{unreachable!()};
                // The class's own item entry answers with the key: a
                // class method bound to the class, a plain routine given
                // the class before the key.
                let Some(entry)=self.inherited_entry(class,self.detail("getitem")) else{return Err(self.detail("unready").to_owned())};
                let asked=if matches!(&entry,Value::Wrapped(5,_)){
                    match self.member_binding(entry,None,class.clone()){Ok(bound)=>self.apply_class_member(bound,vec![v[1].clone()]),Err(escape)=>Err(escape)}
                }else{self.apply_class_member(entry,vec![v[0].clone(),v[1].clone()])};
                asked.map_err(|fault|self.suspension_fault(fault))?
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
                // A whole number too great for any real of the width to
                // hold cannot be carried there, and the meeting is
                // stopped rather than let the width's own standing-past-
                // every-number answer for a number that is not; a real
                // which only happens to overflow the working stays
                // quiet, being at the width already before this asked.
                let (left, right) = match self.holds_reals_to_width() && (self.a_real(&v[0]) || self.a_real(&v[1])) {
                    true => {
                        let carry = |v: &Value| -> Result<Value, String> {
                            let wide = self.as_wide_real(v);
                            if !self.a_real(v) {
                                if let Value::Frac(e) = &wide {
                                    if !e.above.is_zero() && crate::data::nearest_binary(&e.above, &e.beneath).is_infinite() {
                                        return Err("OverflowError: int too large to convert to float".to_string());
                                    }
                                }
                            }
                            Ok(self.at_width(wide))
                        };
                        (carry(&v[0])?, carry(&v[1])?)
                    }
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
                // What the body raised, and what it returned, are parked
                // while words stand in for them on the way out, so a
                // clause round the walk sees the value itself.
                let stepped = match self.resume(state, Value::Nil) {
                    Ok(item) => item,
                    Err(Escape::Thrown(value)) => {
                        let words = self.suspension_fault(Escape::Thrown(value.clone()));
                        self.got_away = Some(Escape::Thrown(value));
                        return Err(words);
                    }
                    Err(other) => return Err(self.suspension_fault(other)),
                };
                match stepped {
                    Some(item) => item,
                    None => match v.get(1) {
                        Some(default) => default.clone(),
                        None => {
                            let state = state.clone();
                            if let escape @ Escape::Thrown(_) = self.exhausted_of(&state) { self.got_away = Some(escape); }
                            return Err(self.generator_words("exhausted"));
                        }
                    },
                }
            }
            Prim::Tupled => {
                if v.is_empty() { Value::Tuple(Rc::new(Vec::new())) }
                else { n(1)?; Value::Tuple(Rc::new(self.gathered_members(&v[0])?)) }
            }
            Prim::Belongs | Prim::Tupling | Prim::Uniques | Prim::Unchanging | Prim::Ordered | Prim::Backwards | Prim::Numbered | Prim::Zipped | Prim::Mapped | Prim::Filtered | Prim::EveryTrue | Prim::Least | Prim::Greatest | Prim::Magnitude | Prim::Rounded | Prim::QuotRem | Prim::Powered | Prim::Hexadecimal | Prim::Octal | Prim::Binary | Prim::Quoted | Prim::Asciied | Prim::Truthful | Prim::CallableValue | Prim::IdentityOf | Prim::Hashed | Prim::Iterator | Prim::NextItem | Prim::HasAttribute | Prim::GetMember | Prim::SetMember | Prim::DropMember | Prim::MembersOf | Prim::ReduceNative | Prim::RebuildNative => unreachable!(),
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
                // A start already text or a row of octets is refused
                // before a single member is read, in the words naming
                // the very kind it is, the way CPython refuses summing
                // any of the three rather than let `+` name what met it.
                let word_at = match v.get(1) {
                    Some(Value::Text(_)) => Some(0),
                    Some(Value::Octets { changeable: false, .. }) => Some(1),
                    Some(Value::Octets { changeable: true, .. }) => Some(2),
                    _ => None,
                };
                if let Some(at) = word_at {
                    return Err(self.table.strings("ext.builtin.sum.non_number").get(at).cloned().unwrap_or_default());
                }
                // A start that meets nothing to add to itself is
                // handed back exactly as it was given, a flag among
                // them: a flag turns to a whole number only where an
                // addition actually asks that of it, never merely for
                // standing where a sum might have needed one.
                // The total of a stepped walk follows from its three
                // numbers; the places are never laid out, so a walk of
                // a thousand million adds up as quickly as a short one.
                if let Value::Progression(walk) = v[0].settled() {
                    let how_many = walk.count();
                    let opening = v.get(1).cloned().unwrap_or(Value::Small(0));
                    if how_many == BigInt::from(0) { return Ok(opening); }
                    let added = (&walk.first + (&walk.first + (&how_many - 1) * &walk.stride)) * &how_many / 2;
                    return self.sum_added(&opening, &Value::from_big(added));
                }
                if self.table.flag("ext.stmt.yield.suspends") {
                    let Value::Generator(walk) = self.make_iterator(v[0].clone()).map_err(|fault| self.suspension_fault(fault))? else { unreachable!() };
                    let counted = |value| if let Value::Flag(flag) = value { Value::Small(flag as i64) } else { value };
                    let mut answer = v.get(1).cloned().unwrap_or(Value::Small(0));
                    loop {
                        let item = self.resume(&walk, Value::Nil).map_err(|fault| self.suspension_fault(fault))?;
                        let Some(item) = item else { return Ok(answer) };
                        answer = self.sum_added(&answer, &counted(item))?;
                    }
                }
                let number = |x| match x { Value::Flag(flag) => Value::Small(flag as i64), x => x };
                let members = self.gathered_members(&v[0])?;
                let mut total = v.get(1).cloned().unwrap_or(Value::Small(0));
                for item in members { total = self.sum_added(&total, &number(item))?; }
                total
            }
            Prim::Span if self.table.flag("ext.builtin.range.value") => {
                let wrong = || self.argument_fault("ext.syntax.call.amiss", None);
                if !(1..=3).contains(&v.len()) { return Err(wrong()); }
                let integer = |item: &Value| match item {
                    Value::Huge(big) => Ok((**big).clone()),
                    Value::Small(small) => Ok(BigInt::from(*small)),
                    Value::Flag(flag) => Ok(BigInt::from(if *flag { 1 } else { 0 })),
                    _ => Err(self.walk_fault("ext.builtin.range.integer", Some(&item.kind_word()))),
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
                if v.is_empty() { Value::text("") } else { return self.text_decoded(&v); }
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
                if let [value @ Value::Unpaired(_)] = v { return Ok(value.clone()); }
                n(1)?;
                self.figures_allowed(&v[0])?;
                Value::text(&self.told(&v[0], w))
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
                    x => {
                        let exact = math::to_decimal(x, math::DEFAULT_PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?;
                        if let Value::Frac(e) = &exact {
                            if !e.above.is_zero() && crate::data::nearest_binary(&e.above, &e.beneath).is_infinite() {
                                return Err("OverflowError: int too large to convert to float".to_string());
                            }
                        }
                        self.at_width(exact)
                    }
                }
            }
            Prim::Length => {
                n(1)?;
                if let Some(held) = self.check_set_walk(&v[0])? { return Ok(Value::Small(held.borrow().entries.len() as i64)); }
                match &v[0] {
                    Value::Octets { cell, .. } => Value::Small(cell.borrow().len() as i64),
                    Value::Unpaired(numbers) => Value::Small(numbers.len() as i64),
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Vector(l) | Value::Tuple(l) | Value::Arguments(l) => Value::Small(l.len() as i64),
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
                    Value::Unpaired(numbers) => Value::Small(numbers[0] as i64),
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
                    None if code >= 0xd800 && code <= 0xdfff && self.table.has_any("ext.system.bytes.encodings") => Value::characters(vec![code]),
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
                    // Anything standing for a kind is itself of the kind
                    // primitive's kind, whichever kind it stands for.
                    if self.stands_for_a_kind(&v[0]) { return Ok(self.kind_builder_word()); }
                    if self.namespace_holding(&v[0]).is_some() { return Ok(self.kind_named_after(&v[0])); }
                    if let Value::Thing(t) = &v[0] { return Ok(Value::Blueprint(t.of.clone())); }
                    let wanted = match &v[0] {
                        Value::Complex(_) => Some(Prim::ComplexMade),
                        Value::Text(_) | Value::Unpaired(_) => Some(Prim::AsText), Value::Flag(_) => Some(Prim::Truthful),
                        Value::Vector(_) => Some(Prim::Listed), Value::Dict(_) => Some(Prim::Dictionary),
                        // What a routine keeps under its names, seen as
                        // a view of the entries, is of the dictionary kind.
                        Value::Attributes(_) => Some(Prim::Dictionary),
                        Value::Set(_) => Some(if v[0].set_sealed() { Prim::Unchanging } else { Prim::Uniques }), Value::Tuple(_) => Some(Prim::Tupling),
                        Value::Small(_) | Value::Huge(_) => Some(Prim::AsInt), Value::Frac(_) => Some(Prim::AsReal),
                        // A progression and a span of bounds are kinds
                        // the table spells, so each answers with the
                        // intrinsic word that builds one.
                        Value::Progression(_) => Some(Prim::Span), Value::Span(_) => Some(Prim::SpanOf), _ => None,
                    };
                    if let Some(operation) = wanted {
                        if let Some((_,word)) = self.table.prim_words.iter().find(|(p,_)| *p == operation) { return Ok(Value::Intrinsic(operation, Rc::from(word.as_str()))); }
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
                // Every kind left over is one the table spells no
                // intrinsic word for. A table that asks after kinds at
                // all is answered with the blueprint named for it: the
                // bare kind worth carries no name of its own, and a
                // class stands where the reference has a class.
                if self.table.has_any("ext.builtin.isinstance") { return Ok(self.kind_named_after(&v[0])); }
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

    /// The kind to name where a value cannot be hashed. A tuple is
    /// hashed by its items, so what is named is the first item that
    /// cannot be, and not the tuple holding it.
    fn unhashable_kind(value: &Value) -> String {
        match value {
            Value::Shared(cell) | Value::Mutable(cell, _) => Self::unhashable_kind(&cell.borrow()),
            Value::Tuple(items) | Value::Row(items) => match items.iter().find(|item| item.hash_number().is_none()) {
                Some(item) => Self::unhashable_kind(item),
                None => value.kind_word(),
            },
            other => other.kind_word(),
        }
    }

    /// Whether this table asks for the sequence workings at all.
    fn works_sequences(&self) -> bool { self.table.flag("ext.op.sequence.values") }

    /// Whether a working writes a row over where the row stands, as
    /// `+=` and `*=` do in a language of sequences. A landing counts by
    /// the plain working it falls back to: a row is no thing and
    /// answers for none of the in-place methods, so what reaches the
    /// row is that plain working, and it must reach it with the row's
    /// cell still about it.
    fn writes_a_row_over(&self, op: Prim) -> bool {
        let plain = match op {
            Prim::Landing(place) => self.table.landing_working(place),
            other => other,
        };
        self.works_sequences() && matches!(plain, Prim::OctetAssign(_))
    }

    /// One piece of a sequence label, or nothing where the table is
    /// silent about it.
    fn sequence_piece(&self, label: &str, at: usize) -> &str {
        self.table.strings(&format!("ext.op.sequence.{label}")).get(at).map_or("", String::as_str)
    }

    /// The refusal of a joining that cannot be made. The kind on the
    /// left is named twice over: once for what was asked of it, once
    /// for what the other side would have to be.
    fn joining_refused(&self, left: &Value, right: &Value) -> String {
        format!("{}{}{}{}{}{}", self.sequence_piece("concat", 0), left.kind_word(),
            self.sequence_piece("concat", 1), right.kind_word(),
            self.sequence_piece("concat", 2), left.kind_word())
    }

    /// How a working's sign is named in a refusal. Under a compound
    /// write it is the compound spelling the program used, and not the
    /// plain working that spelling falls back to.
    fn sign_named(&self, op: Prim) -> String {
        if self.landed > 0 {
            let alike = |one: &Prim| std::mem::discriminant(one) == std::mem::discriminant(&op);
            let mut spellings: Vec<&str> = self.table.compound.iter().filter(|(_, p)| alike(p)).map(|(sign, _)| sign.as_str()).collect();
            spellings.sort_by(|one, other| one.len().cmp(&other.len()).then_with(|| one.cmp(other)));
            if let Some(sign) = spellings.first() { return (*sign).to_string(); }
        }
        self.written_as(&op)
    }

    /// The refusal of a working whose operands are of kinds it means
    /// nothing for, where the table holds arithmetic to its kinds;
    /// nothing at all for a working the kernel answers for. Text takes
    /// part only where it joins with text, stands laid down a whole
    /// number of times, or is given values to write into it; nothing
    /// takes no part whatever. Neither is read here for a number it
    /// might stand for. Repeating and writing into text keep the
    /// refusals of their own, which name what they were handed. A row
    /// of bytes joins to another row of bytes and to nothing else, and
    /// a map takes no part in arithmetic whatever.
    fn kinds_refused(&self, op: Prim, v: &[Value]) -> Option<String> {
        if !self.table.flag("ext.op.arithmetic.strict") { return None; }
        if !matches!(op, Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power) { return None; }
        let [left, right] = v else { return None };
        let worded = |item: &Value| matches!(item, Value::Text(_));
        let of_bytes = |item: &Value| matches!(item, Value::Octets { .. });
        // Bytes hold refusals of their own for all but a joining and a
        // laying down again, and two rows of bytes join as they will.
        if of_bytes(left) || of_bytes(right) {
            // A row of bytes laid down again asks for a whole count, and
            // is refused in the words every sequence holds for a count
            // that is none. Those words name the side that is no
            // sequence, or, where both are, the one on the right.
            if op == Prim::Times {
                let sequenced = |item: &Value| matches!(item, Value::Text(_) | Value::Octets { .. } | Value::Vector(_) | Value::Tuple(_));
                let by = if sequenced(left) { right } else { left };
                if matches!(by, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { return None; }
                return Some(self.repeating_refused(by));
            }
            if op != Prim::Plus || of_bytes(left) && of_bytes(right) { return None; }
            // A row of bytes on the left names in its own words what was
            // handed to it; text or a row on the left names both kinds
            // in the words for a sequence joined to what is none.
            if of_bytes(left) { return Some(self.octet_joining_refused(left, right)); }
            if matches!(left, Value::Text(_) | Value::Vector(_) | Value::Tuple(_)) { return Some(self.joining_refused(left, right)); }
            return Some(self.operands_refused(&self.sign_named(op), left, right));
        }
        let amiss = if worded(left) || worded(right) {
            let text_alone = op == Prim::Times || (op == Prim::Mod && worded(left));
            !text_alone && !(op == Prim::Plus && worded(left) && worded(right))
        } else {
            matches!(left, Value::Nil) || matches!(right, Value::Nil) || matches!(left, Value::Dict(_)) || matches!(right, Value::Dict(_))
        };
        if !amiss { return None; }
        Some(match (op, left) {
            // Only a sequence is joined to, and those words name its
            // kind on either side of what it was handed.
            (Prim::Plus, Value::Text(_) | Value::Vector(_) | Value::Tuple(_)) => self.joining_refused(left, right),
            _ => self.operands_refused(&self.sign_named(op), left, right),
        })
    }

    /// The refusal of a repetition asked for by something that is no
    /// whole number, naming what was handed over instead.
    fn repeating_refused(&self, by: &Value) -> String {
        format!("{}{}{}", self.sequence_piece("repeat", 0), by.kind_word(), self.sequence_piece("repeat", 1))
    }

    /// The words for a count of repetitions too wide to stand for a
    /// place in a row.
    fn repeating_too_wide(&self) -> String { self.sequence_piece("repeat", 2).to_owned() }

    /// The words for a place a sequence does not hold. Text is spoken
    /// of by its longer name here, as the words themselves have it.
    fn place_beyond(&self, of: &Value) -> String {
        let kind = match of { Value::Text(_) => "string".to_owned(), other => other.kind_word() };
        format!("{}{}{}", self.sequence_piece("index", 0), kind, self.sequence_piece("index", 1))
    }

    /// The words for a place a sequence does not hold when that place
    /// is written into or taken out of. A table may word such a place
    /// apart from one only read; wording none, the words for a reading
    /// serve for every way of reaching it.
    fn place_written_beyond(&self, of: &Value) -> String {
        let said = self.table.single("ext.system.fault.index.assign");
        match (said, self.table.single("ext.system.fault.class.index")) {
            (Some(words), Some(named)) => format!("{named}: {words}"),
            _ => self.place_beyond(of),
        }
    }

    /// Whether two values are one and the same holding place, so that
    /// writing either where the other stands changes nothing.
    fn one_cell(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Shared(x) | Value::Mutable(x, _), Value::Shared(y) | Value::Mutable(y, _)) => Rc::ptr_eq(x, y),
            _ => false,
        }
    }

    /// The refusal of a write into a sequence that cannot be changed.
    fn writing_refused(&self, of: &Value) -> String {
        format!("{}{}{}", self.sequence_piece("assign", 0), of.kind_word(), self.sequence_piece("assign", 1))
    }

    /// The refusal of a deletion from a sequence that cannot be changed.
    fn deletion_refused(&self, of: &Value) -> String {
        format!("{}{}{}", self.sequence_piece("delete", 0), of.kind_word(), self.sequence_piece("delete", 1))
    }

    /// The refusal of a key of the wrong kind. Text has wording of its
    /// own, taking no run of places for a key.
    fn key_refused(&self, of: &Value, key: &Value) -> String {
        match of {
            Value::Text(_) => format!("{}{}{}", self.sequence_piece("subscript", 2), key.kind_word(), self.sequence_piece("subscript", 3)),
            other => format!("{}{}{}{}", self.sequence_piece("subscript", 0), other.kind_word(),
                self.sequence_piece("subscript", 1), key.kind_word()),
        }
    }

    /// How often a sequence is to be repeated. A count too wide to name
    /// a place is refused whichever way it leans; one under nought
    /// leaves the sequence with nothing in it.
    fn repeat_count(&self, by: &Value) -> Result<usize, String> {
        let often = match by {
            Value::Small(n) => BigInt::from(*n),
            Value::Huge(n) => n.as_ref().clone(),
            Value::Flag(t) => BigInt::from(u8::from(*t)),
            other => return Err(self.repeating_refused(other)),
        };
        if often.to_isize().is_none() { return Err(self.repeating_too_wide()); }
        Ok(often.to_usize().unwrap_or(0))
    }

    /// A row laid down again and again, with room asked for first.
    fn laid_again(&self, items: &[Value], often: usize) -> Result<Vec<Value>, String> {
        let wanted = items.len().checked_mul(often).ok_or_else(|| self.repeating_too_wide())?;
        let mut row: Vec<Value> = Vec::new();
        row.try_reserve(wanted).map_err(|_| self.repeating_too_wide())?;
        for _ in 0..often { row.extend(items.iter().cloned()); }
        Ok(row)
    }

    /// The workings a language of sequences gives rows, tuples and text:
    /// joining, repeating, and the order two of a kind stand in, which
    /// is settled place by place. Nothing at all where the operation
    /// belongs to none of them.
    fn sequence_working(&mut self, op: Prim, v: &[Value]) -> Result<Option<Value>, String> {
        let [left, right] = v else { return Ok(None) };
        let strung = |x: &Value| matches!(x, Value::Vector(_) | Value::Tuple(_));
        let inside = |x: &Value| match x { Value::Vector(row) | Value::Tuple(row) => row.as_ref().clone(), _ => Vec::new() };
        match op {
            Prim::Plus if strung(left) || strung(right) => {
                let mut row = inside(left);
                match (left, right) {
                    (Value::Vector(_), Value::Vector(_)) => { row.extend(inside(right)); Ok(Some(Value::Vector(Rc::new(row)))) }
                    (Value::Tuple(_), Value::Tuple(_)) => { row.extend(inside(right)); Ok(Some(Value::Tuple(Rc::new(row)))) }
                    // Only a sequence is joined to; where the left
                    // side is none, both kinds are named instead, as a
                    // working neither side answers for.
                    _ if matches!(left, Value::Vector(_) | Value::Tuple(_) | Value::Text(_)) => Err(self.joining_refused(left, right)),
                    _ => Err(self.operands_refused(&self.sign_named(op), left, right)),
                }
            }
            Prim::Times if strung(left) || strung(right) => {
                let (row, by) = if strung(left) { (left, right) } else { (right, left) };
                let times = self.repeat_count(by)?;
                if times == 1 && matches!(row, Value::Tuple(_)) { return Ok(Some(row.clone())); }
                let laid = self.laid_again(&inside(row), times)?;
                Ok(Some(match row { Value::Tuple(_) => Value::Tuple(Rc::new(laid)), _ => Value::Vector(Rc::new(laid)) }))
            }
            // Text laid down again is counted out below; only a count
            // that is no whole number is answered here, so that it is
            // refused in these words rather than the reader's.
            Prim::Times if matches!(left, Value::Text(_)) || matches!(right, Value::Text(_)) => {
                let by = if matches!(left, Value::Text(_)) { right } else { left };
                self.repeat_count(by).map(|_| None)
            }
            Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge if strung(left) && strung(right) && left.kind_word() == right.kind_word() => {
                let (first, second) = (inside(left), inside(right));
                let mut place = 0;
                let rank = loop {
                    match (first.get(place), second.get(place)) {
                        (None, None) => break std::cmp::Ordering::Equal,
                        (None, Some(_)) => break std::cmp::Ordering::Less,
                        (Some(_), None) => break std::cmp::Ordering::Greater,
                        (Some(x), Some(y)) => {
                            let alike = self.prim(Prim::Eq, "", &[x.clone(), y.clone()])?;
                            if self.stands_true(&alike) { place += 1; continue; }
                            let under = self.prim(Prim::Lt, "", &[x.clone(), y.clone()])?;
                            break match self.stands_true(&under) { true => std::cmp::Ordering::Less, false => std::cmp::Ordering::Greater };
                        }
                    }
                };
                Ok(Some(Value::Flag(match op {
                    Prim::Lt => rank.is_lt(), Prim::Le => rank.is_le(), Prim::Gt => rank.is_gt(), _ => rank.is_ge(),
                })))
            }
            _ => Ok(None),
        }
    }

    /// A row written over with `+=` or `*=`. The row keeps its cell and
    /// is changed where it stands; adding takes in whatever can be
    /// walked, and not only another row. Nothing at all where the place
    /// written into holds no row, so the plain working answers instead.
    fn sequence_written_over(&mut self, repeat: bool, target: &Value, given: &Value) -> Result<Option<Value>, String> {
        let (Value::Shared(cell) | Value::Mutable(cell, _)) = target else { return Ok(None) };
        let cell = cell.clone();
        let Value::Vector(items) = cell.borrow().clone() else { return Ok(None) };
        let row = if repeat {
            let original = given.settled();
            let quantity = match self.stood_for_whole(&original)? {
                Some(index) => index,
                None => original,
            };
            self.laid_again(&items, self.repeat_count(&quantity)?)?
        } else {
            let mut row = items.as_ref().clone();
            let kind = given.kind_word();
            row.extend(self.gathered_members(given).map_err(|_| self.core_complaint("core.uniterable", &kind))?);
            row
        };
        cell.replace(Value::Vector(Rc::new(row)));
        Ok(Some(target.clone()))
    }

    /// A thing over a native worth, written over by a compound sign
    /// that its blueprint answered nothing for. The worth is what the
    /// native kind's own writing reaches, so where it is a worth that
    /// can be written into -- a row, a map or a set, each standing in a
    /// cell of its own -- the plain working is handed that worth and the
    /// thing is handed back, and the name written to keeps a thing of
    /// its blueprint while every other name for it sees the change.
    /// Nothing at all for a worth no writing changes: text, a number or
    /// a tuple is left to the plain working, which answers with what the
    /// worth answers and so with a value of the kind beneath.
    fn native_written_over(&mut self, plain: Prim, place: &Value, given: &Value) -> Result<Option<Value>, String> {
        let Some(worth) = Self::underlying(&place.settled()) else { return Ok(None) };
        let celled = |wanted: fn(&Value) -> bool| matches!(&worth, Value::Mutable(cell, _) if wanted(&cell.borrow()));
        let rows = celled(|held| matches!(held, Value::Vector(_)));
        let pairs = celled(|held| matches!(held, Value::Dict(_)));
        let uniques = matches!(worth, Value::Set(_)) && !worth.set_sealed();
        let writes = match plain {
            Prim::OctetAssign(_) => rows && self.works_sequences(),
            Prim::SetAssign(0) => uniques || pairs && self.table.flag("ext.op.bit.or.maps"),
            Prim::SetAssign(_) => uniques,
            _ => false,
        };
        if !writes { return Ok(None); }
        // A compound set sign handed something that is no set is
        // refused as the plain sign would be, but named for the
        // thing's own blueprint and with the compound sign it was
        // written with; the sign is read while the working still
        // stands compound, so `sign_named` answers `|=` and not `|`.
        if uniques {
            if let Prim::SetAssign(operation) = plain {
                let ordinary = [Prim::BitsEither, Prim::BitsBoth, Prim::Minus, Prim::BitsOne][operation as usize];
                self.landed += 1;
                let outcome = self.prim_values(plain, "", &[worth.clone(), given.clone()]);
                let sign = self.sign_named(ordinary);
                self.landed -= 1;
                if let Err(told) = &outcome {
                    if *told == self.operands_refused(&sign, &worth, given) {
                        return Err(self.operands_refused(&sign, &place.settled(), given));
                    }
                }
                outcome?;
                return Ok(Some(place.clone()));
            }
        }
        self.prim_values(plain, "", &[worth, given.clone()])?;
        Ok(Some(place.clone()))
    }

    fn element(&self, target: &Value, at: &Value, how: Reading) -> Result<Value, String> {
        if let Value::Unpaired(numbers) = target {
            return match at {
                Value::Span(bounds) => {
                    let (_, positions, _) = self.span_selection(bounds, numbers.len())?;
                    Ok(Value::characters(positions.iter().map(|&p| numbers[p]).collect()))
                }
                _ => {
                    let raw = at.as_big()?.to_i64().unwrap_or(i64::MAX);
                    let place = if raw < 0 { raw + numbers.len() as i64 } else { raw };
                    let n = numbers.get(place as usize).ok_or_else(|| self.method_fault("index"))?;
                    Ok(Value::characters(vec![*n]))
                }
            };
        }
        if matches!(target, Value::Mutable(..) | Value::Window(..)) { return self.element(&target.settled(), at, how); }
        if let Some(store) = self.check_set_walk(target)? {
            let position = as_index(at)?;
            // A thing kept beside its hash comes back as the thing.
            return store.borrow().entries.get(position)
                .map(|(_, item)| match item { Value::Keyed(thing, _) => thing.as_ref().clone(), held => held.clone() })
                .ok_or_else(|| self.set_complaint("missing", &position.to_string()));
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
            if self.works_sequences() && !matches!(at, Value::Span(_) | Value::Small(_) | Value::Huge(_) | Value::Flag(_)) {
                return Err(self.key_refused(target, at));
            }
            let selected=if let Value::Span(bounds)=at {
                let (_,places,_)=self.span_selection(bounds,values.len())?;
                Value::Tuple(Rc::new(places.into_iter().map(|i|values[i].clone()).collect()))
            }else{
                let index=at.as_big()?;let index=if index<BigInt::from(0){index+values.len()}else{index};
                let beyond = || match self.works_sequences() {
                    true => format!("\0{}", self.place_beyond(target)),
                    false => self.detail("unready").to_owned(),
                };
                index.to_usize().and_then(|i|values.get(i)).cloned().ok_or_else(beyond)?
            };
            return Ok(selected);
        }
        if let Value::Progression(walk) = target {
            match at {
                Value::Small(_) | Value::Huge(_) | Value::Flag(_) => return walk.item(&at.as_big()?).ok_or_else(|| self.walk_fault("ext.builtin.range.index", None)),
                Value::Span(_) => return Err(self.span_complaint("unsupported")),
                _ => return Err(self.walk_fault("ext.builtin.range.integer", Some(&at.kind_word()))),
            }
        }
        // A place holding a cell that names share reads as whatever the
        // cell holds: the sharing lies between the names, not in the
        // value itself.
        return self.element_within(target, at, how).map(|found| match found {
            Value::Shared(cell) if !self.names_in_calls => cell.borrow().clone(),
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
        // A place shared with another name -- a module's own global
        // among them -- is written into through the cell it shares,
        // exactly as `handed` is unwrapped above: the place taken for
        // this write may itself already be shared before this call
        // ever sees it, where the ordinary single-key write below
        // unwraps such a place itself.
        if let Value::Shared(cell) = held {
            let cell = cell.clone();
            return self.span_written(&mut cell.borrow_mut(), bounds, handed);
        }
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
                index => Ok(Value::Small(i64::from(numbers[self.octet_at(index, numbers.len(), *changeable)?]))),
            };
        }
        if self.table.flag("ext.op.index.text.negative") && !matches!(at,Value::Span(_)) {
            if let Value::Text(word) = target {
                if self.works_sequences() && !matches!(at, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) {
                    return Err(self.key_refused(target, at));
                }
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
        // A row of a language of sequences counts its places from the
        // end as well as from the start, and names its own kind both
        // where the key is wrong and where the place is not there.
        if self.works_sequences() && !matches!(at, Value::Span(_)) {
            if let Value::Vector(values) = target {
                let Some(number) = (match at {
                    Value::Small(n) => Some(i128::from(*n)),
                    Value::Flag(t) => Some(i128::from(*t)),
                    Value::Huge(n) => Some(n.to_i128().unwrap_or(i128::MAX)),
                    _ => None,
                }) else { return Err(self.key_refused(target, at)) };
                let place = if number < 0 { number + values.len() as i128 } else { number };
                return usize::try_from(place).ok().and_then(|i| values.get(i)).cloned()
                    .ok_or_else(|| format!("\0{}", self.place_beyond(target)));
            }
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
        if suffix == "unhashable" {
            if let [lead, middle, end] = parts { return format!("{lead}{insert}{middle}{insert}{end}"); }
        }
        let mut words = parts.first().cloned().unwrap_or_default();
        words.push_str(insert);
        if let Some(end) = parts.get(1) { words.push_str(end); }
        words
    }

    /// The address a value takes among a set's entries. A native value
    /// is addressed by its worth alone. A thing is addressed by the
    /// hash its class gives it, and where a thing already among the
    /// entries hashes alike the two are asked whether they agree: things
    /// that agree share the one address, and things that do not stand
    /// apart under the same hash, each by its own identity rather than
    /// by how many others happen to stand there at the time: a thing
    /// put back after another beside it was taken out keeps the very
    /// address it always had, so two sets holding the same things agree
    /// on their addresses however either was gathered or thinned. The
    /// entries are copied out before any of this, since asking a thing
    /// for its hash or its equality runs the program's own code, which
    /// may reach the set itself.
    fn set_address(&mut self, entries: &[(String, Value)], item: &Value) -> Result<String, String> {
        if !matches!(item, Value::Thing(_) | Value::Keyed(..)) { return self.hash_for_set(item); }
        let keyed = self.hash_key(item)?;
        let hash = match &keyed { Value::Keyed(_, hash) => hash.bare(), _ => String::new() };
        let opening = format!("instance:{hash}:");
        for (address, held) in entries {
            if !address.starts_with(&opening) { continue; }
            if self.keys_agree(held, &keyed)? { return Ok(address.clone()); }
        }
        let identity = match &keyed { Value::Keyed(thing, _) => match thing.as_ref() { Value::Thing(t) => Rc::as_ptr(t) as usize, _ => 0 }, _ => 0 };
        Ok(format!("{opening}{identity:x}"))
    }

    /// A value placed in a set at the address it takes there, a thing
    /// kept beside its hash as a map's keys are kept.
    fn set_include(&mut self, store: &Rc<RefCell<crate::data::SetStore>>, item: Value) -> Result<(), String> {
        // An item addressed by its worth alone has no use for the
        // entries: its address is what it is whoever else lies there.
        // Only a thing, whose hash and equality the program answers
        // for, is placed against the entries, so they are copied out
        // for that alone and a store is gathered in a single pass
        // rather than in one pass for every entry it gains.
        if !matches!(item, Value::Thing(_) | Value::Keyed(..)) {
            let address = self.hash_for_set(&item)?;
            store.borrow_mut().put(address, item);
            return Ok(());
        }
        let entries = store.borrow().entries.clone();
        let address = self.set_address(&entries, &item)?;
        let kept = if matches!(item, Value::Thing(_)) { self.hash_key(&item)? } else { item };
        store.borrow_mut().put(address, kept);
        Ok(())
    }

    fn set_search_item(&self, item: &Value) -> Value {
        let settled = item.settled();
        match Self::underlying(&settled) {
            Some(inner) if self.appointment(&settled, 8).map_or(true, |v| matches!(v, Value::Nil)) => {
                let contents = inner.settled();
                if let Value::Set(_) = contents { return contents; }
            }
            _ => {}
        }
        settled
    }

    fn hash_for_set(&self, item: &Value) -> Result<String, String> {
        match item.hash_address() {
            Ok(address) => Ok(address),
            Err("") => Err(self.set_complaint("unsupported", "")),
            Err(kind) => Err(self.set_complaint("unhashable", kind)),
        }
    }

    /// Whether a value is found among a set store's entries, sought the
    /// very way membership itself seeks it: by worth alone for a native
    /// value, and by a thing's own hash and its own equality for a
    /// thing, so that two stores built apart still agree on what each
    /// other holds however their own entries came to be addressed.
    fn set_member_found(&mut self, item: &Value, other: &crate::data::SetStore) -> Result<bool, String> {
        if matches!(item, Value::Thing(_) | Value::Keyed(..)) {
            let keyed = self.hash_key(item)?;
            for held in other.values() {
                if self.keys_agree(&held, &keyed)? { return Ok(true); }
            }
            return Ok(false);
        }
        Ok(other.keys.contains(&self.hash_for_set(item)?))
    }

    /// Whether every entry of the one store is found among the other's,
    /// asked of the entries themselves and not of the addresses they
    /// happen to be kept at: two stores of the very same things agree
    /// here however either was gathered, thinned, or built back up.
    fn set_beneath(&mut self, one: &crate::data::SetStore, other: &crate::data::SetStore) -> Result<bool, String> {
        for value in one.values() {
            if !self.set_member_found(&value, other)? { return Ok(false); }
        }
        Ok(true)
    }

    /// Whether no entry of the one store is found among the other's.
    fn set_disjoint(&mut self, one: &crate::data::SetStore, other: &crate::data::SetStore) -> Result<bool, String> {
        for value in one.values() {
            if self.set_member_found(&value, other)? { return Ok(false); }
        }
        Ok(true)
    }

    /// Two stores are equal where each holds the very count of entries
    /// the other does and every one of the one's is found among the
    /// other's, each by its own hash and its own equality where it is
    /// a thing: the plain working's own `==`, reached for a set store
    /// from below, has no way to call a thing's own `__eq__`.
    fn sets_equal(&mut self, one: &crate::data::SetStore, other: &crate::data::SetStore) -> Result<bool, String> {
        Ok(one.keys.len() == other.keys.len() && self.set_beneath(one, other)?)
    }

    /// A store combined from two others by union, intersection,
    /// difference or symmetric difference, each entry of either side
    /// sought among the other by its own equality and not by the
    /// address it happens to be kept at, so that things which agree by
    /// a custom `__eq__` are told apart from things that merely share
    /// a hash.
    fn set_combine(&mut self, one: &crate::data::SetStore, other: &crate::data::SetStore, rule: u8) -> Result<crate::data::SetStore, String> {
        let mut answer = crate::data::SetStore::new(&one.spelling, one.sealed);
        for (key, item) in &one.entries {
            let bare = match item { Value::Keyed(thing, _) => thing.as_ref().clone(), held => held.clone() };
            let shared = self.set_member_found(&bare, other)?;
            let keep = match rule { 0 => true, 1 => shared, _ => !shared };
            if keep { answer.put(key.clone(), item.clone()); }
        }
        if matches!(rule, 0 | 3) {
            for (key, item) in &other.entries {
                let bare = match item { Value::Keyed(thing, _) => thing.as_ref().clone(), held => held.clone() };
                if !self.set_member_found(&bare, one)? { answer.put(key.clone(), item.clone()); }
            }
        }
        Ok(answer)
    }

    fn gather_set(&mut self, source: Option<&Value>) -> Result<crate::data::SetStore, String> {
        let gathered = Rc::new(RefCell::new(crate::data::SetStore::new(self.table.single("ext.builtin.set").unwrap_or(""), false)));
        if let Some(source) = source {
            for item in self.gathered_members(source)? { self.set_include(&gathered, item)?; }
        }
        let store = gathered.borrow().clone();
        Ok(store)
    }

    fn work_set(&mut self, which: u8, values: &[Value]) -> Result<Value, String> {
        // A view of a map's keys or its pairs answers `isdisjoint` as a
        // set does, being turned into one first; a view of its values
        // is no set and falls to the plain complaint below.
        let turned;
        let values = if which == 14 && matches!(values.first(), Some(Value::Window(_, portion)) if *portion != 'v' && *portion != 'm') {
            turned = std::iter::once(Value::Set(Rc::new(RefCell::new(self.gather_set(values.first())?)))).chain(values[1..].iter().cloned()).collect::<Vec<_>>();
            turned.as_slice()
        } else { values };
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
            let held = target.clone();
            for input in values.iter().skip(1) {
                let additions = self.gathered_members(input)?;
                for value in additions { self.set_include(&held, value)?; }
            }
            return Ok(Value::Nil);
        }
        match which {
            1..=3 => {
                if which == 1 { let held = target.clone(); self.set_include(&held, values[1].clone())?; return Ok(Value::Nil); }
                let entries = target.borrow().entries.clone();
                let address = if let Value::Set(candidate) = self.set_search_item(&values[1]) {
                    candidate.borrow().whole_address()
                } else { self.set_address(&entries, &values[1])? };
                {
                    let taken = target.borrow_mut().take(&address);
                    if taken.is_none() && which == 2 {
                        // A member a set has not is told of as a map
                        // tells of a key it has not, so a number is
                        // named as the number it is and text with its
                        // quotes about it.
                        let absent = self.absent_key(&values[1]);
                        if absent.starts_with('\0') { return Err(absent); }
                        let member = values[1].set_member_spelling(self.wording());
                        return Err(self.set_complaint("missing", &member));
                    }
                }
                Ok(Value::Nil)
            }
            // Copying a sealed set answers with that set: a second
            // could hold nothing the first does not.
            6 if values[0].set_sealed() => Ok(values[0].clone()),
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
                        12 => Some(self.set_beneath(&answer, &operand)?),
                        13 => Some(self.set_beneath(&operand, &answer)?),
                        14 => Some(self.set_disjoint(&answer, &operand)?),
                        _ => None,
                    };
                    if let Some(truth) = truth { return Ok(Value::Flag(truth)); }
                    let operation = match which { 9 | 15 => 1, 10 | 16 => 2, 11 | 17 => 3, _ => 0 };
                    answer = self.set_combine(&answer, &operand, operation)?;
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
        if let Value::Unpaired(numbers) = source { return Ok(numbers.iter().map(|&n| Value::characters(vec![n])).collect()); }
        if let Some(under) = self.underlying_unless(source, &[15]) { return self.gathered_members(&under); }
        // A thing of the program's own that says how it is walked, by a
        // walk method or by reading its places, has the members that
        // walk hands over, the same ones a loop over it would see.
        if matches!(source, Value::Thing(_)) && (self.appointment(source, 15).is_some() || self.placed_walk(source).is_some()) {
            return self.object_members(source);
        }
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

    /// A value as the plain printer and the one-name conversion word
    /// it. A table which asks that a collection read as its
    /// representation does is answered here as well as on the printer
    /// that spells its own keyword arguments, so that the label says
    /// one thing whichever printer a language has; everything besides a
    /// collection is worded as it renders.
    fn told(&self, item: &Value, w: Names) -> String {
        let gathered = matches!(item, Value::Vector(_) | Value::Dict(_) | Value::Row(_) | Value::Tuple(_) | Value::Mutable(..) | Value::Shared(_));
        match gathered && self.collections_read_alike() {
            true => item.repr(&w),
            false => item.render(w),
        }
    }

    fn show(&self, v: &[Value]) -> String {
        let w = self.wording();
        let argument = |x: &Value| {
            let text = self.told(x, w);
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
    // Grown as one store-backed map throughout, so a literal of many
    // keys writes each one into the pairs it already has rather than
    // scanning them all again for every key that joins.
    let mut entries: Rc<MapStore> = Rc::new(Vec::with_capacity(values.len()).into());
    for value in values {
        match value {
            Value::Couple(e) => {
                let key = if plain_keys { key_as_taken(e.0.clone()) } else { e.0.clone() };
                set_key_indexed(&mut entries, key, e.1.clone())
            }
            other => {
                let key = Value::Small(after_keys(entries.as_ref()));
                Rc::make_mut(&mut entries).push((key, other));
            }
        }
    }
    Value::Dict(entries)
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

/// Write a key into a store-backed map, in place: the store's own
/// text-keyed address answers first, for a key of plain, scalar
/// worth its own address names, and only a key its own address
/// cannot place — a list, a map, a changeable set, a thing — falls
/// to the plain walk `set_key` always took, exactly as it always
/// took it.
fn set_key_indexed(entries: &mut Rc<MapStore>, key: Value, value: Value) {
    if let Ok(address) = key.hash_address() {
        match entries.locate(&address) {
            Found::Found(at) => return overwrite_or_shared(entries, at, value),
            Found::Absent => return store_insert_new(entries, key, value),
            Found::Unknown => {}
        }
    }
    match entries.as_ref().iter().position(|(k, _): &(Value, Value)| k.equals(&key)) {
        Some(at) => overwrite_or_shared(entries, at, value),
        None => { Rc::make_mut(entries).push((key, value)); }
    }
}

/// A pair written over, or a place keeping a cell that names share
/// written through instead, not written over.
fn overwrite_or_shared(entries: &mut Rc<MapStore>, at: usize, value: Value) {
    if let Value::Shared(cell) = &entries[at].1 {
        let cell = cell.clone();
        *cell.borrow_mut() = value;
    } else {
        Rc::make_mut(entries).overwrite_at(at, value);
    }
}

/// A key already known to be absent, added to a store-backed map's
/// pairs and, where its own address names it, to the store's place
/// alongside them, so the next key finds it there without a walk.
fn store_insert_new(entries: &mut Rc<MapStore>, key: Value, value: Value) {
    match key.hash_address() {
        Ok(address) => Rc::make_mut(entries).insert_known_absent(key, address, value),
        Err(_) => { Rc::make_mut(entries).push((key, value)); }
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
        false => Value::Dict(Rc::new(all.into())),
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
        // A name a program's own code took out of `sys.modules` is read
        // in again rather than handed the standing instance: that is
        // where CPython keeps such a cache, and a program that empties
        // a name out of it means the next import to run the module
        // afresh, as a name written under a name used before this run
        // into a directory on `sys.path` needs to.
        if let Some(value) = self.imported.get(path) {
            if self.import_cache_names(path) { return Ok(value.clone()); }
        }
        if path.starts_with('.') {
            return Err(self.table.single("ext.stmt.import.relative.unready").unwrap_or_default().to_string());
        }
        // A directory the program itself put on `sys.path` is looked in,
        // in the order it stands there, ahead of the library.
        let from_disk = self.sys_path_source(path);
        let text = match &from_disk {
            Some((_, text)) => text.clone(),
            None => self.library_sources.get(path).cloned().ok_or_else(|| {
                let (before, after) = self.table.around("ext.stmt.import.missing").unwrap_or(("", ""));
                format!("{before}{path}{after}")
            })?,
        };
        let own_file = match &from_disk {
            Some((file, _)) => Some(file.clone()),
            None => self.library_module_file(path),
        };
        let split = path.rsplit_once('.');
        if let Some((owner, _)) = split { self.load_namespace(owner)?; }
        let beginning = self.idents.len();
        let hidden: Vec<String> = (0..beginning).map(|n| format!("\0prior/{n}")).collect();
        let filename = own_file.as_deref().unwrap_or(path);
        let ready = match self.text_tokens(&text, 0) {
            Ok(ready) => ready,
            Err((words, line, offset)) => return Err(self.text_unreadable_at(0, words, filename, line, offset, None, &text)),
        };
        let built = match crate::build::build_module_position(&ready, self.table, &hidden, Rc::from(filename)) {
            Ok(built) => built,
            Err((words, line, (column, end_column, end_line))) => return Err(self.text_unreadable_at(0, words, filename, line, column, Some((end_line, end_column)), &text)),
        };
        let exported = &built.globals[beginning..];
        self.idents.extend(exported.iter().map(|name| format!("\0import/{path}/{name}")));
        // Every name the text can reach at its own top level gets a slot
        // in the world, a builtin read by its bare word among them, so
        // the text still finds one that way. Only a name the text itself
        // bound — by writing to it, not merely reading it — is filed as
        // one of the module's own members: CPython's module answers an
        // attribute lookup from outside out of its own `__dict__` alone,
        // never out of the builtins a name might otherwise fall back to.
        let bound: std::collections::HashSet<&str> = built.bound_globally.iter().map(|word| word.as_str()).collect();
        let module_names = self.table.strings("ext.system.module.name");
        // The word this language spells a module's own file under, if
        // any: the same word the running program's own file is bound
        // to, carried here for a module read in besides it.
        let file_word = self.table.single("ext.system.source.file");
        let mut members = Vec::with_capacity(exported.len());
        {
            let mut world = self.outermost.cells.borrow_mut();
            world.resize(self.idents.len(), Value::Unset);
            for (position, name) in exported.iter().enumerate() {
                let is_module_name = module_names.contains(name);
                let initial = if is_module_name { Value::text(path) }
                    else if file_word == Some(name.as_str()) { own_file.as_deref().map_or(Value::Nil, Value::text) }
                    else { self.fault_kinds.get(name).cloned().unwrap_or_else(|| match self.table.prims.get(name) { Some(op) => Value::Intrinsic(*op, Rc::from(name.as_str())), None => Value::Unset }) };
                let link = Value::Shared(Rc::new(RefCell::new(initial)));
                if is_module_name || bound.contains(name.as_str()) { members.push((name.clone(), link.clone())); }
                world[beginning + position] = link;
            }
        }
        for word in self.table.strings("ext.system.module.name") {
            if !members.iter().any(|(name, _)| name == word) { members.push((word.clone(), Value::text(path))); }
        }
        // `__file__` is read from outside a module (`mod.__file__`) as
        // freely as `__name__` is, in CPython, so it is carried here
        // the same unconditional way, whether or not the module's own
        // text ever names it: a module that never spells `__file__`
        // still answers one to a caller that asks by attribute.
        if let Some(word) = file_word {
            if !members.iter().any(|(name, _)| name == word) {
                members.push((word.to_string(), own_file.as_deref().map_or(Value::Nil, Value::text)));
            }
        }
        let kind = Blueprint { parents: Vec::new(), ancestry: Vec::new(), presentation: None,
            name: path.into(), under: None, methods: Vec::new(), constants: Vec::new(),
            shared: RefCell::new(Vec::new()), fields: Vec::new(), answers: Vec::new(), reaches: Vec::new(),
        };
        self.made += 1;
        let value = Value::Thing(Rc::new(Thing { of: Rc::new(kind), holds: RefCell::new(members), turn: self.made }));
        self.imported.insert(path.into(), value.clone());
        self.importing.insert(path.into());
        let scope = self.outermost.clone();
        let caller_location = (self.written_in.clone(), self.row);
        let caller_activation = self.active_trace.take();
        self.written_in = Rc::from(filename);
        self.frames_named.push(built.program.clone());
        let stopped = self.value_of(&built.program.body, &scope);
        let stopped = self.traced_result(stopped);
        self.frames_named.pop();
        self.active_trace = caller_activation;
        (self.written_in, self.row) = caller_location;
        self.importing.remove(path);
        if let Err(stopped) = stopped {
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

    /// A module written into a directory `sys.path` names, found there
    /// ahead of the library. Only a plain, undotted name is looked for
    /// this way, since a directory a program builds for itself holds no
    /// packages of its own; the file's place is made absolute before it
    /// comes back, since a name on `sys.path` may be relative to a
    /// working directory `__file__` must not depend on later changing.
    fn sys_path_source(&self, path: &str) -> Option<(String, String)> {
        if path.contains('.') { return None; }
        let Value::Thing(sys) = self.imported.get("sys")? else { return None };
        let held = sys.holds.borrow().iter().find(|(word, _)| word == "path").map(|(_, v)| v.clone())?;
        let mut value = held;
        while let Value::Shared(cell) | Value::Mutable(cell, _) = value {
            value = cell.borrow().clone();
        }
        let Value::Vector(items) = value else { return None };
        for item in items.iter() {
            let Value::Text(dir) = item else { continue };
            if dir.is_empty() { continue; }
            let file = format!("{}/{path}.py", dir.trim_end_matches('/'));
            if let Ok(text) = std::fs::read_to_string(&file) {
                return Some((made_absolute(&file), text));
            }
        }
        None
    }

    /// Where a library module's own text lives on disk, if this run
    /// carries the library there to be found: the plain file first, and
    /// a package's own file failing that. Nothing here reads the file
    /// again; the text the module runs from was read in once already,
    /// when the library was gathered into the program. The place named
    /// is always the one on the machine that built this binary --
    /// `CARGO_MANIFEST_DIR` is written in at compile time, not read
    /// from the working directory a later run happens to stand in --
    /// since that is the only machine the library's own text in
    /// `langs/` is promised to still be sitting at.
    fn library_module_file(&self, path: &str) -> Option<String> {
        const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../langs/lib_python/modules");
        let stem = path.replace('.', "/");
        let flat = format!("{ROOT}/{stem}.py");
        if std::path::Path::new(&flat).is_file() { return Some(made_absolute(&flat)); }
        let package = format!("{ROOT}/{stem}/__init__.py");
        if std::path::Path::new(&package).is_file() { return Some(made_absolute(&package)); }
        None
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
        Err(self.import_member_fault(path, wanted))
    }

    /// The words for an entry a namespace has not: the name and the
    /// namespace CPython names in "cannot import name", with the
    /// namespace's own file appended the way CPython appends it for
    /// one read from a file, so that `e.name` and the text before " ("
    /// agree with the reference's own.
    fn import_member_fault(&self, path: &str, wanted: &str) -> String {
        let told = match self.table.strings("ext.stmt.import.member.missing") {
            [head, mid, tail] => format!("{head}{wanted}{mid}{path}{tail}"),
            _ => format!("ImportError: cannot import name '{wanted}' from '{path}'"),
        };
        match self.namespace_file_path(path) {
            Some(file) => format!("{told} ({file})"),
            None => told,
        }
    }

    /// Where a namespace's own text was read from, for one the run
    /// keeps as source read out of a file of its own: the very file
    /// the library keeps it under, so the words naming it point at
    /// the file honestly.
    fn namespace_file_path(&self, path: &str) -> Option<String> {
        if !self.library_sources.contains_key(path) { return None; }
        Some(format!("langs/lib_python/modules/{}.py", path.replace('.', "/")))
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
            Value::Intrinsic(_, word) => word.as_ref() == name,
            Value::Blueprint(kind) => kind.name == name && (self.fault_kinds.contains_key(name) || name == self.detail("root")),
            Value::KindOf(_) | Value::Unset => true,
            _ => false,
        }
    }

    /// What a builtin word names: a fault kind, a native operation, or
    /// the root class.
    fn native_of(&self, name: &str) -> Option<Value> {
        if let Some(kind) = self.fault_kinds.get(name) { return Some(kind.clone()); }
        if let Some(op) = self.table.prims.get(name) { return Some(Value::Intrinsic(*op, Rc::from(name))); }
        if self.has_class_order() && name == self.detail("root") { return self.ancestor.clone().map(Value::Blueprint); }
        None
    }

    /// Read a slot's name out of the dictionary it lives in; nothing
    /// means the cell itself is to be read after all.
    fn booked_read(&mut self, at: usize, name: &str) -> Option<Result<Value, String>> {
        match self.book_holding(at)? {
            Held::World => {
                let book = self.world_book.as_ref()?;
                if let Some(worth) = looked_up(book, name) { return Some(Ok(worth)); }
                let cell = self.outermost.cells.borrow()[at].clone();
                if self.passed_over(name, &cell) { return None; }
                if let Some(spare) = self.spare_name(name) { return Some(Ok(spare)); }
                Some(Err(format!("Undefined variable: {}", name)))
            }
            Held::Reading(which) => {
                let near = self.readings[which].near.clone();
                let outer = self.readings[which].outer.clone();
                if let Some(worth) = looked_up(&near, name) { return Some(Ok(worth)); }
                if let Some(worth) = outer.as_ref().and_then(|held| looked_up(held, name)) { return Some(Ok(worth)); }
                // A builtin is reached through the dictionary of builtins
                // the outer dictionary names, so a program may hand over
                // one of its own and so choose what the text can reach.
                let roots = outer.unwrap_or(near);
                let named = self.table.single("ext.system.module.builtins").and_then(|word| looked_up(&roots, word));
                // A dictionary of builtins the program put there shuts
                // the text in; the one the kernel supplied stands for
                // the ordinary names, and those find the spare namespace
                // the way a name anywhere else does.
                let found = match named {
                    Some(Value::Shared(natives)) => {
                        let supplied = self.natives_book.as_ref().map_or(false, |own| Rc::ptr_eq(&natives, own));
                        match looked_up(&natives, name) {
                            Some(worth) => Some(worth),
                            None if supplied => self.spare_name(name),
                            None => None,
                        }
                    }
                    Some(_) => None,
                    // Naming no dictionary of its own leaves the text
                    // reaching what any other name reaches, the spare
                    // namespace with it.
                    None => match self.native_of(name) {
                        Some(worth) => Some(worth),
                        None => self.spare_name(name),
                    },
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
        let book = Rc::new(RefCell::new(Value::Dict(Rc::new(entries.into()))));
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
        let book = Rc::new(RefCell::new(Value::Dict(Rc::new(entries.into()))));
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
                drop(cells);
                // A name the routine reads from the scope around it is in
                // reach where the call stands, so it belongs here too. It
                // lives in a frame further out; a name still holding
                // nothing there is left out, as an unwritten name of the
                // routine's own is.
                for slot in &program.reaching {
                    if !Self::visible_name(&slot.ident) || entries.iter().any(|(word, _)| word.bare() == slot.ident.as_ref()) { continue; }
                    let held = match ascend(frame, slot.up).cells.borrow().get(slot.at) {
                        Some(Value::Shared(cell)) => cell.borrow().clone(),
                        Some(held) => held.clone(),
                        None => continue,
                    };
                    if !matches!(held, Value::Unset) { entries.push((Value::text(&slot.ident), held)); }
                }
            }
            Rc::new(RefCell::new(Value::Dict(Rc::new(entries.into()))))
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

    /// The complaint for text that could not be read, set about with the
    /// file the text stands for and the line the reading stopped on, as
    /// the reference words such a complaint. The reading's own words are
    /// kept where they name the very complaint the language has for text
    /// it cannot read; any other words are that complaint instead, since
    /// they are the builder's and not the language's.
    fn text_unreadable_at(&mut self, mode: usize, said: String, file: &str, row: u32, column: usize, end: Option<(u32, usize)>, source: &str) -> String {
        let clean_text = source.contains('\r').then(|| source.replace("\r\n", "\n").replace('\r', "\n"));
        let source = clean_text.as_deref().unwrap_or(source);
        let default = self.source_unreadable();
        let parts = said.split_once(": ").filter(|(name, _)| self.fault_kinds.contains_key(*name))
            .or_else(|| default.split_once(": "));
        if let Some((name, message)) = parts {
            if let Some(Value::Blueprint(kind)) = self.fault_kinds.get(name).cloned() {
                if message == "source code string cannot contain null bytes" {
                    let fault = self.make_fault(kind, vec![Value::text(message)], Value::Nil);
                    self.keep_context(&fault); self.got_away = Some(Escape::Thrown(fault));
                    return String::new();
                }
                if row == 0 && source.is_empty() && mode > 0 {
                    let locations = Value::Tuple(Rc::new(vec![Value::text(file), Value::Small(0), Value::Small(0), Value::text(""), Value::Small(0), Value::Small(0)]));
                    let fault = self.make_fault(kind, vec![Value::text(message), locations], Value::Nil);
                    self.keep_context(&fault);
                    self.got_away = Some(Escape::Thrown(fault));
                    return String::new();
                }
                if row == 0 {
                    let (line, at) = match message.strip_prefix("encoding problem:") {
                        Some(detail) if detail.ends_with(" with BOM") => (1, 0),
                        _ => (0, -1),
                    };
                    let origin = vec![Value::text(file), Value::Small(line), Value::Small(at), Value::Nil];
                    let fault = self.make_fault(kind, vec![Value::text(message), Value::Tuple(Rc::new(origin))], Value::Nil);
                    self.keep_context(&fault);
                    self.got_away = Some(Escape::Thrown(fault));
                    return String::new();
                }
                let line = row.max(1);
                let mut text = source.split_inclusive('\n').nth(line as usize - 1).unwrap_or("").to_owned();
                let mut offset = column.max(1) as i64;
                let mut end_offset = end.map_or(offset, |(_, c)| c as i64);
                let ending_line = end.map_or(line, |(r, _)| r.max(1));
                let mut words = message.to_string();
                if words == "unexpected EOF while parsing" && mode > 0 && source.ends_with('\\') {
                    words = String::from("unexpected character after line continuation character");
                    offset = (offset - 1).max(1);
                }
                let needs_closer = words.ends_with("was never closed");
                let bad_indent = matches!(name, "IndentationError" | "TabError");
                if needs_closer || words == "unexpected character after line continuation character" { end_offset = 0; }
                if words == "unexpected EOF while parsing" { end_offset = -1; }
                if end.is_none() && words == "invalid syntax" {
                    let n = text.chars().skip(offset as usize - 1).take_while(|c| c.is_alphanumeric() || *c == '_').count().max(1);
                    end_offset = offset + n as i64;
                    if mode == 0 && !text.ends_with('\n') { text.push('\n'); }
                }
                if words.contains("bytes can only contain ASCII") { end_offset = 1 + text.trim_end_matches(['\n', '\r']).chars().count() as i64; }
                let remainder: String = text.chars().skip(offset as usize - 1).collect();
                if words.starts_with("leading zeros") { end_offset = offset + remainder.chars().take_while(|c| *c == '0' || *c == '_').count() as i64; }
                if end.is_none() && words.contains("prefixes") { end_offset = offset + remainder.chars().take_while(char::is_ascii_alphabetic).count() as i64; }
                if words.starts_with("unterminated ") && !words.contains("detected at line") {
                    words = format!("{words} (detected at line {})", source.lines().count().max(line as usize));
                }
                if bad_indent {
                    end_offset = if name == "TabError" { 0 } else { -1 };
                    if words.starts_with("unindent") { offset = 1 + text.trim_end_matches(['\r', '\n']).chars().count() as i64; }
                    if words.starts_with("expected an indented block") {
                        let following: String = text.chars().skip(offset as usize - 1).collect();
                        let count = following.chars().take_while(|ch| ch.is_alphanumeric() || *ch == '_').count();
                        if count != 0 { end_offset = offset + count as i64; }
                    }
                }
                if mode == 0 && (end.is_some() || needs_closer || bad_indent || words.starts_with("unexpected EOF") || words.starts_with("unexpected character after line continuation")) && !text.ends_with('\n') { text.push('\n'); }
                if words.starts_with("unterminated ") || (needs_closer && source.lines().count() > line as usize) { text.truncate(text.trim_end_matches(['\n', '\r']).len()); }
                if words == "cannot assign to function call" && source.lines().count() > line as usize { text.truncate(text.trim_end_matches(['\n', '\r']).len()); }
                if mode > 0 && words == "invalid syntax" && !source.ends_with('\n') && column > text.chars().count() { offset = 0; end_offset = 0; }
                let omit = words.starts_with("'yield' ") || words.starts_with("comprehension inner loop ") || words.starts_with("future feature ") || words == "not a chance" || words.starts_with("from __future__ imports") || words == "import * only allowed at module level" || words == "nonlocal declaration not allowed at module level" || words == "default 'except:' must be last" || words.starts_with("name ") || words.starts_with("duplicate argument ") || ["'return' outside function", "'break' outside loop", "'continue' not properly in loop"].contains(&words.as_str());
                if omit {
                    let first: String = text.chars().take(offset.saturating_sub(1) as usize).collect();
                    offset = first.len() as i64 + 1;
                    if end_offset > 0 {
                        let final_line = source.split_inclusive('\n').nth(ending_line as usize - 1).unwrap_or("");
                        let upto: String = final_line.chars().take((end_offset - 1) as usize).collect();
                        end_offset = upto.len() as i64 + 1;
                    }
                }
                let source_line = match (omit, if omit { std::fs::read_to_string(file).ok() } else { None }) {
                    (false, _) => Value::text(&text),
                    (true, Some(contents)) => contents.replace("\r\n", "\n").replace('\r', "\n").split_inclusive('\n').nth(line as usize - 1).map(Value::text).unwrap_or(Value::Nil),
                    _ => Value::Nil,
                };
                let details = Value::Tuple(Rc::new(vec![Value::text(file), Value::Small(line as i64), Value::Small(offset), source_line, Value::Small(ending_line as i64), Value::Small(end_offset)]));
                let value = self.make_fault(kind, vec![Value::text(&words), details], Value::Nil);
                self.keep_context(&value);
                self.got_away = Some(Escape::Thrown(value));
                return String::new();
            }
        }
        said
    }

    /// The tokens of text handed over to be read, else the reading's
    /// own words for why it could not be, with the line it stopped on:
    /// the same complaint a file being run keeps, so text read through
    /// `compile`, `eval` or `exec` is told apart the same way a file
    /// is, through `text_unreadable_at`.
    fn text_tokens(&self, source: &str, mode: usize) -> Result<Vec<crate::scan::Token>, (String, u32, usize)> {
        if mode != 0 && source.is_empty() { return Err(("SyntaxError: invalid syntax".into(), 0, 0)); }
        if mode == 1 {
            let head = source.trim_start_matches([' ', '\t', '\n']).split(|ch: char| !ch.is_alphanumeric() && ch != '_').next().unwrap_or("");
            if ["return", "raise", "break", "continue", "pass", "del", "import", "from", "global", "nonlocal", "assert", "class", "def", "for", "while", "if", "try", "with"].contains(&head) {
                let skipped = source.len() - source.trim_start_matches([' ', '\t', '\n']).len();
                let line = source[..skipped].bytes().filter(|c| *c == b'\n').count() + 1;
                let column = source[..skipped].rsplit('\n').next().unwrap_or("").chars().count() + 1;
                return Err(("SyntaxError: invalid syntax".into(), line as _, column));
            }
        }

        crate::scan::scan_position(source, self.table)
            .and_then(|read| crate::indent::indent_position(read, self.table, 0))
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
    fn decode_program(&mut self, raw: &[u8], file: &str) -> Result<Rc<str>, String> {
        let has_bom = raw.starts_with(b"\xef\xbb\xbf");
        let raw = if has_bom { &raw[3..] } else { raw };
        let written = raw.split(|c| *c == 10).take(2).find_map(|line| {
            let ascii = String::from_utf8_lossy(line);
            if !ascii.trim_start().starts_with('#') { return None; }
            let (_, suffix) = ascii.split_once("coding")?;
            let suffix = suffix.strip_prefix('=').or_else(|| suffix.strip_prefix(':'))?;
            Some(suffix.trim_start().chars().take_while(|c| c.is_ascii_alphanumeric() || "-_.".contains(*c)).collect::<String>())
        }).unwrap_or_else(|| "utf-8".into());
        let cookie: String = written.chars().filter(|c| !"-_".contains(*c)).flat_map(char::to_lowercase).collect();
        let normalized = written.to_lowercase().replace('_', "-");
        if has_bom && !(normalized == "utf-8" || normalized.starts_with("utf-8-")) {
            let why = format!("SyntaxError: encoding problem: {written} with BOM");
            return Err(self.text_unreadable_at(0, why, file, 0, 0, None, ""));
        }
        if cookie == "ascii" || cookie == "usascii" {
            match raw.iter().position(|b| *b >= 128) {
                Some(at) => {
                    let why = format!("SyntaxError: 'ascii' codec can't decode byte 0x{:02x} in position {at}: ordinal not in range(128)", raw[at]);
                    return Err(self.text_unreadable_at(0, why, file, 0, 0, None, ""));
                }
                None => return Ok(Rc::from(raw.iter().map(|b| char::from(*b)).collect::<String>())),
            }
        }
        if ["latin", "latin1", "iso88591"].contains(&cookie.as_str()) { return Ok(Rc::from(raw.iter().map(|b| *b as char).collect::<String>())); }
        let alphabet = match cookie.as_str() {
            "cp1251" => Some("ЂЃ‚ѓ„…†‡€‰Љ‹ЊЌЋЏђ‘’“”•–—�™љ›њќћџ ЎўЈ¤Ґ¦§Ё©Є«¬­®Ї°±Ііґµ¶·ё№є»јЅѕїАБВГДЕЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдежзийклмнопрстуфхцчшщъыьэюя"),
            "iso88597" => Some(" ‘’£€₯¦§¨©ͺ«¬­�―°±²³΄΅Ά·ΈΉΊ»Ό½ΎΏΐΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡ�ΣΤΥΦΧΨΩΪΫάέήίΰαβγδεζηθικλμνξοπρςστυφχψωϊϋόύώ�"),
            _ => None,
        };
        if let Some(alphabet) = alphabet {
            let symbols = alphabet.chars().collect::<Vec<_>>();
            let mut text = String::new();
            for &byte in raw { text.push(if byte >= 128 { symbols[(byte - 128) as usize] } else { byte as char }); }
            return Ok(Rc::from(text));
        }
        if cookie != "utf8" && cookie != "utf8sig" {
            return Err(self.text_unreadable_at(0, format!("SyntaxError: unknown encoding: {written}"), file, 0, 0, None, ""));
        }
        match std::str::from_utf8(raw) {
            Ok(text) => Ok(Rc::from(text)),
            Err(bad) => {
                let upto = &raw[..bad.valid_up_to()];
                let line = 1 + upto.iter().filter(|b| **b == 10).count() as u32;
                let prefix = String::from_utf8_lossy(upto);
                let offset = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
                let position = prefix.rsplit(['"', '\'', '\n']).next().unwrap_or("").len();
                let byte = raw[bad.valid_up_to()];
                let reason = match bad.error_len() { None => "unexpected end of data", Some(_) if byte >= 194 => "invalid continuation byte", _ => "invalid start byte" };
                let words = format!("SyntaxError: (unicode error) 'utf-8' codec can't decode byte 0x{byte:02x} in position {position}: {reason}");
                Err(self.text_unreadable_at(0, words, file, line, offset, Some((line, offset + 1)), &String::from_utf8_lossy(raw)))
            }
        }
    }

    fn text_prepared(&mut self, name: &str, v: &[Value]) -> Result<Value, String> {
        let formals = self.table.strings("ext.builtin.compile.parameters");
        if v.len() < 3 || v.len() > formals.len() { return Err(self.core_complaint("core.arity", name)); }
        let (Value::Text(file), Value::Text(manner)) = (v[1].settled(), v[2].settled()) else { return Err(self.source_refused()) };
        let source = match v[0].settled() {
            Value::Text(s) => s,
            Value::Octets { cell, .. } => self.decode_program(&cell.borrow(), &file)?,
            _ => return Err(self.source_refused()),
        };
        let Some(mode) = self.table.strings("ext.builtin.compile.modes").iter().position(|word| word == manner.as_ref()) else { return Err(self.source_refused()) };
        // Flags and inheritance are read and let be; optimisation beyond
        // the ordinary setting is not honoured.
        if v.get(5).map_or(false, |worth| !matches!(worth, Value::Nil | Value::Small(0) | Value::Small(-1))) { return Err(self.source_refused()); }
        let tokens = match self.text_tokens(&source, mode) {
            Ok(tokens) => tokens,
            Err((said, row, col)) => return Err(self.text_unreadable_at(mode, said, &file, row, col, None, &source)),
        };
        self.text_built(&source, &tokens, &[], &file, mode, &[])?;
        let kind = self.code_blueprint();
        let holds = vec![(formals[0].clone(), Value::Text(source)), (formals[1].clone(), Value::Text(file)), (formals[2].clone(), Value::Small(mode as i64))];
        self.made += 1;
        Ok(Value::Thing(Rc::new(Thing { of: kind, holds: RefCell::new(holds), turn: self.made })))
    }

    /// Text or a code value run: as one expression where it is to be
    /// weighed, else as statements; in the dictionaries handed over,
    /// else where the call stands.
    fn text_performed(&mut self, weighing: bool, v: &[Value]) -> Result<Value, String> {
        // The names set aside are only for a reading handed no
        // dictionaries; taken here, no reading that follows finds them.
        let within = self.text_within.take();
        let Some(first) = v.first().map(Value::settled) else { return Err(self.source_refused()) };
        let (source, file, mode) = match first {
            Value::Text(text) => (text.to_string(), None, usize::from(weighing)),
            Value::Octets { cell, .. } => (self.decode_program(&cell.borrow(), "<string>")?.to_string(), None, usize::from(weighing)),
            Value::Thing(code) if self.table.single("ext.builtin.compile.kind") == Some(code.of.name.as_str()) => {
                let holds = code.holds.borrow();
                let mode = match holds.get(2).map(|(_, worth)| worth) { Some(Value::Small(n)) => *n as usize, _ => 0 };
                (holds[0].1.bare(), Some(holds[1].1.bare()), mode)
            }
            _ => return Err(self.source_refused()),
        };
        if v.len() > 3 { return Err(self.source_refused()); }
        // An expression to be weighed may stand in from the edge of its text.
        let source = if mode == 1 { source.trim_start_matches([' ', '\t']).to_owned() } else { source };
        let mut books = Vec::new();
        for place in 1..3 {
            books.push(match v.get(place) {
                None | Some(Value::Nil) => None,
                Some(Value::Shared(cell)) if matches!(&*cell.borrow(), Value::Dict(_)) => Some(cell.clone()),
                Some(_) => return Err(self.source_refused()),
            });
        }
        let (outer, near) = match (books.remove(0), books.remove(0)) {
            (None, None) => return match within.filter(|_| mode != 2) {
                Some((names, mine)) => self.perform_within(&source, file, mode, names, mine),
                None => self.perform_here(&source, file, mode),
            },
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
        let file = file.unwrap_or_else(|| "<string>".to_owned());
        let tokens = match self.text_tokens(source, mode) {
            Ok(tokens) => tokens,
            Err((said, row, col)) => return Err(self.text_unreadable_at(mode, said, &file, row, col, None, &source)),
        };
        let seeded = self.idents.clone();
        let (built, shown) = self.text_built(&source, &tokens, &seeded, &file, mode, &[])?;
        self.idents = built.globals.clone();
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        self.text_concluded(&built, &file, mode, shown)
    }

    /// Text read where a routine stands and handed no dictionaries: it
    /// is read as a piece of that routine, so the routine's names are
    /// its own to read, as the reference has it. What it writes it
    /// writes to a frame of its own, standing apart from the routine's,
    /// so the routine goes on holding what it held and a name the text
    /// makes is gone once the text is done.
    fn perform_within(&mut self, source: &str, file: Option<String>, mode: usize, names: Vec<String>, mine: Rc<Env>) -> Result<Value, String> {
        let source = if mode == 1 { source.trim_start_matches([' ', '\t']) } else { source };
        let file = file.unwrap_or_else(|| "<string>".to_owned());
        let tokens = match self.text_tokens(source, mode) {
            Ok(tokens) => tokens,
            Err((said, row, col)) => return Err(self.text_unreadable_at(mode, said, &file, row, col, None, &source)),
        };
        let knows = (&self.knows_cells.0, &self.knows_cells.1, &self.knows_cells.2);
        // Text read inside a method is read as standing in that
        // method's class, as text read where a language has no manners
        // of reading already is.
        let within = self.standing_in().map(str::to_string).map(|named| {
            let under = match self.class_bound(&named) {
                Some(Value::Blueprint(c)) => c.under.as_ref().map(|b| b.name.clone()),
                _ => None,
            };
            (named, under)
        });
        let built = match crate::build::build_within_at(&tokens, self.table, &self.idents, &names, knows, 0, within, mode == 1, Some(Rc::from(file.as_str()))) {
            Ok(built) => built,
            Err((said, row, col)) => return Err(self.text_unreadable_at(mode, said, &file, row, col.0, Some((col.2, col.1)), &source)),
        };
        self.idents = built.globals.clone();
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        mine.cells.borrow_mut().resize(built.program.idents.len().max(names.len()), Value::Unset);
        let (was_in, was_on) = (self.written_in.clone(), self.row);
        self.written_in = Rc::from(file.as_str());
        self.frames_named.push(built.program.clone());
        let outer_trace = self.active_trace.take();
        let ran = self.value_of(&built.program.body, &mine);
        self.active_trace = outer_trace;
        self.frames_named.pop();
        self.written_in = was_in;
        self.row = was_on;
        let answer = match ran {
            Ok(answer) => answer,
            Err(Escape::Error(told)) => return Err(told),
            Err(Escape::Yield(answer)) => answer,
            Err(other) => { self.got_away = Some(other); return Err("the source read in did not finish".to_owned()); }
        };
        Ok(if mode == 1 { answer } else { Value::Nil })
    }

    /// Text run in dictionaries of its own: its names are given slots
    /// among the outermost cells, and a book kept for them.
    fn perform_booked(&mut self, source: &str, file: Option<String>, mode: usize, outer: Rc<RefCell<Value>>, near: Option<Rc<RefCell<Value>>>) -> Result<Value, String> {
        let file = file.unwrap_or_else(|| "<string>".to_owned());
        let tokens = match self.text_tokens(source, mode) {
            Ok(tokens) => tokens,
            Err((said, row, col)) => return Err(self.text_unreadable_at(mode, said, &file, row, col, None, &source)),
        };
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
        let (built, shown) = self.text_built(&source, &tokens, &prior, &file, mode, &shadowed)?;
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
    fn text_built(&mut self, source: &str, tokens: &[crate::scan::Token], seeded: &[String], file: &str, mode: usize, shadowed: &[String]) -> Result<(crate::build::Built, bool), String> {
        let written_in: Option<Rc<str>> = Some(Rc::from(file));
        if mode == 2 {
            if let Ok(built) = crate::build::build_text(tokens, self.table, seeded, 0, written_in.clone(), true, shadowed) { return Ok((built, true)); }
        }
        crate::build::build_text(tokens, self.table, seeded, 0, written_in, mode == 1, shadowed)
            .map(|built| (built, false)).map_err(|(said, row, col)| self.text_unreadable_at(mode, said, file, row, col.0, Some((col.2, col.1)), source))
    }

    /// Run a built text as standing in its file, and answer what it
    /// left: the value weighed, else nothing.
    fn text_concluded(&mut self, built: &crate::build::Built, file: &str, mode: usize, shown: bool) -> Result<Value, String> {
        let (was_in, was_on) = (self.written_in.clone(), self.row);
        self.written_in = Rc::from(file);
        let top = self.outermost.clone();
        let prior = self.active_trace.take();
        self.frames_named.push(built.program.clone());
        let ran = self.value_of(&built.program.body, &top);
        self.frames_named.pop();
        self.active_trace = prior;
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
        matches!(op, Belongs | Tupling | Uniques | Unchanging | Dictionary | Ordered | Backwards | Numbered | Zipped | Mapped | Filtered | EveryTrue | Least | Greatest | Magnitude | Rounded | QuotRem | Powered | Hexadecimal | Octal | Binary | Quoted | Asciied | Truthful | CallableValue | IdentityOf | Hashed | Iterator | NextItem | HasAttribute | GetMember | SetMember | DropMember | MembersOf | ReduceNative | RebuildNative)
    }

    pub(super) fn core_complaint(&self, key: &str, middle: &str) -> String {
        let label = format!("ext.builtin.{}", key);
        let words = self.table.strings(&label);
        match words { [] => String::new(), [one] => one.clone(), [head, tail, ..] => format!("{head}{middle}{tail}") }
    }

    fn cursor_value(kind: IteratorKind) -> Value {
        Value::Iterator(Rc::new(RefCell::new(IteratorState { kind, peek: None, done: false, walks: None })))
    }

    fn cursor_value_walked(kind: IteratorKind, walks: Option<Rc<str>>) -> Value {
        let walk = Self::cursor_value(kind);
        if let (Value::Iterator(state), Some(word)) = (&walk, walks) { state.borrow_mut().walks = Some(word); }
        walk
    }

    /// What a value keeps no built-in writing for reduces to, for the
    /// module that writes values out to bytes: nothing for a value
    /// with no such reduction, else the pieces that opposite number
    /// reads back into a value the very same as this one -- the same
    /// kind, and, for a walk, standing at the very place this one
    /// does, so that a value already stepped some way into keeps
    /// standing there once it is written out and read back.
    fn native_reduce(&self, value: &Value) -> Value {
        if let Value::Blueprint(b) = value {
            return Value::Tuple(Rc::new(vec![Value::text("class"), Value::text(&b.name)]));
        }
        if let Value::Thing(t) = value {
            let Some(Value::Iterator(cell)) = Self::underlying(value) else { return Value::Nil };
            let IteratorKind::Count(walk, n) = &cell.borrow().kind else { return Value::Nil };
            return Value::Tuple(Rc::new(vec![Value::text("numbered"), Value::Blueprint(t.of.clone()), walk.clone(), Value::from_big(n.clone())]));
        }
        let Value::Iterator(cell) = value else { return Value::Nil };
        let held = cell.borrow();
        let walks = held.walks.clone().map_or(Value::Nil, |w| Value::text(&w));
        match &held.kind {
            IteratorKind::Stored(entries) => {
                let remaining: Vec<Value> = entries.iter().cloned().collect();
                Value::Tuple(Rc::new(vec![Value::text("items"), walks, Value::Tuple(Rc::new(remaining))]))
            }
            IteratorKind::Stepping(walk, at) => Value::Tuple(Rc::new(vec![
                Value::text("counted"), walks,
                Value::from_big(walk.first.clone()), Value::from_big(walk.limit.clone()), Value::from_big(walk.stride.clone()),
                Value::text(&walk.word), Value::from_big(at.clone()),
            ])),
            IteratorKind::PlacedBack(thing, at) => Value::Tuple(Rc::new(vec![Value::text("back"), walks, thing.clone(), Value::from_big(at.clone())])),
            IteratorKind::Count(walk, n) => Value::Tuple(Rc::new(vec![Value::text("numbered"), Value::Nil, walk.clone(), Value::from_big(n.clone())])),
            _ => Value::Nil,
        }
    }

    /// The value a reduction written out by `native_reduce` reads back
    /// into, standing exactly where the value written out stood.
    fn native_rebuild(&mut self, value: &Value) -> Result<Value, String> {
        let malformed = || "TypeError: a written value cannot be read back".to_string();
        let Value::Tuple(parts) = value else { return Err(malformed()) };
        let Some(Value::Text(tag)) = parts.first() else { return Err(malformed()) };
        let text_at = |i: usize| -> Option<Rc<str>> { match parts.get(i) { Some(Value::Text(t)) => Some(t.clone()), _ => None } };
        let big_at = |i: usize| -> Result<BigInt, String> { match parts.get(i) { Some(v @ (Value::Small(_) | Value::Huge(_) | Value::Flag(_))) => v.as_big(), _ => Err(malformed()) } };
        match tag.as_ref() {
            "class" => {
                let Some(name) = text_at(1) else { return Err(malformed()) };
                self.lookup(&name).ok_or_else(malformed)
            }
            "items" => {
                let walks = text_at(1);
                let Some(Value::Tuple(items)) = parts.get(2) else { return Err(malformed()) };
                Ok(Self::cursor_value_walked(IteratorKind::Stored(items.iter().cloned().collect()), walks))
            }
            "counted" => {
                let walks = text_at(1);
                let (first, limit, stride, at) = (big_at(2)?, big_at(3)?, big_at(4)?, big_at(6)?);
                let Some(word) = text_at(5) else { return Err(malformed()) };
                let walk = crate::data::Progression { first, limit, stride, word: word.to_string() };
                Ok(Self::cursor_value_walked(IteratorKind::Stepping(Rc::new(walk), at), walks))
            }
            "back" => {
                let walks = text_at(1);
                let Some(thing) = parts.get(2).cloned() else { return Err(malformed()) };
                let at = big_at(3)?;
                Ok(Self::cursor_value_walked(IteratorKind::PlacedBack(thing, at), walks))
            }
            "numbered" => {
                let Some(walk) = parts.get(2).cloned() else { return Err(malformed()) };
                let n = big_at(3)?;
                let cursor = Self::cursor_value(IteratorKind::Count(walk, n));
                match parts.get(1) {
                    Some(Value::Blueprint(b)) => {
                        self.made += 1;
                        Ok(Value::Thing(Rc::new(Thing { of: b.clone(), holds: RefCell::new(vec![("\0underlying".to_owned(), cursor)]), turn: self.made })))
                    }
                    _ => Ok(cursor),
                }
            }
            _ => Err(malformed()),
        }
    }

    /// The word the reference gives a walk of a thing, for the walks a
    /// gathered row cannot tell apart by itself. Nothing where the walk
    /// already says what it goes through.
    fn walk_named(source: &Value) -> Option<Rc<str>> {
        let word = match &source.settled() {
            Value::Vector(_) => "list_iterator",
            Value::Tuple(_) | Value::Row(_) => "tuple_iterator",
            Value::Text(text) => if text.is_ascii() { "str_ascii_iterator" } else { "str_iterator" },
            Value::Set(_) => "set_iterator",
            // What a map gives up when walked are its keys, so a walk of
            // the map itself is a walk of the keys and named as one.
            Value::Dict(_) => "dict_keyiterator",
            Value::Octets { changeable, .. } => if *changeable { "bytearray_iterator" } else { "bytes_iterator" },
            _ => return None,
        };
        Some(Rc::from(word))
    }

    fn iterated_value(&mut self, source: &Value) -> Result<Value, String> {
        match source {
            // A walk is its own walk, and so is a suspended program: a
            // walk taken of either is the very one, not a copy of what
            // it still has to give.
            Value::Iterator(_) | Value::Generator(_) => Ok(source.clone()),
            Value::Progression(walk) => Ok(Self::cursor_value(IteratorKind::Stepping(walk.clone(), BigInt::from(0)))),
            // A thing of the program's own is walked the way a loop
            // walks it: by the walk it hands over, or by its places
            // where it hands over none. That walk is kept as it stands
            // rather than gathered, so the builtins built upon it ask
            // for a member only when one is wanted.
            Value::Cursor(_) => Ok(Self::cursor_value(IteratorKind::Handed(source.clone()))),
            Value::Thing(_) => {
                if let Some(handed) = self.ask_special(source, 15, &[])? {
                    // What a thing hands over is a walk or it is
                    // nothing: one that cannot be asked for a next
                    // member is refused where the walk is asked for,
                    // not at its first step.
                    return match handed {
                        Value::Iterator(_) | Value::Generator(_) => Ok(handed),
                        Value::Cursor(_) => Ok(Self::cursor_value(IteratorKind::Handed(handed))),
                        other if self.appointed(&other, 16).is_some() => Ok(Self::cursor_value(IteratorKind::Handed(other))),
                        _ => Err(self.bad_answer()),
                    };
                }
                match self.placed_walk(source) {
                    Some(places) => Ok(places),
                    None => Err(self.core_complaint("core.uniterable", &source.kind_word())),
                }
            }
            _ => {
                let entries = self.core_collect(source)?;
                let walk = Self::cursor_value(IteratorKind::Stored(entries.into_iter().collect()));
                if let (Value::Iterator(state), Some(word)) = (&walk, Self::walk_named(source)) { state.borrow_mut().walks = Some(word); }
                Ok(walk)
            }
        }
    }

    fn next_value(&mut self, iterator: &Value) -> Result<Option<Value>, String> {
        if let Value::Generator(frame) = iterator {
            // What the body raised is parked while words stand in for it
            // on the way out, so a clause round the walk sees the value
            // itself rather than a reading of its words.
            return match self.resume(frame, Value::Nil) {
                Ok(item) => Ok(item),
                Err(Escape::Thrown(value)) => {
                    let words = self.suspension_fault(Escape::Thrown(value.clone()));
                    self.got_away = Some(Escape::Thrown(value));
                    Err(words)
                }
                Err(other) => Err(self.suspension_fault(other)),
            };
        }
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
            // A walk that reaches nothing beyond its own row, its own
            // count or its own cell -- calling back to no work of the
            // program's and asking no other iterator -- steps in the
            // one borrow already open on it, since nothing it does
            // could ever reach this same iterator again meanwhile. The
            // place such a walk stands at is a great number even where
            // the walk itself is small, and taking the whole of the
            // kind out to set the walk free, where the walk was never
            // taken up by anything that needed it free, paid for that
            // number twice at every single step for nothing gained by
            // it.
            match &mut held.kind {
                IteratorKind::Stored(entries) => {
                    let item = entries.pop_front();
                    held.done = item.is_none();
                    return Ok(item);
                }
                IteratorKind::Living(home, at) => {
                    let item = match home.borrow().settled() { Value::Vector(items) => items.get(*at).cloned(), _ => None };
                    if item.is_some() { *at += 1; }
                    held.done = item.is_none();
                    return Ok(item);
                }
                IteratorKind::Watching { window, at, size } => {
                    let current = Self::window_extent(window);
                    if current != *size {
                        let index = if current.0 == size.0 { 1 } else { 0 };
                        return Err(format!("\0{}", self.table.strings("ext.builtin.core.dict.changed")[index]));
                    }
                    let item = match window.settled() { Value::Vector(items) => items.get(*at).cloned(), _ => None };
                    if item.is_some() { *at += 1; }
                    held.done = item.is_none();
                    return Ok(item);
                }
                IteratorKind::Stepping(walk, at) => {
                    let item = walk.item(at);
                    if item.is_some() { *at += 1; }
                    held.done = item.is_none();
                    return Ok(item);
                }
                _ => {}
            }
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
                    let current = Self::window_extent(window);
                    if current != *size {
                        let index = if current.0 == size.0 { 1 } else { 0 };
                        return Err(format!("\0{}", self.table.strings("ext.builtin.core.dict.changed")[index]));
                    }
                    let item = match window.settled() { Value::Vector(items) => items.get(*at).cloned(), _ => None };
                    if item.is_some() { *at += 1; }
                    Ok(item)
                }
                IteratorKind::Placed(thing, at) => {
                    let Some((reader, scope)) = self.appointed_within(thing, 11) else { return Ok(None) };
                    match self.invoke(reader, scope, vec![thing.clone(), Value::from_big(at.clone())]) {
                        Ok(item) => { *at += 1; Ok(Some(item)) }
                        Err(Escape::Thrown(Value::Thing(thrown))) if self.ends_places(&thrown) => Ok(None),
                        Err(Escape::Error(complaint)) => if self.places_spent(&complaint) { Ok(None) } else { Err(complaint) },
                        Err(away) => { self.got_away = Some(away); Err(self.bad_answer()) }
                    }
                }
                IteratorKind::Stepping(walk, at) => {
                    let item = walk.item(at);
                    if item.is_some() { *at += 1; }
                    Ok(item)
                }
                // The place asked for counts down rather than up, and
                // the walk is done outright once it would go below
                // nought, without a further place ever being asked for.
                IteratorKind::PlacedBack(thing, at) => {
                    if *at < BigInt::from(0) { return Ok(None); }
                    let Some((reader, scope)) = self.appointed_within(thing, 11) else { return Ok(None) };
                    match self.invoke(reader, scope, vec![thing.clone(), Value::from_big(at.clone())]) {
                        Ok(item) => { *at -= 1; Ok(Some(item)) }
                        Err(Escape::Thrown(Value::Thing(thrown))) if self.ends_places(&thrown) => Ok(None),
                        Err(Escape::Error(complaint)) => if self.places_spent(&complaint) { Ok(None) } else { Err(complaint) },
                        Err(away) => { self.got_away = Some(away); Err(self.bad_answer()) }
                    }
                }
                // A thing of the program's own is asked for its next
                // member the way a loop asks it, so a walk taken from it
                // hands out one member at a time and asks for no more
                // than it is asked for.
                IteratorKind::Handed(thing) => self.advance_object(thing),
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
                                let (short, long) = if mapper.is_some() { ("map.short", "map.long") } else { ("zip.short", "zip.long") };
                                if which > 0 { return Err(self.unequal_zip(short, which)); }
                                for (later, other) in inputs.iter().enumerate().skip(1) {
                                    if self.next_value(other)?.is_some() { return Err(self.unequal_zip(long, later)); }
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

    /// The same question of a fault the kernel words for itself, which
    /// is said as words alone and throws no thing: the words are read
    /// for the class they stand under.
    fn places_spent(&self, told: &str) -> bool {
        match self.class_of_fault(told) {
            Some(class) => self.table.single("ext.system.fault.class.index") == Some(class.as_str())
                || self.table.strings("ext.stmt.class.special.stop").iter().any(|name| *name == class),
            None => false,
        }
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
    fn window_extent(window: &Value) -> (usize, u64) {
        match window { Value::Window(owner, _) => match owner.settled() { Value::Dict(entries) => (entries.len(), entries.serial), _ => (0, 0) }, _ => (0, 0) }
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
            Value::Intrinsic(op, word) => self.prim(*op, word, &values),
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

    fn core_belongs(&mut self, item: &Value, expected: &Value) -> Result<bool, String> {
        match expected {
            Value::Tuple(kinds) => {
                let kinds = kinds.clone();
                for kind in kinds.iter() { if self.core_belongs(item, kind)? { return Ok(true); } }
                Ok(false)
            }
            Value::Blueprint(class) => {
                // A class whose metaclass speaks for the kind is asked first.
                if let Some(told) = self.builder_answers(expected, item, false).map_err(|e| self.suspension_fault(e))? { return Ok(told); }
                // Every value at all is of the class every value is of.
                if class.name == self.detail("root") { return Ok(true); }
                // A blueprint standing for a native kind the table
                // spells no word of its own for is asked about by the
                // kind's own name, no word standing in its place.
                if let Some(word) = Self::native_beneath(class) {
                    if !self.table.prims.contains_key(&word) {
                        if let Value::Thing(thing) = item {
                            return Ok(std::iter::once(&thing.of).chain(thing.of.ancestry.iter()).any(|parent| Rc::ptr_eq(parent, class)));
                        }
                        return Ok(Self::native_word(class).is_some() && item.kind_word() == word);
                    }
                }
                Ok(matches!(item, Value::Thing(t) if t.of.goes_by(&class.name, false)))
            }
            Value::KindOf(Kind::Nothing) => Ok(matches!(item, Value::Nil)),
            // A union built by `|` carries a bare `Nil` for the
            // `NoneType` member, the very value `None` itself is, so
            // a chained union reads it back this way rather than
            // needing `type(None)`.
            Value::Nil => Ok(matches!(item, Value::Nil)),
            // A kind is asked after by the word naming it, whether the
            // word arrived as an intrinsic of its own or as the plain
            // reading of the name; nothing else names a kind.
            Value::Intrinsic(..) | Value::Wrapped(8, _) => {
                let word = match expected {
                    Value::Intrinsic(_, word) => word.clone(),
                    Value::Wrapped(_, parts) => match &parts[0] { Value::Text(word) => word.clone(), _ => return Err(self.core_complaint("core.isinstance.amiss", "")) },
                    _ => unreachable!(),
                };
                // A thing of a blueprint standing on the native kind is of that kind.
                if let Value::Thing(t) = item { if let Some(kind) = Self::native_beneath(&t.of) { return Ok(kind == word.as_ref()); } }
                let Some(op) = self.table.prims.get(word.as_ref()).copied().filter(Self::names_a_kind) else { return Err(self.core_complaint("core.isinstance.amiss", "")) };
                Ok(self.kind_covers(&op, &word, item))
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
        let window_owner = match (op, input.first()) { (Prim::Quoted, Some(Value::Window(owner, 'm'))) => Some(owner.settled()), _ => None };
        // A member read by name is bound to the value as it stands, so
        // that a method which writes into the value is handed the very
        // one; the value is kept before it settles into a copy.
        let standing = if op == Prim::GetMember { input.first().cloned() } else { None };
        if op != Prim::IdentityOf {
            for item in &mut input {
                // `isinstance` asks after a view itself, not after the
                // row its members would stand as, so that a claim made
                // for its very kind is honoured.
                if op == Prim::Belongs && matches!(item, Value::Window(..)) { continue; }
                *item = item.settled();
            }
        }
        if keywords.is_empty() {
            if op == Prim::Hashed && matches!(input.first(), Some(Value::Octets { .. })) { return self.octet_routine(17, &input); }
            if op == Prim::Belongs && matches!(input.get(1), Some(Value::OctetKind { .. })) { return self.octet_routine(16, &input); }
            if op == Prim::Quoted && input.len() == 1 {
                if let Value::Dict(_) = &input[0] {
                    let words = self.object_words(&input[0], true)?;
                    return Ok(Value::text(&words));
                }
            }
            if op == Prim::Quoted && matches!(input.first(), Some(Value::Text(_))) { return crate::text::apply(self.table, crate::text::Work::REPR, name, &input, self.wording()); }
            if self.has_class_order() && input.first().map_or(false, |v| matches!(v, Value::Blueprint(_) | Value::Thing(_) | Value::Routine(_) | Value::Bound(..) | Value::Method(..) | Value::Wrapped(..))) {
                let job = match op { Prim::CallableValue=>Some(2), Prim::GetMember=>Some(3), Prim::SetMember=>Some(4), Prim::DropMember=>Some(5), Prim::HasAttribute=>Some(6), Prim::MembersOf=>Some(7), _=>None };
                if let Some(job) = job { return self.work_on_class(job, input).map_err(|e| self.suspension_fault(e)); }
            }
        }
        if keywords.is_empty() { if let Some(value) = self.user_operation(op, &input)? { return Ok(value); } }
        if op == Prim::Dictionary && input.len() > 1 { return Err(self.table.single("ext.builtin.map.arguments.amiss").unwrap_or_default().to_owned()); }
        use num_traits::{Signed, Zero};
        use num_integer::Integer;
        use Prim::*;
        // `enumerate` counts every positional and keyword argument
        // together before it looks at any of them by name, exactly as
        // the reference does, so a call with too many of either kind
        // is refused the same way regardless of which are spelled right.
        if op == Prim::Numbered && input.len() + keywords.len() > 2 {
            let total = (input.len() + keywords.len()).to_string();
            return Err(self.argument_fault("ext.builtin.enumerate.too_many", Some(&total)));
        }
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
                Zipped | Mapped if is("zip.strict") => { exact = self.stands_true(&value); continue; }
                _ => (),
            }
            if op == Numbered && !is("iterable") && !is("start") {
                return Err(self.argument_fault("ext.builtin.enumerate.keyword", Some(&label)));
            }
            let slot = match (op, ()) {
                (Numbered, _) if is("iterable") => 0,
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
                if portion.is_none() && matches!(input[0], Value::Vector(_) | Value::Tuple(_)) {
                    let rendered = self.object_words(&input[0], true)?;
                    return Ok(Value::text(&rendered));
                }
                // An iterator is quoted by its kind and its identity, and
                // the quoting does not advance it; so is a walk this
                // kernel made of its own, such as a map walked
                // backwards, which the reference words the same way.
                let walk_named = matches!(&input[0], Value::Generator(state) if state.try_borrow().map_or(false, |g| g.walked.is_some()));
                if matches!(input[0], Value::Iterator(_)) || walk_named {
                    let Value::Small(mark) = self.core_primitive(Prim::IdentityOf, name, vec![input[0].clone()], Vec::new())? else { return Err(self.core_complaint("core.unready", name)) };
                    return Ok(Value::text(&format!("<{} object at 0x{:x}>", input[0].kind_word(), mark)));
                }
                // A reading of the map itself is quoted as the map is,
                // under the name CPython gives it.
                if let Some(owner) = &window_owner {
                    return Ok(Value::text(&format!("mappingproxy({})", owner.quoted(self.table.lone("system.real.render") == Some("shortest")))));
                }
                let quoted = input[0].quoted(self.table.lone("system.real.render") == Some("shortest"));
                // A window upon a dictionary is quoted under its own name.
                Ok(Value::text(&match portion {
                    Some(letter) => format!("dict_{}({})", match letter { 'k' => "keys", 'v' => "values", _ => "items" }, quoted),
                    None => quoted,
                }))
            }
            // The ascii builtin writes what the quoting builtin
            // writes and then puts every letter outside ASCII into the
            // escape that stands for it.
            Asciied => {
                require(1, 1)?;
                let word = self.table.prim_words.iter().find(|(p, _)| *p == Prim::Quoted).map(|(_, w)| w.clone()).unwrap_or_default();
                let quoted = self.core_primitive(Prim::Quoted, &word, input.clone(), Vec::new())?;
                Ok(Value::text(&crate::text::ascii_escaped(&quoted.bare())))
            }
            Truthful => { require(0, 1)?; Ok(Value::Flag(input.first().map_or(false, |v| self.stands_true(v)))) }
            CallableValue => { require(1, 1)?; Ok(Value::Flag(matches!(input[0], Value::Intrinsic(..) | Value::Bound(..) | Value::Routine(_) | Value::Blueprint(_) | Value::Member(..) | Value::Method(..)))) }
            Hashed => {
                require(1, 1)?;
                input[0].hash_number().map(Value::Small).ok_or_else(|| self.core_complaint("core.unhashable", &Self::unhashable_kind(&input[0])))
            }
            ReduceNative => { require(1, 1)?; Ok(self.native_reduce(&input[0])) }
            RebuildNative => { require(1, 1)?; self.native_rebuild(&input[0]) }
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
                    Value::Intrinsic(_, word) => {
                        let mut words: Vec<_> = self.table.prims.keys().collect(); words.sort();
                        words.iter().position(|w| w.as_str() == word.as_ref()).unwrap_or(0) as u64 + 16
                    }
                    Value::Text(chars) => chars.as_ptr() as usize as u64,
                    Value::Set(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Dict(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Thing(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Blueprint(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Iterator(p) => Rc::as_ptr(p) as usize as u64,
                    Value::Generator(p) => Rc::as_ptr(p) as usize as u64,
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
            Tupling | Uniques | Unchanging => {
                require(0, 1)?;
                if let (Tupling, [Value::Tuple(values)]) = (op, input.as_slice()) {
                    return Ok(Value::Tuple(Rc::clone(values)));
                }
                // A sealed set given to the maker of its own kind comes
                // straight back: nothing may alter it, so a second
                // would be the first under another name, and the
                // reference answers with the very one it was handed.
                if op == Unchanging {
                    if let Some(standing) = input.first().filter(|v| matches!(v, Value::Set(_)) && v.set_sealed()) {
                        return Ok(standing.clone());
                    }
                }
                let entries = match input.first() { Some(v) => self.core_collect(v)?, None => Vec::new() };
                if op == Tupling { return Ok(Value::Tuple(Rc::new(entries))); }
                // A thing among the entries goes in as the set literal puts
                // it in, under its own hash and its own equality; any other
                // entry with no hash is refused. The sealed kind is
                // gathered the very same way and differs only in kind.
                let sealed = op == Unchanging;
                let tag = if sealed { "ext.builtin.frozenset" } else { "ext.builtin.set" };
                let store = Rc::new(RefCell::new(crate::data::SetStore::new(self.table.single(tag).unwrap_or(""), sealed)));
                for entry in entries {
                    // An entry of a native kind with no hash of its own is
                    // refused by its kind, as the table has it.
                    if !matches!(entry, Value::Thing(_) | Value::Keyed(..)) && entry.hash_number().is_none() {
                        return Err(self.core_complaint("core.unhashable", &entry.kind_word()));
                    }
                    self.set_include(&store, entry)?;
                }
                return Ok(Value::Set(store));
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
                // Grown as one store-backed map throughout, so an
                // iterable of many pairs is not scanned again in full
                // for every pair it grows by.
                let mut entries: Rc<MapStore> = Rc::new(Vec::new().into());
                for (key, item) in incoming {
                    // A thing as a key carries its own hash, as it does in
                    // a dictionary literal.
                    let key = if matches!(key, Value::Thing(_)) { self.hash_key(&key)? } else { key };
                    if !matches!(key, Value::Keyed(..)) && key.hash_number().is_none() && !matches!(key, Value::Nil | Value::Frac(_)) { return Err(self.core_complaint("core.unhashable", &key.kind_word())); }
                    self.store_write(&mut entries, key, item)?;
                }
                Ok(Value::Dict(entries))
            }
            Iterator => {
                require(1, 2)?;
                if input.len() == 2 { return Ok(Self::cursor_value(IteratorKind::Summoned { work: input[0].clone(), stop: input[1].clone() })); }
                match live { Some(kind) => Ok(Self::cursor_value(kind)), None => self.iterated_value(&input[0]) }
            }
            NextItem => {
                require(1, 2)?;
                match self.next_value(&input[0])?.or_else(|| input.get(1).cloned()) {
                    Some(item) => Ok(item),
                    None => {
                        // What the body returned is carried on the
                        // exhaustion, parked behind the words for it.
                        if let Value::Generator(state) = &input[0] {
                            let state = state.clone();
                            if let escape @ Escape::Thrown(_) = self.exhausted_of(&state) { self.got_away = Some(escape); }
                        }
                        Err(self.core_complaint("core.exhausted", ""))
                    }
                }
            }
            Backwards => {
                require(1, 1)?;
                // Octets run backwards as well. A run forwards over them
                // gives up the numbers they keep rather than any letters,
                // so running the other way gives up those same numbers.
                if !matches!(input[0], Value::Vector(_) | Value::Tuple(_) | Value::Text(_) | Value::Progression(_) | Value::Dict(_) | Value::Octets { .. }) {
                    // A thing with no kind of its own that the table
                    // runs backwards directly is asked instead by its
                    // `__getitem__`, place by place from its last down
                    // to its first, where it has that and a `__len__`
                    // to learn how many places it holds -- the fallback
                    // the reference itself falls to for any such thing.
                    if matches!(&input[0], Value::Thing(_)) && self.appointed(&input[0], 11).is_some() {
                        let length = self.prim(Prim::Length, "len", &[input[0].clone()])?;
                        let at = match &length { Value::Small(_) | Value::Huge(_) | Value::Flag(_) => length.as_big(), _ => Err(self.core_complaint("core.integer", &length.kind_word())) }? - BigInt::from(1);
                        let walk = Self::cursor_value(IteratorKind::PlacedBack(input[0].clone(), at));
                        if let Value::Iterator(state) = &walk { state.borrow_mut().walks = Some(Rc::from(name)); }
                        return Ok(walk);
                    }
                    return Err(self.core_complaint("core.unreversible", &input[0].kind_word()));
                }
                // A progression runs backwards as a progression, last
                // place first, never gathered into the row it stands for.
                if let Value::Progression(walk) = &input[0] {
                    let last = &walk.first + (walk.count() - 1) * &walk.stride;
                    let backwards = crate::data::Progression { first: last, limit: &walk.first - &walk.stride, stride: -&walk.stride, word: walk.word.clone() };
                    return self.core_primitive(Prim::Iterator, name, vec![Value::Progression(Rc::new(backwards))], Vec::new());
                }
                let walked = self.core_collect(&input[0])?;
                let backwards = cursor(walked.into_iter().rev().collect());
                // Only a row has a word of its own for the walk that
                // goes through it the other way; for everything else the
                // reference says the name of the builtin itself.
                if let Value::Iterator(state) = &backwards {
                    let word = if matches!(input[0], Value::Vector(_)) { "list_reverseiterator" } else { name };
                    state.borrow_mut().walks = Some(Rc::from(word));
                }
                Ok(backwards)
            }
            Numbered => {
                if input.first().map_or(true, |v| matches!(v, Value::Unset)) {
                    return Err(self.table.single("ext.builtin.enumerate.missing").unwrap_or_default().to_owned());
                }
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
                // The smallest and the largest of a stepped walk are the
                // two ends of it, which its numbers give outright.
                if op != Ordered && input.len() == 1 && matches!(ordering, None | Some(Value::Nil)) {
                    if let Value::Progression(walk) = input[0].settled() {
                        let how_many = walk.count();
                        if how_many == BigInt::from(0) { return fallback.ok_or_else(|| self.core_complaint("core.empty", name)); }
                        let other_end = &walk.first + (&how_many - 1) * &walk.stride;
                        let (under, over) = if walk.stride > BigInt::from(0) { (walk.first.clone(), other_end) } else { (other_end, walk.first.clone()) };
                        return Ok(Value::from_big(if op == Greatest { over } else { under }));
                    }
                }
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
                if input.iter().any(|x| matches!(x, Value::Complex(_))) { return Err(crate::complex::floor(self.table, &input[0], &input[1], "divmod()")); }
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
                    if input.len() > 2 && !matches!(input[2], Value::Nil) { return Err(crate::complex::complaint(self.table,"power.modulo")); }
                    return crate::complex::reckon(self.table, Prim::Power, &input);
                }
                if input.len() < 3 || matches!(input[2], Value::Nil) {
                    return self.prim(Prim::Power, name, &input[..2]);
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
                    // The count of places is asked for its size rather than
                    // turned about, because the very least whole number a
                    // machine holds cannot be turned about and the asking
                    // would stop the run where a refusal is wanted.
                    let unit = BigInt::from(10).pow(u32::try_from(places.unsigned_abs()).ok().filter(|n| *n <= 100000).ok_or_else(|| self.core_complaint("core.unready", name))?);
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
                    // A value of a builtin kind answers the members its
                    // kind gives it. It keeps no namespace of its own,
                    // so nothing may be written into it or taken out.
                    if op == MembersOf { return Err(self.core_complaint("core.vars", "")); }
                    let word = input[1].bare();
                    if matches!(op, SetMember | DropMember) {
                        let told = self.member_unwritable(&input[0], &word);
                        if told.is_empty() { return Err(self.core_complaint("core.unready", name)); }
                        return Err(told);
                    }
                    let found = self.attribute(standing.as_ref().unwrap_or(&input[0]), &word);
                    if op == HasAttribute { return Ok(Value::Flag(found.is_some())); }
                    if let Some(member) = found {
                        return match member {
                            Value::Member(receiver, operation) => self.method_of_value(receiver.as_ref().clone().keep(false), &operation).map_err(|fault| self.suspension_fault(fault)),
                            settled => Ok(settled),
                        };
                    }
                    if input.len() == 3 { return Ok(input[2].clone()); }
                    let told = self.member_missing(&input[0], &word);
                    if told.is_empty() { return Err(self.core_complaint("core.unready", name)); }
                    return Err(told);
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
                Value::Dict(Rc::new(copied.into()))
            }
            _ => value.clone(),
        }
    }
}

impl Machine<'_> {
    /// Whether the language's own cache of modules still names this
    /// one: true wherever that cache has yet to be written at all (an
    /// import too soon for it to hold anything, or a language with no
    /// such name), so an ordinary run is never slowed by looking.
    fn import_cache_names(&self, path: &str) -> bool {
        if self.importing.contains(path) { return true; }
        let names = self.table.strings("ext.system.module.cache");
        if names.len() != 2 { return true; }
        let Some(Value::Thing(namespace)) = self.imported.get(&names[0]) else { return true };
        let Some((_, held)) = namespace.holds.borrow().iter().find(|(key, _)| key == &names[1]).cloned() else { return true };
        let cache = match held { Value::Shared(cell) => cell.borrow().clone(), other => other };
        match cache {
            Value::Dict(entries) => entries.iter().any(|(key, _)| matches!(key, Value::Text(word) if word.as_ref() == path)),
            _ => true,
        }
    }

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
/// The marks of a pattern read off a row of bytes. A mark that shows a
/// value is handed a row of bytes and nothing else, whichever of its
/// two letters it is written with, and a mark that words a value writes
/// the ascii of its representation, so that nothing beyond seven bits
/// can stand in the answer.
struct OctetMarks<'a> {
    layout: crate::formatting::Layout<'a>,
    refusal: &'a [String],
}

impl crate::formatting::Elsewhere for OctetMarks<'_> {
    fn field_laid(&mut self, _item: &Value, _pattern: &str, _convert: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn value_worded(&mut self, item: &Value, quoted: bool) -> Result<Option<String>, String> {
        let held = item.settled();
        if quoted { return self.layout.quote(&held, true).map(Some); }
        match &held {
            Value::Octets { cell, .. } => Ok(Some(cell.borrow().iter().copied().map(char::from).collect())),
            other => {
                let at = |i: usize| self.refusal.get(i).map_or("", String::as_str);
                Err(format!("{}{}{}", at(0), other.kind_word(), at(1)))
            }
        }
    }
}

/// The machine answers for the fields a layout cannot lay out: a thing
/// of the program's own is written by its own methods, and the layout
/// keeps everything else.
impl crate::formatting::Elsewhere for Machine<'_> {
    fn field_laid(&mut self, item: &Value, pattern: &str, convert: &str) -> Result<Option<String>, String> {
        let held = item.settled();
        if !self.speaks_for(&held) { return Ok(None); }
        if convert.is_empty() { return self.thing_in_spec(&held, pattern).map(Some); }
        let said = Value::text(&self.object_words(&held, convert != "s")?);
        let layout = crate::formatting::Layout { table: self.table, names: self.wording() };
        layout.present(&said, pattern, "").map(Some)
    }

    fn value_worded(&mut self, item: &Value, quoted: bool) -> Result<Option<String>, String> {
        let held = item.settled();
        if !self.speaks_for(&held) { return Ok(None); }
        self.object_words(&held, quoted).map(Some)
    }
}

#[path = "classes.rs"]
mod classes;
