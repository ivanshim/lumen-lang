// The engine: one loop over the words of a routine, one data stack for
// the whole run. A call runs the callee's words on the same stack with a
// fresh frame of cells; globals are one table. Fused words read their
// operands from cells by reference and never touch the stack for them.

use std::collections::HashMap;
use std::cell::RefCell;
use num_bigint::BigInt;
use std::rc::Rc;

use num_traits::ToPrimitive;

use crate::lang::{Complaint, Lang};
use crate::arith::{self, Operation};
use crate::value::{Descriptor, Class, Instance, Reach, Sort, Value, Wording, Generator, CursorState, CursorSource};
use crate::code::{Operand, Builtin, Action, Routine, Cell, Instr};

/// An arm may end where it stands, or leave for a routine's end or a
/// loop written around it. Only the latter must pass through last parts.
#[derive(Clone, Copy)]
enum Passage {
    Along(usize),
    Leaves { to: usize, cycle: Option<usize> },
}

pub struct Engine<'a> {
    lang: &'a Lang,
    world: Vec<Value>,
    /// The names of the globals, kept whole so that source read while
    /// the program runs can be assembled against the same ones.
    registry: crate::compile::Registry,
    data: Vec<Value>,
    caught: Vec<Value>,
    memo: HashMap<String, Value>,
    core_ids: HashMap<String, usize>,
    /// The arguments of a builtin call, one buffer reused across calls.
    buffer: Vec<Value>,
    /// What each call still running was given, the innermost last. Kept
    /// only where the language can read it.
    given: Vec<Vec<Value>>,
    /// How many objects have been made, so that each carries its turn.
    made: usize,
    /// Which line of the source is running, for a complaint to name.
    line: u32,
    /// Where the program is written, as the request carried it.
    source: Rc<str>,
    /// Where the language's own pages are kept, as the run was started.
    /// Nothing where the run was started with nowhere named, and then a
    /// complaint about a word of the language points at no page.
    pages_kept: Option<String>,
    /// The calls under way, innermost last: what each called, where the
    /// call was written, and where among the arguments kept aside its
    /// own stand. A fault raised inside them names them all.
    calls: Vec<Called>,
    /// The calls a fault was raised under, written out as they stood
    /// then, since by the time it is told they have all been left. The
    /// first fault to leave a call writes this; a guard taking one
    /// clears it again.
    under: Option<String>,
    /// The routine a value nobody took is handed to, where the program
    /// put one in its way.
    untaken: RefCell<Option<Value>>,
    /// Whether anything has gone out of the run yet. What is held back
    /// in a piece of output kept aside has not gone out.
    written_out: std::cell::Cell<bool>,
    /// Where the routine a fault was raised on the way into is written.
    /// Such a fault belongs there and not where the call stood, which
    /// is worth saying only where nothing takes it.
    entering: Option<(Rc<str>, u32, bool)>,
    /// Nothing at all, to hand back where a binding never written is
    /// read in place and the language only complains about it.
    nothing: Value,
    /// The line the last value raised was raised on, which a language
    /// that tells where a run ended names.
    hurled_at: std::cell::Cell<u32>,
    /// How long the run may take, in seconds, and when the count began;
    /// nought is no limit at all.
    limit: std::cell::Cell<usize>,
    /// The runs begun beside this one, each under the number it
    /// was begun with, so that it may be stopped again later.
    beside: std::cell::RefCell<HashMap<i64, std::process::Child>>,
    began: std::cell::Cell<Option<std::time::Instant>>,
    /// How much room the run may take, in bytes; nought is no limit at
    /// all. What the run has taken is not kept here: the tally of it
    /// belongs to the allocator the host set up, and is only read.
    ceiling: std::cell::Cell<usize>,
    /// How many pieces of the program now being found asked for quiet.
    /// Counted rather than flagged, since one hushed piece may hold
    /// another.
    hushed: std::cell::Cell<usize>,
    /// The same for a piece the program silenced outright: nothing at
    /// all is said of it, not even what is said of how it is written.
    muted: std::cell::Cell<usize>,
    /// The class each call is running inside, innermost last: what a
    /// class keeps to itself is reached from there and nowhere else.
    inside: Vec<Option<Rc<str>>>,
    /// What the run has written out while it was being kept rather than
    /// let go, innermost last. A keeping within a keeping writes into
    /// the one around it when it is given up.
    holding: RefCell<Vec<String>>,
    /// The routines to run once the program's own last statement is
    /// done, each with what it is to be handed, in the order they were
    /// named.
    when_done: RefCell<Vec<(Value, Vec<Value>)>>,
    /// Every object made, held loosely, in the order they were made, so
    /// that those still standing when the run ends can be let go.
    things_made: RefCell<Vec<std::rc::Weak<Instance>>>,
    /// The files already read where the program asked for them to be
    /// read only once, by the whole name each stands under.
    read_already: RefCell<std::collections::HashSet<String>>,
    /// A fault raised inside a source read in, carried out past the
    /// reading: what the reading answers with is a note and no fault of
    /// its own, so a throw or an ending is kept here and raised again
    /// where the reading stood.
    carried: Option<Fault>,
    /// The routine every complaint is handed to, where the program has
    /// put one in the way of them; the complaints waiting to be handed
    /// over, since one may be raised where the run cannot reach back
    /// into the program; and whether any are waiting, which the word
    /// loop asks before every word.
    complainer: RefCell<Option<Value>>,
    waiting: RefCell<Vec<(Complaint, String, u32)>>,
    any_waiting: std::cell::Cell<bool>,
    args_cell: Option<usize>,
    memo_cell: Option<usize>,
    /// Text read in that could not be read at all: what was said of it,
    /// where that text is reckoned to stand, and which of its own lines
    /// the reading stopped on. The reading answers with a note like any
    /// other, so where the text stands is kept here for the telling at
    /// the end of the run, and is believed only for that same note.
    reading_amiss: Option<(String, Rc<str>, u32)>,
}

/// A call under way: what it called, where the call itself was
/// written, and where among the arguments kept aside its own stand.
struct Called {
    named: Rc<str>,
    within: Option<Rc<str>>,
    from: Rc<str>,
    on: u32,
    given_at: Option<usize>,
    /// Whether what was called belongs to the language's own library,
    /// which stands before the program's own text, and whether the call
    /// was made from within it. A call the library made has no line of
    /// the program's to name, and one the library made of its own is no
    /// business of the program's at all.
    of_library: bool,
    from_library: bool,
}

/// How a place is being read: plainly, while a value is being taken
/// apart, or so that what comes of it may be written back there. Only
/// the last refuses a value with no places at all, since there is no
/// place there to write.
#[derive(Clone, Copy, PartialEq)]
enum Reading {
    Plain,
    Apart,
    Toward,
}

type Res<T> = Result<T, String>;
/// What stops a run: a fault of the kernel's own words, or a value the
/// program raised for a catch to take.
pub enum Fault {
    Note(String),
    Thrown(Value),
    /// The run is over and no guard may take it back: a limit the
    /// language set on the run itself was passed.
    Stopped(String),
    /// The program said the run was over. Nothing went wrong and nothing
    /// is told; whatever was to be written was written before this.
    Finished,
}

impl From<String> for Fault {
    fn from(note: String) -> Fault {
        Fault::Note(note)
    }
}

impl From<&str> for Fault {
    fn from(note: &str) -> Fault {
        Fault::Note(note.to_string())
    }
}

impl Fault {
    /// The words to show when nothing caught it.
    pub fn told(self, sp: &Wording) -> String {
        match self {
            Fault::Note(note) => note,
            Fault::Thrown(Value::Object(o)) => {
                let told = o.fields.borrow().iter().find(|(n, _)| n == "message").map(|(_, v)| v.plain());
                match told {
                    Some(told) => format!("Uncaught {}: {}", o.class.name, told),
                    None => format!("Uncaught {}", o.class.name),
                }
            }
            Fault::Thrown(v) => format!("Uncaught {}", v.display(sp)),
            Fault::Stopped(told) => told,
            Fault::Finished => String::new(),
        }
    }
}

/// A run that a raised value may stop.
type Flow<T> = Result<T, Fault>;

impl<'a> Engine<'a> {
    pub fn new(lang: &'a Lang, registry: crate::compile::Registry) -> Engine<'a> {
        let idents = &registry.idents;
        let find = |wanted: &Option<String>| wanted.as_ref().and_then(|w| idents.iter().position(|n| n == w));
        let mut world: Vec<Value> = idents.iter().map(|word| if lang.builtins.values().any(|b| *b == Builtin::InstanceOf) {
            lang.builtins.get(word).map_or(Value::Blank, |b| Value::Native(*b, Rc::from(word.as_str())))
        } else { Value::Blank }).collect();
        if !lang.catch_as.is_empty() {
            if let (Some(slot), Some(name)) = (find(&lang.fault_value), &lang.fault_value) {
                world[slot] = Value::Class(Rc::new(Class {
                    name: name.clone(), base: None, answers: Vec::new(), fields: Vec::new(),
                    reaches: Vec::new(), methods: Vec::new(), constants: Vec::new(), shared: RefCell::new(Vec::new()),
                }));
            }
        }
        Engine {
            lang,
            world,
            data: Vec::new(),
            caught: Vec::new(),
            memo: HashMap::new(),
            core_ids: HashMap::new(),
            buffer: Vec::new(),
            given: Vec::new(),
            made: 0,
            line: 0,
            source: Rc::from(""),
            pages_kept: None,
            calls: Vec::new(),
            under: None,
            entering: None,
            untaken: RefCell::new(None),
            written_out: std::cell::Cell::new(false),
            nothing: Value::Null,
            hurled_at: std::cell::Cell::new(0),
            limit: std::cell::Cell::new(0),
            beside: std::cell::RefCell::new(HashMap::new()),
            began: std::cell::Cell::new(None),
            ceiling: std::cell::Cell::new(0),
            hushed: std::cell::Cell::new(0),
            muted: std::cell::Cell::new(0),
            inside: Vec::new(),
            holding: RefCell::new(Vec::new()),
            when_done: RefCell::new(Vec::new()),
            things_made: RefCell::new(Vec::new()),
            read_already: RefCell::new(std::collections::HashSet::new()),
            carried: None,
            complainer: RefCell::new(None),
            waiting: RefCell::new(Vec::new()),
            any_waiting: std::cell::Cell::new(false),
            args_cell: find(&lang.args_binding),
            memo_cell: find(&lang.memo_binding),
            reading_amiss: None,
            registry,
        }
    }

    /// Assemble source against the globals this run already has and run
    /// it where it stands, giving back whatever it answered with.
    /// Text read while the run is going, built and run where it stands.
    /// Where it came from a file of its own, that file is where the run
    /// is written for as long as it lasts: a complaint names it, and a
    /// file it asks for in turn is looked for beside it.
    /// Text read while the run goes, read as standing where the call to
    /// read it stands: the names of the routine around it are its own,
    /// and what it writes to one of them the routine sees. Names it
    /// makes itself go on the end and are gone once it is done, since
    /// the frame it was handed is not its own to lengthen.
    fn run_text_here(&mut self, program: &Rc<Routine>, frame: &mut [Value]) -> Flow<()> {
        let given = self.drop_top()?;
        let sp = self.wording();
        let source = match &self.lang.prologue {
            Some(open) => format!("{}
{}", open, given.display(&sp)),
            None => given.display(&sp),
        };
        let tokens = self.tokens_of_text(&source)?;
        let names = program.idents.clone();
        // Text read inside a method is read as standing in that method's
        // class: what the class keeps to itself is reached from there,
        // and a call written through a class is a call from within it.
        let within = self.standing_in().map(str::to_string).map(|named| {
            let base = match self.class_named(&named) {
                Some(Value::Class(c)) => c.base.as_ref().map(|b| b.name.clone()),
                _ => None,
            };
            (named, base)
        });
        let read = match crate::compile::compile_within(&tokens, self.lang, &mut self.registry, 0, None, Some(names), within, true) {
            Ok(read) => read,
            Err(said) => {
                let row = self.registry.stopped_at;
                return Err(self.text_amiss(said, row).into());
            }
        };
        self.world.resize(self.registry.idents.len(), Value::Blank);
        let mut mine: Vec<Value> = frame.to_vec();
        mine.resize(read.idents.len().max(frame.len()), Value::Blank);
        let base = self.data.len();
        let outcome = self.run_instrs(&read, &mut mine);
        // What the text wrote to a name the routine already had, the
        // routine sees.
        for (place, worth) in frame.iter_mut().zip(mine) {
            *place = worth;
        }
        match outcome {
            Ok(()) => {}
            Err(Fault::Note(told)) => return Err(told.into()),
            Err(other) => return Err(other),
        }
        // What it left behind is its answer; nothing left is a plain yes.
        let answer = match self.data.len() > base {
            true => self.drop_top()?,
            false => Value::Small(1),
        };
        self.data.push(answer);
        Ok(())
    }

    /// How many lines of a text stand ahead of the program in it. Only
    /// the mark that opens code puts any there, and it puts one.
    fn lines_ahead(&self) -> usize {
        usize::from(self.lang.prologue.is_some())
    }

    /// Where text read in is reckoned to stand: the file the reading
    /// was asked for in, and the line the asking is written on, set
    /// about by the words the language has for saying it was text read
    /// in and not a file of its own.
    fn place_of_text(&self) -> Rc<str> {
        match &self.lang.eval_place {
            Some((before, after)) => Rc::from(format!("{}{}{}{}", self.source, before, self.line, after).as_str()),
            None => self.source.clone(),
        }
    }

    /// Text read in that cannot be read at all, with where it stands
    /// written down so the end of the run may name it.
    fn text_amiss(&mut self, said: String, row: usize) -> String {
        let place = self.place_of_text();
        let on = row.saturating_sub(self.lines_ahead()).max(1) as u32;
        self.reading_amiss = Some((said.clone(), place, on));
        said
    }

    /// The tokens of text read in, or the words for why it could not be
    /// read, with the line it stopped on counted from the program's own
    /// first line rather than the text's.
    fn tokens_of_text(&mut self, source: &str) -> Res<Vec<crate::lex::Token>> {
        let ahead = self.lines_ahead();
        let read = crate::lex::lex_at(source, self.lang).and_then(|tokens| crate::layout::layout(tokens, self.lang, ahead));
        match read {
            Ok(tokens) => Ok(tokens),
            Err((said, row)) => Err(self.text_amiss(said, row)),
        }
    }

    fn run_source(&mut self, source: &str, came_from: Option<String>) -> Res<Value> {
        // Text given outright stands where the call to read it stands;
        // a file stands as itself, and is read from its own first line.
        let tokens = match &came_from {
            None => self.tokens_of_text(source)?,
            Some(_) => crate::layout::layout(crate::lex::lex(source, self.lang)?, self.lang, 0).map_err(|(said, _)| said)?,
        };
        let program = match crate::compile::compile_from(&tokens, self.lang, &mut self.registry, 0, came_from.as_deref().map(Rc::from)) {
            Ok(program) => program,
            Err(said) if came_from.is_none() => {
                let row = self.registry.stopped_at;
                return Err(self.text_amiss(said, row));
            }
            Err(said) => return Err(said),
        };
        // Names the new source brought with it want room in the world.
        self.world.resize(self.registry.idents.len(), Value::Blank);
        let (was_written_in, was_on) = (self.source.clone(), self.line);
        if let Some(place) = came_from {
            self.source = Rc::from(place.as_str());
        }
        let base = self.data.len();
        let ran = self.invoke(&program, Vec::new());
        self.source = was_written_in;
        self.line = was_on;
        match ran {
            Ok(()) => {}
            Err(Fault::Note(told)) => return Err(told),
            // A throw, or an ending, belongs to whatever stands around
            // the reading and not to the reading, so it is carried out
            // and raised again there.
            Err(other) => {
                self.carried = Some(other);
                return Err("the source read in did not finish".to_string());
            }
        }
        // What it left behind is its answer; nothing left is a plain yes.
        Ok(match self.data.len() > base {
            true => self.drop_top()?,
            false => Value::Small(1),
        })
    }

    /// Where the program is written, which a complaint names.
    pub fn written_in(&mut self, place: &str) {
        self.source = Rc::from(place);
    }

    /// Where the language's own pages are kept, as the run was started.
    pub fn pages_are_kept(&mut self, where_at: Option<String>) {
        self.pages_kept = where_at;
    }

    /// Whether a name never written is worth complaining about rather
    /// than stopping for. Only a variable is: where a language marks
    /// its variables with a sign, a name without that sign is a
    /// constant or a class, and reaching for one that is not there is
    /// a fault, not a complaint. The cells a kernel makes for itself
    /// carry no such sign either, and are no business of the program's.
    fn warns_about(&self, ident: &str) -> bool {
        self.lang.warns_of_unwritten && self.lang.sigil.map_or(true, |mark| ident.starts_with(mark))
    }

    /// Tell a complaint the way this language tells one, and go on. A
    /// language with no word for the kind says nothing at all.
    /// Whether a value counts as true in this language. Beyond nought
    /// and nothing, a language may count an array holding nothing as
    /// untrue, and may name pieces of text it counts as untrue.
    fn truth(&self, v: &Value) -> bool {
        match v {
            Value::Collection(cell, _) => self.truth(&cell.borrow()),
            Value::Bond(shared) => self.truth(&shared.borrow()),
            Value::Array(items) if self.lang.untrue_empty => !items.is_empty(),
            Value::Map(pairs) if self.lang.untrue_empty => !pairs.is_empty(),
            Value::Text(s) if self.lang.untrue_text.iter().any(|w| w == s.as_ref()) => false,
            other => other.is_true(),
        }
    }

    /// The binding a piece of text names. Where a language marks its
    /// variables, the mark belongs to the name and not to the text that
    /// spells it, so it is put back on.
    fn name_spelled(&self, spelled: &Value) -> String {
        let sp = self.wording();
        let said = spelled.display(&sp);
        match self.lang.sigil {
            Some(mark) if !said.starts_with(mark) => format!("{}{}", mark, said),
            _ => said,
        }
    }

    /// What a value is worth as a number: text for the number it opens
    /// with, a flag for one or nought, nothing for nought, and an array
    /// for whether it holds anything.
    fn as_number(&self, v: &Value) -> Value {
        match v {
            Value::Text(s) => number_opening(s).0.unwrap_or(Value::Small(0)),
            Value::Flag(yes) => Value::Small(i64::from(*yes)),
            Value::Null | Value::Blank | Value::Gap | Value::Fence => Value::Small(0),
            Value::Array(_) | Value::Map(_) => Value::Small(i64::from(self.truth(v))),
            held => held.clone(),
        }
    }

    /// Everything the run writes out goes through here, so that a
    /// language which can keep its own output has one place to keep it.
    fn utter(&self, text: &str) {
        let mut holding = self.holding.borrow_mut();
        match holding.last_mut() {
            Some(kept) => kept.push_str(text),
            None => {
                drop(holding);
                if !text.is_empty() {
                    self.written_out.set(true);
                }
                self.let_out(text);
            }
        }
    }

    /// Text put where the run writes. Where the language holds text as
    /// bytes, each character stands for one byte and is written as that
    /// byte alone; a character standing for no byte cannot arise there.
    /// Where text is letters, it goes out as the letters spell it.
    fn let_out(&self, text: &str) {
        if !self.lang.text_is_bytes {
            print!("{}", text);
            return;
        }
        use std::io::Write as _;
        let bytes: Vec<u8> = text.chars().map(|c| c as u32 as u8).collect();
        let out = std::io::stdout();
        let _ = out.lock().write_all(&bytes);
    }

    /// The routines named to run once the program is done, in the order
    /// they were named. One that raises something stops the rest, as a
    /// fault anywhere else does.
    pub fn run_when_done(&mut self) -> Result<(), Fault> {
        // The clock the run was timed against starts afresh: what a
        // program named to run at the end runs even where the run was
        // stopped for taking too long, and is given the whole of the
        // time the run was allowed rather than what was left of it.
        if self.limit.get() > 0 {
            self.began.set(Some(std::time::Instant::now()));
        }
        loop {
            let next = {
                let mut waiting = self.when_done.borrow_mut();
                if waiting.is_empty() {
                    return Ok(());
                }
                waiting.remove(0)
            };
            let (work, given) = next;
            if let Value::Routine(p) = self.what_it_spells(work) {
                self.invoke(&p, given)?;
            }
        }
    }

    /// Every object still standing when the run ends is let go, in the
    /// order the objects were made, each by the method its class names
    /// for it. An object made by one of those is let go in its turn.
    pub fn let_things_go(&mut self) {
        let Some(named) = self.lang.destructor.clone() else { return };
        let mut at = 0;
        loop {
            let standing = {
                let made = self.things_made.borrow();
                match made.get(at) {
                    Some(loosely) => loosely.upgrade(),
                    None => break,
                }
            };
            at += 1;
            let Some(thing) = standing else { continue };
            let Some(method) = thing.class.method(&named).cloned() else { continue };
            let _ = self.invoke(&method, vec![Value::Object(thing)]);
        }
        self.things_made.borrow_mut().clear();
    }

    /// What is still being kept when the run ends is let go, outermost
    /// last, as a language that keeps its output does at the end.
    pub fn let_go_all(&self) {
        loop {
            let held = self.holding.borrow_mut().pop();
            match held {
                Some(text) => self.utter(&text),
                None => break,
            }
        }
    }

    fn complain(&self, kind: Complaint, message: &str) {
        if self.hushed.get() > 0 || self.muted.get() > 0 {
            return;
        }
        self.said_anyway(kind, message);
    }

    /// The same, said even where the run is keeping quiet about what is
    /// not there: a word about how a program is written is not a word
    /// about what the run found, and only a piece silenced outright
    /// keeps it back.
    fn said_anyway(&self, kind: Complaint, message: &str) {
        if self.muted.get() > 0 {
            return;
        }
        // A program may put a routine in the way of every complaint. A
        // complaint is often raised where the run is only reading and
        // cannot reach back into the program, so it waits here and is
        // handed over before the next word runs.
        if self.complainer.borrow().is_some() {
            self.waiting.borrow_mut().push((kind, message.to_string(), self.line));
            self.any_waiting.set(true);
            return;
        }
        self.say_complaint(kind, message, self.line);
    }

    fn say_complaint(&self, kind: Complaint, message: &str, line: u32) {
        let Some((_, word)) = self.lang.complaint_words.iter().find(|(k, _)| *k == kind) else { return };
        let opening = complaint_opening(self.lang, word, message);
        self.utter(&format!("{}{}", opening, complaint_place(self.lang, &self.source, line)));
    }

    /// Where the page for a word of the language is to be found, said
    /// in a complaint about that word. Nothing at all unless the run
    /// was started knowing where the pages are kept and asking to be
    /// told for a reader of markup, an address being of no use to a
    /// reader who is given no way to follow it.
    fn page_for(&self, word: &str) -> String {
        let (Some(kept), Some((before, between, after)), Some((opens, closes))) =
            (&self.pages_kept, &self.lang.markup_page, &self.lang.page_named)
        else {
            return String::new();
        };
        let named = match &self.lang.page_mark {
            Some((mark, instead)) => word.replace(mark.as_str(), instead),
            None => word.to_string(),
        };
        let page = format!("{}{}{}", opens, named, closes);
        format!("{}{}{}{}{}{}", before, kept, page, between, page, after)
    }

    /// The complaints waiting are handed to the routine the program put
    /// in their way, oldest first. A routine answering false leaves its
    /// complaint to be written out as it would have been.
    fn hand_over_complaints(&mut self) -> Flow<()> {
        self.any_waiting.set(false);
        loop {
            let next = {
                let mut waiting = self.waiting.borrow_mut();
                if waiting.is_empty() {
                    return Ok(());
                }
                waiting.remove(0)
            };
            let (kind, message, line) = next;
            let hook = self.complainer.borrow().clone();
            let Some(Value::Routine(p)) = hook.map(|v| self.what_it_spells(v)) else {
                self.say_complaint(kind, &message, line);
                continue;
            };
            let word = self.lang.complaint_words.iter().find(|(k, _)| *k == kind).map(|(_, w)| w.clone());
            let told = vec![
                Value::text(&word.unwrap_or_default()),
                Value::text(&message),
                Value::text(&self.source),
                Value::Small(line as i64),
            ];
            self.invoke(&p, told)?;
            let answered = self.drop_top().unwrap_or(Value::Null);
            if !answered.is_true() {
                self.say_complaint(kind, &message, line);
            }
        }
    }

    /// Leaving a call: where it is left by a fault and none has been
    /// written down yet, the calls are written out as they stand, since
    /// the moment after this they are gone.
    fn left_the_call(&mut self, amiss: bool, watching: bool, noted: bool) {
        if amiss && self.under.is_none() {
            self.under = Some(self.calls_told());
        }
        if watching {
            self.given.pop();
        }
        if noted {
            self.calls.pop();
        }
    }

    /// Whether a call stands for the run rather than for the program:
    /// the routine the run hands its complaints to is the run's own
    /// doing, so a trace looks past it to whatever raised the
    /// complaint.
    /// Whether a call is the library's own doing from end to end: what
    /// the language's own library does among itself is no part of the
    /// program's calls and stands in no trace of them.
    fn library_alone(&self, call: &Called) -> bool {
        call.of_library && call.from_library
    }

    fn stands_for_the_run(&self, named: &str) -> bool {
        let put = self.complainer.borrow();
        match put.as_ref() {
            Some(Value::Routine(p)) => p.ident == named,
            Some(v) => matches!(v, Value::Text(t) if t.as_ref() == named),
            None => false,
        }
    }

    /// The calls under way, innermost first, each named with where it
    /// was written and what it was given, and the outermost body last.
    fn calls_told(&self) -> String {
        let mut out = String::from("Stack trace:\n");
        let mut at = 0;
        for call in self.calls.iter().rev() {
            if self.stands_for_the_run(&call.named) || self.library_alone(call) {
                continue;
            }
            let named = match &call.within {
                Some(class) => format!("{}::{}", class, call.named),
                None => call.named.to_string(),
            };
            let handed = match self.handed_to(call) {
                Some(values) => values.iter().map(|v| self.argument_told(v)).collect::<Vec<_>>().join(", "),
                None => String::new(),
            };
            // A call the library made has no line of the program's to
            // name, so where it stands is told as being nowhere the
            // program was written.
            let stood = match call.from_library {
                true => "[internal function]".to_string(),
                false => format!("{}({})", call.from, call.on),
            };
            out.push_str(&format!("#{} {}: {}({})\n", at, stood, named, handed));
            at += 1;
        }
        out.push_str(&format!("#{} {{main}}\n", at));
        out
    }

    /// What a call was handed, as a trace tells it: a method is handed
    /// the thing it is for before everything else, and that is no
    /// argument of the call's.
    fn handed_to(&self, call: &Called) -> Option<&[Value]> {
        let given = call.given_at.and_then(|i| self.given.get(i))?;
        match call.within.is_some() && !given.is_empty() {
            true => Some(&given[1..]),
            false => Some(given),
        }
    }

    /// An argument as a trace writes it: enough of it to know it by,
    /// never the whole of it.
    fn argument_told(&self, v: &Value) -> String {
        const MOST: usize = 15;
        match v {
            Value::Bond(cell) => self.argument_told(&cell.borrow()),
            Value::Text(s) => {
                let mut kept: String = s.chars().take(MOST).collect();
                if s.chars().nth(MOST).is_some() {
                    kept.push_str("...");
                }
                format!("'{}'", kept)
            }
            Value::Object(o) => format!("Object({})", o.class.name),
            Value::Array(_) | Value::Map(_) => "Array".to_string(),
            Value::Null | Value::Blank | Value::Gap => "NULL".to_string(),
            Value::Flag(true) => "true".to_string(),
            Value::Flag(false) => "false".to_string(),
            other => other.plain(),
        }
    }

    /// A value nobody took, handed to the routine the program put in
    /// its way. Answers whether it was taken up, since a run whose
    /// fault was handed over says nothing more of it in its own words.
    pub fn taken_up(&mut self, fault: &Fault) -> bool {
        let put = self.untaken.borrow().clone();
        let Some(v) = put else { return false };
        let Value::Routine(p) = self.what_it_spells(v) else { return false };
        let raised = match fault {
            Fault::Thrown(raised) => raised.clone(),
            Fault::Note(told) => match self.as_fault(told) {
                Some(made) => made,
                None => return false,
            },
            _ => return false,
        };
        let _ = self.invoke(&p, vec![raised]);
        true
    }

    /// A value raised and never caught, told the way a language that
    /// has a word for the end of a run tells it: what was raised, where
    /// it was raised, and how the run stood when it was. Nothing is
    /// written where a language has no word for it.
    pub fn ended_uncaught(&self, fault: &Fault) {
        let Some((_, word)) = self.lang.complaint_words.iter().find(|(k, _)| *k == Complaint::Fatal) else { return };
        let sp = self.wording();
        let raised = match fault {
            // A run the program itself ended is not told at all.
            Fault::Finished => return,
            // A limit passed is told plainly, since nothing was raised.
            Fault::Stopped(told) => {
                self.utter(&format!("{}{}", complaint_opening(self.lang, word, told), complaint_place(self.lang, &self.source, self.line)));
                return;
            }
            Fault::Thrown(raised) => raised,
            // A fault of the kernel's own is told under the class the
            // language names for one, where it names any.
            Fault::Note(told) => {
                let Some(named) = self.class_for(told) else { return };
                // A program that could not be read is told as a reading
                // that stopped and not as a fault nobody took, since
                // nothing was ever running for it to stop.
                if let (true, Some(word)) = (self.unreadable(told), &self.lang.reading_word) {
                    let (place, at) = match &self.reading_amiss {
                        Some((said, place, on)) if said == told => (place.clone(), *on),
                        _ => (self.source.clone(), self.line),
                    };
                    self.utter(&format!("{}{}", complaint_opening(self.lang, word, told), complaint_place(self.lang, &place, at)));
                    return;
                }
                // A fault raised on the way into a routine belongs where
                // that routine is written, and says so.
                let (told, place, at) = match &self.entering {
                    // Where the words named where the call stood, the
                    // place that follows is where the routine itself is
                    // written, and is said to be.
                    Some((place, on, defined)) => match defined {
                        true => (format!("{} and defined", told), place.clone(), *on),
                        false => (told.clone(), place.clone(), *on),
                    },
                    None => (told.clone(), self.source.clone(), self.line),
                };
                let said = format!("Uncaught {}: {}", named, told);
                self.utter(&format!("{} in {}:{}\n", complaint_opening(self.lang, word, &said), place, at));
                self.utter(&format!("{}  thrown{}", self.calls_under(), complaint_place(self.lang, &place, at)));
                return;
            }
        };
        let said = match raised {
            Value::Object(o) => {
                let told = o.fields.borrow().iter().find(|(n, _)| n == "message").map(|(_, v)| v.plain());
                match told.filter(|m| !m.is_empty()) {
                    Some(told) => format!("Uncaught {}: {}", o.class.name, told),
                    None => format!("Uncaught {}", o.class.name),
                }
            }
            v => format!("Uncaught {}", v.display(&sp)),
        };
        let at = self.hurled_at.get();
        self.utter(&format!("{} in {}:{}\n", complaint_opening(self.lang, word, &said), self.source, at));
        self.utter(&format!("{}  thrown{}", self.calls_under(), complaint_place(self.lang, &self.source, at)));
    }

    /// The calls a fault was raised under, where any were written down,
    /// and the outermost body alone where none were.
    fn calls_under(&self) -> String {
        match &self.under {
            Some(told) => told.clone(),
            None => "Stack trace:\n#0 {main}\n".to_string(),
        }
    }

    /// A fault of the kernel's own as a value of the class the language
    /// names for one, so a program may take it. Nothing where the
    /// language names no such class, or where it is not to be found.
    /// The class a fault of the kernel's own is raised as. A language
    /// may name one for a fault of a kind, and the kernel knows which of
    /// its own faults are of which kind, since it is the one that says
    /// them. Where the language names none for the kind, the plain class
    /// stands.
    fn class_for(&self, told: &str) -> Option<String> {
        let named = match told {
            // Words the definition itself gave for a place outside the
            // range a value may take are known by being those very words.
            _ if self.lang.args_below.as_deref() == Some(told) || self.lang.args_beyond.as_deref() == Some(told) => &self.lang.fault_value,
            // So too the words for a name that spells one of a class's
            // own values where a constant's name was wanted.
            _ if self.lang.define_scoped.as_deref() == Some(told) => &self.lang.fault_value,
            // The words a definition gave for the remainder by nought
            // and for a shift below nought are known by being those
            // very words; each is a fault of the same kind as the one
            // the kernel words for itself.
            _ if self.lang.fault_modulo.as_deref() == Some(told) => &self.lang.fault_division,
            _ if self.lang.fault_shift.as_deref() == Some(told) => &self.lang.fault_arithmetic,
            _ if told.starts_with("Division by zero") => &self.lang.fault_division,
            _ if told.starts_with("Bit shift by") => &self.lang.fault_arithmetic,
            _ if told.starts_with("Cannot coerce") => &self.lang.fault_kind,
            // An argument that is not of the class its parameter takes.
            _ if told.contains(" must be of type ") => &self.lang.fault_kind,
            // Words the definition gave for an operand that can take no
            // part are known by the message opening with them.
            _ if self.lang.operand_fault.as_ref().map_or(false, |w| told.starts_with(w.as_str())) => &self.lang.fault_kind,
            // Words the definition gave for what was handed over to be
            // walked, known the same way.
            _ if self.lang.giver_unwalkable.as_ref().map_or(false, |(w, _)| told.starts_with(w.as_str())) => &self.lang.fault_walk,
            // A program that could not be read is known the same way:
            // by the words the definition gave for the reading opening
            // with them.
            _ if self.unreadable(told) => &self.lang.fault_reading,
            _ => &None,
        };
        named.clone().or_else(|| self.lang.fault_class.clone())
    }

    /// Whether these are the words of a reading that stopped: any of
    /// the three the definition gives for one, said at the opening.
    fn unreadable(&self, told: &str) -> bool {
        let openings = [
            self.lang.reading_unexpected.clone(),
            self.lang.unclosed_words.as_ref().map(|(before, _)| before.clone()),
            self.lang.unmatched_words.as_ref().map(|(before, _)| before.clone()),
        ];
        openings.into_iter().flatten().any(|word| told.starts_with(&word))
    }

    fn as_fault(&mut self, told: &str) -> Option<Value> {
        if self.lang.core_words.get("core.exhausted").and_then(|w| w.first()).map_or(false, |w| w == told) && !self.lang.special_stop.is_empty() {
            let class = Class { name: self.lang.special_stop[0].clone(), base: None, answers: vec![], fields: vec![], reaches: vec![], methods: vec![], constants: vec![], shared: RefCell::new(vec![]) };
            self.made += 1;
            return Some(Value::Object(Rc::new(Instance { class: Rc::new(class), fields: RefCell::new(vec![]), mark: self.made })));
        }
        let named = self.class_for(told)?;
        let Some(Value::Class(class)) = self.class_named(&named).cloned() else { return None };
        self.hurled_at.set(self.line);
        self.made += 1;
        let mut fields = class.all_fields();
        // What a fault of the kernel's own holds: the words said, and
        // where in the program it was raised.
        for (named, held) in [
            ("message", Value::text(told)),
            ("file", Value::text(&self.source)),
            ("line", Value::Small(self.line as i64)),
        ] {
            match fields.iter_mut().find(|(n, _)| n == named) {
                Some(place) => place.1 = held,
                None => fields.push((named.to_string(), held)),
            }
        }
        Some(Value::Object(Rc::new(Instance { class, fields: RefCell::new(fields), mark: self.made })))
    }

    /// Offer a fault of the kernel's own to the innermost guard as a
    /// raised value. Nothing where the language names no class for one,
    /// or where no guard is watching.
    fn offer_to_guard(&mut self, told: &str, guards: &mut Vec<(usize, usize, usize)>) -> Option<usize> {
        if guards.is_empty() {
            return None;
        }
        let made = self.as_fault(told)?;
        let (catch, depth, quiet) = guards.pop().expect("a guard was watching");
        self.hushed.set(quiet);
        self.data.truncate(depth);
        self.data.push(made);
        Some(catch)
    }

    pub fn define(&mut self, name: &str, v: Value) {
        if let Some(i) = self.registry.idents.iter().position(|n| n == name) {
            self.world[i] = v;
        }
    }

    /// What a value stands for where a routine or a class is wanted.
    /// Where a language lets a name be worked out as the run goes, a
    /// piece of text spells one of the outermost bindings, and that
    /// binding is what stands there.
    /// Where a language lets a name spelled out stand for the class of
    /// that name, a name no class answers to is that much said: which
    /// class it is that there is none of, rather than that a piece of
    /// text is not a class.
    fn no_such_class(&self, v: &Value) -> Option<String> {
        match (self.lang.spelled_stands, v) {
            (true, Value::Text(spelled)) => Some(format!("Class \"{}\" not found", spelled)),
            _ => None,
        }
    }

    /// What a thing hands over to be walked in its stead, where it is a
    /// thing that hands one over and what it hands back is not itself.
    fn walk_handed(&mut self, held: &Value) -> Result<Option<(String, Value)>, Fault> {
        let (Some(class), Some(gives)) = (self.lang.giver_class.clone(), self.lang.walk_giver.clone()) else { return Ok(None) };
        let Value::Object(o) = held else { return Ok(None) };
        if !o.class.named(&class, self.lang.classes_folded) {
            return Ok(None);
        }
        let Some(method) = o.class.method(&gives).cloned() else { return Ok(None) };
        self.invoke(&method, vec![held.clone()])?;
        let handed = self.drop_top()?;
        let itself = matches!((&handed, held), (Value::Object(a), Value::Object(b)) if Rc::ptr_eq(a, b));
        Ok(match itself {
            true => None,
            false => Some((o.class.name.clone(), handed)),
        })
    }

    /// A whole number, with the language's complaint for another kind.
    fn whole_for_bits(&self, v: &Value) -> Res<BigInt> {
        if matches!(v.sort(), Some(Sort::Integer | Sort::Boolean)) {
            v.as_big()
        } else {
            Err(self.lang.operand_fault.clone().unwrap_or_else(|| "Working on bits needs a whole number".to_string()))
        }
    }

    /// The bits of a value, with a word said where a real is too wide
    /// for the whole numbers this language holds and working on its bits
    /// means taking something else. A language with no word for a
    /// warning says nothing and takes it just the same.
    fn bits_said(&self, v: &Value) -> Res<i64> {
        let bits = bits_of(v)?;
        if !self.lang.warns_of_unwritten {
            return Ok(bits);
        }
        if let Value::Frac(_) | Value::Real(_) = v {
            let Some(exact) = arith::Exact::from_value(v) else { return Ok(bits) };
            // The nearest real of the width is what a language of that
            // width holds, and whether *it* can be held as a whole
            // number is the question, not whether the exact ratio can.
            let near = crate::value::as_binary(&exact.p, &exact.q);
            let widest = 9223372036854775808.0f64;
            if !(near >= -widest && near < widest) {
                // Written to the last figure that tells it apart from its
                // neighbours, since the words are about this very number
                // and not about how the language shows one.
                let told = format!(
                    "The float {} is not representable as an int, cast occurred",
                    crate::value::binary_string(near, None)
                );
                self.complain(Complaint::Warning, &told);
            }
        }
        Ok(bits)
    }

    fn walkable(&mut self, held: &Value) -> Result<(), Fault> {
        if matches!(held, Value::Tuple(_) | Value::Set(_) | Value::Cursor(_) | Value::Array(_) | Value::Map(_) | Value::Object(_) | Value::Counted(_)) {
            return Ok(());
        }
        if !self.lang.warns_of_unwritten {
            return Err("Cannot walk a value that is not an array".to_string().into());
        }
        let told = format!("foreach() argument must be of type array|object, {} given", self.kind_named(held));
        self.complain(Complaint::Warning, &told);
        Ok(())
    }

    /// Whether a class shares what it keeps for itself and those along
    /// its line with the class the run stands in: the two are along one
    /// line when either stands on the other.
    fn shares_with(&self, holder: &str, here: Option<&str>) -> bool {
        let Some(here) = here else { return false };
        if here == holder {
            return true;
        }
        let stands_on = |below: &str, above: &str| match self.class_named(below) {
            Some(Value::Class(c)) => c.named(above, self.lang.classes_folded),
            _ => false,
        };
        stands_on(here, holder) || stands_on(holder, here)
    }

    /// The class the run stands inside, where it stands inside one.
    fn standing_in(&self) -> Option<&str> {
        self.inside.last().and_then(|named| named.as_deref())
    }

    /// The place a thing holds a property of that name, reached from
    /// where the run stands: what a class keeps to itself is filed under
    /// its own name and the class's together, so a program written in
    /// that class finds it there and every other finds the open one.
    fn member_at(&self, held: &[(String, Value)], name: &str) -> Option<usize> {
        if let Some(here) = self.standing_in() {
            let alone = crate::value::kept_alone(name, here);
            if let Some(at) = held.iter().position(|(n, v)| *n == alone && standing(v)) {
                return Some(at);
            }
        }
        held.iter().position(|(n, v)| n == name && standing(v))
    }

    /// The method a class answers with for a property the thing does
    /// not hold, and the one it takes such a write with, where the
    /// language names them and the class is written with them.
    fn reads_for(&self, o: &Instance) -> Option<Rc<Routine>> {
        let named = self.lang.reader.as_deref()?;
        o.class.method(named).cloned()
    }

    fn writes_for(&self, o: &Instance) -> Option<Rc<Routine>> {
        let named = self.lang.writer.as_deref()?;
        o.class.method(named).cloned()
    }

    /// The thing that is its own walk, where this value is one.
    fn walker(&self, held: &Value) -> Option<Rc<Instance>> {
        let class = self.lang.walker_class.as_deref()?;
        match held {
            Value::Object(o) if o.class.named(class, self.lang.classes_folded) => Some(o.clone()),
            _ => None,
        }
    }

    /// Ask a thing that is its own walk one of the walk's questions.
    /// Nothing where the value is not such a thing, so that the caller
    /// counts it through instead.
    fn walk_asked(&mut self, held: &Value, named: Option<String>) -> Result<Option<Value>, Fault> {
        let (Some(thing), Some(named)) = (self.walker(held), named) else { return Ok(None) };
        let Some(method) = thing.class.method(&named).cloned() else { return Ok(None) };
        self.invoke(&method, vec![held.clone()])?;
        Ok(Some(self.drop_top()?))
    }

    fn what_it_spells(&self, v: Value) -> Value {
        let Value::Text(name) = &v else { return v };
        if !self.lang.spelled_stands {
            return v;
        }
        // A routine and a class may go by one name. Standing where
        // either would do, the routine is meant: a class named by a
        // value stands where only a class will do, and is looked for
        // there.
        match self.lookup(name) {
            Some(found @ Value::Routine(_)) => found.clone(),
            _ => self.class_named(name).cloned().unwrap_or(v),
        }
    }

    /// The same, where only a class will do: `new $c`, `$c::m()` and the
    /// rest, which a routine of that name is no answer to.
    fn class_it_spells(&self, v: Value) -> Value {
        let Value::Text(name) = &v else { return v };
        if !self.lang.spelled_stands {
            return v;
        }
        self.class_named(name).cloned().unwrap_or(v)
    }

    /// The class of that name. Where a language knows a class by its
    /// name however the name is written, a name nothing answers to is
    /// tried again with every letter made small.
    fn class_named(&self, name: &str) -> Option<&Value> {
        let filed = format!("{}{}", name, crate::code::OF_A_CLASS);
        if let found @ Some(_) = self.lookup(&filed) {
            return found;
        }
        match self.lang.classes_folded {
            true => self.lookup(&format!("{}{}", name.to_lowercase(), crate::code::OF_A_CLASS)),
            false => None,
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        let i = self.registry.idents.iter().position(|n| n == name)?;
        match &self.world[i] {
            Value::Blank => None,
            v => Some(v),
        }
    }

    /// The language's words for the literals, for telling a fault.
    pub fn names(&self) -> Wording<'a> {
        self.wording()
    }

    fn wording(&self) -> Wording<'a> {
        let word = |list: &'a [String], fallback: &'a str| list.first().map_or(fallback, String::as_str);
        let nothing = if self.lang.null_silent { "" } else { word(&self.lang.null_words, "null") };
        Wording {
            true_word: word(&self.lang.true_words, "true"),
            false_word: word(&self.lang.false_words, "false"),
            null_word: nothing,
            flag_counts: self.lang.flags_count,
            real_digits: self.lang.real_bits.and(self.lang.real_digits),
            binary_reals: self.lang.real_bits.is_some(),
            text_is_bytes: self.lang.text_is_bytes,
            guarded_word: self.lang.guarded_words.first().map(String::as_str),
            hidden_word: self.lang.hidden_words.first().map(String::as_str),
        }
    }

    fn drop_top(&mut self) -> Res<Value> {
        self.data.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    fn drop_many(&mut self, n: usize) -> Res<Vec<Value>> {
        if self.data.len() < n {
            return Err("Stack underflow".to_string());
        }
        let at = self.data.len() - n;
        Ok(self.data.split_off(at))
    }

    /// A load: the first local slot holding a value, else the global. A
    /// taking load moves the value out and leaves a hole.
    fn load_cell(&mut self, slot: &Cell, frame: &mut [Value]) -> Res<Value> {
        for &s in &slot.near {
            if let Value::Binding(shared) = &frame[s] {
                let value = shared.borrow().clone();
                if self.lang.closes_over && matches!(value, Value::Blank) {
                    return Err(Self::named_fault(if slot.free { &self.lang.free_unbound } else { &self.lang.local_unbound }, &slot.ident));
                }
                return Ok(value);
            }
            if let Value::Bond(shared) = &frame[s] {
                return Ok(if self.lang.bind_names { Value::Bond(shared.clone()) } else { shared.borrow().clone() });
            }
            if !matches!(frame[s], Value::Blank) {
                return Ok(if slot.moving { std::mem::replace(&mut frame[s], Value::Gap) } else { frame[s].clone() });
            }
        }
        if self.lang.closes_over && !slot.near.is_empty() {
            return Err(Self::named_fault(&self.lang.local_unbound, &slot.ident));
        }
        if let Value::Bond(shared) = &self.world[slot.far] {
            return Ok(if self.lang.bind_names { Value::Bond(shared.clone()) } else { shared.borrow().clone() });
        }
        // Where a language makes a place on writing into it, a name
        // that holds nothing holds an empty array as far as the write
        // is concerned, and nothing is said about it.
        if slot.moving && self.lang.makes_places && matches!(self.world[slot.far], Value::Blank | Value::Null) {
            return Ok(Value::Array(std::rc::Rc::new(Vec::new())));
        }
        // A language that has a word for a warning does not stop for a
        // binding never written: it says so and reads nothing there.
        if matches!(self.world[slot.far], Value::Blank) && self.warns_about(&slot.ident) {
            let told = format!("Undefined variable {}", slot.ident);
            self.complain(Complaint::Warning, &told);
            return Ok(Value::Null);
        }
        let g = &mut self.world[slot.far];
        match g {
            Value::Blank if slot.moving => Err(format!("Undefined variable '{}'", slot.ident)),
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            _ if slot.moving => Ok(std::mem::replace(g, Value::Gap)),
            v => Ok(v.clone()),
        }
    }

    /// The shared cell a binding stands for, made from what it holds if
    /// it is not shared already.
    fn share_cell(&mut self, slot: &Cell, frame: &mut [Value]) -> Res<Rc<RefCell<Value>>> {
        if self.lang.closes_over && self.lang.bind_names {
            if let Some(&s) = slot.near.first() {
                if let Value::Binding(cell) = &frame[s] { return Ok(cell.clone()); }
                let cell = Rc::new(RefCell::new(frame[s].clone()));
                frame[s] = Value::Binding(cell.clone());
                return Ok(cell);
            }
        }
        for &s in &slot.near {
            if let Value::Bond(shared) = &frame[s] {
                return Ok(shared.clone());
            }
            if !matches!(frame[s], Value::Blank) {
                let held = std::mem::replace(&mut frame[s], Value::Null);
                let shared = Rc::new(RefCell::new(held));
                frame[s] = Value::Bond(shared.clone());
                return Ok(shared);
            }
        }
        // A name a unit of its own has a place for belongs to that unit,
        // so where nothing has been written to it anywhere the cell is
        // made in the unit's own place and not the outermost one.
        if let Some(&s) = slot.near.first() {
            let shared = Rc::new(RefCell::new(if self.lang.closes_over { Value::Blank } else { Value::Null }));
            frame[s] = Value::Bond(shared.clone());
            return Ok(shared);
        }
        if let Value::Bond(shared) = &self.world[slot.far] {
            return Ok(shared.clone());
        }
        // A name nothing was ever written to becomes a shared cell
        // holding nothing, as a write to it would have made it.
        let held = match std::mem::replace(&mut self.world[slot.far], Value::Null) {
            Value::Blank => Value::Null,
            other => other,
        };
        let shared = Rc::new(RefCell::new(held));
        self.world[slot.far] = Value::Bond(shared.clone());
        Ok(shared)
    }

    /// Put a value straight into a binding's own place, past any shared
    /// cell it holds: how a name is fastened to another's cell.
    fn put_cell(&mut self, slot: &Cell, frame: &mut [Value], v: Value) {
        match slot.near.first() {
            Some(&s) => frame[s] = v,
            None => self.world[slot.far] = v,
        }
    }

    /// Whether a binding stands for a shared cell, which cannot be read
    /// in place because what it holds lives elsewhere.
    fn shares_cell(&self, slot: &Cell, frame: &[Value]) -> bool {
        for &s in &slot.near {
            if matches!(frame[s], Value::Bond(_) | Value::Binding(_)) {
                return true;
            }
            if !matches!(frame[s], Value::Blank) {
                return false;
            }
        }
        matches!(self.world[slot.far], Value::Bond(_))
    }

    /// The cell a binding lives in, for reading in place.
    fn peek_cell<'f>(&'f self, slot: &Cell, frame: &'f [Value]) -> Res<&'f Value> {
        for &s in &slot.near {
            if !matches!(frame[s], Value::Blank) {
                return Ok(&frame[s]);
            }
        }
        if self.lang.closes_over && !slot.near.is_empty() {
            return Err(Self::named_fault(&self.lang.local_unbound, &slot.ident));
        }
        if matches!(self.world[slot.far], Value::Blank) && self.warns_about(&slot.ident) {
            self.complain(Complaint::Warning, &format!("Undefined variable {}", slot.ident));
            return Ok(&self.nothing);
        }
        match &self.world[slot.far] {
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            v => Ok(v),
        }
    }

    /// Captured bindings must be loaded before an optimized operation:
    /// the wrapper is a binding, not an operand or a collection value.
    fn operand_value(&mut self, operand: &Operand, frame: &mut [Value]) -> Res<Option<Value>> {
        match operand {
            Operand::Top => self.drop_top().map(Some),
            Operand::Cell(slot) if self.shares_cell(slot, frame) => self.load_cell(slot, frame).map(Some),
            _ => Ok(None),
        }
    }

    /// Two both read in place: a binding by reference, a constant
    /// from the word, a data value from the ones already popped.
    fn both<'f>(&'f self, a: &'f Operand, b: &'f Operand, at: &'f Option<Value>, bt: &'f Option<Value>, frame: &'f [Value]) -> Res<(&'f Value, &'f Value)> {
        // A value worked out beforehand stands for the operand: what was
        // popped for a stack operand, or what a shared cell holds.
        let av: &Value = match (at, a) {
            (Some(v), _) => v,
            (None, Operand::Const(v)) => v,
            (None, Operand::Cell(s)) => self.peek_cell(s, frame)?,
            (None, Operand::Top) => return Err("Stack underflow".to_string()),
        };
        let bv: &Value = match (bt, b) {
            (Some(v), _) => v,
            (None, Operand::Const(v)) => v,
            (None, Operand::Cell(s)) => self.peek_cell(s, frame)?,
            (None, Operand::Top) => return Err("Stack underflow".to_string()),
        };
        Ok((av, bv))
    }

    fn peek_cell_mut<'f>(&'f mut self, slot: &Cell, frame: &'f mut [Value]) -> Res<&'f mut Value> {
        for &s in &slot.near {
            if !matches!(frame[s], Value::Blank) {
                return Ok(&mut frame[s]);
            }
        }
        if self.lang.closes_over && !slot.near.is_empty() {
            return Err(Self::named_fault(&self.lang.local_unbound, &slot.ident));
        }
        if matches!(self.world[slot.far], Value::Blank) && self.warns_about(&slot.ident) {
            self.complain(Complaint::Warning, &format!("Undefined variable {}", slot.ident));
            self.world[slot.far] = Value::Null;
        }
        match &self.world[slot.far] {
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            _ => Ok(&mut self.world[slot.far]),
        }
    }

    /// A store: into the hole a taking load left, if one is addressed;
    /// else the first local, or the global when there is none.
    fn store_cell(&mut self, slot: &Cell, frame: &mut [Value], v: Value) -> Res<()> {
        if self.lang.bind_names {
            let value = self.keep_collection(v);
            if let Some(&s) = slot.near.first() {
                if let Value::Binding(cell) = &frame[s] {
                    *cell.borrow_mut() = value;
                    return Ok(());
                }
            }
            self.put_cell(slot, frame, value);
            return Ok(());
        }
        // A cell only ever becomes a name's own through fastening. A
        // plain write of one writes what it holds, so that a routine
        // giving back a cell, called without the mark that shares one,
        // hands over a copy like any other.
        let v = match v {
            Value::Bond(shared) => shared.borrow().clone(),
            held => held,
        };
        for &s in &slot.near {
            if let Value::Bond(shared) = &frame[s] {
                *shared.borrow_mut() = v;
                return Ok(());
            }
            if matches!(frame[s], Value::Gap) {
                frame[s] = v;
                return Ok(());
            }
        }
        if let Value::Bond(shared) = &self.world[slot.far] {
            *shared.borrow_mut() = v;
            return Ok(());
        }
        if matches!(self.world[slot.far], Value::Gap) {
            self.world[slot.far] = v;
            return Ok(());
        }
        match slot.near.first() {
            Some(&s) => frame[s] = v,
            None => {
                if Some(slot.far) == self.args_cell {
                    return Err(format!("Cannot reassign {} (system-provided immutable value)", slot.ident));
                }
                self.world[slot.far] = v;
            }
        }
        Ok(())
    }

    /// Run a program on its arguments. A function's result, the value it
    /// returned or the last expression statement's, or what it assigned to
    /// its own name where the language says so, is pushed; a postfix
    /// program leaves what it pushed.
    /// Every argument is of the class its parameter was declared to
    /// take. A parameter declared with a kind that names no class is
    /// left alone, since a language may bring such a value to the kind
    /// rather than refusing it, and nothing at all is let through, since
    /// a parameter with nothing of its own may be given nothing.
    fn of_the_kind_named(&mut self, program: &Rc<Routine>, frame: &[Value]) -> Flow<()> {
        for (at, named) in program.formal_kinds.iter().enumerate() {
            let Some(named) = named else { continue };
            let Some(held) = frame.get(at) else { continue };
            // A parameter handed a cell holds the cell; what it was
            // given is what the cell holds, and that is what has a kind.
            let held = &match held {
                Value::Bond(cell) => cell.borrow().clone(),
                other => other.clone(),
            };
            if matches!(held, Value::Null | Value::Blank) {
                continue;
            }
            let Some(Value::Class(_)) = self.class_named(named) else { continue };
            if matches!(held, Value::Object(o) if o.class.named(named, self.lang.classes_folded)) {
                continue;
            }
            let given = match held {
                Value::Object(o) => o.class.name.clone(),
                other => self.kind_named(other),
            };
            // A fault of this kind says where the call stood, unless the
            // call was made from inside the language itself, which has
            // no line of the program's to name. Where the routine is
            // written is kept aside: a guard taking it wants the words
            // alone, and only a run ended by it says as much.
            let from_library = self.calls.last().map_or(false, |c| c.from_library);
            let head = format!("{}(): Argument #{} ({}) must be of type {}, {} given", program.ident, at + 1, program.formals[at], named, given);
            let told = match from_library {
                true => head,
                false => format!("{}, called in {} on line {}", head, self.source, self.line),
            };
            let place = program.written_in.clone().unwrap_or_else(|| self.source.clone());
            self.entering = Some((place, program.declared_on, !from_library));
            return Err(told.into());
        }
        Ok(())
    }

    /// Untie call arguments only at the call boundary. A literal's ties
    /// have already become a map by then and remain ordinary values.
    fn call_items(&mut self, args: Vec<Value>) -> Flow<Vec<(Option<String>, Value)>> {
        let mut items = Vec::new();
        for value in args {
            match value {
                Value::Tie(pair) => match &pair.0 {
                    Value::Text(name) => items.push((Some(name.to_string()), pair.1.clone())),
                    Value::Flag(false) => match &pair.1.contents() {
                        Value::Cursor(_) => items.extend(self.core_members(&pair.1)?.into_iter().map(|v| (None,v))),
                        Value::Counted(r) => {
                            let mut i = BigInt::from(0);
                            while let Some(v) = r.at(i.clone()) { items.push((None, v)); i += 1; }
                        }
                        Value::Array(a) | Value::Tuple(a) => items.extend(a.iter().cloned().map(|v| (None, v))),
                        Value::Set(s) => items.extend(s.borrow().items().into_iter().map(|v| (None, v))),
                        Value::Text(t) => items.extend(t.chars().map(|c| (None, Value::text(&c.to_string())))),
                        Value::Map(m) => items.extend(m.iter().map(|(k, _)| (None, k.clone()))),
                        _ => return Err(self.lang.spread_amiss[0].clone().into()),
                    },
                    Value::Flag(true) => {
                        let Value::Map(m) = pair.1.contents() else { return Err(self.lang.spread_pairs_amiss[0].clone().into()); };
                        for (key, held) in m.iter() {
                            let Value::Text(name) = key else { return Err(self.lang.spread_pairs_amiss[0].clone().into()); };
                            items.push((Some(name.to_string()), held.clone()));
                        }
                    }
                    _ => return Err(self.lang.call_amiss[0].clone().into()),
                },
                other => items.push((None, other)),
            }
        }
        Ok(items)
    }

    fn named_fault(words: &[String], name: &str) -> String {
        format!("{}{}{}", words.first().map_or("", String::as_str), name, words.get(1).map_or("", String::as_str))
    }

    /// Fill positional places first, then the named ones. Gatherers
    /// keep what has no ordinary place, and defaults keep the holes.
    fn bind_call(&mut self, program: &Routine, args: Vec<Value>, rules: &[u8]) -> Flow<Vec<Value>> {
        let items = self.call_items(args)?;
        let mut frame = vec![Value::Blank; rules.len()];
        let slots: Vec<usize> = rules.iter().enumerate().filter_map(|(i, r)| (*r < 2).then_some(i)).collect();
        let rest = rules.iter().position(|r| *r == 3);
        let pairs = rules.iter().position(|r| *r == 4);
        let mut tail = Vec::new();
        for (at, (_, value)) in items.iter().filter(|(name, _)| name.is_none()).enumerate() {
            if let Some(slot) = slots.get(at) { frame[*slot] = value.clone(); }
            else if rest.is_some() { tail.push(value.clone()); }
            else { return Err(self.lang.call_amiss[0].clone().into()); }
        }
        let mut keywords = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (name, value) in items {
            let Some(name) = name else { continue };
            if !seen.insert(name.clone()) {
                return Err(Self::named_fault(&self.lang.call_duplicate, &name).into());
            }
            let slot = program.formals.iter().enumerate().position(|(i, n)| *n == name && matches!(rules[i], 0 | 2));
            match slot {
                Some(i) if matches!(frame[i], Value::Blank) => frame[i] = value,
                Some(_) => return Err(Self::named_fault(&self.lang.call_duplicate, &name).into()),
                None if pairs.is_some() => keywords.push((Value::text(&name), value)),
                None => return Err(Self::named_fault(&self.lang.call_unknown, &name).into()),
            }
        }
        if let Some(i) = rest { frame[i] = Value::array(tail); }
        if let Some(i) = pairs { frame[i] = Value::Map(Rc::new(keywords)); }
        for (i, value) in frame.iter().enumerate() {
            if matches!(value, Value::Blank) && !program.carried.contains(&i) {
                return Err(Self::named_fault(&self.lang.call_missing, &program.formals[i]).into());
            }
        }
        Ok(frame)
    }

    pub fn invoke(&mut self, program: &Rc<Routine>, args: Vec<Value>) -> Flow<()> {
        let n = args.len();
        self.data.extend(args);
        self.invoke_top(program, n)
    }

    /// A call whose arguments are the top `n` of the data stack: they move
    /// straight into the frame, one allocation instead of two.
    pub fn invoke_top(&mut self, program: &Rc<Routine>, n: usize) -> Flow<()> {
        let n = if let Some(rules) = &program.parameter_rules {
            let given = self.drop_many(n)?;
            let bound = self.bind_call(program, given, rules)?;
            let count = bound.len();
            self.data.extend(bound);
            count
        } else { n };
        // Where a language can read what a call was given, a call may
        // give more than the routine names; the rest is kept aside.
        let most = if self.lang.spare_args || program.rest_at.is_some() { usize::MAX } else { program.formals.len() };
        if n > most || n < program.least {
            let wanted = program.formals.len();
            return Err(format!("Function {} expects {} arguments, got {}", program.ident, wanted, n).into());
        }
        if self.data.len() < n {
            return Err("Stack underflow".to_string().into());
        }
        let at = self.data.len() - n;
        let memoized = program.returns_value && self.memo_cell.map_or(false, |s| matches!(self.world[s], Value::Flag(true)));
        let key = memoized.then(|| {
            let mut key = format!("{}(", program.ident);
            for a in &self.data[at..] {
                a.memo_key(&mut key);
            }
            key
        });
        if let Some(hit) = key.as_ref().and_then(|k| self.memo.get(k)) {
            self.data.truncate(at);
            self.data.push(hit.clone());
            return Ok(());
        }
        let mut frame: Vec<Value> = Vec::with_capacity(program.idents.len());
        let watching = self.lang.spare_args && !program.body_of_all;
        if watching {
            self.given.push(self.data[at..].to_vec());
        }
        // The call is written down before anything of it runs, so that
        // a fault raised on the way in names it too. The outermost body
        // is nobody's call and is not written down: a trace names it as
        // where everything stands and nothing more.
        let noted = !program.body_of_all;
        if noted {
            let from_library = self.calls.last().map_or(false, |c| c.of_library && !self.stands_for_the_run(&c.named));
            self.calls.push(Called {
                named: Rc::from(program.ident.as_str()),
                within: program.within.clone(),
                from: self.source.clone(),
                on: self.line,
                given_at: watching.then(|| self.given.len() - 1),
                of_library: program.declared_on == 0,
                from_library,
            });
        }
        frame.extend(self.data.drain(at..));
        if let Some(start) = program.rest_at {
            let tail = if frame.len() > start { frame.split_off(start) } else { Vec::new() };
            frame.resize(start, Value::Blank);
            frame.push(Value::array(tail));
        }
        // What the routine does not name stays aside rather than
        // spilling into the slots its own names sit in.
        frame.truncate(program.formals.len());
        if let Err(fault) = self.of_the_kind_named(program, &frame) {
            self.left_the_call(true, watching, noted);
            return Err(fault);
        }
        frame.resize(program.idents.len(), Value::Blank);
        // A routine written where a value stands carried names away
        // from around it: each fills the slot it was given here.
        for (slot, held) in program.carried.iter().zip(program.held.iter()) {
            if program.parameter_rules.is_none() || *slot >= program.formals.len() || matches!(frame[*slot], Value::Blank) {
                frame[*slot] = held.clone();
            }
        }
        if self.lang.closes_over {
            // Each local owns a binding cell; mutable values inside it
            // retain their separate identity when the name is rebound.
            for value in &mut frame {
                *value = Value::Binding(Rc::new(RefCell::new(self.keep_collection(value.clone()))));
            }
        }
        for (at, cell) in &program.enclosed { frame[*at] = cell.clone(); }
        if program.generator && self.lang.yield_suspends {
            self.left_the_call(false, watching, noted);
            self.data.push(Value::Generator(Rc::new(RefCell::new(Generator::new(Some(program.clone()), frame, Vec::new())))));
            return Ok(());
        }
        let base = self.data.len();
        // A routine written in a file of its own is run as being in it:
        // a complaint names that file, and a file the routine asks for
        // is looked for beside it, wherever the call was made.
        let elsewhere = program.written_in.as_ref().map(|place| {
            let was = std::mem::replace(&mut self.source, place.clone());
            (was, self.line)
        });
        self.inside.push(program.within.clone());
        let outcome = self.run_instrs(program, &mut frame);
        self.inside.pop();
        if let Some((was, on)) = elsewhere {
            self.source = was;
            self.line = on;
        }
        self.left_the_call(outcome.is_err(), watching, noted);
        outcome?;
        if !program.returns_value {
            return Ok(());
        }
        let mut result = if self.data.len() > base { self.drop_top()? } else { Value::Null };
        if self.lang.named_result {
            if let Some(s) = program.idents.iter().position(|n| *n == program.ident) {
                match &frame[s] {
                    Value::Blank | Value::Gap | Value::Routine(_) => {}
                    v => result = v.clone(),
                }
            }
        }
        self.data.truncate(base);
        if let Some(key) = key {
            self.memo.insert(key, result.clone());
        }
        self.data.push(result);
        Ok(())
    }

    /// Whether the run has taken longer than the language allowed it.
    /// Counted now and then rather than at every word, since asking the
    /// clock costs more than the words between two askings.
    fn out_of_time(&self) -> Option<Fault> {
        let seconds = self.limit.get();
        let began = self.began.get()?;
        if seconds == 0 || began.elapsed().as_secs() < seconds as u64 {
            return None;
        }
        let ending = if seconds == 1 { "second" } else { "seconds" };
        Some(Fault::Stopped(format!("Maximum execution time of {} {} exceeded", seconds, ending)))
    }

    /// Whether the run has taken more room than the language allowed it.
    /// Looked at where the clock is looked at and for the same reason:
    /// the tally the allocator keeps is exact at every byte, but reading
    /// it at every word would cost more than the words between two
    /// readings. A run may therefore pass the mark by whatever it takes
    /// between one look and the next, and a single value grown past the
    /// mark in one step is not caught until the step is done.
    fn out_of_room(&self) -> Option<Fault> {
        let ceiling = self.ceiling.get();
        if ceiling == 0 {
            return None;
        }
        let taken = lumen_room::used();
        if taken <= ceiling {
            return None;
        }
        // What the run was keeping rather than writing out is dropped
        // where it stands: there is no room to hold it and none to write
        // it with, and the words that end the run would otherwise be
        // kept along with it and come out behind the whole of it.
        self.holding.borrow_mut().clear();
        // The mark is lifted now that it has been passed, so that the
        // ending can be told and whatever was named to run at the end
        // can run without meeting the same wall a second time.
        self.ceiling.set(0);
        Some(Fault::Stopped(format!("Allowed memory size of {} bytes exhausted ({} bytes were taken)", ceiling, taken)))
    }

    /// What a call was given, brought level with what its parameters
    /// hold now. The reference reads back the values, not the cells, so
    /// a parameter handed one reads as what that cell holds.
    fn as_they_stand(&mut self, program: &Rc<Routine>, frame: &[Value]) {
        let Some(given) = self.given.last_mut() else { return };
        for (at, held) in frame.iter().take(program.formals.len()).enumerate() {
            let Some(place) = given.get_mut(at) else { break };
            *place = match held {
                Value::Bond(cell) => cell.borrow().clone(),
                other => other.clone(),
            };
        }
    }

    fn run_instrs(&mut self, program: &Rc<Routine>, frame: &mut [Value]) -> Flow<()> {
        let instrs = &program.instrs;
        // A raised value leaves the marks that end a quiet piece unrun,
        // so how much quiet stood when the piece began is what stands
        // again once it has gone.
        let quiet = self.hushed.get();
        let mut outcome = self.run_body(program, frame, instrs);
        // A complaint raised by the last word of a body would have
        // nowhere left to be handed over, so it is handed over here.
        if outcome.is_ok() && self.any_waiting.get() {
            outcome = self.hand_over_complaints();
        }
        if outcome.is_err() {
            self.hushed.set(quiet);
        }
        outcome
    }

    fn run_body(&mut self, program: &Rc<Routine>, frame: &mut [Value], instrs: &[crate::code::Instr]) -> Flow<()> {
        self.run_span(program, frame, instrs, (0, instrs.len())).map(|_| ())
    }

    /// Each arm runs in the same frame. A leap beyond its span is an
    /// outward return or loop step, and the last part runs before it goes.
    fn run_attempt(&mut self, program: &Rc<Routine>, frame: &mut [Value], instrs: &[Instr], plan: &crate::code::Attempt) -> Flow<Passage> {
        if self.lang.catch_group_unsupported.is_some() && plan.clauses.iter().any(|arm| arm.grouped) {
            return Err(self.lang.catch_group_unsupported.as_deref().unwrap_or("Exception groups are not supported").into());
        }
        let depth = self.data.len();
        let active = self.caught.len();
        let ending = match self.run_span(program, frame, instrs, plan.body) {
            Err(Fault::Note(words)) => match self.as_fault(&words) { Some(value) => Err(Fault::Thrown(value)), None => Err(Fault::Note(words)) },
            other => other,
        };
        if let Some(cell) = &plan.context {
            let object = self.load_cell(cell, frame)?;
            if !matches!(object, Value::Object(_)) {
                return ending.map(|end| match end { Passage::Along(at) if at == plan.body.1 => Passage::Along(plan.after), other => other });
            }
            if matches!(&ending, Err(Fault::Note(_))) || matches!(&ending, Err(Fault::Thrown(value)) if !matches!(value, Value::Object(_))) {
                return Err(self.lang.special_unready.first().cloned().unwrap_or_default().into());
            }
            let args = match &ending {
                Err(Fault::Thrown(value @ Value::Object(o))) => vec![Value::Class(o.class.clone()), value.clone(), Value::Trace(Rc::from(self.lang.special_unready.first().map_or("", String::as_str)))],
                _ => vec![Value::Null, Value::Null, Value::Null],
            };
            let method = self.special_method(&object, 34).ok_or_else(|| self.special_fault())?;
            let kept = self.data.len();
            let mut given = vec![object];
            given.extend(args);
            self.invoke(&method, given)?;
            let answer = self.drop_top()?;
            self.data.truncate(kept);
            if matches!(&ending, Err(Fault::Thrown(_))) && self.special_truth(&answer)? {
                self.data.truncate(depth);
                return Ok(Passage::Along(plan.after));
            }
            return ending.map(|end| match end {
                Passage::Along(at) if at == plan.body.1 => Passage::Along(plan.after),
                other => other,
            });
        }
        let mut ending = match ending {
            Ok(Passage::Along(at)) if at == plan.body.1 => match plan.otherwise {
                Some(span) => self.run_span(program, frame, instrs, span).map(|end| match end {
                    Passage::Along(at) if at == span.1 => Passage::Along(plan.after),
                    other => other,
                }),
                None => Ok(Passage::Along(plan.after)),
            },
            Err(Fault::Thrown(raised)) => {
                self.data.truncate(depth);
                self.caught.push(raised.clone());
                let handled = (|| {
                    for arm in &plan.clauses {
                        let mut takes = arm.bare;
                        for span in &arm.kinds {
                            self.run_span(program, frame, instrs, *span)?;
                            let kind = self.drop_top()?;
                            match kind {
                                Value::Blank => {},
                                Value::Class(class) => {
                                    if let Value::Object(object) = &raised {
                                        takes |= object.class.named(&class.name, self.lang.classes_folded);
                                    }
                                }
                                _ => return Err(self.lang.catch_invalid.as_deref().unwrap_or("A catch needs a class").into()),
                            }
                            if takes { break; }
                        }
                        if !takes { continue; }
                        self.under = None;
                        self.entering = None;
                        if let Some(slot) = &arm.held { self.store_cell(slot, frame, raised.clone())?; }
                        let outcome = self.run_span(program, frame, instrs, arm.body);
                        if let Some(slot) = &arm.held { self.put_cell(slot, frame, Value::Blank); }
                        return outcome.map(|end| match end {
                            Passage::Along(at) if at == arm.body.1 => Passage::Along(plan.after),
                            other => other,
                        });
                    }
                    Err(Fault::Thrown(raised))
                })();
                self.caught.truncate(active);
                handled
            }
            other => other,
        };
        if let Some(last) = plan.last {
            if let Err(Fault::Thrown(raised)) = &ending { self.caught.push(raised.clone()); }
            let saved = self.data.len();
            let finished = self.run_span(program, frame, instrs, last);
            self.caught.truncate(active);
            match finished {
                Ok(Passage::Along(at)) if at == last.1 => self.data.truncate(saved),
                Ok(end @ Passage::Leaves { cycle: None, .. }) => {
                    let returned = self.drop_top()?;
                    self.data.truncate(depth);
                    self.data.push(returned);
                    ending = Ok(end);
                }
                other => {
                    self.data.truncate(depth);
                    ending = other;
                }
            }
        }
        ending
    }

    fn close_generator(&mut self, held: &Rc<RefCell<Generator>>) -> Flow<()> {
        let mut state = held.try_borrow_mut().map_err(|_| self.lang.yield_busy[0].clone())?;
        if let Some(Value::Generator(inner)) = state.delegate.take() { self.close_generator(&inner)?; }
        state.closed = true;
        state.current = None;
        state.frame.clear();
        state.stack.clear();
        state.items.clear();
        Ok(())
    }

    fn iterator(&mut self, source: Value) -> Flow<Value> {
        if matches!(source, Value::Generator(_)) { return Ok(source); }
        let items = self.comprehension_items(&source)?;
        Ok(Value::Generator(Rc::new(RefCell::new(Generator::new(None, Vec::new(), items)))))
    }

    fn resume_generator(&mut self, held: &Rc<RefCell<Generator>>, sent: Value) -> Flow<Option<Value>> {
        let mut kept = held.try_borrow_mut().map_err(|_| self.lang.yield_busy[0].clone())?;
        if kept.closed { kept.returned = Value::Null; return Ok(None); }
        if !kept.started && !matches!(sent, Value::Null) {
            return Err(self.lang.yield_unstarted[0].clone().into());
        }
        if let Some(item) = kept.current.take() { return Ok(Some(item)); }
        kept.started = true;
        let Some(program) = kept.program.clone() else {
            if !matches!(sent, Value::Null) { return Err(self.lang.yield_unsupported[0].clone().into()); }
            let item = kept.items.get(kept.pc).cloned();
            kept.pc += usize::from(item.is_some());
            kept.closed = item.is_none();
            return Ok(item);
        };
        let outer = std::mem::replace(&mut self.data, std::mem::take(&mut kept.stack));
        let mut locals = std::mem::take(&mut kept.frame);
        if kept.waiting { self.data.push(sent.clone()); kept.waiting = false; }
        kept.sent = sent;
        let source = self.source.clone();
        let line = self.line;
        if let Some(place) = &program.written_in { self.source = place.clone(); }
        self.inside.push(program.within.clone());
        let result = self.run_portion(&program, &mut locals, &program.instrs, (0, program.instrs.len()), Some(&mut kept));
        self.inside.pop();
        self.source = source;
        self.line = line;
        kept.frame = locals;
        kept.stack = std::mem::replace(&mut self.data, outer);
        if kept.handed.is_none() || result.is_err() {
            kept.closed = true;
            kept.returned = if result.is_err() { Value::Null } else { kept.stack.pop().unwrap_or(Value::Null) };
            kept.stack.clear();
            kept.frame.clear();
        }
        result?;
        Ok(kept.handed.take())
    }

    fn run_span(&mut self, program: &Rc<Routine>, frame: &mut [Value], instrs: &[Instr], span: (usize, usize)) -> Flow<Passage> {
        self.run_portion(program, frame, instrs, span, None)
    }

    fn run_portion(&mut self, program: &Rc<Routine>, frame: &mut [Value], instrs: &[Instr], span: (usize, usize), mut suspended: Option<&mut Generator>) -> Flow<Passage> {
        let mut pc = suspended.as_ref().map_or(span.0, |g| g.pc);
        let mut counted = 0u32;
        // Where a raised value is caught, how deep the stack was when
        // the guard was set, and how much quiet was asked for then.
        let mut guards: Vec<(usize, usize, usize)> = Vec::new();
        while pc >= span.0 && pc < span.1 {
            // A complaint raised where the run could only read waits to
            // be handed over; here, before the next word, is where the
            // run can reach back into the program to hand it on.
            if self.any_waiting.get() {
                if let Err(fault) = self.hand_over_complaints() {
                    // A routine in the way of a complaint may raise
                    // something of its own, which a guard here takes as
                    // it would take any other.
                    let Fault::Thrown(raised) = fault else { return Err(fault) };
                    let Some((catch, depth, quiet)) = guards.pop() else { return Err(Fault::Thrown(raised)) };
                    self.under = None;
                    self.entering = None;
                    self.hushed.set(quiet);
                    self.data.truncate(depth);
                    self.data.push(raised);
                    pc = catch;
                    continue;
                }
            }
            counted = counted.wrapping_add(1);
            if counted % 4096 == 0 {
                if let Some(over) = self.out_of_time() {
                    return Err(over);
                }
                if let Some(over) = self.out_of_room() {
                    return Err(over);
                }
            }
            match &instrs[pc] {
                Instr::Const(Value::Routine(program)) if self.lang.closes_over && !program.enclosing.is_empty() => {
                    let mut closed = (**program).clone();
                    for (at, source) in &program.enclosing {
                        let shared = self.share_cell(source, frame)?;
                        closed.enclosed.push((*at, if self.lang.bind_names { Value::Binding(shared) } else { Value::Bond(shared) }));
                    }
                    self.data.push(Value::Routine(Rc::new(closed)));
                }
                Instr::Const(v) => self.data.push(self.keep_collection(v.clone())),
                Instr::Read(slot) => match self.load_cell(slot, frame) {
                    Ok(v) => self.data.push(v),
                    Err(told) => match self.offer_to_guard(&told, &mut guards) {
                        Some(catch) => {
                            pc = catch;
                            continue;
                        }
                        None => return Err(told.into()),
                    },
                },
                Instr::Glance(slot) => {
                    let held = slot
                        .near
                        .iter()
                        .map(|&s| match &frame[s] {
                            Value::Binding(cell) if self.lang.closes_over => cell.borrow().clone(),
                            value => value.clone(),
                        })
                        .find(|v| !matches!(v, Value::Blank))
                        .unwrap_or_else(|| self.world[slot.far].clone());
                    self.data.push(match held {
                        Value::Bond(shared) if !self.lang.bind_names => shared.borrow().clone(),
                        other => other,
                    });
                }
                Instr::Write(slot) => {
                    let v = self.drop_top()?;
                    self.store_cell(slot, frame, v)?;
                }
                Instr::Act(Action::Suspend | Action::Delegate, _) => {
                    let kept = suspended.as_deref_mut().ok_or_else(|| self.lang.yield_unsupported[0].clone())?;
                    if matches!(&instrs[pc], Instr::Act(Action::Suspend, _)) {
                        kept.handed = Some(self.drop_top()?);
                        kept.pc = pc + 1;
                        kept.waiting = true;
                    } else {
                        if kept.delegate.is_none() {
                            let source = self.drop_top()?;
                            kept.delegate = Some(self.iterator(source)?);
                            kept.sent = Value::Null;
                        }
                        let Value::Generator(inner) = kept.delegate.as_ref().expect("delegated walk").clone() else { unreachable!() };
                        match self.resume_generator(&inner, std::mem::replace(&mut kept.sent, Value::Null))? {
                            Some(item) => { kept.handed = Some(item); kept.pc = pc; }
                            None => {
                                self.data.push(inner.borrow().returned.clone());
                                kept.delegate = None;
                                pc += 1;
                                continue;
                            }
                        }
                    }
                    return Ok(Passage::Along(pc));
                }
                Instr::Act(op, argc) => {
                    // Text read while the run goes is read where it
                    // stands: inside a routine it sees that routine's
                    // names, as the reference has it, and only the
                    // outermost body has none but the globals.
                    let done = match op {
                        Action::Builtin(Builtin::Eval, _) if !program.body_of_all && *argc == 1 => self.run_text_here(program, frame),
                        // What a call was given is read back as it
                        // stands now: a parameter written to since holds
                        // what was written, and one handed a cell reads
                        // as what the cell holds.
                        Action::Builtin(Builtin::Given | Builtin::GivenCount | Builtin::GivenAt, _) if !program.body_of_all => {
                            self.as_they_stand(program, frame);
                            self.perform(op, *argc)
                        }
                        _ => self.perform(op, *argc),
                    };
                    if let Err(fault) = done {
                        // A language that names a class for the kernel's
                        // own faults has them raised as one of that
                        // class, so a program may take them like any
                        // other raised value.
                        let fault = match fault {
                            Fault::Note(told) if !guards.is_empty() => match self.as_fault(&told) {
                                Some(made) => Fault::Thrown(made),
                                None => Fault::Note(told),
                            },
                            other => other,
                        };
                        let Fault::Thrown(raised) = fault else { return Err(fault) };
                        let Some((catch, depth, quiet)) = guards.pop() else { return Err(Fault::Thrown(raised)) };
                        self.under = None;
                        self.entering = None;
                        self.hushed.set(quiet);
                        self.data.truncate(depth);
                        self.data.push(raised);
                        pc = catch;
                        continue;
                    }
                }
                Instr::Attempt(plan) => {
                    if suspended.is_some() && instrs[plan.body.0..plan.after].iter().any(|i| matches!(i, Instr::Act(Action::Suspend | Action::Delegate, _))) {
                        return Err(self.lang.yield_unsupported[0].clone().into());
                    }
                    match self.run_attempt(program, frame, instrs, plan)? {
                        Passage::Along(at) => pc = at,
                        end @ Passage::Leaves { to, cycle } => {
                            if !cycle.map_or(false, |i| i >= span.0 && i < span.1) { return Ok(end); }
                            pc = to;
                        }
                    }
                    continue;
                }
                Instr::Depart { to, cycle } => {
                    if !cycle.map_or(false, |i| i >= span.0 && i < span.1) {
                        return Ok(Passage::Leaves { to: *to, cycle: *cycle });
                    }
                    pc = *to;
                    continue;
                }
                Instr::Guard(catch) => guards.push((*catch, self.data.len(), self.hushed.get())),
                Instr::Unguard => {
                    guards.pop();
                }
                Instr::Bond(slot) => {
                    // The binding becomes a shared cell, so that another
                    // name fastened to it sees the same writes.
                    let shared = self.share_cell(slot, frame)?;
                    self.data.push(Value::Bond(shared));
                }
                Instr::Fasten(slot) => {
                    // What has no cell to share is simply written, as a
                    // language that asks to share one from something
                    // that has none does rather than stopping.
                    match self.drop_top()? {
                        Value::Bond(shared) => self.put_cell(slot, frame, Value::Bond(shared)),
                        held => self.store_cell(slot, frame, held)?,
                    }
                }
                Instr::BondItem(slot) => {
                    let at = self.drop_top()?;
                    let held = self.peek_cell_mut(slot, frame)?;
                    // A name standing for a shared cell is walked
                    // through that cell, since the array lives inside it.
                    let shared = match held {
                        Value::Bond(cell) => {
                            let cell = cell.clone();
                            let mut inside = cell.borrow_mut();
                            shared_item(&mut inside, &at)?
                        }
                        _ => shared_item(held, &at)?,
                    };
                    self.data.push(Value::Bond(shared));
                }
                Instr::BondPlace(slot, count) => {
                    let mut keys = self.drop_many(*count)?;
                    for k in keys.iter_mut() {
                        *k = self.key(k);
                    }
                    let makes = self.lang.makes_places;
                    let held = self.peek_cell_mut(slot, frame)?;
                    let shared = match held {
                        Value::Bond(cell) => {
                            let cell = cell.clone();
                            let mut inside = cell.borrow_mut();
                            shared_deep(&mut inside, &keys, makes)?
                        }
                        _ => shared_deep(held, &keys, makes)?,
                    };
                    self.data.push(Value::Bond(shared));
                }
                Instr::Shed => {
                    self.drop_top()?;
                }
                Instr::Emptied(slot) => {
                    self.store_cell(slot, frame, Value::Null)?;
                }
                Instr::Nothing => {}
                Instr::Forget(slot) => {
                    for &s in &slot.near {
                        if self.lang.closes_over {
                            if let Value::Binding(cell) = &frame[s] { *cell.borrow_mut() = Value::Blank; continue; }
                        }
                        frame[s] = Value::Blank;
                    }
                    if slot.near.is_empty() {
                        self.world[slot.far] = Value::Blank;
                    }
                }
                Instr::Ready(slot) => {
                    self.world.resize(self.registry.idents.len(), Value::Blank);
                    if matches!(self.world[slot.far], Value::Blank) {
                        self.world[slot.far] = Value::Null;
                    }
                }
                Instr::Missing(slot) => {
                    let empty = match &frame[*slot] {
                        Value::Binding(cell) if self.lang.closes_over => matches!(*cell.borrow(), Value::Blank),
                        value => matches!(value, Value::Blank),
                    };
                    self.data.push(Value::Flag(empty));
                }
                Instr::Unwritten(at) => {
                    let empty = matches!(self.world[*at], Value::Blank);
                    self.data.push(Value::Flag(empty));
                }
                Instr::Line(row) => {
                    self.line = *row;
                    // A statement reached is a fault gone by: the calls
                    // an earlier one was raised under are none of its
                    // business.
                    self.under = None;
                    self.entering = None;
                }
                Instr::Mute(quiet) => {
                    let deep = self.muted.get();
                    self.muted.set(match quiet {
                        true => deep + 1,
                        false => deep.saturating_sub(1),
                    });
                }
                Instr::Hush(quiet) => {
                    let deep = self.hushed.get();
                    self.hushed.set(match quiet {
                        true => deep + 1,
                        false => deep.saturating_sub(1),
                    });
                }
                Instr::Skip(to) => {
                    let held = self.drop_top()?;
                    if !self.special_truth(&held)? {
                        pc = *to;
                        continue;
                    }
                }
                Instr::SkipCmp { op, a, b, to } => {
                    let bt = self.operand_value(b, frame)?;
                    let at = self.operand_value(a, frame)?;
                    let (av, bv) = self.both(a, b, &at, &bt, frame)?;
                    let (av, bv) = (av.clone(), bv.clone());
                    let (av, bv) = (&av, &bv);
                    let holds = match (av, bv) {
                        (Value::Small(x), Value::Small(y)) => match op {
                            Action::Lt => x < y,
                            Action::Le => x <= y,
                            Action::Gt => x > y,
                            Action::Ge => x >= y,
                            Action::Eq => x == y,
                            _ => x != y,
                        },
                        _ => {
                            let told = self.special_dyad(op, av, bv)?;
                            self.truth(&told)
                        }
                    };
                    if !holds {
                        pc = *to;
                        continue;
                    }
                }
                Instr::Dyad { op, a, b } => {
                    let bt = self.operand_value(b, frame)?;
                    let at = self.operand_value(a, frame)?;
                    let (av, bv) = self.both(a, b, &at, &bt, frame)?;
                    let (av, bv) = (av.clone(), bv.clone());
                    let (av, bv) = (&av, &bv);
                    let fast = match (av, bv) {
                        (Value::Small(x), Value::Small(y)) => match op {
                            Action::Add => x.checked_add(*y).map(Value::Small),
                            Action::Sub => x.checked_sub(*y).map(Value::Small),
                            Action::Mul => x.checked_mul(*y).map(Value::Small),
                            Action::Lt => Some(Value::Flag(x < y)),
                            Action::Le => Some(Value::Flag(x <= y)),
                            Action::Gt => Some(Value::Flag(x > y)),
                            Action::Ge => Some(Value::Flag(x >= y)),
                            Action::Eq => Some(Value::Flag(x == y)),
                            Action::Ne => Some(Value::Flag(x != y)),
                            Action::Mod if *y != 0 => x.checked_rem(*y).map(Value::Small),
                            Action::IntDiv if *y != 0 => x.checked_div(*y).map(Value::Small),
                            _ => None,
                        },
                        _ => None,
                    };
                    let r = match fast {
                        Some(v) => v,
                        None => self.special_dyad(op, av, bv)?,
                    };
                    self.data.push(r);
                }
                Instr::Bump { slot, by } => {
                    // A shared cell holds its value elsewhere, so it is
                    // read and written the long way.
                    if self.shares_cell(slot, frame) {
                        let held = self.load_cell(slot, frame)?;
                        let sum = self.special_dyad(&Action::Add, &held, by)?;
                        self.store_cell(slot, frame, sum)?;
                        pc += 1;
                        continue;
                    }
                    let cell = self.peek_cell_mut(slot, frame)?;
                    let fast = match (&*cell, by) {
                        (Value::Small(x), Value::Small(k)) => x.checked_add(*k),
                        _ => None,
                    };
                    match fast {
                        Some(sum) => *cell = Value::Small(sum),
                        None => {
                            let v = cell.clone();
                            let r = self.special_dyad(&Action::Add, &v, by)?;
                            self.store_cell(slot, frame, r)?;
                        }
                    }
                }
            }
            pc += 1;
        }
        Ok(Passage::Along(pc))
    }

    fn descriptor_of(&self, subject: &Value, name: &str) -> Option<Rc<Descriptor>> {
        let class = match subject {
            Value::Class(c) => c,
            Value::Object(o) => &o.class,
            _ => return None,
        };
        let owner = class.holder(name)?;
        let held = owner.shared.borrow();
        held.iter().find_map(|(n, v)| match v {
            Value::Descriptor(d) if n == name => {
                if !matches!(d.as_ref(), Descriptor::Property(..)) {
                    if let Value::Object(o) = subject {
                        if self.member_at(&o.fields.borrow(), name).is_some() { return None; }
                    }
                }
                Some(d.clone())
            },
            _ => None,
        })
    }

    fn descriptor_read(&mut self, d: &Rc<Descriptor>, subject: Value) -> Flow<Value> {
        match d.as_ref() {
            Descriptor::Static(f) => Ok(f.clone()),
            Descriptor::Class(f) => {
                let class = match subject { Value::Object(o) => Value::Class(o.class.clone()), other => other };
                Ok(Value::Descriptor(Rc::new(Descriptor::Bound(f.clone(), class))))
            }
            Descriptor::Property(get, _) if matches!(subject, Value::Object(_)) => {
                self.data.push(subject);
                self.data.push(get.clone());
                self.perform(&Action::Invoke(Rc::from("property")), 2)?;
                Ok(self.drop_top()?)
            }
            _ => Ok(Value::Descriptor(d.clone())),
        }
    }

    fn special_key(&mut self, key: &Value) -> Res<Value> {
        if matches!(key, Value::Object(_)) && !self.lang.class_special.is_empty() {
            let hash = self.special_builtin(Builtin::Hash, std::slice::from_ref(key))?.ok_or_else(|| self.special_fault())?;
            return Ok(Value::Hashed(Rc::new((key.clone(), hash))));
        }
        Ok(key.clone())
    }

    fn special_keys_equal(&mut self, a: &Value, b: &Value) -> Res<bool> {
        let (left, right) = match (a, b) {
            (Value::Hashed(x), Value::Hashed(y)) => {
                if !x.1.equals(&y.1) { return Ok(false); }
                (&x.0, &y.0)
            }
            (Value::Hashed(x), y) | (y, Value::Hashed(x)) => {
                if !matches!(y, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) || !x.1.equals(y) { return Ok(false); }
                (&x.0, y)
            }
            _ => return Ok(a.equals(b)),
        };
        if matches!((left, right), (Value::Object(x), Value::Object(y)) if Rc::ptr_eq(x, y)) { return Ok(true); }
        let equal = self.special_dyad(&Action::Eq, left, right)?;
        self.special_truth(&equal)
    }

    fn special_value(&self, value: &Value, place: usize) -> Option<Value> {
        let Value::Object(object) = value else { return None };
        let named = self.lang.class_special.get(place)?;
        let mut class = Some(&object.class);
        while let Some(current) = class {
            if let Some((_, value)) = current.shared.borrow().iter().find(|(n, _)| n == named) { return Some(value.clone()); }
            if let Some((_, routine)) = current.methods.iter().find(|(n, _)| n == named) { return Some(Value::Routine(routine.clone())); }
            if place == 8 && self.lang.class_special.get(2).map_or(false, |eq| current.methods.iter().any(|(n, _)| n == eq)) { return Some(Value::Null); }
            class = current.base.as_ref();
        }
        None
    }

    fn special_method(&self, value: &Value, place: usize) -> Option<Rc<Routine>> {
        match self.special_value(value, place) { Some(Value::Routine(routine)) => Some(routine), _ => None }
    }

    fn special_fault(&self) -> String {
        self.lang.special_amiss.first().cloned().unwrap_or_default()
    }

    fn special_call(&mut self, value: &Value, place: usize, args: Vec<Value>) -> Res<Option<Value>> {
        if place == 3 && self.special_value(value, place).is_none() {
            return match self.special_call(value, 2, args)? {
                Some(v @ Value::Declined(_)) => Ok(Some(v)),
                Some(v) => Ok(Some(Value::Flag(!self.special_truth(&v)?))),
                None => Ok(None),
            };
        }
        let method = match self.special_value(value, place) {
            None => return Ok(None), Some(Value::Routine(routine)) => routine,
            _ => return Err(self.special_fault()),
        };
        let mut given = vec![value.clone()];
        given.extend(args);
        match self.invoke(&method, given) {
            Ok(()) => Ok(Some(self.drop_top().map_err(|_| self.special_fault())?)),
            Err(Fault::Note(words)) => Err(words),
            Err(other) => {
                self.carried = Some(other);
                Err(self.special_fault())
            }
        }
    }

    fn holds_object(value: &Value) -> bool {
        match value {
            Value::Trace(_) | Value::Hashed(_) | Value::Fields(_) | Value::Object(_) => true,
            Value::Bond(c) | Value::Binding(c) | Value::Collection(c, _) => Self::holds_object(&c.borrow()),
            Value::Tuple(items) | Value::Array(items) => items.iter().any(Self::holds_object),
            Value::Map(items) => items.iter().any(|(k, v)| Self::holds_object(k) || Self::holds_object(v)),
            _ => false,
        }
    }

    fn special_text(&mut self, value: &Value, representation: bool) -> Res<String> {
        if let Value::Binding(cell) = value { return self.special_text(&cell.borrow(), representation); }
        if matches!(value, Value::Bond(_) | Value::Collection(..)) { return self.special_text(&value.contents(), representation); }
        if let Value::Trace(words) = value { return Err(words.to_string()); }
        if self.lang.class_special.is_empty() || (!representation && !Self::holds_object(value)) { return Ok(self.render(std::slice::from_ref(value))); }
        if let Value::Object(object) = value {
            let place = if representation || self.special_value(value, 0).is_none() { 1 } else { 0 };
            return match self.special_call(value, place, Vec::new())? {
                Some(Value::Text(text)) => Ok(text.to_string()),
                Some(_) => Err(self.special_fault()),
                None => Ok(format!("<{} object>", object.class.name)),
            };
        }
        match value {
            Value::Hashed(pair) => self.special_text(&pair.0, representation),
            Value::Fields(object) => {
                let entries = object.fields.borrow().iter().filter(|(_, v)| !matches!(v, Value::Blank)).map(|(k, v)| (Value::text(k), v.clone())).collect();
                self.special_text(&Value::Map(Rc::new(entries)), true)
            }
            Value::Array(items) => {
                let mut parts = Vec::new();
                for item in items.iter() { parts.push(self.special_text(item, true)?); }
                Ok(format!("[{}]", parts.join(", ")))
            }
            Value::Map(items) => {
                let mut parts = Vec::new();
                for (key, item) in items.iter() {
                    parts.push(format!("{}: {}", self.special_text(key, true)?, self.special_text(item, true)?));
                }
                Ok(format!("{{{}}}", parts.join(", ")))
            }
            Value::Text(_) if representation => self.rem_repr(value),
            _ => Ok(value.display(&self.wording())),
        }
    }

    fn special_truth(&mut self, value: &Value) -> Res<bool> {
        if let Value::Fields(o) = value { return Ok(o.fields.borrow().iter().any(|(_, v)| !matches!(v, Value::Blank))); }
        if matches!(value, Value::Declined(_)) { return Err(self.lang.special_unready.first().cloned().unwrap_or_default()); }
        if let Some(answer) = self.special_call(value, 9, Vec::new())? {
            return match answer { Value::Flag(flag) => Ok(flag), _ => Err(self.special_fault()) };
        }
        if let Some(answer) = self.special_call(value, 10, Vec::new())? {
            return match answer {
                Value::Small(n) if n >= 0 => Ok(n != 0),
                Value::Huge(n) if *n >= BigInt::from(0) => Ok(*n != BigInt::from(0)),
                _ => Err(self.special_fault()),
            };
        }
        Ok(self.truth(value))
    }

    fn special_dyad(&mut self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        if matches!(a, Value::Collection(..) | Value::Bond(_)) || matches!(b, Value::Collection(..) | Value::Bond(_)) { return self.special_dyad(op, &a.contents(), &b.contents()); }
        if self.lang.class_special.is_empty() { return self.dyadic(op, a, b); }
        if let Value::Fields(o) = a {
            let entries = o.fields.borrow().iter().filter(|(_, v)| !matches!(v, Value::Blank)).map(|(k, v)| (Value::text(k), v.clone())).collect();
            return self.special_dyad(op, &Value::Map(Rc::new(entries)), b);
        }
        let places = match op {
            Action::Eq => Some((2, 2)), Action::Ne => Some((3, 3)),
            Action::Lt => Some((4, 6)), Action::Le => Some((5, 7)),
            Action::Gt => Some((6, 4)), Action::Ge => Some((7, 5)),
            Action::Add => Some((18, 26)), Action::Sub => Some((19, 27)),
            Action::Mul => Some((20, 28)), Action::Div | Action::DivReal => Some((21, 29)),
            Action::IntDiv => Some((22, 30)), Action::Mod => Some((23, 31)),
            Action::Power => Some((24, 32)),
            Action::At | Action::Apart | Action::Toward => Some((11, usize::MAX)),
            _ => None,
        };
        if let Some((direct, reflected)) = places {
            let first_right = match (a, b) {
                (Value::Object(left), Value::Object(right)) if left.class.name != right.class.name && right.class.named(&left.class.name, false) => {
                    if direct < 8 { true } else {
                        match (self.special_method(a, reflected), self.special_method(b, reflected)) {
                            (Some(one), Some(two)) => !Rc::ptr_eq(&one, &two),
                            (None, Some(_)) => true,
                            _ => false,
                        }
                    }
                }
                _ => false,
            };
            if first_right {
                if let Some(answer) = self.special_call(b, reflected, vec![a.clone()])? {
                    if !matches!(answer, Value::Declined(_)) { return Ok(answer); }
                }
            }
            if let Some(answer) = self.special_call(a, direct, vec![b.clone()])? { if !matches!(answer, Value::Declined(_)) { return Ok(answer); } }
            let same_class = matches!((a, b), (Value::Object(x), Value::Object(y)) if Rc::ptr_eq(&x.class, &y.class));
            if !first_right && (direct < 8 || !same_class) {
                if let Some(answer) = self.special_call(b, reflected, vec![a.clone()])? { if !matches!(answer, Value::Declined(_)) { return Ok(answer); } }
            }

        }
        if matches!(op, Action::Contains | Action::Lacks) {
            if let Value::Map(entries) = b {
                let wanted = self.special_key(a)?;
                let mut found = false;
                for (key, _) in entries.iter() { if self.special_keys_equal(key, &wanted)? { found = true; break; } }
                return Ok(Value::Flag(found != matches!(op, Action::Lacks)));
            }
            if let Some(answer) = self.special_call(b, 14, vec![a.clone()])? {
                return Ok(Value::Flag(self.special_truth(&answer)? != matches!(op, Action::Lacks)));
            }
        }
        if let (Action::At, Value::Map(entries)) = (op, a) {
            let wanted = self.special_key(b)?;
            for (key, value) in entries.iter() {
                if self.special_keys_equal(key, &wanted)? { return Ok(value.clone()); }
            }
        }
        self.dyadic(op, a, b)
    }

    fn special_builtin(&mut self, op: Builtin, args: &[Value]) -> Res<Option<Value>> {
        if self.lang.class_special.is_empty() { return Ok(None); }
        let first = args.first();
        if Self::core_builtin(op) && !args.iter().any(|v| Self::holds_object(v) || matches!(v, Value::Walk(_))) { return Ok(None); }
        if op == Builtin::InstanceOf { return Ok(None); }
        let answer = match op {
            Builtin::Repr if args.len() == 1 => Value::text(&self.special_text(&args[0], true)?),
            Builtin::ToText if args.len() == 1 => Value::text(&self.special_text(&args[0], false)?),
            Builtin::Bool if args.len() <= 1 => Value::Flag(match first { Some(v) => self.special_truth(v)?, None => false }),
            Builtin::Replace if args.len() == 3 && matches!(&args[2], Value::Map(_)) => {
                let Value::Map(entries) = &args[2] else { unreachable!() };
                let mut entries = entries.as_ref().clone();
                let key = self.special_key(&args[0])?;
                let mut found = None;
                for (at, (old, _)) in entries.iter().enumerate() { if self.special_keys_equal(old, &key)? { found = Some(at); break; } }
                if let Some(at) = found { entries[at].1 = args[1].clone(); } else { entries.push((key, args[1].clone())); }
                Value::Map(Rc::new(entries))
            }
            Builtin::Erase if args.len() == 2 && matches!(&args[0], Value::Map(_)) => {
                let Value::Map(entries) = &args[0] else { unreachable!() };
                let key = self.special_key(&args[1])?;
                let mut kept = Vec::new();
                let mut found = false;
                for (old, value) in entries.iter() {
                    if self.special_keys_equal(old, &key)? { found = true; } else { kept.push((old.clone(), value.clone())); }
                }
                if !found { return Err(self.lang.del_unrun.clone()); }
                Value::Map(Rc::new(kept))
            }
            Builtin::Fetch if args.len() == 2 => self.special_dyad(&Action::At, &args[0], &args[1])?,
            Builtin::Replace if args.len() == 3 && matches!(&args[2], Value::Fields(_)) => {
                let (Value::Fields(o), Value::Text(key)) = (&args[2], &args[0]) else { return Err(self.special_fault()) };
                let mut fields = o.fields.borrow_mut();
                if let Some((_, v)) = fields.iter_mut().find(|(k, _)| k == key.as_ref()) { *v = args[1].clone(); }
                else { fields.push((key.to_string(), args[1].clone())); }
                args[2].clone()
            }
            Builtin::Length if args.len() == 1 && matches!(&args[0], Value::Fields(_)) => {
                let Value::Fields(o) = &args[0] else { unreachable!() };
                Value::Small(o.fields.borrow().iter().filter(|(_, v)| !matches!(v, Value::Blank)).count() as i64)
            }
            Builtin::Replace if args.len() == 3 && self.special_method(&args[2], 12).is_some() => {
                self.special_call(&args[2], 12, vec![args[0].clone(), args[1].clone()])?;
                args[2].clone()
            }
            Builtin::Length if args.len() == 1 && self.special_method(&args[0], 10).is_some() => {
                let answer = self.special_call(&args[0], 10, Vec::new())?.unwrap();
                match answer {
                    Value::Small(n) if n >= 0 => Value::Small(n),
                    Value::Huge(ref n) if **n >= BigInt::from(0) => answer,
                    _ => return Err(self.special_fault()),
                }
            }
            Builtin::Hash if args.len() == 1 => {
                if let Some(answer) = self.special_call(&args[0], 8, Vec::new())? {
                    if !matches!(answer, Value::Small(_) | Value::Huge(_)) { return Err(self.special_fault()); }
                    answer
                } else {
                    match &args[0] {
                        Value::Object(object) if self.special_method(&args[0], 2).is_none() => Value::Small(object.mark as i64),
                        Value::Small(_) | Value::Huge(_) => args[0].clone(),
                        Value::Flag(flag) => Value::Small(i64::from(*flag)),
                        _ => return Err(self.special_fault()),
                    }
                }
            }
            Builtin::Sorted if args.len() == 1 => {
                let mut items = self.special_items(&args[0])?;
                for next in 1..items.len() {
                    let mut at = next;
                    while at > 0 {
                        let less = self.special_dyad(&Action::Lt, &items[at], &items[at - 1])?;
                        if !self.special_truth(&less)? { break; }
                        items.swap(at, at - 1);
                        at -= 1;
                    }
                }
                Value::array(items)
            }
            Builtin::List if args.len() == 1 => Value::array(self.special_items(&args[0])?),
            Builtin::Iter if args.len() == 1 => {
                if matches!(&args[0], Value::Walk(_)) { return Ok(Some(args[0].clone())); }
                if let Some(answer) = self.special_call(&args[0], 15, Vec::new())? { answer }
                else { Value::Walk(Rc::new(RefCell::new((self.comprehension_items(&args[0])?, 0)))) }
            }
            Builtin::Next if args.len() == 2 => {
                self.special_step(&args[0])?.unwrap_or_else(|| args[1].clone())
            }
            Builtin::Next if args.len() == 1 => {
                if let Value::Walk(walk) = &args[0] {
                    let mut walk = walk.borrow_mut();
                    let Some(value) = walk.0.get(walk.1).cloned() else {
                        let class = Class { name: self.lang.special_stop.first().cloned().unwrap_or_default(), base: None, answers: Vec::new(), fields: Vec::new(), reaches: Vec::new(), methods: Vec::new(), constants: Vec::new(), shared: RefCell::new(Vec::new()) };
                        self.made += 1;
                        self.carried = Some(Fault::Thrown(Value::Object(Rc::new(Instance { class: Rc::new(class), fields: RefCell::new(Vec::new()), mark: self.made }))));
                        return Err(self.lang.special_stop.first().cloned().unwrap_or_default());
                    };
                    walk.1 += 1;
                    value
                } else { self.special_call(&args[0], 16, Vec::new())?.ok_or_else(|| self.special_fault())? }
            }
            Builtin::InstanceOf if args.len() == 2 => {
                let Value::Class(class) = &args[1] else { return Err(self.special_fault()) };
                Value::Flag(matches!(&args[0], Value::Object(o) if o.class.named(&class.name, false)))
            }
            Builtin::Repr | Builtin::Hash | Builtin::Bool | Builtin::Sorted | Builtin::Iter | Builtin::Next | Builtin::InstanceOf => return Err(self.special_fault()),
            _ => return Ok(None),
        };
        Ok(Some(answer))
    }

    fn special_step(&mut self, value: &Value) -> Res<Option<Value>> {
        if matches!(value, Value::Cursor(_) | Value::Generator(_)) { return self.core_step(value); }
        if let Value::Walk(walk) = value {
            let mut walk = walk.borrow_mut();
            let next = walk.0.get(walk.1).cloned();
            if next.is_some() { walk.1 += 1; }
            return Ok(next);
        }
        let method = self.special_method(value, 16).ok_or_else(|| self.special_fault())?;
        match self.invoke(&method, vec![value.clone()]) {
            Ok(()) => Ok(Some(self.drop_top().map_err(|_| self.special_fault())?)),
            Err(Fault::Thrown(Value::Object(o))) if self.lang.special_stop.iter().any(|n| o.class.named(n, false)) => Ok(None),
            Err(Fault::Note(words)) => Err(words),
            Err(fault) => { self.carried = Some(fault); Err(self.special_fault()) }
        }
    }

    fn special_items(&mut self, value: &Value) -> Res<Vec<Value>> {
        if let Value::Fields(o) = value {
            return Ok(o.fields.borrow().iter().filter(|(_, v)| !matches!(v, Value::Blank)).map(|(k, _)| Value::text(k)).collect());
        }
        if let Value::Walk(walk) = value {
            let mut walk = walk.borrow_mut();
            let tail = walk.0[walk.1..].to_vec();
            walk.1 = walk.0.len();
            return Ok(tail);
        }
        if let Some(iterator) = self.special_call(value, 15, Vec::new())? {
            let mut items = Vec::new();
            while let Some(item) = self.special_step(&iterator)? { items.push(item); }
            return Ok(items);
        }
        self.comprehension_items(value)
    }

    fn perform(&mut self, op: &Action, argc: usize) -> Flow<()> {
        if self.lang.bind_names && matches!(op, Action::Extent | Action::KeyAt | Action::ValueAt
            | Action::WalkFrom | Action::WalkAlone | Action::WalkMore | Action::WalkThis
            | Action::WalkKey | Action::WalkOnward) {
            let at = self.data.len().saturating_sub(argc);
            if let Some(value) = self.data.get_mut(at) { *value = collection_contents(value); }
        }
        let result = match op {
            Action::Match(pattern, names, tuple) => {
                let subject = self.drop_top()?;
                let mut bindings = Vec::new();
                match pattern.fit(&subject, &mut bindings, *tuple) {
                    Err(()) => return Err(self.lang.match_unready.first().cloned().unwrap_or_default().into()),
                    Ok(false) => Value::Null,
                    Ok(true) => Value::Array(Rc::new(names.iter().map(|name| {
                        bindings.iter().find(|(n, _)| n == name).expect("a pattern binding").1.clone()
                    }).collect())),
                }
            }
            Action::KeepPoint => self.drop_top()?.with_point(true),
            Action::BindValueMethod(operation) => {
                let target = self.drop_top()?.held(false);
                Value::ValueMethod(Rc::new((target, operation.to_string())))
            }
            Action::Not => {
                let held = self.drop_top()?;
                Value::Flag(!self.special_truth(&held)?)
            }
            Action::AsBool => {
                let held = self.drop_top()?;
                Value::Flag(self.special_truth(&held)?)
            }
            // A name worked out while the run goes stands for the
            // binding of that name among the outermost ones, since only
            // those have names the run can still see.
            // The cell of the outermost binding a value names, so that a
            // name may be fastened to it, one handed over, or one given
            // back: a binding named as the run goes has a cell like any
            // other, and this is how it is asked for.
            Action::BondNamed => {
                let spelled = self.drop_top()?;
                let name = self.name_spelled(&spelled);
                let at = self.registry.slot(&name);
                self.world.resize(self.registry.idents.len(), Value::Blank);
                if let Value::Bond(shared) = &self.world[at] {
                    Value::Bond(shared.clone())
                } else {
                    let held = match std::mem::replace(&mut self.world[at], Value::Null) {
                        Value::Blank => Value::Null,
                        other => other,
                    };
                    let shared = Rc::new(RefCell::new(held));
                    self.world[at] = Value::Bond(shared.clone());
                    Value::Bond(shared)
                }
            }
            Action::Named => {
                let spelled = self.drop_top()?;
                let name = self.name_spelled(&spelled);
                let at = self.registry.slot(&name);
                self.world.resize(self.registry.idents.len(), Value::Blank);
                match &self.world[at] {
                    Value::Bond(shared) => shared.borrow().clone(),
                    Value::Blank if self.warns_about(&name) => {
                        self.complain(Complaint::Warning, &format!("Undefined variable {}", name));
                        Value::Null
                    }
                    Value::Blank => return Err(format!("Undefined variable: {}", name).into()),
                    held => held.clone(),
                }
            }
            Action::WriteNamed => {
                let value = self.drop_top()?;
                let spelled = self.drop_top()?;
                let name = self.name_spelled(&spelled);
                let at = self.registry.slot(&name);
                self.world.resize(self.registry.idents.len(), Value::Blank);
                match &self.world[at] {
                    Value::Bond(shared) => *shared.borrow_mut() = value,
                    _ => self.world[at] = value,
                }
                Value::Null
            }
            Action::ForgetNamed => {
                let spelled = self.drop_top()?;
                let name = self.name_spelled(&spelled);
                let at = self.registry.slot(&name);
                self.world.resize(self.registry.idents.len(), Value::Blank);
                self.world[at] = Value::Blank;
                Value::Null
            }
            Action::ReadyNamed => {
                let spelled = self.drop_top()?;
                let name = self.name_spelled(&spelled);
                let at = self.registry.slot(&name);
                self.world.resize(self.registry.idents.len(), Value::Blank);
                if matches!(self.world[at], Value::Blank) {
                    self.world[at] = Value::Null;
                }
                Value::Null
            }
            Action::BondWithin(count) => {
                let mut keys = self.drop_many(*count)?;
                for k in keys.iter_mut() {
                    *k = self.key(k);
                }
                let holder = self.drop_top()?;
                let makes = self.lang.makes_places;
                match holder {
                    Value::Bond(cell) => {
                        let shared = {
                            let mut inside = cell.borrow_mut();
                            shared_deep(&mut inside, &keys, makes)?
                        };
                        Value::Bond(shared)
                    }
                    _ => return Err("Cannot take a cell from a place in something that is not an array".into()),
                }
            }
            Action::BondOwn(name) => {
                let stands = {
                    let top = self.drop_top()?;
                    self.class_it_spells(top)
                };
                match stands {
                    Value::Class(c) => {
                        let holder = c.holder(&name).unwrap_or(&c);
                        let mut own = holder.shared.borrow_mut();
                        let at = match own.iter().position(|(n, _)| n == name.as_ref()) {
                            Some(at) => at,
                            None => {
                                own.push((name.to_string(), Value::Null));
                                own.len() - 1
                            }
                        };
                        if let Value::Bond(shared) = &own[at].1 {
                            let shared = shared.clone();
                            drop(own);
                            Value::Bond(shared)
                        } else {
                            let was = std::mem::replace(&mut own[at].1, Value::Null);
                            let shared = Rc::new(RefCell::new(was));
                            own[at].1 = Value::Bond(shared.clone());
                            drop(own);
                            Value::Bond(shared)
                        }
                    }
                    v => {
                        let told = self.no_such_class(&v).unwrap_or_else(|| format!("Cannot reach '{}' in {}", name, v.plain()));
                        return Err(told.into());
                    }
                }
            }
            Action::ForgetWithin => {
                let named = self.drop_top()?;
                let holder = self.data.last().cloned().ok_or_else(|| self.special_fault())?;
                let target = match &holder { Value::Bond(cell) => cell.borrow().clone(), other => other.clone() };
                if let Value::Fields(o) = &target {
                    let Value::Text(key) = named else { return Err(self.special_fault().into()) };
                    let mut fields = o.fields.borrow_mut();
                    let at = fields.iter().position(|(k, _)| k == key.as_ref()).ok_or_else(|| self.special_fault())?;
                    fields.remove(at);
                    self.drop_top()?;
                    self.data.push(Value::Null);
                    return Ok(());
                }
                if matches!(&target, Value::Map(pairs) if pairs.iter().any(|(k, _)| matches!(k, Value::Hashed(_)))) {
                    let left = self.special_builtin(Builtin::Erase, &[target, named])?.ok_or_else(|| self.special_fault())?;
                    let Value::Bond(cell) = self.drop_top()? else { return Err(self.special_fault().into()) };
                    *cell.borrow_mut() = left;
                    self.data.push(Value::Null);
                    return Ok(());
                }
                if self.special_method(&target, 13).is_some() {
                    self.drop_top()?;
                    self.special_call(&target, 13, vec![named])?;
                    self.data.push(Value::Null);
                    return Ok(());
                }
                let at = self.key_quietly(&named);
                let holder = self.drop_top()?;
                let Value::Bond(cell) = holder else {
                    return Err("Cannot take a place out of something that is not an array".into());
                };
                let nested = match &*cell.borrow() { Value::Collection(held, _) => Some(held.clone()), _ => None };
                let cell = nested.unwrap_or(cell);
                let mut inside = cell.borrow_mut();
                let left = match &*inside {
                    Value::Array(items) if !self.lang.del_words.is_empty() => {
                        let raw = (match &at { Value::Small(n) => Some(*n), Value::Huge(n) => n.to_i64(), Value::Flag(b) => Some(i64::from(*b)), _ => None }).ok_or_else(|| self.lang.del_unrun.clone())?;
                        let i = if raw < 0 { items.len() as i64 + raw } else { raw };
                        if i < 0 || i as usize >= items.len() { return Err(self.lang.del_unrun.clone().into()); }
                        let mut left = items.as_ref().clone();
                        left.remove(i as usize);
                        Value::array(left)
                    }
                    Value::Array(items) | Value::Tuple(items) => {
                        let i = as_index(&at)?;
                        Value::Map(Rc::new(
                            items
                                .iter()
                                .enumerate()
                                .filter(|(j, _)| *j != i)
                                .map(|(j, v)| (Value::Small(j as i64), v.clone()))
                                .collect(),
                        ))
                    }
                    Value::Map(pairs) => {
                        if !self.lang.del_words.is_empty() && !pairs.iter().any(|(k, _)| k.equals(&at)) {
                            return Err(self.lang.del_unrun.clone().into());
                        }
                        Value::Map(Rc::new(pairs.iter().filter(|(k, _)| !k.equals(&at)).cloned().collect()))
                    },
                    v => return Err(format!("Cannot take a place out of {}", v.plain()).into()),
                };
                *inside = left;
                drop(inside);
                Value::Null
            }
            Action::FastenNamed => {
                let held = self.drop_top()?;
                let spelled = self.drop_top()?;
                let name = self.name_spelled(&spelled);
                let at = self.registry.slot(&name);
                self.world.resize(self.registry.idents.len(), Value::Blank);
                // What has no cell to share is simply written, as a
                // language that asks to share one from something that
                // has none does rather than stopping.
                self.world[at] = held;
                Value::Null
            }
            // A value made a value of another kind. Numbers give up what
            // lies past the point, text is read for the number it opens
            // with, and anything that is not an array becomes an array
            // holding just itself.
            Action::Cast(kind) => {
                let v = self.drop_top()?;
                let sp = self.wording();
                match kind {
                    Sort::Set => { let items = self.comprehension_items(&v)?; Value::Set(Rc::new(RefCell::new(self.set_from(items)?))) },
                    Sort::Text => Value::text(&v.display(&sp)),
                    Sort::Boolean => Value::Flag(self.truth(&v)),
                    Sort::Null => Value::Null,
                    Sort::Array => match v {
                        held @ (Value::Array(_) | Value::Map(_)) => held,
                        Value::Null | Value::Blank | Value::Gap => Value::array(Vec::new()),
                        held => Value::array(vec![held]),
                    },
                    Sort::Integer => {
                        let worth = self.as_number(&v);
                        let whole = arith::whole_of(&worth).unwrap_or_else(|| BigInt::from(0));
                        self.within_width(Value::of_big(whole))
                    }
                    Sort::Real | Sort::Rational => {
                        let worth = self.as_number(&v);
                        let places = self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
                        let made = arith::to_real(&worth, places).unwrap_or(Value::Small(0));
                        self.at_real_width(made)
                    }
                }
            }
            Action::BitTurn if self.lang.bits_unbounded && !self.lang.whole_bits => {
                let v = self.drop_top()?;
                Value::of_big(!self.whole_bits(&v)?)
            }
            Action::BitTurn => {
                let v = self.drop_top()?;
                match &v {
                    _ if self.lang.whole_bits => Value::of_big(!self.whole_for_bits(&v)?),
                    Value::Text(s) => {
                        let out: Vec<u8> = self.lang.bytes_of(s).iter().map(|c| !c).collect();
                        Value::text(&self.lang.text_of(&out))
                    }
                    _ => Value::Small(!self.bits_said(&v)?),
                }
            }
            Action::Positive => {
                match self.drop_top()? {
                    Value::Imaginary(_, words) => return Err(words.to_string().into()),
                    Value::Flag(flag) => Value::Small(i64::from(flag)),
                    number @ (Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Real(_)) => number,
                    _ => return Err(self.lang.plus_non_number.clone().unwrap_or_default().into()),
                }
            }
            Action::Negate => {
                // 0 - x, so a real keeps its precision.
                let v = self.drop_top()?;
                if let Value::Imaginary(_, words) = &v { return Err(words.to_string().into()); }
                if let Some(answer) = self.special_call(&v, 25, Vec::new())? {
                    self.data.push(answer);
                    return Ok(());
                }
                let v = if self.lang.arithmetic_flags { match v { Value::Flag(b) => Value::Small(i64::from(b)), other => other } } else { v };
                // Text turned about is text taken times minus one, which
                // is how a language that reads a number out of text does
                // it: the number the text opens with is turned about, and
                // text opening with none is a pair no such step can take.
                if matches!(v, Value::Text(_)) && self.lang.warns_of_unwritten {
                    let worth = self.dyadic(&Action::Mul, &v, &Value::Small(-1))?;
                    self.data.push(worth);
                    return Ok(());
                }
                let turned = match arith::calculate(Operation::Minus, &Value::Small(0), &v) {
                    Some(r) => r?,
                    None => return Err("Cannot negate non-numeric value".to_string().into()),
                };
                // A nought turned about is the other nought.
                match (&v, &turned) {
                    (Value::Real(was), Value::Real(now)) if num_traits::Zero::is_zero(&now.p) => {
                        arith::shape_signed(now.p.clone(), now.q.clone(), Some(now.places), !was.below).with_point(was.point)
                    }
                    // Turning the lowest whole number about takes it past
                    // the width the language holds, as adding to the
                    // highest one does.
                    _ => self.within_width(turned),
                }
            }
            Action::Adorn(kind) => {
                let descriptor = match kind {
                    1 => Descriptor::Static(self.drop_top()?),
                    2 => Descriptor::Class(self.drop_top()?),
                    3 => Descriptor::Property(self.drop_top()?, None),
                    _ => {
                        let previous = self.drop_top()?;
                        let setter = self.drop_top()?;
                        match previous {
                            Value::Descriptor(d) => match d.as_ref() {
                                Descriptor::Property(get, _) => Descriptor::Property(get.clone(), Some(setter)),
                                _ => return Err(self.lang.class_unready[0].clone().into()),
                            },
                            _ => return Err(self.lang.class_unready[0].clone().into()),
                        }
                    }
                };
                Value::Descriptor(Rc::new(descriptor))
            }
            Action::Invoke(name) => {
                let top = self.drop_top()?;
                let callee = self.what_it_spells(top);
                return match callee {
                    Value::ValueMethod(method) => {
                        let raw = self.drop_many(argc - 1)?;
                        let items = self.call_items(raw)?;
                        let mut positional = Vec::new();
                        let mut named = Vec::new();
                        for (key, value) in items { if let Some(key) = key { named.push((key, value)); } else { positional.push(value); } }
                        let result = self.value_method(&method.0, &method.1, positional, named)?;
                        self.data.push(result);
                        return Ok(());
                    }
                    Value::Native(b, word) => {
                        let args = self.drop_many(argc - 1)?;
                        let items = self.call_items(args)?;
                        let answer = self.builtin_call(b, &word, items)?;
                        self.data.push(answer);
                        Ok(())
                    }
                    Value::Descriptor(d) => {
                        let mut args = self.drop_many(argc - 1)?;
                        let target = match d.as_ref() {
                            Descriptor::Static(f) => f.clone(),
                            Descriptor::Bound(f, receiver) => { args.insert(0, receiver.clone()); f.clone() }
                            _ => return Err(self.lang.class_unready[0].clone().into()),
                        };
                        let count = args.len() + 1;
                        self.data.extend(args);
                        self.data.push(target);
                        self.perform(&Action::Invoke(name.clone()), count)
                    }
                    Value::Routine(p) => self.invoke_top(&p, argc - 1),
                    Value::Method(object, method) => {
                        let args = self.drop_many(argc - 1)?;
                        let mut given = vec![Value::Object(object)];
                        given.extend(args);
                        self.invoke(&method, given)
                    }
                    Value::Object(object) if !self.lang.class_special.is_empty() => {
                        let args = self.drop_many(argc - 1)?;
                        let answer = self.special_call(&Value::Object(object), 17, args)?.ok_or_else(|| self.special_fault())?;
                        self.data.push(answer);
                        Ok(())
                    }
                    Value::Class(c) if self.lang.explicit_this => {
                        let args = self.drop_many(argc - 1)?;
                        self.data.push(Value::Class(c));
                        self.data.extend(args);
                        self.perform(&Action::Make, argc)
                    }
                    // A pair of a thing and a method's name stands for
                    // that method of that thing, which is how a language
                    // hands one routine over where any other would do.
                    Value::Array(pair) if self.lang.spelled_stands && pair.len() == 2 => {
                        let sp = self.wording();
                        let called: Rc<str> = Rc::from(pair[1].display(&sp).as_str());
                        // The thing may stand in the pair through a cell
                        // it shares with a name, which reads as what the
                        // cell holds, as reading that name would.
                        let first = match &pair[0] {
                            Value::Bond(cell) => cell.borrow().clone(),
                            held => held.clone(),
                        };
                        let subject = match self.class_it_spells(first.clone()) {
                            Value::Class(_) => Value::Null,
                            held => held,
                        };
                        let mut args = self.drop_many(argc - 1)?;
                        if matches!(subject, Value::Null) {
                            let stands = self.class_it_spells(first);
                            self.data.push(subject);
                            self.data.push(stands);
                            for a in args.drain(..) {
                                self.data.push(a);
                            }
                            return self.perform(&Action::Summon(called), argc + 1);
                        }
                        self.data.push(subject);
                        for a in args.drain(..) {
                            self.data.push(a);
                        }
                        return self.perform(&Action::Send(called), argc);
                    }
                    // A word of the language's own stands where a
                    // routine stands: text spelling one is called as
                    // though the word itself had been written there.
                    Value::Text(word) => match self.lang.builtins.get(word.as_ref()).copied() {
                        Some(native) => {
                            let mut given = self.drop_many(argc - 1)?;
                            let outcome = self.builtin(native, &word, &mut given);
                            let held = match self.carried.take() {
                                Some(fled) => return Err(fled),
                                None => outcome?,
                            };
                            self.data.push(held);
                            Ok(())
                        }
                        None => Err(format!("'{}' is not a function", name).into()),
                    },
                    _ => Err(format!("'{}' is not a function", name).into()),
                };
            }
            Action::Evaluate => {
                return match self.drop_top()? {
                    Value::Routine(p) => self.invoke(&p, Vec::new()),
                    _ => Err("eval needs a program".to_string().into()),
                };
            }
            Action::Execute => {
                return match self.drop_top()? {
                    Value::Routine(p) => self.invoke(&p, Vec::new()),
                    v => {
                        self.data.push(v);
                        Ok(())
                    }
                };
            }
            Action::TupleJoin => {
                let portions = self.drop_many(2)?;
                let mut together = Vec::new();
                for portion in portions {
                    let Value::Array(items) = collection_contents(&portion) else { return Err("Tuple portion is not an array".to_string().into()); };
                    together.extend(items.iter().cloned());
                }
                Value::array(together)
            }
            Action::Unpack(count, rest) => {
                let source = collection_contents(&self.drop_top()?);
                let mut items = match source {
                    Value::Generator(ref generator) => {
                        let mut found = Vec::new();
                        while rest.is_some() || found.len() <= *count {
                            let Some(value) = self.resume_generator(generator, Value::Null)? else { break };
                            found.push(value);
                        }
                        found
                    }
                    Value::Set(set) => set.borrow().items(),
                    Value::Counted(_) => self.comprehension_items(&source)?,
                    Value::Tuple(items) | Value::Array(items) => items.as_ref().clone(),
                    Value::Text(text) => text.chars().map(|c| Value::text(&c.to_string())).collect(),
                    Value::Map(pairs) => pairs.iter().map(|(key, _)| key.clone()).collect(),
                    _ => return Err(self.lang.unpack_unwalkable.clone().unwrap_or_else(|| "Value cannot be taken apart".to_string()).into()),
                };
                let least = count - usize::from(rest.is_some());
                if items.len() < least {
                    return Err(self.lang.unpack_short.clone().unwrap_or_else(|| "Too few values".to_string()).into());
                }
                if let Some(at) = rest {
                    let until = items.len() - (count - at - 1);
                    let middle = items.drain(*at..until).collect();
                    items.insert(*at, Value::array(middle));
                } else if items.len() > *count {
                    return Err(self.lang.unpack_long.clone().unwrap_or_else(|| "Too many values".to_string()).into());
                }
                Value::array(items)
            }
            Action::ComprehensionItems => {
                let source = self.drop_top()?;
                if matches!(source, Value::Object(_)) { self.data.push(source); return Ok(()); }
                match self.set_walk(&source) {
                    Some(walk) => walk,
                    None => Value::array(self.special_items(&source)?),
                }
            }
            Action::UnpackCount(wanted) | Action::BindCount(wanted) => {
                let source = self.drop_top()?;
                let parts = if let Value::Generator(walk) = &source {
                    let mut parts = Vec::new();
                    for _ in 0..=*wanted {
                        match self.resume_generator(walk, Value::Null)? {
                            Some(item) => parts.push(item),
                            None => break,
                        }
                    }
                    parts
                } else { self.comprehension_items(&source)? };
                if parts.len() != *wanted {
                    let said = if matches!(op, Action::BindCount(_)) { self.lang.binding_unrun.clone() }
                        else { self.lang.comprehension_unpack_amiss.first().cloned().unwrap_or_else(|| "Wrong number of parts in a comprehension target".to_string()) };
                    return Err(said.into());
                }
                Value::array(parts)
            }
            Action::GatherItem { map, spread } => {
                let mut both = self.drop_many(2)?.into_iter();
                let gathered_so_far = both.next().expect("growing literal").contents();
                let next = both.next().expect("literal item");
                if let Value::Set(cell) = gathered_so_far {
                    let incoming = if *spread { self.comprehension_items(&next)? } else { vec![next] };
                    for item in incoming {
                        if matches!(item, Value::Object(_)) {
                            let hashed = self.special_key(&item)?;
                            let prior = cell.borrow().items();
                            let mut found = false;
                            for old in prior { if self.special_keys_equal(&old, &hashed)? { found = true; break; } }
                            if !found { let key = format!("object:{}", cell.borrow().held.len()); cell.borrow_mut().insert(key, hashed); }
                        } else { cell.borrow_mut().insert(self.set_key(&item)?, item); }
                    }
                    Value::Set(cell)
                } else if *map {
                    let mut pairs = match gathered_so_far { Value::Map(p) => p.as_ref().clone(), _ => unreachable!() };
                    let new_pairs = match (spread, next.contents()) {
                        (true, Value::Map(p)) => p.as_ref().clone(),
                        (false, Value::Tie(p)) => vec![(p.0.clone(), p.1.clone())],
                        _ => return Err(self.lang.spread_unmapped.first().cloned().unwrap_or_else(|| "Value has no map pairs".to_string()).into()),
                    };
                    for (key, value) in new_pairs {
                        if self.lang.class_special.is_empty() { put_key(&mut pairs, key, value); continue; }
                        let key = self.special_key(&key)?;
                        let mut found = None;
                        for (at, (old, _)) in pairs.iter().enumerate() {
                            if self.special_keys_equal(old, &key)? { found = Some(at); break; }
                        }
                        if let Some(at) = found { pairs[at].1 = value; } else { pairs.push((key, value)); }
                    }
                    Value::Map(Rc::new(pairs))
                } else {
                    let mut items = match gathered_so_far { Value::Array(a) | Value::Tuple(a) => a.as_ref().clone(), _ => unreachable!() };
                    if *spread { items.extend(self.comprehension_items(&next)?); } else { items.push(next); }
                    Value::array(items)
                }
            }
            Action::Suspend | Action::Delegate => return Err(self.lang.yield_unsupported.first().cloned().unwrap_or_default().into()),
            Action::MakeTuple => Value::Tuple(Rc::new(self.drop_many(argc)?)),
            Action::MakeSet => Value::Set(Rc::new(RefCell::new(self.set_from(Vec::new())?))),
            Action::MakeArray => gathered(self.drop_many(argc)?, false, self.lang.plain_keys),
            Action::MakeMap => gathered(self.drop_many(argc)?, true, self.lang.plain_keys),
            Action::Tie => {
                let pair = self.drop_many(2)?;
                let mut pair = pair.into_iter();
                let (k, v) = (pair.next().expect("the key"), pair.next().expect("the value"));
                Value::Tie(Rc::new((k, v)))
            }
            Action::KeyAt | Action::ValueAt => {
                let pair = self.drop_many(2)?;
                let at = as_index(&pair[1])?;
                let key = matches!(op, Action::KeyAt);
                if self.walked_set(&pair[0])?.is_some() {
                    let item = self.element(&pair[0], &pair[1], Reading::Plain)?;
                    self.data.push(item);
                    return Ok(());
                }
                match &pair[0] {
                    Value::Set(s) => s.borrow().items().get(at).cloned().ok_or_else(|| self.set_said(".missing", &at.to_string()))?,
                    Value::Counted(r) => if key { Value::Small(at as i64) } else {
                        r.at(BigInt::from(at)).ok_or_else(|| self.lang.range_index[0].clone())?
                    },
                    Value::Array(items) | Value::Tuple(items) => match items.get(at) {
                        Some(v) if !key => v.clone(),
                        Some(_) => Value::Small(at as i64),
                        None => return Err(format!("Array index {} out of bounds (length: {})", at, items.len()).into()),
                    },
                    Value::Map(pairs) => match pairs.get(at) {
                        Some((k, v)) => if key { k.clone() } else { v.clone() },
                        None => return Err(format!("Array index {} out of bounds (length: {})", at, pairs.len()).into()),
                    },
                    // A thing holds named values too, and walking it
                    // walks those, in the order they were written.
                    Value::Object(o) => {
                        let held = o.fields.borrow();
                        match held.get(at) {
                            // What a member is called, not the name it is
                            // filed under: a class keeping one to itself
                            // files it under its own name as well.
                            Some((n, v)) => if key { Value::text(crate::value::who_keeps(n).0) } else { v.clone() },
                            None => return Err(format!("Array index {} out of bounds (length: {})", at, held.len()).into()),
                        }
                    }
                    _ => return Err("Cannot walk a value that is not an array".to_string().into()),
                }
            }
            Action::Matrix => return Err(self.lang.matrix_unready.clone().unwrap_or_default().into()),
            Action::SliceUnavailable => return Err(self.lang.slice_unsupported.clone().unwrap_or_default().into()),
            Action::Slice => {
                let step = self.drop_top()?;
                let stop = self.drop_top()?;
                let start = self.drop_top()?;
                Value::Slice(Rc::new([start, stop, step]))
            }
            Action::AtEnd => return Err("An empty index belongs on the left of an assignment".to_string().into()),
            // A place in text holds one letter and no more, so what is
            // written there is the first letter of what was handed
            // over, and the write is worth that letter and not the
            // whole. The language says as much where it has words for
            // it. The thing written into is looked at and handed back
            // as it was: only the value beneath it changes.
            Action::Fitted => {
                let into = self.drop_top()?;
                if let (Value::Text(_), true) = (&into, self.lang.text_places) {
                    let sp = self.wording();
                    let handed = self.data.last().ok_or("Stack underflow")?.display(&sp);
                    let mut letters = handed.chars();
                    let Some(letter) = letters.next() else {
                        return Err("Cannot write nothing into a place in text".to_string().into());
                    };
                    if letters.next().is_some() {
                        if let Some(said) = self.lang.text_place_first.clone() {
                            self.complain(Complaint::Warning, &said);
                        }
                    }
                    *self.data.last_mut().expect("the value written") = Value::text(&letter.to_string());
                }
                into
            }
            Action::Forge(plan) => {
                // What was pushed: the class to stand on, then a value for
                // every property, every value of the class's own, and
                // every constant, in the order the plan names them.
                let mut given = self.drop_many(argc)?.into_iter();
                let base = match plan.extends {
                    false => None,
                    true => match given.next() {
                        Some(Value::Class(c)) => Some(c),
                        Some(v) => return Err(format!("Class {} cannot stand on {}", plan.name, v.plain()).into()),
                        None => return Err("Stack underflow".to_string().into()),
                    },
                };
                let mut answers = Vec::with_capacity(plan.answers);
                for _ in 0..plan.answers {
                    match given.next() {
                        Some(Value::Class(c)) => answers.push(c),
                        Some(v) => return Err(format!("Class {} cannot answer to {}", plan.name, v.plain()).into()),
                        None => return Err("Stack underflow".to_string().into()),
                    }
                }
                let mut take = |names: &[String]| -> Vec<(String, Value)> {
                    names.iter().map(|n| (n.clone(), given.next().unwrap_or(Value::Null))).collect()
                };
                Value::Class(Rc::new(Class {
                    name: plan.name.clone(),
                    base,
                    answers,
                    fields: take(&plan.field_names),
                    reaches: plan.field_reach.clone(),
                    methods: plan.methods.clone(),
                    shared: RefCell::new(take(&plan.shared_names)),
                    constants: take(&plan.constant_names),
                }))
            }
            Action::Make => {
                let mut args = self.drop_many(argc)?;
                let stands = self.class_it_spells(args.remove(0));
                let Value::Class(class) = stands else {
                    return Err("Only a class can be made into an object".to_string().into());
                };
                self.made += 1;
                let object = Rc::new(Instance { class: class.clone(), fields: RefCell::new(class.all_fields()), mark: self.made });
                if self.lang.destructor.is_some() {
                    self.things_made.borrow_mut().push(Rc::downgrade(&object));
                }
                let maker = self.lang.constructor.as_deref().and_then(|m| class.method(m)).cloned();
                match maker {
                    Some(maker) => {
                        let mut all = vec![Value::Object(object.clone())];
                        all.extend(args);
                        self.invoke(&maker, all)?;
                        // What the maker leaves is not the object.
                        self.drop_top()?;
                    }
                    None if !args.is_empty() => {
                        return Err(format!("Class {} takes no arguments when it is made", class.name).into());
                    }
                    None => {}
                }
                Value::Object(object)
            }
            Action::HasMember(name) => {
                let held = self.drop_top()?;
                let class = match &held {
                    Value::Object(o) => Some(&o.class),
                    Value::Class(c) => Some(c),
                    _ => None,
                };
                let field = match &held {
                    Value::Object(o) => self.member_at(&o.fields.borrow(), name).is_some(),
                    _ => false,
                };
                Value::Flag(matches!(held, Value::ValueMethod(_)) || field || (!self.lang.class_special.is_empty() && class.is_some()) || class.map_or(false, |c| c.method(name).is_some() || c.holder(name).is_some() || c.constant(name).is_some()))
            }
            Action::Grab(name) => match self.drop_top()? {
                Value::ValueMethod(_) => return Err(self.lang.class_unready.first().cloned().unwrap_or_default().into()),
                subject if self.descriptor_of(&subject, name).is_some() => {
                    let d = self.descriptor_of(&subject, name).expect("the descriptor");
                    self.descriptor_read(&d, subject)?
                }
                Value::Trace(words) => return Err(words.to_string().into()),
                Value::Class(c) if self.lang.class_special.get(37).map_or(false, |n| n == name.as_ref()) => Value::text(&c.name),
                Value::Object(o) if self.lang.class_special.get(35).map_or(false, |n| n == name.as_ref()) => Value::Class(o.class.clone()),
                Value::Object(o) if self.lang.class_special.get(36).map_or(false, |n| n == name.as_ref()) => {
                    Value::Fields(o)
                }
                Value::Class(c) if self.lang.member_pipes => {
                    if let Some(method) = c.method(name).filter(|_| c.holder(name).is_none() && c.constant(name).is_none()) {
                        Value::Routine(method.clone())
                    } else {
                        self.data.push(Value::Class(c));
                        return self.perform(&Action::Reach(name.clone()), 1);
                    }
                }
                Value::Object(o) => {
                    let found = {
                        let held = o.fields.borrow();
                        self.member_at(&held, name).map(|at| held[at].1.clone())
                    };
                    match found {
                        // A property held in a shared cell reads as what
                        // the cell holds, the sharing being between the
                        // names and not in the value.
                        Some(Value::Bond(shared)) => shared.borrow().clone(),
                        Some(v) => v,
                        // A class may answer for a property the thing
                        // does not hold: where the language has a word
                        // for such a method, it is called with the name
                        // that was asked for.
                        None if self.lang.member_pipes && o.class.holder(name).is_some() => {
                            self.data.push(Value::Class(o.class.clone()));
                            self.perform(&Action::Reach(name.clone()), 1)?;
                            match self.drop_top()? {
                                Value::Routine(method) => Value::Method(o, method),
                                held => held,
                            }
                        }
                        None if self.lang.member_pipes && o.class.method(name).is_some() => {
                            let method = o.class.method(name).expect("the member exists").clone();
                            Value::Method(o, method)
                        }
                        None if self.reads_for(&o).is_some() => {
                            let method = self.reads_for(&o).expect("the method");
                            let asked = Value::Text(Rc::from(name.as_ref()));
                            return self.invoke(&method, vec![Value::Object(o), asked]);
                        }
                        // A language with a word for a warning says a
                        // property is not there and reads nothing.
                        None if self.lang.warns_of_unwritten => {
                            let told = format!("Undefined property: {}::${}", o.class.name, name);
                            self.complain(Complaint::Warning, &told);
                            Value::Null
                        }
                        None => return Err(format!("Undefined property: {}::${}", o.class.name, name).into()),
                    }
                }
                Value::Routine(_) | Value::Method(..) if !self.lang.scope_unready.is_empty() => return Err(self.lang.scope_unready[0].clone().into()),
                v => return Err(format!("Cannot read property '{}' of {}", name, v.plain()).into()),
            },
            // The property becomes a cell the object and the name that
            // takes it both stand for, so a write through either is a
            // write both see.
            Action::BondField(name) => {
                match self.drop_top()? {
                    Value::Object(o) => {
                        let mut held = o.fields.borrow_mut();
                        let at = match self.member_at(&held, name) {
                            Some(at) => at,
                            None => {
                                held.push((name.to_string(), Value::Null));
                                held.len() - 1
                            }
                        };
                        if let Value::Bond(shared) = &held[at].1 {
                            let shared = shared.clone();
                            drop(held);
                            Value::Bond(shared)
                        } else {
                            let was = std::mem::replace(&mut held[at].1, Value::Null);
                            let shared = Rc::new(RefCell::new(was));
                            held[at].1 = Value::Bond(shared.clone());
                            drop(held);
                            Value::Bond(shared)
                        }
                    }
                    v => return Err(format!("Cannot share property '{}' of {}", name, v.plain()).into()),
                }
            }
            Action::Uproot(name) => {
                match self.drop_top()? {
                    Value::Object(o) => {
                        // The place stays where it was, holding nothing:
                        // a walk under way counts places, and closing
                        // one up would move every later member back a
                        // step under its feet.
                        let mut held = o.fields.borrow_mut();
                        if let Some(at) = self.member_at(&held, name) {
                            if !self.lang.del_words.is_empty() && matches!(held[at].1, Value::Blank) {
                                return Err(self.lang.del_unrun.clone().into());
                            }
                            held[at].1 = Value::Blank;
                        } else if !self.lang.del_words.is_empty() {
                            return Err(self.lang.del_unrun.clone().into());
                        }
                        Value::Null
                    }
                    v => return Err(format!("Cannot take property '{}' off {}", name, v.plain()).into()),
                }
            }
            Action::Plant(name) => {
                let mut pair = self.drop_many(2)?;
                if self.lang.class_special.get(35..38).map_or(false, |names| names.iter().any(|n| n == name.as_ref())) {
                    return Err(self.lang.special_unready.first().cloned().unwrap_or_default().into());
                }
                let value = pair.pop().expect("the value");
                match pair.pop().expect("the object") {
                    Value::Object(o) if self.descriptor_of(&Value::Object(o.clone()), name).map_or(false, |d| matches!(d.as_ref(), Descriptor::Property(..))) => {
                        let d = self.descriptor_of(&Value::Object(o.clone()), name).expect("the property");
                        if let Descriptor::Property(_, Some(set)) = d.as_ref() {
                            self.data.extend([Value::Object(o), value, set.clone()]);
                            self.perform(&Action::Invoke(name.clone()), 3)?;
                            self.drop_top()?;
                            Value::Null
                        } else { return Err(self.lang.class_unready[0].clone().into()); }
                    }
                    Value::Class(c) if self.lang.member_pipes => {
                        let mut fields = c.shared.borrow_mut();
                        if let Some((_, old)) = fields.iter_mut().find(|(n, _)| n == name.as_ref()) {
                            *old = value;
                        } else {
                            fields.push((name.to_string(), value));
                        }
                        Value::Null
                    }
                    Value::Object(o) => {
                        let taken = {
                            let fields = o.fields.borrow();
                            self.member_at(&fields, name)
                        };
                        // A class may take the write of a property the
                        // thing does not hold, the same way it answers
                        // for one it cannot read.
                        if taken.is_none() {
                            if let Some(method) = self.writes_for(&o) {
                                let asked = Value::Text(Rc::from(name.as_ref()));
                                return self.invoke(&method, vec![Value::Object(o), asked, value]);
                            }
                        }
                        let mut fields = o.fields.borrow_mut();
                        match taken {
                            Some(at) => fields[at].1 = value,
                            None => fields.push((name.to_string(), value)),
                        }
                        Value::Null
                    }
                    v => return Err(format!("Cannot write property '{}' of {}", name, v.plain()).into()),
                }
            }
            // The property or method a value names, worked out while
            // the program runs. The name is on top; what it belongs to
            // is under it, with any arguments under that.
            Action::GrabNamed => {
                let spelled = self.drop_top()?;
                let sp = self.wording();
                let name: Rc<str> = Rc::from(spelled.display(&sp).as_str());
                return self.perform(&Action::Grab(name), 1);
            }
            Action::PlantNamed => {
                let value = self.drop_top()?;
                let spelled = self.drop_top()?;
                let sp = self.wording();
                let name: Rc<str> = Rc::from(spelled.display(&sp).as_str());
                self.data.push(value);
                return self.perform(&Action::Plant(name), 2);
            }
            Action::ReachNamed => {
                let spelled = self.drop_top()?;
                let sp = self.wording();
                let name: Rc<str> = Rc::from(spelled.display(&sp).as_str());
                return self.perform(&Action::Reach(name), 1);
            }
            Action::SummonNamed(count) => {
                let spelled = self.drop_top()?;
                let sp = self.wording();
                let name: Rc<str> = Rc::from(spelled.display(&sp).as_str());
                return self.perform(&Action::Summon(name), count + 2);
            }
            Action::SowNamed => {
                let value = self.drop_top()?;
                let spelled = self.drop_top()?;
                let sp = self.wording();
                let name: Rc<str> = Rc::from(spelled.display(&sp).as_str());
                self.data.push(value);
                return self.perform(&Action::Sow(name), 2);
            }
            Action::SendNamed(count) => {
                let spelled = self.drop_top()?;
                let sp = self.wording();
                let name: Rc<str> = Rc::from(spelled.display(&sp).as_str());
                return self.perform(&Action::Send(name), count + 1);
            }
            Action::Send(name) => {
                let mut args = self.drop_many(argc)?;
                let subject = args.remove(0);
                if let Value::Generator(held) = subject {
                    let result = if Lang::spells(&self.lang.yield_close, name) && args.is_empty() {
                        self.close_generator(&held)?;
                        Value::Null
                    } else if Lang::spells(&self.lang.yield_send, name) && args.len() == 1 {
                        self.resume_generator(&held, args.remove(0))?.ok_or_else(|| self.lang.yield_exhausted[0].clone())?
                    } else if Lang::spells(&self.lang.yield_throw, name) {
                        return Err(self.lang.yield_throw_unavailable[0].clone().into());
                    } else {
                        return Err(self.lang.yield_unsupported[0].clone().into());
                    };
                    self.data.push(result);
                    return Ok(());
                }
                if self.lang.member_pipes {
                    if let Value::Class(c) = &subject {
                        if let Some(method) = c.method(name).filter(|_| c.holder(name).is_none() && c.constant(name).is_none()) {
                            return self.invoke(&method.clone(), args);
                        }
                    }
                    let field = match &subject {
                        Value::Object(o) => self.member_at(&o.fields.borrow(), name).is_some() || o.class.holder(name).is_some(),
                        Value::Class(c) => c.holder(name).is_some() || c.constant(name).is_some(),
                        _ => false,
                    };
                    if field {
                        self.data.push(subject);
                        self.perform(&Action::Grab(name.clone()), 1)?;
                        let callee = self.drop_top()?;
                        let count = args.len();
                        self.data.extend(args);
                        self.data.push(callee);
                        return self.perform(&Action::Invoke(name.clone()), count + 1);
                    }
                }
                let Value::Object(o) = subject else {
                    return Err(format!("Cannot call method '{}' on a value that is not an object", name).into());
                };
                let method = o.class.method(&name).cloned();
                // A class may answer for a call it does not have: where
                // the language names such a method and the class is
                // written with it, it is called with the name asked for
                // and the arguments gathered into an array.
                let Some(method) = method else {
                    let stands = self.lang.caller.as_deref().and_then(|named| o.class.method(named).cloned());
                    let Some(stands) = stands else {
                        return Err(format!("Call to undefined method {}::{}()", o.class.name, name).into());
                    };
                    let asked = Value::Text(Rc::from(name.as_ref()));
                    return self.invoke(&stands, vec![Value::Object(o), asked, Value::array(args)]);
                };
                let mut all = vec![Value::Object(o)];
                all.extend(args);
                return self.invoke(&method, all);
            }
            Action::Reach(name) => match {
                let top = self.drop_top()?;
                self.class_it_spells(top)
            } {
                Value::Class(c) => match c.constant(&name) {
                    Some(v) => v.clone(),
                    None => match c.holder(&name) {
                        Some(holder) => {
                            let found = holder.shared.borrow().iter().find(|(n, _)| n == name.as_ref()).map(|(_, v)| v.clone());
                            found.expect("the holder has it")
                        }
                        None => return Err(format!("Undefined constant {}::{}", c.name, name).into()),
                    },
                },
                v => {
                    let told = self.no_such_class(&v).unwrap_or_else(|| format!("Cannot reach '{}' in {}", name, v.plain()));
                    return Err(told.into());
                }
            },
            Action::Sow(name) => {
                let mut pair = self.drop_many(2)?;
                let value = pair.pop().expect("the value");
                let stands = self.class_it_spells(pair.pop().expect("the class"));
                match stands {
                    Value::Class(c) => {
                        let holder = c.holder(&name).unwrap_or(&c);
                        let mut shared = holder.shared.borrow_mut();
                        match shared.iter_mut().find(|(n, _)| n == name.as_ref()) {
                            Some(place) => place.1 = value,
                            None => shared.push((name.to_string(), value)),
                        }
                        Value::Null
                    }
                    v => {
                        let told = self.no_such_class(&v).unwrap_or_else(|| format!("Cannot write '{}' in {}", name, v.plain()));
                        return Err(told.into());
                    }
                }
            }
            Action::Summon(name) => {
                let mut args = self.drop_many(argc)?;
                let this = args.remove(0);
                let stands = self.class_it_spells(args.remove(0));
                let Value::Class(class) = stands else {
                    let told = self.no_such_class(&stands).unwrap_or_else(|| format!("Cannot call '{}' on a value that is not a class", name));
                    return Err(told.into());
                };
                let method = class.method(&name).cloned();
                let method = method.ok_or_else(|| format!("Call to undefined method {}::{}()", class.name, name))?;
                let mut all = vec![this];
                all.extend(args);
                return self.invoke(&method, all);
            }
            Action::Kindred(name) => match self.drop_top()? {
                Value::Object(o) => Value::Flag(o.class.named(&name, self.lang.classes_folded)),
                _ => Value::Flag(false),
            },
            // A routine written where a value stands, taking away with
            // it the values under it: one for each slot it names.
            Action::Close => {
                let held = self.drop_top()?;
                let Value::Routine(program) = held else {
                    return Err("Only a routine can carry names away with it".to_string().into());
                };
                let carried = self.drop_many(argc - 1)?;
                let mut made = (*program).clone();
                made.held = carried;
                Value::Routine(Rc::new(made))
            }
            Action::KindredTo => {
                let pair = self.drop_many(2)?;
                let against = match &pair[1] {
                    Value::Object(o) => Some(o.class.name.clone()),
                    Value::Class(c) => Some(c.name.clone()),
                    Value::Text(spelled) => Some(spelled.to_string()),
                    _ => None,
                };
                match (&pair[0], against) {
                    (Value::Object(o), Some(name)) => Value::Flag(o.class.named(&name, self.lang.classes_folded)),
                    _ => Value::Flag(false),
                }
            }
            Action::Twin => {
                let top = self.data.last().cloned().ok_or("Stack underflow")?;
                top
            }
            Action::Matches(names) => match self.drop_top()? {
                Value::Object(o) => Value::Flag(names.iter().any(|n| o.class.named(n, self.lang.classes_folded))),
                _ => Value::Flag(false),
            },
            Action::Reraise => {
                let value = self.caught.last().cloned().ok_or_else(|| self.lang.throw_empty.clone().unwrap_or_default())?;
                return Err(Fault::Thrown(value));
            }
            Action::AssertFault => {
                let message = self.drop_top()?;
                let class = Rc::new(Class {
                    name: self.lang.assert_kind.clone().unwrap_or_default(), base: None,
                    answers: Vec::new(), fields: Vec::new(), reaches: Vec::new(),
                    methods: Vec::new(), constants: Vec::new(), shared: RefCell::new(Vec::new()),
                });
                self.made += 1;
                let fields = if matches!(&message, Value::Text(words) if words.is_empty()) {
                    Vec::new()
                } else { vec![("message".to_string(), message)] };
                let raised = Value::Object(Rc::new(Instance {
                    class, fields: RefCell::new(fields), mark: self.made,
                }));
                self.hurled_at.set(self.line);
                return Err(Fault::Thrown(raised));
            }
            Action::Hurl => {
                self.hurled_at.set(self.line);
                let mut raised = self.drop_top()?;
                if !self.lang.catch_as.is_empty() {
                    if matches!(raised, Value::Class(_)) && !self.lang.class_special.is_empty() {
                        self.data.push(raised); self.perform(&Action::Make, 1)?; raised = self.drop_top()?;
                    } else if let Value::Class(class) = raised {
                        self.made += 1;
                        raised = Value::Object(Rc::new(Instance { class, fields: RefCell::new(Vec::new()), mark: self.made }));
                    }
                }
                return Err(Fault::Thrown(raised));
            }
            Action::Titled => match self.drop_top()? {
                Value::Object(o) => Value::text(&o.class.name),
                Value::Class(c) => Value::text(&c.name),
                v => return Err(format!("{} has no class name", v.plain()).into()),
            },
            Action::Nothing => Value::Flag(matches!(self.drop_top()?, Value::Null | Value::Blank | Value::Gap)),
            // Words a definition has ready for a shape it still allows,
            // said where the shape is reached and nowhere else.
            Action::Remark(kind, said) => {
                self.complain(*kind, said);
                Value::Null
            }
            Action::HeldOrSaid(kind, said) => {
                let held = self.drop_top()?;
                if !matches!(held, Value::Bond(_)) {
                    self.complain(*kind, said);
                }
                held
            }
            Action::HeldOrStop(told) => match self.drop_top()? {
                held @ Value::Bond(_) => held,
                _ => return Err(told.to_string().into()),
            },
            // Only the run can say whether what a routine named had a
            // cell of its own; where it had none, one is made for it, so
            // that a routine giving back a cell always gives one.
            Action::HeldAnyway(kind, said) => match self.drop_top()? {
                held @ Value::Bond(_) => held,
                held => {
                    self.complain(*kind, said);
                    Value::Bond(Rc::new(RefCell::new(held)))
                }
            },
            Action::Standing(within) => {
                let pair = self.drop_many(2)?;
                match (&pair[0], as_index(&pair[1])) {
                    // A thing that is its own walk hands out what it
                    // pleases: what it holds is no part of the walk.
                    (Value::Object(_), _) if self.walker(&pair[0]).is_some() => Value::Flag(true),
                    (Value::Object(o), Ok(at)) => {
                        let here = within.as_deref();
                        let held = o.fields.borrow();
                        let reached = |filed: &str| match crate::value::who_keeps(filed) {
                            // A member a class keeps to itself is the
                            // business of that class alone.
                            (_, Some(keeper)) => here == Some(keeper),
                            (name, None) => match o.class.reach_of(name) {
                                Some((Reach::Guarded, holder)) => self.shares_with(holder, here),
                                _ => true,
                            },
                        };
                        Value::Flag(held.get(at).map_or(true, |(n, v)| standing(v) && reached(n)))
                    }
                    _ => Value::Flag(true),
                }
            }
            // A thing may be its own walk, or may hand another over to
            // be walked in its stead. Either way the walk begins here.
            Action::ContextEnter => {
                let object = self.drop_top()?;
                if matches!(&object, Value::Object(_)) {
                    if self.special_value(&object, 34).is_none() { return Err(self.special_fault().into()); }
                    self.special_call(&object, 33, Vec::new())?.ok_or_else(|| self.special_fault())?
                } else { object }
            }
            Action::SettleObjects => {
                let source = self.drop_top()?;
                if matches!(source, Value::Set(_)) { self.data.push(source); return Ok(()); }
                let Value::Array(items) = source else { return Err(self.special_fault().into()) };
                let mut kept = Vec::new();
                for item in items.iter() {
                    if matches!(item, Value::Object(_)) {
                        let hashed = self.special_key(item)?;
                        let mut found = false;
                        for prior in &kept {
                            if self.special_keys_equal(prior, &hashed)? { found = true; break; }
                        }
                        if !found { kept.push(hashed); }
                        continue;
                    }
                    kept.push(item.clone());
                }
                Value::array(kept.into_iter().map(|v| match v { Value::Hashed(pair) => pair.0.clone(), other => other }).collect())
            }
            Action::WalkFrom => {
                let mut handed = self.drop_top()?;
                if let Some(walk) = self.set_walk(&handed) { self.data.push(walk); return Ok(()); }
                if self.lang.yield_suspends && !matches!(handed, Value::Object(_)) {
                    let walk = self.iterator(handed)?;
                    self.data.push(walk);
                    return Ok(());
                }
                if !self.lang.class_special.is_empty() && matches!(handed, Value::Object(_) | Value::Walk(_)) {
                    let iterator = self.special_builtin(Builtin::Iter, std::slice::from_ref(&handed))?.ok_or_else(|| self.special_fault())?;
                    self.data.push(Value::Walking(Rc::new(RefCell::new((iterator, None)))));
                    return Ok(());
                }
                // One thing may hand over another that hands over a
                // third, so the asking goes on until what comes back is
                // no longer a thing that hands one over. A thing that
                // hands back itself hands back nothing further.
                // Asking a thing what to walk in its stead runs a piece
                // of the program written elsewhere. The walk stands
                // where it is written, and whatever is said of it says
                // so, so where it stands is kept over the asking.
                let stood_on = self.line;
                let mut gave = None;
                while let Some((giver, next)) = self.walk_handed(&handed)? {
                    gave = Some(giver);
                    handed = next;
                }
                self.line = stood_on;
                if let Some(walk) = self.set_walk(&handed) { self.data.push(walk); return Ok(()); }
                match self.walker(&handed) {
                    Some(_) => {
                        self.walk_asked(&handed, self.lang.walk_rewind.clone())?;
                    }
                    // What is handed over to be walked in another's
                    // stead must be a walk itself; a language with words
                    // for it stops rather than walking what it was given.
                    None => match (gave, &self.lang.giver_unwalkable, &self.lang.walk_giver) {
                        (Some(giver), Some((before, after)), Some(gives)) => {
                            return Err(format!("{} {}::{}() {}", before, giver, gives, after).into());
                        }
                        _ => self.walkable(&handed)?,
                    },
                }
                handed
            }
            Action::WalkAlone => {
                let held = self.drop_top()?;
                match (self.walker(&held), self.lang.walk_no_cell.clone()) {
                    (Some(_), Some(said)) => return Err(said.into()),
                    (Some(_), None) => {}
                    (None, _) => self.walkable(&held)?,
                }
                Value::Null
            }
            // Where a walk keeps its place by the item it handed out,
            // the pass after it looks for that item where it left it and
            // then everywhere else, since the body may have moved it.
            // A thing's places stay where they are, so a walk over one
            // counts them as it always did.
            Action::WalkPast => {
                let three = self.drop_many(3)?;
                let at = as_index(&three[1])?;
                let onward = match (&three[0], &three[2]) {
                    (Value::Object(_), _) => at + 1,
                    (walked, Value::Bond(cell)) => match lies_at(walked, cell, at) {
                        Some(found) => found + 1,
                        // The item is gone from the array altogether, so
                        // the place it stood at now holds whatever came
                        // after it, and that is where the walk goes on.
                        None => at,
                    },
                    _ => at + 1,
                };
                Value::Small(onward as i64)
            }
            Action::WalkMore => {
                let pair = self.drop_many(2)?;
                if let Value::Generator(held) = &pair[0] {
                    let item = self.resume_generator(held, Value::Null)?;
                    let more = item.is_some();
                    held.borrow_mut().current = item;
                    self.data.push(Value::Flag(more));
                    return Ok(());
                }
                if matches!(pair[0], Value::Cursor(_)) {
                    let more = self.core_more(&pair[0])?; self.data.push(Value::Flag(more)); return Ok(());
                }
                if let Some(set) = self.walked_set(&pair[0])? {
                    self.data.push(Value::Flag(as_index(&pair[1]).map_or(false, |at| at < set.borrow().held.len())));
                    return Ok(());
                }
                if let Value::Walking(walk) = &pair[0] {
                    let iterator = walk.borrow().0.clone();
                    let answer = self.special_step(&iterator)?;
                    let more = answer.is_some();
                    walk.borrow_mut().1 = answer;
                    self.data.push(Value::Flag(more));
                    return Ok(());
                }
                match self.walk_asked(&pair[0], self.lang.walk_more.clone())? {
                    Some(answer) => Value::Flag(self.truth(&answer)),
                    None if matches!(&pair[0], Value::Counted(_)) => {
                        let Value::Counted(r) = &pair[0] else { unreachable!() };
                        Value::Flag(pair[1].as_big().map_or(false, |i| i >= BigInt::from(0) && i < r.length()))
                    }
                    None => {
                        let reach = match &pair[0] {
                            Value::Array(items) | Value::Tuple(items) => items.len(),
                            Value::Set(s) => s.borrow().held.len(),
                            Value::Map(pairs) => pairs.len(),
                            Value::Object(o) => o.fields.borrow().len(),
                            _ => 0,
                        };
                        Value::Flag(as_index(&pair[1]).map_or(false, |at| at < reach))
                    }
                }
            }
            Action::WalkThis | Action::WalkKey => {
                let key = matches!(op, Action::WalkKey);
                let named = match key {
                    true => self.lang.walk_key.clone(),
                    false => self.lang.walk_this.clone(),
                };
                let pair = self.drop_many(2)?;
                if let Value::Generator(held) = &pair[0] {
                    let item = if key { pair[1].clone() } else { held.borrow_mut().current.take().unwrap_or(Value::Null) };
                    self.data.push(item);
                    return Ok(());
                }
                if let Value::Walking(walk) = &pair[0] {
                    self.data.push(if key { pair[1].clone() } else { walk.borrow().1.clone().ok_or_else(|| self.special_fault())? });
                    return Ok(());
                }
                match self.walk_asked(&pair[0], named)? {
                    None if matches!(pair[0], Value::Cursor(_)) => if key { pair[1].clone() } else { self.core_step(&pair[0])?.ok_or_else(|| self.core_fault("core.exhausted", ""))? },
                    Some(answer) => answer,
                    None => {
                        self.data.push(pair[0].clone());
                        self.data.push(pair[1].clone());
                        return self.perform(if key { &Action::KeyAt } else { &Action::ValueAt }, 2);
                    }
                }
            }
            Action::WalkOnward => {
                let held = self.drop_top()?;
                self.walk_asked(&held, self.lang.walk_onward.clone())?;
                Value::Null
            }
            Action::Extent => match self.drop_top()? {
                value @ Value::SetWalk(..) if self.walked_set(&value)?.is_some() => Value::Small(self.walked_set(&value)?.unwrap().borrow().held.len() as i64),
                Value::Counted(r) => Value::of_big(r.length()),
                Value::Set(set) => Value::Small(set.borrow().held.len() as i64),
                Value::Array(items) | Value::Tuple(items) => Value::Small(items.len() as i64),
                Value::Map(pairs) => Value::Small(pairs.len() as i64),
                Value::Object(o) => Value::Small(o.fields.borrow().len() as i64),
                // A language with a word for a warning is told a value
                // cannot be walked and walks it no times, rather than
                // having the run stopped over it.
                other if self.lang.warns_of_unwritten => {
                    let told = format!("foreach() argument must be of type array|object, {} given", self.kind_named(&other));
                    self.complain(Complaint::Warning, &told);
                    Value::Small(0)
                }
                _ => return Err("Cannot walk a value that is not an array".to_string().into()),
            },
            Action::Collect => {
                let mark = self.data.iter().rposition(|v| matches!(v, Value::Fence)).ok_or("Stack underflow")?;
                let items = self.data.split_off(mark + 1);
                self.data.pop();
                Value::array(items)
            }
            Action::StringFault => {
                let message = self.drop_top()?.plain();
                return Err(message.into());
            }
            Action::StringRender => {
                let conversion = self.drop_top()?.plain();
                let specification = self.drop_top()?.plain();
                let value = self.drop_top()?;
                let rendered = value.string_field(&self.wording(), &specification, &conversion)
                    .ok_or_else(|| self.lang.format_unavailable.clone().unwrap_or_else(|| "This formatted value is not supported".into()))?;
                Value::text(&rendered)
            }
            Action::Builtin(builtin, name) => {
                if self.data.len() < argc {
                    return Err("Stack underflow".to_string().into());
                }
                let at = self.data.len() - argc;
                // A language may say the run is over where it stands.
                // Text given is written out first; a number is not.
                if let Builtin::Leave = builtin {
                    if let Some(Value::Text(said)) = self.data.get(at) {
                        self.utter(said);
                    }
                    return Err(Fault::Finished);
                }
                let mut args = std::mem::take(&mut self.buffer);
                args.clear();
                args.extend(self.data.drain(at..));
                if matches!(builtin, Builtin::MapFrom) {
                    let items = self.call_items(std::mem::take(&mut args))?;
                    let made = self.map_from(items)?;
                    self.buffer = args;
                    self.data.push(made);
                    return Ok(());
                }
                let result = if self.lang.bind_names {
                    let items = self.call_items(std::mem::take(&mut args))?;
                    self.builtin_call(*builtin, name, items)
                } else { self.builtin(*builtin, name, &mut args) };
                self.buffer = args;
                match self.carried.take() {
                    Some(fled) => return Err(fled),
                    None => result?,
                }
            }
            dyadic => {
                let b = self.drop_top()?;
                let a = self.drop_top()?;
                self.special_dyad(dyadic, &a, &b)?
            }
        };
        self.data.push(self.keep_collection(result));
        Ok(())
    }

    // ---------- operations ----------

    /// The number a piece of text spells, brought to the width the
    /// language holds its numbers in. Without that a number written out
    /// and the same number spelled in text would be told apart, since
    /// one had been brought to the width and the other had not.
    /// Whether two numbers are the same at the width the language holds
    /// its reals in. Where it holds none, or neither is a real, they
    /// are the same only where they are exactly so.
    fn same_at_width(&self, a: &Value, b: &Value) -> bool {
        let real_here = |v: &Value| matches!(v, Value::Real(_) | Value::Frac(_));
        if self.lang.real_bits.is_none() || !(real_here(a) || real_here(b)) {
            return a.equals(b);
        }
        let places = self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
        let widened = |v: &Value| match arith::to_real(v, places) {
            Some(real) => self.at_real_width(real),
            None => v.clone(),
        };
        widened(a).equals(&widened(b))
    }

    fn number_of(&self, s: &str) -> Option<Value> {
        number_spelled(s).map(|n| self.at_real_width(n))
    }

    fn rem_repr(&self, value: &Value) -> Res<String> {
        if let Value::Bond(cell) = value { return self.rem_repr(&cell.borrow()); }
        Ok(match value {
            Value::Text(s) => {
                let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
                let mut out = String::from(quote);
                for c in s.chars() {
                    match c {
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        '\\' => out.push_str("\\\\"),
                        c if c == quote => { out.push('\\'); out.push(c); }
                        c if c.is_control() => out.push_str(&format!("\\x{:02x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push(quote);
                out
            }
            Value::Array(items) | Value::Tuple(items) => {
                let parts = items.iter().map(|v| self.rem_repr(v)).collect::<Res<Vec<_>>>()?;
                format!("[{}]", parts.join(", "))
            }
            Value::Map(entries) => {
                let mut parts = Vec::new();
                for (key, worth) in entries.iter() { parts.push(format!("{}: {}", self.rem_repr(key)?, self.rem_repr(worth)?)); }
                format!("{{{}}}", parts.join(", "))
            }
            Value::Real(real) => {
                let mut text = value.display(&self.wording());
                if !real.outside() && !text.contains(['.', 'e', 'E']) { text.push_str(".0"); }
                text
            }
            Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Null | Value::Ellipsis => value.display(&self.wording()),
            _ => return Err(self.lang.format_unsupported.clone().unwrap_or_default()),
        })
    }

    /// Remainder over text fills one mark at a time. A list supplies
    /// the marks in order; every other value supplies just one.
    fn rem_text(&self, template: &str, arguments: &Value) -> Res<String> {
        let bad = || self.lang.format_unsupported.clone().unwrap_or_default();
        let wrong = || self.lang.format_arguments.clone().unwrap_or_default();
        let values: Vec<&Value> = match arguments {
            Value::Array(items) | Value::Tuple(items) => items.iter().collect(),
            one => vec![one],
        };
        let mut used = 0;
        let mut out = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' { out.push(c); continue; }
            if chars.peek() == Some(&'%') { chars.next(); out.push('%'); continue; }
            let mut precision = None;
            if chars.peek() == Some(&'.') {
                chars.next();
                let mut digits = String::new();
                while chars.peek().map_or(false, char::is_ascii_digit) { digits.push(chars.next().unwrap()); }
                precision = Some(digits.parse::<usize>().map_err(|_| bad())?);
            }
            let kind = chars.next().ok_or_else(bad)?;
            if precision.is_some() && kind != 'f' { return Err(bad()); }
            let value = values.get(used).copied().ok_or_else(wrong)?;
            used += 1;
            let filled = match kind {
                's' => match value { Value::Text(text) => text.to_string(), _ => self.rem_repr(value)? },
                'r' => self.rem_repr(value)?,
                'd' | 'x' => {
                    if !matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_) | Value::Real(_)) { return Err(wrong()); }
                    if kind == 'x' && matches!(value, Value::Real(_)) { return Err(wrong()); }
                    let whole = value.as_big().map_err(|_| wrong())?;
                    if kind == 'x' { whole.to_str_radix(16) } else { whole.to_string() }
                }
                'f' => {
                    let number = match value {
                        Value::Small(n) => *n as f64,
                        Value::Huge(n) => n.to_f64().ok_or_else(wrong)?,
                        Value::Flag(b) => u8::from(*b) as f64,
                        Value::Real(r) => crate::value::as_binary(&r.p, &r.q),
                        _ => return Err(wrong()),
                    };
                    let places = precision.unwrap_or(6);
                    if places > 10000 { return Err(bad()); }
                    format!("{:.*}", places, number)
                }
                _ => return Err(bad()),
            };
            out.push_str(&filled);
        }
        if used != values.len() { return Err(wrong()); }
        Ok(out)
    }

    fn whole_bits(&self, value: &Value) -> Res<BigInt> {
        match value {
            Value::Small(n) => Ok(BigInt::from(*n)),
            Value::Huge(n) => Ok((**n).clone()),
            Value::Flag(b) => Ok(BigInt::from(i64::from(*b))),
            _ => Err(self.lang.bits_integer[0].clone()),
        }
    }

    fn wide_bits(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        let (left, right) = (self.whole_bits(a)?, self.whole_bits(b)?);
        let result = match op {
            Action::BitBoth => left & right,
            Action::BitEither => left | right,
            Action::BitOne => left ^ right,
            _ => {
                if right < BigInt::from(0) { return Err(self.lang.fault_shift.clone().unwrap_or_default()); }
                if matches!(op, Action::BitDown) && right >= BigInt::from(left.bits()) {
                    return Ok(Value::Small(if left < BigInt::from(0) { -1 } else { 0 }));
                }
                if left == BigInt::from(0) { return Ok(Value::Small(0)); }
                let count = right.to_usize().ok_or_else(|| self.lang.bits_beyond[0].clone())?;
                if matches!(op, Action::BitUp) { left << count } else { left >> count }
            }
        };
        if matches!((a, b), (Value::Flag(_), Value::Flag(_)))
            && matches!(op, Action::BitBoth | Action::BitEither | Action::BitOne) {
            Ok(Value::Flag(result != BigInt::from(0)))
        } else { Ok(Value::of_big(result)) }
    }

    fn mapping_equality(&self, left: &Value, right: &Value) -> bool {
        if let Value::Bond(cell) = left { return self.mapping_equality(&cell.borrow(), right); }
        if let Value::Bond(cell) = right { return self.mapping_equality(left, &cell.borrow()); }
        match (left, right) {
            (Value::Map(a), Value::Map(b)) => a.len() == b.len() && a.iter().all(|(key, value)| {
                b.iter().find(|(other, _)| self.mapping_equality(key, other))
                    .map_or(false, |(_, other)| self.mapping_equality(value, other))
            }),
            (Value::Array(a), Value::Array(b)) => a.len() == b.len()
                && a.iter().zip(b.iter()).all(|(x, y)| self.mapping_equality(x, y)),
            _ => left.equals(right),
        }
    }

    fn dyadic(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        if matches!(a, Value::Collection(..)) || matches!(b, Value::Collection(..)) { return self.dyadic(op, &a.contents(), &b.contents()); }
        if self.lang.arithmetic_flags && matches!(op, Action::Add | Action::Sub | Action::Mul | Action::Div | Action::DivReal | Action::IntDiv | Action::Mod | Action::Power | Action::Eq | Action::Ne | Action::Lt | Action::Le | Action::Gt | Action::Ge)
            && (matches!(a, Value::Flag(_)) || matches!(b, Value::Flag(_))) {
            let counted = |v: &Value| match v { Value::Flag(t) => Value::Small(i64::from(*t)), _ => v.clone() };
            return self.dyadic(op, &counted(a), &counted(b));
        }
        // An operand read in place may be a shared cell; what it holds is
        // what the operation works on.
        if let Value::Bond(shared) = a {
            let held = shared.borrow().clone();
            return self.dyadic(op, &held, b);
        }
        if let Value::Bond(shared) = b {
            let held = shared.borrow().clone();
            return self.dyadic(op, a, &held);
        }
        if let Action::SetWrite(how) = op {
            let plain = match how { 0 => Action::BitEither, 1 => Action::BitBoth, 2 => Action::Sub, _ => Action::BitOne };
            let answer = self.dyadic(&plain, a, b)?;
            if let (Value::Set(cell), Value::Set(result)) = (a, &answer) {
                *cell.borrow_mut() = result.borrow().clone();
                return Ok(a.clone());
            }
            return Ok(answer);
        }
        if matches!(a, Value::Set(_)) || matches!(b, Value::Set(_)) {
            let how = match op { Action::BitEither => Some(0), Action::BitBoth => Some(1), Action::Sub => Some(2), Action::BitOne => Some(3), _ => None };
            if how.is_some() || matches!(op, Action::Lt | Action::Le | Action::Gt | Action::Ge) {
                let (Value::Set(left), Value::Set(right)) = (a, b) else { return Err(self.set_said(".operands", "")); };
                let (left, right) = (left.borrow(), right.borrow());
                if let Some(how) = how { return Ok(Value::Set(Rc::new(RefCell::new(left.combine(&right, how))))); }
                let answer = match op {
                    Action::Le => left.beneath(&right),
                    Action::Lt => left.held.len() < right.held.len() && left.beneath(&right),
                    Action::Ge => right.beneath(&left),
                    _ => right.held.len() < left.held.len() && right.beneath(&left),
                };
                return Ok(Value::Flag(answer));
            }
        }
        if !matches!(op, Action::And | Action::Or | Action::Eq | Action::Ne | Action::Same | Action::Unsame) {
            if let Value::Imaginary(_, words) = a { return Err(words.to_string()); }
            if let Value::Imaginary(_, words) = b { return Err(words.to_string()); }
        }
        if !self.lang.whole_bits && matches!(op, Action::BitBoth | Action::BitEither | Action::BitOne) && !(matches!(op, Action::BitEither) && self.lang.or_maps && matches!((a, b), (Value::Map(_), Value::Map(_)))) {
            if let Some(words) = &self.lang.bit_operands {
                if ![a, b].iter().all(|v| matches!(v, Value::Small(_) | Value::Huge(_) | Value::Flag(_))) {
                    return Err(words.clone());
                }
                let (x, y) = (a.as_big()?, b.as_big()?);
                let bits = match op {
                    Action::BitBoth => x & y,
                    Action::BitEither => x | y,
                    _ => x ^ y,
                };
                return Ok(if matches!((a, b), (Value::Flag(_), Value::Flag(_))) {
                    Value::Flag(bits != BigInt::from(0))
                } else { Value::of_big(bits) });
            }
        }
        let sp = self.wording();
        let joined = || Value::text(&format!("{}{}", a.display(&sp), b.display(&sp)));
        Ok(match op {
            Action::Matrix => return Err(self.lang.matrix_unready.clone().unwrap_or_else(|| "Matrix multiplication cannot run".into())),
            Action::And => Value::Flag(self.truth(a) && self.truth(b)),
            Action::Or => Value::Flag(self.truth(a) || self.truth(b)),
            // The one no number answers to comes before nothing and
            // after nothing, so every question of which of two comes
            // first is answered no, whichever way it is put. It is
            // answered here rather than deeper down because the wider
            // questions are asked as the narrow one turned about.
            Action::Lt | Action::Le | Action::Gt | Action::Ge if a.no_number() || b.no_number() => Value::Flag(false),
            // Where a language has an operator for being the very same,
            // being equal is the looser question: text that spells a
            // number stands for that number, and a number met by text
            // that spells none is itself read as text.
            // Which of two comes first is asked the same loose way, so
            // that arrays, flags, nothing and text that spells no number
            // are all told apart as such a language tells them apart.
            Action::Lt | Action::Le | Action::Gt | Action::Ge if self.lang.loose_equality => {
                let first = |x: &Value, y: &Value| self.loosely_below(x, y);
                Value::Flag(match op {
                    Action::Lt => first(a, b)?,
                    Action::Gt => first(b, a)?,
                    Action::Le => !first(b, a)?,
                    _ => !first(a, b)?,
                })
            }
            Action::Eq | Action::Ne if self.lang.loose_equality => {
                let numeric = |v: &Value| v.sort().map_or(false, |k| matches!(k, Sort::Integer | Sort::Rational | Sort::Real));
                let nothing = |v: &Value| matches!(v, Value::Null | Value::Blank | Value::Gap | Value::Fence);
                let alike = match (a, b) {
                    // A flag on either side turns the question into
                    // whether the other side is true, and nothing
                    // counts as untrue.
                    (Value::Flag(_), _) | (_, Value::Flag(_)) => self.truth(a) == self.truth(b),
                    // Nothing whatever is equal to the one no number
                    // answers to, itself least of all, and text that
                    // spells its name no more than the rest. A flag is
                    // asked first, since a flag turns the question into
                    // whether the other side is true, and it is.
                    (x, y) if x.no_number() || y.no_number() => false,
                    (one, other) | (other, one) if nothing(one) && numeric(other) => !self.truth(other),
                    (one, Value::Text(s)) | (Value::Text(s), one) if nothing(one) => s.is_empty(),
                    (one, other) | (other, one) if nothing(one) && matches!(other, Value::Array(_) | Value::Map(_)) => !self.truth(other) || {
                        match other {
                            Value::Array(items) | Value::Tuple(items) => items.is_empty(),
                            Value::Map(pairs) => pairs.is_empty(),
                            _ => false,
                        }
                    },
                    // Two pieces of text that both spell numbers stand
                    // for those numbers.
                    (Value::Text(x), Value::Text(y)) => match (self.number_of(x), self.number_of(y)) {
                        (Some(m), Some(n)) => m.equals(&n),
                        _ => x == y,
                    },
                    (Value::Text(s), other) | (other, Value::Text(s)) if numeric(other) => match self.number_of(s) {
                        Some(n) => self.same_at_width(&n, other),
                        None => s.as_ref() == other.display(&sp),
                    },
                    // Two numbers are set against each other at the
                    // width the language holds them in, as they are
                    // worked at it: a whole number too wide for that
                    // width and the real it comes to are one number.
                    (x, y) if numeric(x) && numeric(y) => self.same_at_width(x, y),
                    _ => a.equals(b),
                };
                Value::Flag(matches!(op, Action::Eq) == alike)
            }
            Action::Eq | Action::Ne if self.lang.unordered_maps => {
                Value::Flag(self.mapping_equality(a, b) == matches!(op, Action::Eq))
            }
            Action::Eq => Value::Flag(a.equals(b)),
            Action::Ne => Value::Flag(!a.equals(b)),
            Action::Contains | Action::Lacks => {
                let found = match b {
                    Value::Counted(range) => a.as_big().ok().filter(|n| Self::member_matches(a, &Value::of_big(n.clone()))).map_or(false, |n| {
                        let delta = &n - &range.start;
                        let inside = if range.step > BigInt::from(0) { n >= range.start && n < range.stop }
                            else { n <= range.start && n > range.stop };
                        inside && delta % &range.step == BigInt::from(0)
                    }),
                    Value::Array(items) | Value::Tuple(items) => items.iter().any(|v| Self::member_matches(a, v)),
                    Value::Map(items) => items.iter().any(|(key, _)| Self::member_matches(a, key)),
                    Value::Set(s) => s.borrow().held.contains_key(&self.set_key(a)?),
                    Value::Text(haystack) => match a {
                        Value::Text(needle) => haystack.contains(needle.as_ref()),
                        _ => return Err(self.lang.membership_unsupported.clone().unwrap_or_default()),
                    },
                    _ => return Err(self.lang.membership_unsupported.clone().unwrap_or_default()),
                };
                Value::Flag(found != matches!(op, Action::Lacks))
            }
            Action::Mod if self.lang.rem_formats_text && matches!(a, Value::Text(_)) => {
                let Value::Text(template) = a else { unreachable!() };
                Value::text(&self.rem_text(template, b)?)
            }
            Action::Same | Action::Unsame if !self.lang.identity_not.is_empty() => {
                let same = match (a, b) {
                    (Value::Set(x), Value::Set(y)) => Rc::ptr_eq(x, y),
                    (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y),
                    (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y),
                    (Value::Null, Value::Null) | (Value::Ellipsis, Value::Ellipsis) => true,
                    (Value::Flag(x), Value::Flag(y)) => x == y,
                    (Value::Small(x), Value::Small(y)) if (-5..=256).contains(x) => x == y,
                    (Value::Object(x), Value::Object(y)) => Rc::ptr_eq(x, y),
                    _ if !a.identical(b) => false,
                    _ => return Err(self.lang.identity_unsupported.clone().unwrap_or_default()),
                };
                Value::Flag(same != matches!(op, Action::Unsame))
            }
            Action::Same => Value::Flag(a.identical(b)),
            Action::Unsame => Value::Flag(!a.identical(b)),
            Action::Join => joined(),
            Action::At => self.element(a, b, Reading::Plain)?,
            Action::Apart => self.element(a, b, Reading::Apart)?,
            Action::Toward => self.element(a, b, Reading::Toward)?,
            // Reaching inside makes the place on the way where nothing
            // is there yet, which is what a write to it means.
            Action::Nested if matches!(b, Value::Slice(_)) => {
                return Err(self.lang.slice_detached.clone().unwrap_or_default());
            }
            Action::Nested if self.lang.makes_places => {
                self.hushed.set(self.hushed.get() + 1);
                let found = self.element(a, b, Reading::Plain).unwrap_or(Value::Null);
                self.hushed.set(self.hushed.get() - 1);
                match found {
                    Value::Null | Value::Blank | Value::Gap => Value::Array(std::rc::Rc::new(Vec::new())),
                    held => held,
                }
            }
            Action::Nested => self.element(a, b, Reading::Plain)?,
            // Looking has nothing to say about what is not there.
            Action::Peek => {
                self.hushed.set(self.hushed.get() + 1);
                let found = self.element(a, b, Reading::Plain).unwrap_or(Value::Null);
                self.hushed.set(self.hushed.get() - 1);
                found
            }
            // Where a language asks equality loosely, ranking is asked
            // the same way, so that `<=>` and `<` never disagree about
            // two values.
            Action::Rank if self.lang.loose_equality => Value::Small(if self.dyadic(&Action::Eq, a, b)?.is_true() {
                0
            } else if self.loosely_below(a, b)? {
                -1
            } else {
                1
            }),
            Action::Rank => match crate::arith::order_values(a, b) {
                Some(std::cmp::Ordering::Less) => Value::Small(-1),
                Some(std::cmp::Ordering::Equal) => Value::Small(0),
                Some(std::cmp::Ordering::Greater) => Value::Small(1),
                None => Value::Small(match (a.display(&sp), b.display(&sp)) {
                    (x, y) if x < y => -1,
                    (x, y) if x > y => 1,
                    _ => 0,
                }),
            },
            // Adding text joins it only where the language has no
            // operator of its own for joining; where it has one, adding
            // is arithmetic and the text stands for a number.
            // Two pieces of text take their bits letter by letter, which
            // is what a language that spells these operators means by
            // them; the shorter side decides the length, save for `or`,
            // where the longer one stands on as it is.
            Action::BitEither if self.lang.or_maps && matches!((a, b), (Value::Map(_), Value::Map(_))) => {
                let (Value::Map(left), Value::Map(right)) = (a, b) else { unreachable!() };
                let mut merged = left.as_ref().clone();
                for (key, value) in right.iter() {
                    put_key(&mut merged, key.clone(), value.clone());
                }
                Value::Map(Rc::new(merged))
            }
            Action::BitBoth | Action::BitEither | Action::BitOne | Action::BitUp | Action::BitDown if self.lang.whole_bits => {
                let (x, y) = (self.whole_for_bits(a)?, self.whole_for_bits(b)?);
                let joined = match op {
                    Action::BitBoth => x & y,
                    Action::BitEither => x | y,
                    Action::BitOne => x ^ y,
                    _ => {
                        if y < BigInt::from(0) {
                            return Err(self.lang.fault_shift.clone().unwrap_or_else(|| "Bit shift by a negative number".to_string()));
                        }
                        if matches!(op, Action::BitDown) && y >= BigInt::from(x.bits()) {
                            BigInt::from(if x < BigInt::from(0) { -1 } else { 0 })
                        } else if x == BigInt::from(0) {
                            x
                        } else {
                            let by = y.to_usize().filter(|n| (*n as u128) + u128::from(x.bits()) < isize::MAX as u128)
                                .ok_or_else(|| self.lang.bit_room.clone().unwrap_or_else(|| "Bit shift count is too large".to_string()))?;
                            if matches!(op, Action::BitUp) { x << by } else { x >> by }
                        }
                    }
                };
                if matches!((a, b), (Value::Flag(_), Value::Flag(_)))
                    && matches!(op, Action::BitBoth | Action::BitEither | Action::BitOne)
                {
                    Value::Flag(joined != BigInt::from(0))
                } else {
                    Value::of_big(joined)
                }
            }
            Action::BitBoth | Action::BitEither | Action::BitOne | Action::BitUp | Action::BitDown if self.lang.bits_unbounded => self.wide_bits(op, a, b)?,
            Action::BitBoth | Action::BitEither | Action::BitOne if matches!(a, Value::Text(_)) && matches!(b, Value::Text(_)) => {
                let (x, y) = (a.display(&sp), b.display(&sp));
                let (x, y) = (self.lang.bytes_of(&x), self.lang.bytes_of(&y));
                let (x, y) = (x.as_slice(), y.as_slice());
                let mut out: Vec<u8> = Vec::new();
                let reach = if matches!(op, Action::BitEither) { x.len().max(y.len()) } else { x.len().min(y.len()) };
                for i in 0..reach {
                    let (p, q) = (x.get(i).copied().unwrap_or(0), y.get(i).copied().unwrap_or(0));
                    out.push(match op {
                        Action::BitBoth => p & q,
                        Action::BitEither => p | q,
                        _ => p ^ q,
                    });
                }
                Value::text(&String::from_utf8_lossy(&out))
            }
            // Working on the bits reads each side as a whole number of
            // sixty-four bits, sign and all, whatever it was written as.
            // A language whose shifts read each side for the number it
            // is worth stands apart: there text is left to the reading
            // further down — the reading a language with a word for a
            // warning gets, and the only one that never hands text back
            // — and this arm works on the numbers it gives.
            Action::BitBoth | Action::BitEither | Action::BitOne | Action::BitUp | Action::BitDown
                if !(self.lang.shift_by_number
                    && self.lang.warns_of_unwritten
                    && matches!(op, Action::BitUp | Action::BitDown)
                    && (matches!(a, Value::Text(_)) || matches!(b, Value::Text(_)))) =>
            {
                let (x, y) = (self.bits_said(a)?, self.bits_said(b)?);
                Value::Small(match op {
                    Action::BitBoth => x & y,
                    Action::BitEither => x | y,
                    Action::BitOne => x ^ y,
                    _ => match arith::moved_bits(matches!(op, Action::BitUp), x, y) {
                        Some(moved) => moved,
                        // A shift by a count below nought is no shift,
                        // and the language says so in its own words.
                        None => match &self.lang.fault_shift {
                            Some(words) => return Err(words.clone().into()),
                            None => return Err("Bit shift by a negative number".to_string()),
                        },
                    },
                })
            }
            // A step onward or back is adding or taking away one, save
            // on text that spells no number: a language may step such
            // text along its letters instead, or leave it as it stands,
            // and says so either way.
            Action::Step(onward) => {
                let said = match onward {
                    true => &self.lang.step_up_text,
                    false => &self.lang.step_down_text,
                };
                if let (Value::Text(letters), Some(words)) = (a, said) {
                    if number_spelled(letters).is_none() {
                        let words = words.clone();
                        self.complain(Complaint::Deprecated, &words);
                        return Ok(match onward {
                            true => Value::text(&letters_onward(letters)),
                            false => a.clone(),
                        });
                    }
                }
                let plain = match onward {
                    true => Action::Add,
                    false => Action::Sub,
                };
                return self.dyadic(&plain, a, b);
            }
            Action::Add if self.lang.concat.is_none() && (matches!(a, Value::Text(_)) || matches!(b, Value::Text(_))) => joined(),
            // Text that spells a number is worked with as that number,
            // fractions included, rather than only as a whole one. Text
            // that spells one and then goes on saying something else is
            // worth the number it opens with, and text that spells none
            // is worth nothing; a language with a word for a warning is
            // told of both rather than stopped.
            _ if matches!(a, Value::Text(_)) || matches!(b, Value::Text(_)) => {
                // A language may refuse text that spells no number at
                // all where a number is wanted, and name the kinds it
                // was handed instead of working with nothing.
                if let Some(words) = &self.lang.operand_fault {
                    let unnumbered = |v: &Value| matches!(v, Value::Text(s) if number_opening(s).0.is_none());
                    if unnumbered(a) || unnumbered(b) {
                        let told = format!("{}: {} {} {}", words, self.kind_named(a), self.written_as(op), self.kind_named(b));
                        return Err(told.into());
                    }
                }
                let read = |v: &Value| match v {
                    Value::Text(s) => Some(number_opening(s)),
                    _ => None,
                };
                let (x, y) = (read(a), read(b));
                if !self.lang.warns_of_unwritten {
                    let taken = |told: Option<(Option<Value>, bool)>, held: &Value| match told {
                        Some((Some(n), true)) => n,
                        _ => held.clone(),
                    };
                    let (x, y) = (taken(x, a), taken(y, b));
                    return self.dyadic_numbers(op, &x, &y);
                }
                let worth = |told: Option<(Option<Value>, bool)>, held: &Value| match told {
                    None => held.clone(),
                    Some((Some(n), true)) => n,
                    Some((Some(n), false)) => {
                        self.complain(Complaint::Warning, "A non-numeric value encountered");
                        n
                    }
                    Some((None, _)) => {
                        self.complain(Complaint::Warning, "A non-numeric value encountered");
                        Value::Small(0)
                    }
                };
                let (x, y) = (worth(x, a), worth(y, b));
                // Whatever is left is a number, so the whole question
                // is asked again: an operation the reading stood aside
                // for, a shift among them, now finds its own arm.
                return self.dyadic(op, &x, &y);
            }
            _ => return self.dyadic_numbers(op, a, b),
        })
    }

    /// A number brought within the widths the language holds numbers
    /// in: a whole number too wide to be one becomes a real, and a real
    /// is brought to the nearest one of its width.
    fn within_width(&self, v: Value) -> Value {
        if let (Some(bits), Value::Huge(n)) = (self.lang.integer_bits, &v) {
            if n.bits() >= bits as u64 {
                let places = self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
                return arith::shape_number((**n).clone(), BigInt::from(1), Some(places));
            }
        }
        self.at_real_width(v)
    }

    /// A real brought to the nearest one the language can hold. Where
    /// its reals are exact, it is left as it stands.
    fn at_real_width(&self, v: Value) -> Value {
        let places = self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
        crate::value::to_binary_width(v, self.lang.real_bits, places)
    }

    /// The arithmetic itself, both values already numbers as far as they
    /// can be made so. Where a language holds its reals to a width, a
    /// whole number meeting a real is brought to that width first, so
    /// that the two are added as such a language adds them.
    fn real_power(&self, a: &Value, b: &Value) -> Res<Option<Value>> {
        let as_number = |v: &Value| match v { Value::Flag(b) => Value::Small(i64::from(*b)), _ => v.clone() };
        let (a, b) = (as_number(a), as_number(b));
        let (Some(x), Some(y)) = (arith::Exact::from_value(&a), arith::Exact::from_value(&b)) else { return Ok(None) };
        if x.places.is_none() && y.places.is_none() && y.p >= BigInt::from(0) { return Ok(None); }
        let binary = |v: &Value, e: &arith::Exact| {
            if matches!(v, Value::Real(r) if r.below && r.p == BigInt::from(0)) { -0.0 }
            else { crate::value::as_binary(&e.p, &e.q) }
        };
        let (left, right) = (binary(&a, &x), binary(&b, &y));
        if left == 0.0 && right < 0.0 { return Err(self.lang.power_zero[0].clone()); }
        if left.is_finite() && left < 0.0 && right.is_finite() && right.fract() != 0.0 {
            return Err(self.lang.power_nonreal[0].clone());
        }
        let raised = left.powf(right);
        if raised.is_infinite() && left.is_finite() && right.is_finite() {
            return Err(self.lang.power_overflow[0].clone());
        }
        Ok(Some(crate::value::real_of(raised, arith::DEFAULT_PLACES)))
    }

    fn dyadic_numbers(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        if self.lang.power_real && matches!(op, Action::Power) {
            if let Some(answer) = self.real_power(a, b)? { return Ok(answer); }
        }
        if !self.lang.division_zero.is_empty() && matches!(op, Action::Div | Action::DivReal | Action::IntDiv | Action::Mod) {
            if arith::Exact::from_value(b).map_or(false, |e| e.p == BigInt::from(0) && e.q != BigInt::from(0)) || matches!(b, Value::Flag(false)) {
                let real = matches!(a, Value::Real(_)) || matches!(b, Value::Real(_));
                let words = match (op, real) {
                    (Action::Mod, true) => &self.lang.remainder_real_zero,
                    (Action::Mod, false) => return Err(self.lang.fault_modulo.clone().unwrap_or_default()),
                    (Action::IntDiv, true) => &self.lang.quotient_real_zero,
                    (Action::IntDiv, false) => &self.lang.quotient_zero,
                    _ => &self.lang.division_zero,
                };
                return Err(words[0].clone());
            }
        }
        // A language whose division gives a real may still give a whole
        // number where two whole ones divide evenly, which is what the
        // exact division answers with when they do.
        if self.lang.div_stays_whole && matches!(op, Action::DivReal) {
            let whole = |v: &Value| matches!(v, Value::Small(_) | Value::Huge(_));
            if whole(a) && whole(b) {
                if let Some(Ok(exact)) = arith::calculate(Operation::Over, a, b) {
                    if whole(&exact) {
                        return Ok(self.within_width(exact));
                    }
                }
            }
        }
        let real_here = |v: &Value| matches!(v, Value::Real(_) | Value::Frac(_));
        // A language whose remainder is taken between whole numbers
        // brings what it is given to one first, the way it brings a
        // value to the bits it works on.
        if self.lang.mod_whole && matches!(op, Action::Mod) && (real_here(a) || real_here(b)) {
            let (x, y) = (self.bits_said(a)?, self.bits_said(b)?);
            return self.dyadic_numbers(op, &Value::Small(x), &Value::Small(y));
        }
        if self.lang.real_bits.is_some() && (real_here(a) || real_here(b)) {
            let places = self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
            let widened = |v: &Value| match arith::to_real(v, places) {
                Some(real) => self.at_real_width(real),
                None => v.clone(),
            };
            let (a, b) = (widened(a), widened(b));
            return Ok(self.within_width(self.dyadic_exact(op, &a, &b)?));
        }
        Ok(self.within_width(self.dyadic_exact(op, a, b)?))
    }

    fn dyadic_exact(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        Ok(match op {
            Action::Add | Action::Sub | Action::Mul | Action::Div | Action::DivReal | Action::IntDiv | Action::Mod | Action::Power => {
                let calc = match op {
                    Action::Add => Operation::Plus,
                    Action::Sub => Operation::Minus,
                    Action::Mul => Operation::Times,
                    Action::Div => Operation::Over,
                    Action::DivReal => Operation::OverReal,
                    Action::IntDiv => Operation::Floor,
                    Action::Mod => Operation::Remainder,
                    _ => Operation::Raise,
                };
                let result = if self.lang.arithmetic_binary { arith::binary_work(calc, a, b) } else { None };
                match result.or_else(|| arith::calculate(calc, a, b)) {
                    // A language may tell taking the remainder by
                    // nought apart from dividing by it, and word the
                    // one its own way. The class is the same either
                    // way, so it is the words alone that are put in.
                    Some(Err(told)) if matches!(op, Action::Mod) && told == "Division by zero" => {
                        return Err(self.lang.fault_modulo.clone().unwrap_or(told).into())
                    }
                    Some(r) => r?,
                    // The closed operations coerce booleans and null.
                    None => match calc {
                        Operation::Plus => Value::of_big(a.as_big()? + b.as_big()?),
                        Operation::Minus => Value::of_big(a.as_big()? - b.as_big()?),
                        Operation::Times => Value::of_big(a.as_big()? * b.as_big()?),
                        Operation::Over | Operation::OverReal => return Err("Division requires numeric both".to_string()),
                        Operation::Floor => return Err("Integer quotient requires numeric both".to_string()),
                        Operation::Remainder => return Err("Modulo requires numeric both".to_string()),
                        Operation::Raise => return Err("Exponentiation requires numeric both".to_string()),
                    },
                }
            }
            Action::Lt | Action::Le | Action::Gt | Action::Ge => {
                // From less-than alone: a > b is b < a, a <= b is not b < a.
                let below = |x: &Value, y: &Value| -> Res<bool> {
                    // Nothing comes before what no number answers to,
                    // and it comes before nothing.
                    if x.no_number() || y.no_number() {
                        return Ok(false);
                    }
                    match arith::order_values(x, y) {
                        Some(order) => Ok(order == std::cmp::Ordering::Less),
                        None => Ok(x.as_big()? < y.as_big()?),
                    }
                };
                Value::Flag(match op {
                    Action::Lt => below(a, b)?,
                    Action::Gt => below(b, a)?,
                    Action::Le => !below(b, a)?,
                    _ => !below(a, b)?,
                })
            }
            other => return Err(format!("{:?} is not a two-value operation", other)),
        })
    }

    /// The keys and values of an array, a place's number standing for
    /// its key where the places are unnamed.
    fn keyed_places(v: &Value) -> Vec<(Value, Value)> {
        match v {
            Value::Array(items) | Value::Tuple(items) => items.iter().enumerate().map(|(i, x)| (Value::Small(i as i64), x.clone())).collect(),
            Value::Map(pairs) => pairs.as_ref().clone(),
            _ => Vec::new(),
        }
    }

    /// Which of two values comes first, asked the loose way a language
    /// with an operator for being the very same means by it. A flag or
    /// nothing on either side turns the question into which of the two
    /// is true; an array stands above whatever is not an array, and two
    /// arrays are told apart by how much they hold and then place by
    /// place; text that spells a number stands for that number, and a
    /// number met by text that spells none is itself read as text.
    fn loosely_below(&self, a: &Value, b: &Value) -> Res<bool> {
        let sp = self.wording();
        let numeric = |v: &Value| v.sort().map_or(false, |k| matches!(k, Sort::Integer | Sort::Rational | Sort::Real));
        let nothing = |v: &Value| matches!(v, Value::Null | Value::Blank | Value::Gap | Value::Fence);
        let listed = |v: &Value| matches!(v, Value::Array(_) | Value::Map(_));
        // Two numbers are set against each other at the width the
        // language holds them in, as they are worked at it.
        let exactly = |x: &Value, y: &Value| -> Res<bool> { Ok(self.dyadic_numbers(&Action::Lt, x, y)?.is_true()) };
        Ok(match (a, b) {
            (Value::Flag(_), _) | (_, Value::Flag(_)) => !self.truth(a) && self.truth(b),
            // Nothing met by text is the empty piece of text, which
            // nothing comes before.
            (one, Value::Text(s)) if nothing(one) => !s.is_empty(),
            (Value::Text(_), other) if nothing(other) => false,
            (one, other) if nothing(one) || nothing(other) => !self.truth(one) && self.truth(other) && nothing(one),
            (x, y) if listed(x) && listed(y) => {
                let (left, right) = (Self::keyed_places(x), Self::keyed_places(y));
                if left.len() != right.len() {
                    return Ok(left.len() < right.len());
                }
                for (key, held) in left {
                    let Some((_, theirs)) = right.iter().find(|(k, _)| k.equals(&key)) else {
                        // A place the other does not name leaves the two
                        // beyond telling apart, and an array is not below
                        // what it cannot be told from.
                        return Ok(false);
                    };
                    if !self.dyadic(&Action::Eq, &held, theirs)?.is_true() {
                        return self.loosely_below(&held, theirs);
                    }
                }
                false
            }
            (x, _) if listed(x) => false,
            (_, y) if listed(y) => true,
            (Value::Text(x), Value::Text(y)) => match (self.number_of(x), self.number_of(y)) {
                (Some(m), Some(n)) => exactly(&m, &n)?,
                _ => x.as_ref() < y.as_ref(),
            },
            (Value::Text(s), other) if numeric(other) => match self.number_of(s) {
                Some(n) => exactly(&n, other)?,
                None => s.as_ref() < other.display(&sp).as_str(),
            },
            (other, Value::Text(s)) if numeric(other) => match self.number_of(s) {
                Some(n) => exactly(other, &n)?,
                None => other.display(&sp).as_str() < s.as_ref(),
            },
            _ => exactly(a, b)?,
        })
    }

    /// A source read in and run: text given outright, or a file
    /// sought beside the one asking for it before it is sought where
    /// the run began.
    fn read_in(&mut self, builtin: Builtin, name: &str, args: &[Value]) -> Res<Value> {
                if args.len() != 1 {
                    return Err(format!("{}() expects 1 argument, got {}", name, args.len()));
                }
                let sp = self.wording();
                let given = args[0].display(&sp);
                let mut came_from = None;
                let source = match builtin {
                    // Text given to be run is code already, where a
                    // file is text with code marked out inside it, so
                    // the mark that opens code is put before the one
                    // and not the other.
                    Builtin::Eval => match &self.lang.prologue {
                        Some(open) => format!("{}\n{}", open, given),
                        None => given,
                    },
                    // A file is looked for beside the one asking for it
                    // before it is looked for where the run was started,
                    // since a program naming a file beside itself means
                    // the one beside itself.
                    _ => {
                        let beside = std::path::Path::new(self.source.as_ref()).parent().map(|near| near.join(&given));
                        let near = beside.filter(|near| near.exists());
                        came_from = Some(match &near {
                            Some(place) => place.to_string_lossy().into_owned(),
                            None => given.clone(),
                        });
                        let found = match near {
                            Some(place) => std::fs::read(place),
                            None => std::fs::read(&given),
                        };
                        // A file asked for only once is read the first
                        // time and passed over after, whichever name it
                        // was asked for under, since it is the file and
                        // not the name that stands.
                        if builtin == Builtin::IncludeOnce {
                            let place = came_from.clone().unwrap_or_else(|| given.clone());
                            let whole = std::fs::canonicalize(&place).map(|p| p.to_string_lossy().into_owned()).unwrap_or(place);
                            if !self.read_already.borrow_mut().insert(whole) {
                                return Ok(Value::Flag(true));
                            }
                        }
                        match found {
                            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                            // A file that cannot be read is not there as
                            // far as the run is concerned: it says so of
                            // the file, and then either stops, where the
                            // word will not go on without it, or says so
                            // of the reading in besides and answers
                            // false.
                            Err(_) => {
                                let page = self.page_for(name);
                                self.complain(Complaint::Warning, &format!("{}(){}: Failed to open stream: No such file or directory", name, page));
                                let demanded = self.lang.include_demanded.iter().any(|word| word == name);
                                if let (true, Some((before, after))) = (demanded, &self.lang.include_demanded_missing) {
                                    return Err(format!("{}{}{}", before, given, after));
                                }
                                let told = format!("{}(){}: Failed opening '{}' for inclusion (include_path='.')", name, page, given);
                                self.complain(Complaint::Warning, &told);
                                return Ok(Value::Flag(false));
                            }
                        }
                    }
                };
                return self.run_source(&source, came_from);
    }

    /// A key as this language takes one.
    fn key(&self, at: &Value) -> Value {
        // Naming a place by nothing at all names the place the empty
        // text names, which a language may ask to be written out.
        if let (Value::Null | Value::Blank | Value::Gap, Some(said)) = (at, &self.lang.nothing_index) {
            self.said_anyway(Complaint::Deprecated, &said.clone());
        }
        self.key_quietly(at)
    }

    /// The same, with nothing said about it: how a place is named where
    /// it is being taken away rather than read or written.
    fn key_quietly(&self, at: &Value) -> Value {
        match self.lang.plain_keys {
            true => key_taken(at.clone()),
            false => at.clone(),
        }
    }

    /// How a value is named where a complaint names its kind. A flag is
    /// named by the word a program writes for it, since that is what
    /// was written; anything else by its kind, in the shorter form
    /// where the language gives one. Nothing is named by its kind too,
    /// since a language may write it as no word at all and a complaint
    /// still has to name it.
    fn kind_named(&self, v: &Value) -> String {
        if let Value::Flag(_) = v {
            // The word, whatever a flag becomes where text is wanted:
            // a complaint names what was written, not what it counts as.
            let mut sp = self.wording();
            sp.flag_counts = false;
            return v.display(&sp);
        }
        let Some(kind) = v.sort() else { return "value".to_string() };
        let at = [Sort::Integer, Sort::Rational, Sort::Real, Sort::Text, Sort::Boolean, Sort::Array, Sort::Null]
            .iter()
            .position(|k| *k == kind);
        let brief = at.and_then(|i| self.lang.brief_kinds.get(i)).filter(|word| *word != "-");
        match brief {
            Some(word) => word.clone(),
            None => self.lang.sort_bindings.iter().find(|(_, k)| *k == kind).map_or("value".to_string(), |(n, _)| n.clone()),
        }
    }

    /// How an operation is written in this language. Where more than
    /// one way of writing it stands for the same, the shortest is the
    /// one a complaint names, and the first in order among those.
    fn written_as(&self, op: &Action) -> String {
        let same = |one: &Action| std::mem::discriminant(one) == std::mem::discriminant(op);
        let mut ways: Vec<&String> = self.lang.dyadic.iter().filter(|(_, o)| same(&o.action)).map(|(lex, _)| lex).collect();
        ways.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
        ways.first().map_or_else(|| "?".to_string(), |lex| (*lex).clone())
    }

    /// How a key an array does not hold is named in a complaint: text
    /// in quotation marks, a number as it stands.
    fn key_named(&self, at: &Value) -> String {
        match at {
            Value::Text(s) => format!("\"{}\"", s),
            other => other.plain(),
        }
    }

    fn element(&self, target: &Value, at: &Value, how: Reading) -> Res<Value> {
        if matches!(target, Value::Collection(..) | Value::Bond(_) | Value::View(_)) { return self.element(&target.contents(), at, how); }
        if let Some(cell) = self.walked_set(target)? {
            let held = cell.borrow();
            let index = as_index(at)?;
            return held.row.get(index).map(|key| held.held[key].clone()).ok_or_else(|| self.set_said(".missing", &index.to_string()));
        }
        if let Value::Counted(r) = target {
            if matches!(at, Value::Slice(_)) { return Err(self.lang.slice_unsupported.clone().unwrap_or_default()); }
            let index = match at {
                Value::Small(n) => BigInt::from(*n), Value::Huge(n) => (**n).clone(),
                Value::Flag(b) => BigInt::from(i64::from(*b)),
                _ => return Err(self.lang.range_integer[0].clone()),
            };
            return r.at(index).ok_or_else(|| self.lang.range_index[0].clone());
        }
        // A place holding a cell two names share reads as what the cell
        // holds, since the sharing is between the names and not
        // something the value itself carries.
        let seen = |v: Value| match v {
            Value::Bond(shared) if !self.lang.bind_names => shared.borrow().clone(),
            held => held,
        };
        return self.element_held(target, at, how).map(seen);
    }

    /// Bring the bounds within the row before walking it. A missing
    /// last bound on a backward walk lies before the first place;
    /// an expressly written minus one lies at the last place instead.
    fn slice_places(&self, parts: &[Value; 3], size: usize) -> Res<(usize, usize, i128, Vec<usize>)> {
        let count = |v: &Value| -> Res<Option<i128>> {
            Ok(match v {
                Value::Null => None,
                Value::Small(n) => Some(*n as i128),
                Value::Flag(b) => Some(i128::from(*b)),
                Value::Huge(n) => Some(n.to_i128().unwrap_or_else(|| {
                    if n.sign() == num_bigint::Sign::Minus { i128::MIN } else { i128::MAX }
                })),
                _ => return Err(self.lang.slice_bounds.clone().unwrap_or_default()),
            })
        };
        let stride = count(&parts[2])?.unwrap_or(1);
        if stride == 0 {
            return Err(self.lang.slice_zero.clone().unwrap_or_default());
        }
        let length = size as i128;
        let backwards = stride < 0;
        let lower = if backwards { -1 } else { 0 };
        let upper = if backwards { length - 1 } else { length };
        let bound = |v: &Value, absent: i128| -> Res<i128> {
            Ok(match count(v)? {
                None => absent,
                Some(n) => {
                    let n = if n < 0 { n.saturating_add(length) } else { n };
                    n.clamp(lower, upper)
                }
            })
        };
        let start = bound(&parts[0], if backwards { length - 1 } else { 0 })?;
        let stop = bound(&parts[1], if backwards { -1 } else { length })?;
        let mut places = Vec::new();
        let mut at = start;
        while if backwards { at > stop } else { at < stop } {
            places.push(at as usize);
            at = at.saturating_add(stride);
        }
        Ok((start.max(0) as usize, stop.max(start).max(0) as usize, stride, places))
    }

    fn read_slice(&self, target: &Value, parts: &[Value; 3]) -> Res<Value> {
        match target {
            Value::Array(items) | Value::Tuple(items) => {
                let (_, _, _, places) = self.slice_places(parts, items.len())?;
                let selected = places.into_iter().map(|i| items[i].clone()).collect();
                Ok(if matches!(target, Value::Tuple(_)) { Value::Tuple(Rc::new(selected)) } else { Value::array(selected) })
            }
            Value::Text(text) if self.lang.text_indexable => {
                let letters: Vec<char> = text.chars().collect();
                let (_, _, _, places) = self.slice_places(parts, letters.len())?;
                Ok(Value::text(&places.into_iter().map(|i| letters[i]).collect::<String>()))
            }
            _ => Err(self.lang.slice_unsupported.clone().unwrap_or_default()),
        }
    }

    fn write_slice(&self, target: Value, parts: &[Value; 3], given: Value) -> Res<Value> {
        let given = collection_contents(&given);
        let Value::Array(mut items) = target else {
            return Err(self.lang.slice_unsupported.clone().unwrap_or_default());
        };
        let (start, stop, step, places) = self.slice_places(parts, items.len())?;
        let replacement = match given.contents() {
            Value::Set(items) => items.borrow().items(),
            Value::Array(values) | Value::Tuple(values) => values.as_ref().clone(),
            Value::Text(text) => text.chars().map(|c| Value::text(&c.to_string())).collect(),
            Value::Map(pairs) => pairs.iter().map(|(key, _)| key.clone()).collect(),
            _ => return Err(self.lang.slice_assign.clone().unwrap_or_default()),
        };
        if step == 1 {
            Rc::make_mut(&mut items).splice(start..stop, replacement);
        } else {
            if replacement.len() != places.len() {
                let words = &self.lang.slice_length;
                return Err(format!("{} {} {} {}", words.first().map_or("", String::as_str), replacement.len(),
                    words.get(1).map_or("", String::as_str), places.len()));
            }
            let row = Rc::make_mut(&mut items);
            for (at, value) in places.into_iter().zip(replacement) {
                row[at] = value;
            }
        }
        Ok(Value::Array(items))
    }

    fn element_held(&self, target: &Value, at: &Value, how: Reading) -> Res<Value> {
        if matches!(target, Value::Set(_)) { return Err(self.core_fault("core.unindexable", &target.core_kind())); }
        if let Value::Slice(parts) = at {
            return self.read_slice(target, parts);
        }
        // A language may say that a place an array does not hold reads as
        // nothing rather than stopping the program, and one with a word
        // for a warning says so before reading nothing there.
        let absent = |told: String, key: &Value| {
            if !self.lang.absent_index {
                return Err(told);
            }
            self.complain(Complaint::Warning, &format!("Undefined array key {}", self.key_named(key)));
            Ok(Value::Null)
        };
        // Looking into nothing at all is told apart from looking for a
        // place an array does not hold: there is no array to hold it.
        if self.lang.absent_index && matches!(target, Value::Null | Value::Blank | Value::Gap) {
            self.complain(Complaint::Warning, "Trying to access array offset on null");
            return Ok(Value::Null);
        }
        if let Value::Map(pairs) = target {
            let at = &self.key(at);
            let found = pairs.iter().find(|(k, _)| k.equals(at));
            return match found {
                Some((_, v)) => Ok(v.clone()),
                None => absent(format!("Undefined array key {}", at.plain()), at),
            };
        }
        // Where text is a row of places, a place named by text is the
        // number that text opens with, as text counts as a number
        // anywhere else it is asked to.
        let named = match (target, at) {
            (Value::Text(_), Value::Text(spelling)) if self.lang.text_places => {
                let (opens, whole) = number_opening(spelling);
                // Text naming a place that is not simply a number names
                // the number it opens with, and the run says as much
                // rather than reading it quietly.
                if !whole && opens.is_some() {
                    self.complain(Complaint::Warning, &format!("Illegal string offset \"{}\"", spelling));
                }
                opens.unwrap_or_else(|| at.clone())
            }
            _ => at.clone(),
        };
        let i = match as_index(&named) {
            Ok(i) => i,
            Err(told) => return absent(told, at),
        };
        match target {
            Value::Array(items) | Value::Tuple(items) => match items.get(i) {
                Some(v) => Ok(v.clone()),
                None => absent(format!("Array index {} out of bounds (length: {})", i, items.len()), at),
            },
            Value::Text(s) if self.lang.text_indexable => s
                .chars()
                .nth(i)
                .map(|c| Value::text(&c.to_string()))
                .ok_or_else(|| format!("String index {} out of bounds (length: {})", i, s.chars().count())),
            // A value with no places at all. A language reading a place
            // an array does not hold as nothing reads a place of such a
            // value the same way, naming the kind it was asked of; a
            // thing is another matter and is refused.
            _ if self.lang.absent_index && how != Reading::Toward && !matches!(target, Value::Object(_)) => {
                let kind = self.kind_named(target);
                let told = match how {
                    Reading::Apart => format!("Cannot use {} as array", kind),
                    _ => format!("Trying to access array offset on {}", kind),
                };
                self.complain(Complaint::Warning, &told);
                Ok(Value::Null)
            }
            _ => Err(self.not_an_array()),
        }
    }

    /// What a language calls using a value with no places at all as
    /// though it had them.
    fn not_an_array(&self) -> String {
        match &self.lang.scalar_index {
            Some(said) => said.clone(),
            None => "Cannot index non-array value".to_string(),
        }
    }

    // ---------- builtins ----------

    /// Membership asks identity before equality, and counts truth as one.
    fn member_matches(a: &Value, b: &Value) -> bool {
        if let Value::Bond(cell) = a { return Self::member_matches(&cell.borrow(), b); }
        if let Value::Bond(cell) = b { return Self::member_matches(a, &cell.borrow()); }
        match (a, b) {
            (Value::Flag(x), _) => Self::member_matches(&Value::Small(i64::from(*x)), b),
            (_, Value::Flag(y)) => Self::member_matches(a, &Value::Small(i64::from(*y))),
            (Value::Real(x), Value::Real(y)) if Rc::ptr_eq(x, y) => true,
            (Value::Array(x), Value::Array(y)) => Rc::ptr_eq(x, y)
                || (x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| Self::member_matches(p, q))),
            (Value::Map(x), Value::Map(y)) => Rc::ptr_eq(x, y)
                || (x.len() == y.len() && x.iter().all(|(k, v)| y.iter().any(|(l, w)| Self::member_matches(k, l) && Self::member_matches(v, w)))),
            _ => a.equals(b),
        }
    }

    /// The walk keeps its first size beside the shared members.
    fn set_walk(&self, source: &Value) -> Option<Value> {
        if matches!(source, Value::SetWalk(..)) { return Some(source.clone()); }
        let Value::Set(cell) = source else { return None };
        Some(Value::SetWalk(cell.clone(), cell.borrow().held.len()))
    }

    fn walked_set(&self, value: &Value) -> Res<Option<Rc<RefCell<crate::value::Members>>>> {
        if let Value::SetWalk(cell, count) = value {
            if *count != cell.borrow().held.len() { return Err(self.set_said(".changed", "")); }
            return Ok(Some(cell.clone()));
        }
        Ok(None)
    }

    fn set_said(&self, label: &str, piece: &str) -> String {
        let words = &self.lang.set_words[&format!("ext.builtin.set{}", label)];
        format!("{}{}{}", words.first().map_or("", String::as_str), piece, words.get(1).map_or("", String::as_str))
    }

    fn set_key(&self, value: &Value) -> Res<String> {
        value.member_key().map_err(|kind| self.set_said(if kind.is_empty() { ".unsupported" } else { ".unhashable" }, kind))
    }

    fn set_from(&self, items: Vec<Value>) -> Res<crate::value::Members> {
        let mut set = crate::value::Members::empty(self.lang.set_words["ext.builtin.set"].first().cloned().unwrap_or_default());
        for item in items { set.insert(self.set_key(&item)?, item); }
        Ok(set)
    }

    fn set_builtin(&mut self, op: Builtin, args: &[Value]) -> Res<Value> {
        use Builtin::*;
        if op == SetMake {
            if args.len() > 1 { return Err(self.set_said(".arguments", "")); }
            let items = if args.is_empty() { Vec::new() } else { self.comprehension_items(&args[0])? };
            return Ok(Value::Set(Rc::new(RefCell::new(self.set_from(items)?))));
        }
        if op == SetSorted {
            if args.len() != 1 { return Err(self.set_said(".arguments", "")); }
            let mut items = self.comprehension_items(&args[0])?;
            for next in 1..items.len() {
                let mut at = next;
                while at > 0 {
                    let order = match (&items[at], &items[at - 1]) {
                        (Value::Text(a), Value::Text(b)) => a.cmp(b),
                        (a, b) => {
                            let number = |v: &Value| match v { Value::Flag(b) => Value::Small(i64::from(*b)), _ => v.clone() };
                            arith::order_values(&number(a), &number(b)).ok_or_else(|| self.set_said(".unsortable", ""))?
                        }
                    };
                    if order != std::cmp::Ordering::Less { break; }
                    items.swap(at, at - 1);
                    at -= 1;
                }
            }
            return Ok(Value::array(items));
        }
        let Some(Value::Set(cell)) = args.first() else { return Err(self.set_said(".operands", "")); };
        let unary = matches!(op, SetPop | SetClear | SetCopy);
        let many = matches!(op, SetUpdate | SetUnion | SetIntersection | SetDifference | SetMeetUpdate | SetLessUpdate);
        if (unary && args.len() != 1) || (!unary && !many && args.len() != 2) {
            return Err(self.set_said(".arguments", ""));
        }
        if matches!(op, SetAdd | SetRemove | SetDiscard) {
            let key = self.set_key(&args[1])?;
            if op == SetAdd { cell.borrow_mut().insert(key, args[1].clone()); }
            else if cell.borrow_mut().remove(&key).is_none() && op == SetRemove {
                return Err(self.set_said(".missing", &args[1].member_text(&self.wording())));
            }
            return Ok(Value::Null);
        }
        if op == SetPop {
            let key = cell.borrow().row.first().cloned().ok_or_else(|| self.set_said(".empty", ""))?;
            return Ok(cell.borrow_mut().remove(&key).unwrap());
        }
        if op == SetClear {
            let mut set = cell.borrow_mut();
            set.row.clear();
            set.held.clear();
            return Ok(Value::Null);
        }
        if op == SetUpdate {
            for source in &args[1..] {
                for item in self.comprehension_items(source)? { cell.borrow_mut().insert(self.set_key(&item)?, item); }
            }
            return Ok(Value::Null);
        }
        let mut result = cell.borrow().clone();
        for other in &args[1..] {
            let items = self.comprehension_items(other)?;
            let rhs = self.set_from(items)?;
            let comparison = match op {
                SetSubset => Some(result.beneath(&rhs)),
                SetSuperset => Some(rhs.beneath(&result)),
                SetDisjoint => Some(result.held.keys().all(|k| !rhs.held.contains_key(k))),
                _ => None,
            };
            if let Some(answer) = comparison { return Ok(Value::Flag(answer)); }
            let how = match op {
                SetIntersection | SetMeetUpdate => 1,
                SetDifference | SetLessUpdate => 2,
                SetSymmetric | SetXorUpdate => 3,
                _ => 0,
            };
            result = result.combine(&rhs, how);
        }
        if matches!(op, SetUpdate | SetMeetUpdate | SetLessUpdate | SetXorUpdate) {
            *cell.borrow_mut() = result;
            Ok(Value::Null)
        } else { Ok(Value::Set(Rc::new(RefCell::new(result)))) }
    }

    /// The collections this reader can walk without asking a protocol.
    fn comprehension_items(&mut self, value: &Value) -> Res<Vec<Value>> {
        match value {
            Value::Generator(held) => {
                let mut items = Vec::new();
                while let Some(item) = self.resume_generator(held, Value::Null).map_err(|f| f.told(&self.wording()))? { items.push(item); }
                Ok(items)
            }
            Value::Tuple(items) | Value::Array(items) => Ok(items.as_ref().clone()),
            Value::Collection(cell, _) => self.comprehension_items(&cell.borrow()),
            Value::View(_) => self.comprehension_items(&value.contents()),
            Value::Cursor(_) => self.core_members(value),
            Value::Set(s) => Ok(s.borrow().items()),
            Value::Map(pairs) => Ok(pairs.iter().map(|(k, _)| match k { Value::Hashed(p) => p.0.clone(), _ => k.clone() }).collect()),
            Value::Text(text) => Ok(text.chars().map(|c| Value::text(&c.to_string())).collect()),
            Value::Counted(range) => {
                let mut items = Vec::new();
                let mut next = range.start.clone();
                let forward = range.step > BigInt::from(0);
                while if forward { next < range.stop } else { next > range.stop } {
                    items.push(Value::of_big(next.clone()));
                    next += &range.step;
                }
                Ok(items)
            }
            Value::Bond(cell) => self.comprehension_items(&cell.borrow()),
            _ => Err(self.lang.collection_unwalkable.first().cloned().unwrap_or_else(|| "Value has no members to gather".to_string())),
        }
    }

    /// print and write: the values joined by spaces, or a template holding
    /// the definition's placeholders filled from the rest.
    fn render(&self, values: &[Value]) -> String {
        let sp = self.wording();
        let printed = |v: &Value| {
            let mut said = v.display(&sp);
            if self.lang.print_real_point && v.keeps_point()
                && said.chars().all(|c| c.is_ascii_digit() || c == '-')
            {
                said.push_str(".0");
            }
            said
        };
        let holes = &self.lang.holes;
        let hole_in = |s: &str| holes.iter().filter_map(|h| s.find(h.as_str()).map(|at| (at, h.len()))).min();
        if let (Some(Value::Text(template)), true) = (values.first(), values.len() > 1) {
            if hole_in(template).is_some() {
                let mut out = String::new();
                let mut fill = values[1..].iter();
                let mut s: &str = template;
                while let Some((at, width)) = hole_in(s) {
                    out.push_str(&s[..at]);
                    match fill.next() {
                        Some(v) => out.push_str(&printed(v)),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
        values.iter().map(printed).collect::<Vec<_>>().join(" ")
    }

    /// Builtins take the same opened arguments as a declared routine,
    /// but each names its own few places, where the definition spells them.
    fn builtin_call(&mut self, builtin: Builtin, name: &str, items: Vec<(Option<String>, Value)>) -> Res<Value> {
        let mut args = Vec::new();
        let mut named: Vec<(String, Value)> = Vec::new();
        for (key, value) in items {
            if let Some(key) = key {
                if named.iter().any(|(seen, _)| seen == &key) {
                    return Err(Self::named_fault(&self.lang.call_duplicate, &key));
                }
                named.push((key, value));
            } else { args.push(value); }
        }
        if builtin == Builtin::Say && !self.lang.print_sep.is_empty() {
            let mut between = " ".to_string();
            let mut ending = "\n".to_string();
            let mut error = false;
            for (key, value) in named {
                if Lang::spells(&self.lang.print_sep, &key) || Lang::spells(&self.lang.print_end, &key) {
                    let sep = Lang::spells(&self.lang.print_sep, &key);
                    let text = match value {
                        Value::Null => continue,
                        Value::Text(s) => s.to_string(),
                        _ => return Err(if sep { self.lang.print_sep_amiss[0].clone() } else { self.lang.print_end_amiss[0].clone() }),
                    };
                    if sep { between = text; } else { ending = text; }
                } else if Lang::spells(&self.lang.print_file, &key) {
                    error = match value {
                        Value::Null | Value::Stream(false) => false,
                        Value::Stream(true) => true,
                        _ => return Err(self.lang.print_file_unready[0].clone()),
                    };
                } else if !Lang::spells(&self.lang.print_flush, &key) {
                    return Err(Self::named_fault(&self.lang.call_unknown, &key));
                }
            }
            let mut parts = Vec::new();
            for value in &args { parts.push(self.special_text(value, false)?); }
            let text = parts.join(&between) + &ending;
            if error { eprint!("{}", text); } else { self.utter(&text); }
            return Ok(Value::Null);
        }
        if Self::core_builtin(builtin) { return self.core_call(builtin, name, args, named); }
        for (key, value) in named {
            let place = if builtin == Builtin::Sum && self.lang.core_words.get("start").map_or(false, |words| Lang::spells(words, &key)) {
                1
            } else if builtin == Builtin::ToInt && Lang::spells(&self.lang.to_int_base, &key) {
                1
            } else if builtin == Builtin::ToText && Lang::spells(&self.lang.to_string_object, &key) {
                0
            } else if builtin == Builtin::ToText && (Lang::spells(&self.lang.to_string_encoding, &key) || Lang::spells(&self.lang.to_string_errors, &key)) {
                return Err(self.lang.to_string_unready[0].clone());
            } else {
                let words = &self.lang.call_builtin_amiss;
                return Err(if words.len() > 1 { Self::named_fault(words, name.rsplit('.').next().unwrap_or(name)) }
                    else { words.first().cloned().unwrap_or_default() });
            };
            if args.len() > place { return Err(Self::named_fault(&self.lang.call_duplicate, &key)); }
            if args.len() < place { return Err(self.lang.call_amiss[0].clone()); }
            args.push(value);
        }
        self.builtin(builtin, name, &mut args)
    }

    fn value_method(&mut self, receiver: &Value, operation: &str, args: Vec<Value>, named: Vec<(String, Value)>) -> Res<Value> {
        if matches!(receiver.contents(), Value::Set(_)) {
            let method = match operation { "remove" => Some(Builtin::SetRemove), "pop" => Some(Builtin::SetPop), "clear" => Some(Builtin::SetClear), "copy" => Some(Builtin::SetCopy), "update" => Some(Builtin::SetUpdate), _ => None };
            if let Some(method) = method {
                if !named.is_empty() { return Err(self.set_said(".arguments", "")); }
                let mut given = vec![receiver.contents()]; given.extend(args);
                return self.set_builtin(method, &given);
            }
        }
        if !named.is_empty() && !matches!(operation, "sort" | "split" | "rsplit" | "format" | "update" | "encode") {
            let name = self.lang.value_methods.iter().find(|(_, op)| op.as_str() == operation).map(|(word, _)| word.as_str()).unwrap_or(operation);
            return Err(Self::named_fault(&self.lang.call_builtin_amiss, name));
        }
        let mut used = std::collections::HashSet::new();
        if named.iter().any(|(key,_)| !used.insert(key)) { return Err(self.lang.method_errors["arguments"].clone()); }

        let named = if matches!(operation, "split" | "rsplit") {
            named.into_iter().map(|(key,value)| {
                let key = self.lang.method_keywords.get(&key).cloned().unwrap_or_default();
                (key, value)
            }).collect()
        } else { named };
        if operation == "sort" {
            if !args.is_empty() || !matches!(receiver.contents(), Value::Array(_)) { return Err(self.lang.method_errors["arguments"].clone()); }
            let row = self.order_values(receiver, &named)?;
            let Value::Collection(cell, _) = receiver else { return Err(self.lang.method_errors["unready"].clone()); };
            *cell.borrow_mut() = Value::array(row);
            return Ok(Value::Null);
        }
        crate::methods::call(receiver, operation, &args, &named, &self.wording(), &|key| self.lang.method_errors[key].clone())
    }

    fn order_values(&mut self, source: &Value, named: &[(String, Value)]) -> Res<Vec<Value>> {
        let mut backwards = false;
        let mut key = Value::Null;
        let mut seen = std::collections::HashSet::new();
        for (name, value) in named {
            if !seen.insert(name) { return Err(self.lang.method_errors["arguments"].clone()); }
            match self.lang.method_keywords.get(name).map(String::as_str).unwrap_or("") { "reverse" => { if !matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { return Err(self.lang.method_errors["arguments"].clone()); } backwards = self.truth(value); }, "key" => key = value.clone(), _ => return Err(self.lang.method_errors["arguments"].clone()) }
        }
        let mut decorated = Vec::new();
        for item in crate::methods::members(source, &|key| self.lang.method_errors[key].clone())? {
            let rank = match &key {
                Value::Null => item.clone(),
                Value::Native(b, word) => self.builtin(*b, word, &mut vec![item.clone()])?,
                Value::Routine(routine) => {
                    self.invoke(routine, vec![item.clone()]).map_err(|_| self.lang.method_errors["unready"].clone())?;
                    self.drop_top()?
                }
                Value::ValueMethod(method) => self.value_method(&method.0, &method.1, vec![item.clone()], Vec::new())?,
                Value::Text(word) => {
                    let native = self.lang.builtins.get(word.as_ref()).copied().ok_or_else(|| self.lang.method_errors["arguments"].clone())?;
                    self.builtin(native, word, &mut vec![item.clone()])?
                }
                _ => return Err(self.lang.method_errors["unready"].clone()),
            };
            decorated.push((rank, item));
        }
        for i in 1..decorated.len() {
            let mut j = i;
            while j > 0 {
                let (left, right) = if backwards { (&decorated[j-1].0, &decorated[j].0) } else { (&decorated[j].0, &decorated[j-1].0) };
                let lower = match (left.contents(), right.contents()) { (Value::Text(a), Value::Text(b)) => a < b, _ => self.truth(&self.dyadic(&Action::Lt, left, right).map_err(|_| self.lang.method_errors["unready"].clone())?) };
                if !lower { break; }
                decorated.swap(j, j-1); j -= 1;
            }
        }
        Ok(decorated.into_iter().map(|(_, value)| value).collect())
    }

    fn integer_call(&self, args: &[Value]) -> Res<Value> {
        if args.len() > 2 { return Err(self.lang.call_amiss[0].clone()); }
        let Some(value) = args.first() else { return Ok(Value::Small(0)) };
        let base = match args.get(1) {
            None => 10,
            Some(Value::Small(n)) => *n,
            Some(Value::Huge(_)) => return Err(self.lang.to_int_base_amiss[0].clone()),
            Some(_) => return Err(self.lang.call_amiss[0].clone()),
        };
        if base != 0 && !(2..=36).contains(&base) { return Err(self.lang.to_int_base_amiss[0].clone()); }
        let Value::Text(text) = value else {
            if args.len() == 2 { return Err(self.lang.to_int_text_required[0].clone()); }
            if let Value::Flag(b) = value { return Ok(Value::Small(i64::from(*b))); }
            return arith::whole_of(value).map(Value::of_big).ok_or_else(|| self.lang.call_amiss[0].clone());
        };
        let invalid = || {
            if self.lang.integer_text_detail.len() == 2 {
                format!("{}{}{}{}", self.lang.integer_text_detail[0], base, self.lang.integer_text_detail[1], self.rem_repr(value).unwrap_or_default())
            } else { self.lang.to_int_text_amiss[0].clone() }
        };
        let text = text.trim();
        let (minus, digits) = if let Some(tail) = text.strip_prefix('-') { (true, tail) }
            else { (false, text.strip_prefix('+').unwrap_or(text)) };
        let prefix = if digits.starts_with("0x") || digits.starts_with("0X") { 16 }
            else if digits.starts_with("0b") || digits.starts_with("0B") { 2 }
            else if digits.starts_with("0o") || digits.starts_with("0O") { 8 } else { 0 };
        let radix = if base == 0 { if prefix == 0 { 10 } else { prefix } } else { base as u32 };
        let prefixed = prefix != 0 && prefix == radix;
        let digits = if prefixed { digits[2..].strip_prefix('_').unwrap_or(&digits[2..]) } else { digits };
        let valid = !digits.is_empty() && !digits.starts_with('_') && !digits.ends_with('_') && !digits.contains("__")
            && digits.chars().all(|c| c == '_' || c.is_ascii() && c.is_digit(radix));
        let cleaned = digits.replace('_', "");
        if !valid || (base == 0 && !prefixed && cleaned.starts_with('0') && cleaned.chars().any(|c| c != '0')) {
            return Err(invalid());
        }
        let whole = BigInt::parse_bytes(cleaned.as_bytes(), radix).ok_or_else(invalid)?;
        Ok(Value::of_big(if minus { -whole } else { whole }))
    }

    /// Collections kept by a routine remain the same thing when read,
    /// passed on, or bound to another name.
    fn keep_collection(&self, held: Value) -> Value {
        if self.lang.bind_names && matches!(held, Value::Array(_) | Value::Map(_)) {
            Value::Bond(Rc::new(RefCell::new(held)))
        } else { held }
    }

    fn map_from(&mut self, items: Vec<(Option<String>, Value)>) -> Res<Value> {
        let positional: Vec<Value> = items.iter().filter_map(|(name, value)| name.is_none().then(|| collection_contents(value))).collect();
        if positional.len() > 1 {
            return Err(self.lang.map_argument_amiss.clone().unwrap_or_else(|| "A map takes at most one source".into()));
        }
        let mut pairs = Vec::new();
        if let Some(source) = positional.first() {
            match source {
                Value::Map(prior) => pairs = prior.as_ref().clone(),
                other => {
                    for item in self.comprehension_items(other)? {
                        let pair = self.comprehension_items(&item)?;
                        if pair.len() != 2 {
                            return Err(self.lang.map_pair_amiss.clone().unwrap_or_else(|| "A map item needs two values".into()));
                        }
                        put_key(&mut pairs, pair[0].clone(), pair[1].clone());
                    }
                }
            }
        }
        let mut named = std::collections::HashSet::new();
        for (key, value) in items {
            if let Some(key) = key {
                if !named.insert(key.clone()) { return Err(Self::named_fault(&self.lang.call_duplicate, &key)); }
                put_key(&mut pairs, Value::text(&key), value);
            }
        }
        Ok(Value::Map(Rc::new(pairs)))
    }

    fn builtin(&mut self, builtin: Builtin, name: &str, args: &mut Vec<Value>) -> Res<Value> {
        if !self.lang.bind_names { return self.builtin_values(builtin, name, args); }
        let writes = matches!(builtin, Builtin::Append | Builtin::Replace);
        let target = if writes { args.last().cloned() } else { None };
        let last = args.len().saturating_sub(1);
        for (at, value) in args.iter_mut().enumerate() {
            if writes && at + 1 == last { continue; }
            if let Value::Bond(cell) = value {
                let held = cell.borrow().clone();
                *value = held;
            }
        }
        let result = self.builtin_values(builtin, name, args)?;
        if let Some(Value::Bond(cell)) = target {
            *cell.borrow_mut() = result;
            Ok(Value::Bond(cell))
        } else { Ok(self.keep_collection(result)) }
    }

    fn replace_item(&self, pairs: &mut Vec<(Value, Value)>, key: Value, value: Value) {
        if self.lang.bind_names {
            if let Some((_, old)) = pairs.iter_mut().find(|(k, _)| k.equals(&key)) {
                *old = value;
            } else { pairs.push((key, value)); }
        } else { put_key(pairs, key, value); }
    }

    fn builtin_values(&mut self, builtin: Builtin, name: &str, args: &mut Vec<Value>) -> Res<Value> {
        if matches!(builtin, Builtin::Append | Builtin::Replace) {
            if let Some(original @ Value::Collection(..)) = args.last().cloned() {
                let Value::Collection(cell, _) = &original else { unreachable!() };
                let last = args.len()-1;
                args[last] = original.contents();
                let result = self.builtin(builtin, name, args)?;
                *cell.borrow_mut() = result;
                return Ok(original);
            }
        }
        if !matches!(builtin, Builtin::Say | Builtin::List | Builtin::Out | Builtin::Append | Builtin::Replace) { for value in args.iter_mut() { *value = value.contents(); } }
        if let Some(answer) = self.special_builtin(builtin, args)? { return Ok(answer); }
        if Self::core_builtin(builtin) { return self.core_call(builtin, name, args.clone(), Vec::new()); }
        let sp = self.wording();
        let arity = |n: usize| -> Res<()> {
            if args.len() == n {
                return Ok(());
            }
            Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, args.len()))
        };
        Ok(match builtin {
            Builtin::MapFrom => self.map_from(args.drain(..).map(|v| (None, v)).collect())?,
            Builtin::ValueMethod => return Err(self.lang.method_errors["attribute"].clone()),
            Builtin::Sorted => { if args.len() != 1 { return Err(self.lang.method_errors["arguments"].clone()); } return self.order_values(&args[0], &[]).map(|v| Value::array(v).held(true)); },
            Builtin::SetMake | Builtin::SetAdd | Builtin::SetRemove | Builtin::SetDiscard | Builtin::SetPop | Builtin::SetClear | Builtin::SetCopy | Builtin::SetUpdate | Builtin::SetUnion | Builtin::SetIntersection | Builtin::SetDifference | Builtin::SetSymmetric | Builtin::SetSubset | Builtin::SetSuperset | Builtin::SetDisjoint | Builtin::SetMeetUpdate | Builtin::SetLessUpdate | Builtin::SetXorUpdate | Builtin::SetSorted => self.set_builtin(builtin, args)?,
            Builtin::Echo => {
                arity(1)?;
                let Value::Text(s) = &args[0] else { return Err(format!("{}() requires a string argument", name)) };
                self.utter(s);
                Value::Null
            }
            // Source read while the program runs, assembled against
            // the same globals and run where it stands. A file that
            // cannot be read gives false back, as such a language says.
            Builtin::Eval | Builtin::Include | Builtin::IncludeOnce => {
                // A source read in stands as a call of its own, so that
                // a fault raised in the reading, or by what it reads,
                // names the reading among the calls it stood under.
                let from_library = self.calls.last().map_or(false, |c| c.of_library && !self.stands_for_the_run(&c.named));
                self.calls.push(Called {
                    named: Rc::from(name),
                    within: None,
                    from: self.source.clone(),
                    on: self.line,
                    given_at: None,
                    of_library: false,
                    from_library,
                });
                let done = self.read_in(builtin, name, args);
                // A complaint the reading raised is handed over while
                // the reading still stands, so whatever takes it sees
                // the reading and not what came after.
                if self.any_waiting.get() {
                    if let Err(fault) = self.hand_over_complaints() {
                        self.carried = Some(fault);
                    }
                }
                self.calls.pop();
                return done;
            }
            // Reaching outside the run: only a language that spells
            // these labels can, and what cannot be done gives false
            // back rather than stopping, as such a language expects.
            Builtin::FileRead => {
                arity(1)?;
                let sp = self.wording();
                match std::fs::read(args[0].display(&sp)) {
                    Ok(bytes) => Value::text(&String::from_utf8_lossy(&bytes)),
                    Err(_) => Value::Flag(false),
                }
            }
            Builtin::FileWrite => {
                if args.len() != 2 {
                    return Err(format!("{}() expects 2 arguments, got {}", name, args.len()));
                }
                let sp = self.wording();
                let (where_to, what) = (args[0].display(&sp), args[1].display(&sp));
                match std::fs::write(where_to, self.lang.bytes_of(&what)) {
                    Ok(()) => Value::Small(what.len() as i64),
                    Err(_) => Value::Flag(false),
                }
            }
            Builtin::FileThere => {
                arity(1)?;
                let sp = self.wording();
                Value::Flag(std::path::Path::new(&args[0].display(&sp)).exists())
            }
            Builtin::FileGone => {
                arity(1)?;
                let sp = self.wording();
                Value::Flag(std::fs::remove_file(args[0].display(&sp)).is_ok())
            }
            // A command put before the host's own shell. It travels as
            // the bytes its text stands for, and everything the shell
            // wrote where a run writes is read back from bytes the same
            // way, the break of line ending it kept as it came. What
            // the shell said in complaint is left to go where this
            // run's own complaints go. A command that wrote nothing at
            // all answers with nothing, and a shell that would not
            // start at all answers false.
            Builtin::ShellSaid => {
                arity(1)?;
                let sp = self.wording();
                let told = self.lang.bytes_of(&args[0].display(&sp));
                // What this run has written goes out before the shell
                // writes anything, so that the two stand in the order
                // they were said.
                use std::io::Write as _;
                use std::os::unix::ffi::OsStrExt as _;
                let _ = std::io::stdout().flush();
                let asked = std::process::Command::new(HOST_SHELL)
                    .arg(SHELL_TAKES_A_COMMAND)
                    .arg(std::ffi::OsStr::from_bytes(&told))
                    .stderr(std::process::Stdio::inherit())
                    .output();
                match asked {
                    Err(_) => Value::Flag(false),
                    Ok(done) if done.stdout.is_empty() => Value::Null,
                    Ok(done) => Value::text(&self.lang.text_of(&done.stdout)),
                }
            }
            // A connection made to a host and a port, written on
            // once and then read until the far end closes it. That is
            // the whole of what a request saying the connection is to
            // be closed needs, and nothing here answers otherwise.
            // Standing still for so many millionths of a second. A run
            // that is waiting on something outside itself must be able
            // to wait, or it asks after it as fast as it can and calls
            // the first refusal final.
            Builtin::Waited => {
                arity(1)?;
                let millionths = as_index(&args[0])?;
                std::thread::sleep(std::time::Duration::from_micros(millionths as u64));
                Value::Null
            }
            Builtin::NetAsk => {
                arity(4)?;
                use std::io::{Read as _, Write as _};
                let sp = self.wording();
                let host = args[0].display(&sp);
                let port = as_index(&args[1])?;
                let sent = self.lang.bytes_of(&args[2].display(&sp));
                let seconds = as_index(&args[3])?;
                let waiting = match seconds {
                    0 => None,
                    n => Some(std::time::Duration::from_secs(n as u64)),
                };
                let reached = |mut joined: std::net::TcpStream| -> std::io::Result<Vec<u8>> {
                    joined.set_read_timeout(waiting)?;
                    joined.set_write_timeout(waiting)?;
                    joined.write_all(&sent)?;
                    joined.flush()?;
                    let mut came = Vec::new();
                    joined.read_to_end(&mut came)?;
                    Ok(came)
                };
                let named = format!("{}:{}", host, port);
                let joined = match waiting {
                    None => std::net::TcpStream::connect(&named),
                    Some(how_long) => std::net::ToSocketAddrs::to_socket_addrs(&named)
                        .and_then(|mut each| {
                            each.next().ok_or_else(|| {
                                std::io::Error::new(std::io::ErrorKind::NotFound, "no such host")
                            })
                        })
                        .and_then(|one| std::net::TcpStream::connect_timeout(&one, how_long)),
                };
                match joined.and_then(reached) {
                    Ok(came) => Value::text(&self.lang.text_of(&came)),
                    Err(_) => Value::Flag(false),
                }
            }
            // A program started beside this one, whose own writing goes
            // nowhere: this is for raising something that answers on a
            // connection, not for reading what a program says.
            Builtin::RunBegin => {
                arity(3)?;
                use std::os::unix::ffi::OsStrExt as _;
                let sp = self.wording();
                let program = self.lang.bytes_of(&args[0].display(&sp));
                let mut asked = std::process::Command::new(std::ffi::OsStr::from_bytes(&program));
                for word in one_after_another(&args[1]) {
                    let word = self.lang.bytes_of(&word.display(&sp));
                    asked.arg(std::ffi::OsStr::from_bytes(&word));
                }
                for (name, worth) in named_pairs(&args[2]) {
                    let name = self.lang.bytes_of(&name.display(&sp));
                    let worth = self.lang.bytes_of(&worth.display(&sp));
                    asked.env(std::ffi::OsStr::from_bytes(&name), std::ffi::OsStr::from_bytes(&worth));
                }
                let begun = asked
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn();
                match begun {
                    Err(_) => Value::Flag(false),
                    Ok(child) => {
                        let which = child.id() as i64;
                        self.beside.borrow_mut().insert(which, child);
                        Value::Small(which)
                    }
                }
            }
            Builtin::RunEnd => {
                arity(1)?;
                let which = as_index(&args[0])? as i64;
                match self.beside.borrow_mut().remove(&which) {
                    None => Value::Flag(false),
                    Some(mut child) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        Value::Flag(true)
                    }
                }
            }
            Builtin::TimeLimit => {
                arity(1)?;
                let seconds = as_index(&args[0])?;
                self.limit.set(seconds);
                self.began.set(Some(std::time::Instant::now()));
                Value::Flag(true)
            }
            // The room the run has taken, as the allocator the host set
            // up has counted it: the bytes it holds at this moment, the
            // most it ever held at once, and the forgetting of that
            // highest reading so that it is counted afresh from here.
            Builtin::RoomUsed => {
                arity(0)?;
                Value::Small(lumen_room::used() as i64)
            }
            Builtin::RoomMost => {
                arity(0)?;
                Value::Small(lumen_room::most() as i64)
            }
            Builtin::RoomForget => {
                arity(0)?;
                lumen_room::forget_most();
                Value::Null
            }
            // How much room the run may take from here, in bytes.
            Builtin::RoomLimit => {
                arity(1)?;
                let bytes = as_index(&args[0])?;
                self.ceiling.set(bytes);
                Value::Flag(true)
            }
            // Words said as a complaint of a kind the language names,
            // where the run stands.
            // The calls under way, as the program may read them: each
            // one what it called, the class it stands in, where the call
            // itself is written, and what it was handed.
            Builtin::Calls => {
                arity(0)?;
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
                    pairs.push((Value::text("args"), Value::array(handed)));
                    told.push(Value::Map(Rc::new(pairs)));
                }
                Value::array(told)
            }
            Builtin::Complain => {
                arity(2)?;
                let sp = self.wording();
                let said = args.pop().expect("what to say").display(&sp);
                let named = args.pop().expect("the kind").display(&sp);
                match self.lang.complaint_words.iter().find(|(_, word)| *word == named) {
                    Some((kind, _)) => {
                        self.complain(*kind, &said);
                        Value::Flag(true)
                    }
                    None => Value::Flag(false),
                }
            }
            // The routine every complaint is to be handed to, or none.
            // What the run has bound under a name, by name: the
            // classes, and the routines.
            Builtin::ClassesBound | Builtin::RoutinesBound => {
                arity(0)?;
                let wanted = |v: &Value| match builtin {
                    Builtin::ClassesBound => matches!(v, Value::Class(_)),
                    _ => matches!(v, Value::Routine(_)),
                };
                let mut named = Vec::new();
                for (at, held) in self.world.iter().enumerate() {
                    if wanted(held) {
                        // A class is filed under more than its name, so
                        // what it is filed under is taken off again.
                        if let Some(name) = self.registry.idents.get(at) {
                            named.push(Value::text(name.trim_end_matches(crate::code::OF_A_CLASS)));
                        }
                    }
                }
                Value::array(named)
            }
            // The words the language spells of its own, by name: what
            // a program may call without anybody having written it.
            Builtin::Spelled => {
                arity(0)?;
                let mut words: Vec<Value> = self.lang.builtins.keys().map(|w| Value::text(w)).collect();
                words.sort_by(|a, b| a.plain().cmp(&b.plain()));
                Value::array(words)
            }
            // What a class answers to, by name: the methods it has, or
            // the properties its things hold, its own first and then
            // those of the class it stands on. A thing is asked of the
            // class it is of.
            Builtin::ClassMethods | Builtin::ClassProperties => {
                arity(1)?;
                let of = match self.class_it_spells(args[0].clone()) {
                    Value::Object(o) => Some(o.class.clone()),
                    Value::Class(c) => Some(c),
                    _ => None,
                };
                let mut named = Vec::new();
                let mut here = of;
                while let Some(class) = here {
                    match builtin {
                        Builtin::ClassMethods => named.extend(class.methods.iter().map(|(n, _)| n.clone())),
                        _ => named.extend(class.fields.iter().map(|(n, _)| crate::value::who_keeps(n).0.to_string())),
                    }
                    here = class.base.clone();
                }
                let mut seen = Vec::new();
                for name in named {
                    if !seen.iter().any(|held: &String| held == &name) {
                        seen.push(name);
                    }
                }
                Value::array(seen.into_iter().map(|n| Value::text(&n)).collect())
            }
            // The class a class stands on, by name: a thing is asked of
            // the class it is of. Nothing where it stands on none.
            Builtin::ClassBeneath => {
                arity(1)?;
                let of = match self.class_it_spells(args[0].clone()) {
                    Value::Object(o) => Some(o.class.clone()),
                    Value::Class(c) => Some(c),
                    _ => None,
                };
                match of.and_then(|c| c.base.clone()) {
                    Some(under) => Value::text(&under.name),
                    None => Value::Null,
                }
            }
            // How far the clock the system keeps has come since the
            // year it counts from. A clock that will not answer counts
            // as standing at the start of it.
            Builtin::Clock => {
                arity(0)?;
                let since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
                Value::Small(since.map_or(0, |gone| gone.as_secs() as i64))
            }
            // The roots, the curves and the angles, worked over the
            // reals of the width and named by the first thing given.
            // They are worked where the width is worked and nowhere
            // else, since a root is seldom a ratio and would otherwise
            // have to be held to some chosen count of figures.
            Builtin::Math => {
                let Some(working) = args.first().map(|v| v.display(&sp)) else {
                    return Err(format!("{}() wants the name of a working first of all", name));
                };
                let wants = match working.as_str() {
                    "atan2" | "hypot" | "pow" | "fdiv" => 2,
                    _ => 1,
                };
                if args.len() != wants + 1 {
                    return Err(format!("{}('{}') expects {} argument(s) after the name, got {}", name, working, wants, args.len() - 1));
                }
                let given = |at: usize| -> f64 {
                    let worth = self.as_number(&args[at]);
                    // The nought below nought is a nought of its own at
                    // the width, and some of these answer differently
                    // for it, so the minus is put back on.
                    if let Value::Real(r) = &worth {
                        if r.below && num_traits::Zero::is_zero(&r.p) {
                            return -0.0;
                        }
                    }
                    match arith::Exact::from_value(&worth) {
                        Some(e) => crate::value::as_binary(&e.p, &e.q),
                        None => f64::NAN,
                    }
                };
                let (x, y) = (given(1), if wants == 2 { given(2) } else { 0.0 });
                let got = match working.as_str() {
                    "sqrt" => x.sqrt(),
                    "exp" => x.exp(),
                    "expm1" => x.exp_m1(),
                    "log" => x.ln(),
                    "log10" => x.log10(),
                    "log2" => x.log2(),
                    "log1p" => x.ln_1p(),
                    "sin" => x.sin(),
                    "cos" => x.cos(),
                    "tan" => x.tan(),
                    "asin" => x.asin(),
                    "acos" => x.acos(),
                    "atan" => x.atan(),
                    "sinh" => x.sinh(),
                    "cosh" => x.cosh(),
                    "tanh" => x.tanh(),
                    "asinh" => x.asinh(),
                    "acosh" => x.acosh(),
                    "atanh" => x.atanh(),
                    "atan2" => x.atan2(y),
                    "hypot" => x.hypot(y),
                    "pow" => x.powf(y),
                    // Dividing at the width answers with what lies past
                    // every number rather than stopping the run, which
                    // is the whole of why it is asked for here.
                    "fdiv" => x / y,
                    _ => return Err(format!("{}(): there is no working called '{}'", name, working)),
                };
                crate::value::real_of(got, self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES))
            }
            Builtin::OutBegun => {
                arity(0)?;
                Value::Flag(self.written_out.get())
            }
            Builtin::Untaken => {
                let put = args.first().cloned().unwrap_or(Value::Null);
                let held = matches!(put, Value::Null | Value::Blank);
                *self.untaken.borrow_mut() = if held { None } else { Some(put) };
                Value::Flag(true)
            }
            Builtin::Complainer => {
                let put = args.first().cloned().unwrap_or(Value::Null);
                let held = matches!(put, Value::Null | Value::Blank);
                *self.complainer.borrow_mut() = if held { None } else { Some(put) };
                Value::Flag(true)
            }
            // A routine to run when the run is over, with whatever else
            // was given to stand as its arguments.
            Builtin::WhenDone => {
                if args.is_empty() {
                    return Err(format!("{}() needs a program to run", name));
                }
                let mut given = args.clone();
                let work = given.remove(0);
                self.when_done.borrow_mut().push((work, given));
                Value::Null
            }
            // Keeping what the run writes out, and giving it up again.
            Builtin::HoldOut => {
                self.holding.borrow_mut().push(String::new());
                Value::Flag(true)
            }
            Builtin::HeldOut => match self.holding.borrow().last() {
                Some(kept) => Value::text(kept),
                None => Value::Flag(false),
            },
            Builtin::DropOut => {
                let held = self.holding.borrow_mut().pop();
                Value::Flag(held.is_some())
            }
            Builtin::DeepOut => Value::Small(self.holding.borrow().len() as i64),
            Builtin::Given | Builtin::GivenCount | Builtin::GivenAt => {
                let Some(given) = self.given.last().cloned() else {
                    // What a language says when one of these is reached
                    // where no call is running, in its own words where
                    // it has them.
                    let said = match builtin {
                        Builtin::Given => &self.lang.args_outside_all,
                        Builtin::GivenCount => &self.lang.args_outside_count,
                        _ => &self.lang.args_outside_at,
                    };
                    return Err(said.clone().unwrap_or_else(|| format!("{}() belongs inside a function", name)));
                };
                match builtin {
                    Builtin::Given => {
                        arity(0)?;
                        Value::array(given.clone())
                    }
                    Builtin::GivenCount => {
                        arity(0)?;
                        Value::Small(given.len() as i64)
                    }
                    _ => {
                        arity(1)?;
                        // A place below the first and a place past the
                        // last are each told in the language's own words
                        // where it has them.
                        if matches!(&args[0], Value::Small(n) if *n < 0) {
                            let said = self.lang.args_below.clone();
                            return Err(said.unwrap_or_else(|| format!("{}(): the place asked for comes before the first", name)));
                        }
                        let at = as_index(&args[0])?;
                        match given.get(at) {
                            Some(v) => v.clone(),
                            None => {
                                let said = self.lang.args_beyond.clone();
                                return Err(said.unwrap_or_else(|| format!("{}(): the call was given no argument {}", name, at)));
                            }
                        }
                    }
                }
            }
            Builtin::Say => {
                self.utter(&format!("{}\n", self.render(&args)));
                Value::Null
            }
            Builtin::Out => {
                self.utter(&self.render(&args));
                Value::Null
            }
            Builtin::Tell => {
                let sp = self.wording();
                for v in args.iter() {
                    self.utter(&v.display(&sp));
                }
                Value::Null
            }
            Builtin::Define => return Err(format!("{}() needs a quoted name as its first argument", name)),
            Builtin::Dump => {
                for v in args.iter() {
                    self.utter(&format!("{}\n", dumped(v, 0, self.lang.real_bits.is_some(), &self.wording())));
                }
                Value::Null
            }
            Builtin::InstanceOf | Builtin::Tuple | Builtin::Set | Builtin::Dict | Builtin::Reversed | Builtin::Enumerate | Builtin::Zip | Builtin::Map | Builtin::Filter | Builtin::All | Builtin::Minimum | Builtin::Maximum | Builtin::Absolute | Builtin::Round | Builtin::Divmod | Builtin::Power | Builtin::Hex | Builtin::Oct | Builtin::Bin | Builtin::Repr | Builtin::Bool | Builtin::Callable | Builtin::Identity | Builtin::Hash | Builtin::Iter | Builtin::Next | Builtin::HasAttr | Builtin::GetAttr | Builtin::SetAttr | Builtin::DelAttr | Builtin::Vars => unreachable!(),
            Builtin::List => {
                if args.is_empty() { return Ok(Value::array(Vec::new())); }
                arity(1)?;
                let result = Value::array(self.comprehension_items(&args[0])?);
                if matches!(args[0], Value::View(_) | Value::Collection(_, true)) { result.held(true) } else { result }
            }
            Builtin::Any => {
                arity(1)?;
                if matches!(args[0], Value::Cursor(_)) {
                    while let Some(value) = self.core_step(&args[0])? { if self.truth(&value) { return Ok(Value::Flag(true)); } }
                } else if self.lang.yield_suspends {
                    let Value::Generator(walk) = self.iterator(args[0].clone()).map_err(|f| f.told(&self.wording()))? else { unreachable!() };
                    while let Some(item) = self.resume_generator(&walk, Value::Null).map_err(|f| f.told(&self.wording()))? {
                        if self.truth(&item) { return Ok(Value::Flag(true)); }
                    }
                    return Ok(Value::Flag(false));
                }
                Value::Flag(self.comprehension_items(&args[0])?.iter().any(|v| self.truth(v)))
            }
            Builtin::Sum => {
                if args.is_empty() || args.len() > 2 { return Err(format!("{}() expects one or two arguments", name)); }
                let mut total = args.get(1).cloned().unwrap_or(Value::Small(0));
                if let Value::Flag(b) = total { total = Value::Small(i64::from(b)); }
                if self.lang.yield_suspends {
                    let Value::Generator(walk) = self.iterator(args[0].clone()).map_err(|f| f.told(&self.wording()))? else { unreachable!() };
                    while let Some(item) = self.resume_generator(&walk, Value::Null).map_err(|f| f.told(&self.wording()))? {
                        let number = match item { Value::Flag(flag) => Value::Small(i64::from(flag)), other => other };
                        total = arith::calculate(Operation::Plus, &total, &number).ok_or_else(|| self.lang.sum_non_number[0].clone())??;
                    }
                    return Ok(total);
                }
                for item in self.comprehension_items(&args[0])? {
                    let item = match item { Value::Flag(b) => Value::Small(i64::from(b)), other => other };
                    total = arith::calculate(Operation::Plus, &total, &item).ok_or_else(|| self.lang.sum_non_number.first().cloned().unwrap_or_else(|| "Invalid collection argument".to_string()))??;
                }
                total
            }
            Builtin::Span if self.lang.range_value => {
                if args.is_empty() || args.len() > 3 { return Err(self.lang.call_amiss[0].clone()); }
                let mut bounds = Vec::new();
                for v in args.iter() {
                    bounds.push(match v {
                        Value::Small(n) => BigInt::from(*n),
                        Value::Huge(n) => (**n).clone(),
                        Value::Flag(b) => BigInt::from(i64::from(*b)),
                        _ => return Err(self.lang.range_integer[0].clone()),
                    });
                }
                if bounds.len() == 1 { bounds.insert(0, BigInt::from(0)); }
                if bounds.len() == 2 { bounds.push(BigInt::from(1)); }
                if bounds[2] == BigInt::from(0) { return Err(self.lang.range_zero[0].clone()); }
                Value::Counted(Rc::new(crate::value::Counted { start: bounds[0].clone(), stop: bounds[1].clone(), step: bounds[2].clone(), name: name.to_string() }))
            }
            Builtin::Span => return Err(format!("{}() spells a range, which belongs in a for loop", name)),
            Builtin::MakeReal => {
                if args.is_empty() || args.len() > 2 {
                    return Err(format!("{}() expects 1 or 2 arguments, got {}", name, args.len()));
                }
                let places = match args.get(1) {
                    None => arith::DEFAULT_PLACES,
                    Some(Value::Small(n)) if *n >= 0 => *n as usize,
                    Some(Value::Small(_)) | Some(Value::Huge(_)) => return Err("Precision must be a positive integer".to_string()),
                    Some(_) => return Err("Precision argument must be an integer".to_string()),
                };
                arith::to_real(&args[0], places)
                    .ok_or_else(|| format!("{}() requires a number, rational, or real argument", name))?
            }
            Builtin::Places => {
                arity(1)?;
                match &args[0] {
                    Value::Real(r) => Value::Small(r.places as i64),
                    _ => return Err(format!("{}() requires a real argument", name)),
                }
            }
            Builtin::ToText if !self.lang.to_string_object.is_empty() && args.is_empty() => Value::text(""),
            Builtin::ToText if !self.lang.to_string_object.is_empty() && args.len() > 1 => return Err(self.lang.to_string_unready[0].clone()),
            Builtin::ToText => {
                arity(1)?;
                Value::text(&args[0].display(&sp))
            }
            Builtin::ToInt if !self.lang.to_int_base.is_empty() => return self.integer_call(args),
            Builtin::ToInt => {
                arity(1)?;
                let whole = arith::whole_of(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::of_big(whole)
            }
            Builtin::AsReal if self.lang.to_real_text && args.is_empty() => arith::to_real(&Value::Small(0), arith::DEFAULT_PLACES).unwrap(),
            Builtin::AsReal if self.lang.to_real_text && matches!(args.first(), Some(Value::Text(_))) => {
                arity(1)?;
                let Value::Text(text) = &args[0] else { unreachable!() };
                let plain = text.trim().to_ascii_lowercase();
                let unsigned = plain.strip_prefix(['+', '-']).unwrap_or(&plain);
                if Lang::spells(&self.lang.infinity_words, unsigned) || Lang::spells(&self.lang.nan_words, unsigned) {
                    let special = if Lang::spells(&self.lang.nan_words, unsigned) { f64::NAN }
                        else if plain.starts_with('-') { f64::NEG_INFINITY } else { f64::INFINITY };
                    return Ok(crate::value::real_of(special, arith::DEFAULT_PLACES));
                }
                let number = number_spelled(text).ok_or_else(|| self.lang.to_real_text_amiss[0].clone())?;
                self.at_real_width(arith::to_real(&number, arith::DEFAULT_PLACES).ok_or_else(|| self.lang.to_real_text_amiss[0].clone())?)
            }
            Builtin::AsReal if self.lang.arithmetic_flags && matches!(args.as_slice(), [Value::Flag(_)]) => {
                let Value::Flag(b) = args[0] else { unreachable!() };
                arith::to_real(&Value::Small(i64::from(b)), arith::DEFAULT_PLACES).unwrap()
            }
            Builtin::AsReal => {
                arity(1)?;
                match &args[0] {
                    v @ Value::Real(_) => v.clone(),
                    v => self.at_real_width(arith::to_real(v, arith::DEFAULT_PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?),
                }
            }
            Builtin::Length => {
                arity(1)?;
                match &args[0] {
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Array(items) | Value::Tuple(items) => Value::Small(items.len() as i64),
                    Value::Set(s) => Value::Small(s.borrow().held.len() as i64),
                    Value::Map(pairs) => Value::Small(pairs.len() as i64),
                    Value::Counted(r) => Value::of_big(r.length()),
                    _ => return Err(format!("{}() requires a string or array argument", name)),
                }
            }
            Builtin::CharAtIndex => {
                arity(2)?;
                match (&args[0], &args[1]) {
                    (Value::Text(s), Value::Small(i)) => match usize::try_from(*i).ok().and_then(|i| s.chars().nth(i)) {
                        Some(c) => Value::text(&c.to_string()),
                        None => return Err(format!("{} index out of bounds", name)),
                    },
                    (Value::Text(_), _) => return Err(format!("{}() second argument must be an integer", name)),
                    _ => return Err(format!("{}() first argument must be a string", name)),
                }
            }
            Builtin::CodeOf => {
                arity(1)?;
                let Value::Text(s) = &args[0] else { return Err(format!("{}() requires a string argument", name)) };
                match s.chars().next() {
                    Some(c) => Value::Small(c as i64),
                    None => return Err(format!("{}() requires a non-empty string", name)),
                }
            }
            Builtin::CharOf => {
                arity(1)?;
                let code = match &args[0] {
                    Value::Small(n) => u32::try_from(*n).ok(),
                    Value::Huge(_) => None,
                    _ => return Err(format!("{}() requires an integer argument", name)),
                };
                let code = code
                    .ok_or_else(|| format!("{}() argument must be a non-negative integer within valid Unicode range", name))?;
                match char::from_u32(code) {
                    Some(c) => Value::text(&c.to_string()),
                    None => return Err(format!("{}() argument {} is not a valid Unicode code point", name, code)),
                }
            }
            Builtin::Leave => return Err("the run is over".to_string()),
            Builtin::Raise => {
                arity(1)?;
                return match &args[0] {
                    Value::Text(s) => Err(s.to_string()),
                    _ => Err(format!("{}() argument must be a string", name)),
                };
            }
            Builtin::SortOf => {
                arity(1)?;
                if self.lang.builtins.values().any(|b| *b == Builtin::InstanceOf) {
                    if let Value::Object(o) = &args[0] { return Ok(Value::Class(o.class.clone())); }
                    let which = match &args[0] {
                        Value::Small(_) | Value::Huge(_) => Some(Builtin::ToInt), Value::Real(_) => Some(Builtin::AsReal),
                        Value::Text(_) => Some(Builtin::ToText), Value::Flag(_) => Some(Builtin::Bool),
                        Value::Array(_) => Some(Builtin::List), Value::Tuple(_) => Some(Builtin::Tuple),
                        Value::Set(_) => Some(Builtin::Set), Value::Map(_) => Some(Builtin::Dict), _ => None,
                    };
                    if let Some(b) = which { if let Some((word, _)) = self.lang.builtins.iter().find(|(_,v)| **v == b) { return Ok(Value::Native(b, Rc::from(word.as_str()))); } }
                }
                // A thing is of no kind the core knows, so a language
                // with a word of its own for one answers with that.
                if let (Value::Object(_), Some(word)) = (&args[0], &self.lang.object_kind) {
                    return Ok(Value::text(word));
                }
                let Some(kind) = args[0].sort() else {
                    return Err(format!("{}(): unknown value type", name));
                };
                // Some languages say a kind in words rather than hand
                // back a value standing for it.
                match self.lang.kind_spelled {
                    false => Value::SortOf(kind),
                    true => match self.lang.sort_bindings.iter().find(|(_, k)| *k == kind) {
                        Some((word, _)) => Value::text(word),
                        None => return Err(format!("{}(): the language has no word for that kind", name)),
                    },
                }
            }
            Builtin::Numer | Builtin::Denom => {
                arity(1)?;
                let (p, q) = arith::parts(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::of_big(if builtin == Builtin::Numer { p } else { q })
            }
            Builtin::Fetch => {
                arity(2)?;
                self.element(&args[0], &args[1], Reading::Plain)?
            }
            Builtin::Append => {
                arity(2)?;
                let target = args.pop().expect("the array");
                if matches!(target, Value::Tuple(_) | Value::Set(_)) { return Err(self.core_fault("core.immutable", &target.core_kind())); }
                let v = args.pop().expect("the value");
                // A language that makes a place on writing into it finds
                // an array where nothing at all was there.
                let target = match target {
                    Value::Null | Value::Blank if self.lang.makes_places => Value::Array(Rc::new(Vec::new())),
                    held => held,
                };
                match target {
                    Value::Array(mut items) => {
                        Rc::make_mut(&mut items).push(v);
                        Value::Array(items)
                    }
                    // The next whole-number key one past the highest.
                    Value::Map(mut pairs) => {
                        let key = next_key(&pairs);
                        Rc::make_mut(&mut pairs).push((Value::Small(key), v));
                        Value::Map(pairs)
                    }
                    _ => return Err(self.not_an_array()),
                }
            }
            Builtin::Replace => {
                arity(3)?;
                let target = args.pop().expect("the array");
                if matches!(target, Value::Tuple(_) | Value::Set(_)) { return Err(self.core_fault("core.immutable", &target.core_kind())); }
                let v = args.pop().expect("the value");
                let at = self.key(&args.pop().expect("the key"));
                if let Value::Slice(parts) = &at {
                    return self.write_slice(target, parts, v);
                }
                // A language that makes a place on writing into it finds
                // an array where nothing at all was there.
                let target = match target {
                    Value::Null | Value::Blank if self.lang.makes_places => Value::Array(std::rc::Rc::new(Vec::new())),
                    held => held,
                };
                // A place in a piece of text holds one letter, and
                // writing there puts a letter in its stead. A place past
                // the end is reached by filling the way to it with
                // spaces, as a language that writes into text does.
                if let (Value::Text(held), true) = (&target, self.lang.text_places) {
                    let put = v.display(&sp);
                    let mut letters = put.chars();
                    let Some(letter) = letters.next() else {
                        return Err("Cannot write nothing into a place in text".to_string());
                    };
                    // More than one letter handed to a place that holds
                    // one: the first goes in and the language says so.
                    if letters.next().is_some() {
                        if let Some(said) = self.lang.text_place_first.clone() {
                            self.complain(Complaint::Warning, &said);
                        }
                    }
                    let mut letters: Vec<char> = held.chars().collect();
                    // A place named by text is the number that text
                    // opens with, as a language that reads a number out
                    // of text does.
                    let at = match &at {
                        Value::Text(spelling) => match number_opening(spelling).0 {
                            Some(counted) => counted,
                            None => return Err("A place in text is named by a whole number".to_string()),
                        },
                        _ => at,
                    };
                    let at = match as_index(&at) {
                        Ok(at) => at,
                        Err(_) => match arith::whole_of(&at).and_then(|n| n.to_i64()) {
                            // A place counted from the end.
                            Some(back) if back < 0 && (-back as usize) <= letters.len() => letters.len() - (-back as usize),
                            _ => return Err("A place in text is named by a whole number".to_string()),
                        },
                    };
                    while letters.len() <= at {
                        letters.push(' ');
                    }
                    letters[at] = letter;
                    return Ok(Value::text(&letters.into_iter().collect::<String>()));
                }
                match target {
                    // A list written at a place it already holds stays a list.
                    Value::Array(mut items) if as_index(&at).map_or(false, |i| i < items.len()) => {
                        let i = as_index(&at)?;
                        // A place holding a cell that names share is
                        // written through, not written over.
                        if !self.lang.bind_names {
                            if let Value::Bond(shared) = &items[i] {
                                *shared.borrow_mut() = v;
                                return Ok(Value::Array(items));
                            }
                        }
                        Rc::make_mut(&mut items)[i] = v;
                        Value::Array(items)
                    }
                    // Any other key makes it a map, its places the keys.
                    Value::Array(items) | Value::Tuple(items) => {
                        let mut pairs: Vec<(Value, Value)> =
                            items.iter().enumerate().map(|(i, x)| (Value::Small(i as i64), x.clone())).collect();
                        self.replace_item(&mut pairs, at, v);
                        Value::Map(Rc::new(pairs))
                    }
                    Value::Map(mut pairs) => {
                        self.replace_item(Rc::make_mut(&mut pairs), at, v);
                        Value::Map(pairs)
                    }
                    _ => return Err(self.not_an_array()),
                }
            }
            Builtin::Pack => return Err(format!("{}() is a literal, not a call", name)),
            // Asking is done where the program is put together, since
            // what is asked about is a name and not its value.
            Builtin::Held | Builtin::Hollow => return Err("Only a name or a place in an array can be asked about".into()),
            Builtin::Lead => {
                if args.len() < 2 {
                    return Err(format!("{}() expects an array and a value at least", name));
                }
                let target = args.pop().expect("the array");
                // A place a whole number names is named anew from
                // nought, the ones put in front taking the first
                // numbers; a place a word names keeps its word.
                let mut counted = -1i64;
                let mut number = || {
                    counted += 1;
                    Value::Small(counted)
                };
                let mut kept: Vec<(Value, Value)> = args.drain(..).map(|v| (number(), v)).collect();
                match target {
                    Value::Array(items) | Value::Tuple(items) => kept.extend(items.iter().map(|v| (number(), v.clone()))),
                    Value::Map(pairs) => kept.extend(pairs.iter().map(|(k, v)| match k {
                        Value::Text(_) => (k.clone(), v.clone()),
                        _ => (number(), v.clone()),
                    })),
                    v => return Err(format!("{}() cannot put a value in front of {}", name, v.plain())),
                }
                // Where every place is named by its own number in turn,
                // the array is the plain one it looks like.
                let plain = kept.iter().enumerate().all(|(i, (k, _))| matches!(k, Value::Small(n) if *n == i as i64));
                match plain {
                    true => Value::array(kept.into_iter().map(|(_, v)| v).collect()),
                    false => Value::Map(Rc::new(kept)),
                }
            }
            Builtin::Erase => {
                // Taking a place out of an array: the array is given back
                // without it.
                arity(2)?;
                let at = self.key_quietly(&args.pop().expect("the place"));
                match args.pop().expect("the array") {
                    Value::Array(items) if !self.lang.del_words.is_empty() => {
                        let raw = (match &at { Value::Small(n) => Some(*n), Value::Huge(n) => n.to_i64(), Value::Flag(b) => Some(i64::from(*b)), _ => None }).ok_or_else(|| self.lang.del_unrun.clone())?;
                        let i = if raw < 0 { items.len() as i64 + raw } else { raw };
                        if i < 0 || i as usize >= items.len() { return Err(self.lang.del_unrun.clone().into()); }
                        let mut left = items.as_ref().clone();
                        left.remove(i as usize);
                        Value::array(left)
                    }
                    Value::Array(items) | Value::Tuple(items) => {
                        let i = as_index(&at)?;
                        let kept: Vec<(Value, Value)> = items
                            .iter()
                            .enumerate()
                            .filter(|(j, _)| *j != i)
                            .map(|(j, v)| (Value::Small(j as i64), v.clone()))
                            .collect();
                        Value::Map(Rc::new(kept))
                    }
                    Value::Map(pairs) => {
                        if !self.lang.del_words.is_empty() && !pairs.iter().any(|(k, _)| k.equals(&at)) {
                            return Err(self.lang.del_unrun.clone());
                        }
                        let kept: Vec<(Value, Value)> = pairs.iter().filter(|(k, _)| !k.equals(&at)).cloned().collect();
                        Value::Map(Rc::new(kept))
                    }
                    v => return Err(format!("{}() cannot take a place out of {}", name, v.plain())),
                }
            }
            Builtin::Layout => {
                arity(1)?;
                self.utter(&laid_out(&args[0], 0, &sp));
                Value::Flag(true)
            }
            Builtin::External => self.external(name, &args)?,
        })
    }

    /// The external capabilities of docs/LUMEN_LANGUAGE_EXTERN_SYSTEM.md.
    fn external(&self, name: &str, args: &[Value]) -> Res<Value> {
        let sp = self.wording();
        let which = match args.first() {
            Some(Value::Text(s)) => s.to_string(),
            Some(_) => return Err(format!("First argument to {} must be a string (function name)", name)),
            None => return Err(format!("{} requires at least one argument (function name)", name)),
        };
        let rest = &args[1..];
        if !matches!(which.as_str(), "print_native" | "debug_info" | "value_type") {
            return Err(format!("Unknown external function: {}", which));
        }
        if rest.len() != 1 {
            return Err(format!("{} expects 1 argument, got {}", which, rest.len()));
        }
        let v = &rest[0];
        match which.as_str() {
            "print_native" => self.utter(&format!("{}\n", v.display(&sp))),
            "debug_info" => eprintln!("[DEBUG] {}", v.display(&sp)),
            _ => {
                return Ok(Value::Small(match v {
                    Value::Small(_) | Value::Huge(_) | Value::Frac(_) | Value::Real(_) => 0,
                    Value::Flag(_) => 1,
                    Value::Text(_) => 2,
                    _ => return Err("Unknown value type".to_string()),
                }))
            }
        }
        Ok(v.clone())
    }
}

fn as_index(v: &Value) -> Res<usize> {
    match v {
        Value::Small(n) => usize::try_from(*n).map_err(|_| "Array index out of bounds".to_string()),
        Value::Huge(n) => n.to_usize().ok_or_else(|| "Array index out of bounds".to_string()),
        _ => Err("Array index must be a number".to_string()),
    }
}

/// A complaint's opening, from the break of line it begins with to the
/// end of what it says: the word for the kind and then the words
/// themselves. Where the language gives a dressing for a reader of
/// markup the opening wears it; where it gives none the plain words
/// stand.
/// The shell the host keeps, and the switch that hands it a command
/// written out rather than a file to read it from.
const HOST_SHELL: &str = "/bin/sh";
const SHELL_TAKES_A_COMMAND: &str = "-c";

pub fn complaint_opening(lang: &Lang, word: &str, message: &str) -> String {
    match &lang.markup_kind {
        Some((ahead, before, after)) => format!("{}\n{}{}{}{}", ahead, before, word, after, message),
        None => format!("\n{}: {}", word, message),
    }
}

/// Where a complaint was raised, said after what it says: the file and
/// the line, each in whatever the language dresses it in, and the break
/// of line that ends a complaint after them both.
pub fn complaint_place(lang: &Lang, place: &str, line: u32) -> String {
    let named = match &lang.markup_place {
        Some((before, after)) => format!("{}{}{}", before, place, after),
        None => place.to_string(),
    };
    let on = match &lang.markup_line {
        Some((before, after)) => format!("{}{}{}", before, line, after),
        None => line.to_string(),
    };
    format!(" in {} on line {}\n", named, on)
}

pub fn sort_value(kind: Sort) -> Value {
    Value::SortOf(kind)
}

pub fn places_default() -> Value {
    Value::Small(arith::DEFAULT_PLACES as i64)
}

/// A value with its kind, as PHP's var_dump shows it: a number as
/// `int(n)` or `float(x)`, text with its byte length, an array one
/// entry per line, nested arrays indented two more.
/// How wide a piece of text is, counted in bytes. Where text is held as
/// bytes each character stands for one; otherwise the count is of the
/// bytes the letters are spelled with.
fn text_width(s: &str, as_bytes: bool) -> usize {
    match as_bytes {
        true => s.chars().count(),
        false => s.len(),
    }
}

/// The worths a value holds in a row, however it happens to hold them:
/// a list keeps them plainly, a thing of keys and worths keeps them
/// under their keys, and a cell that names share is looked through.
fn one_after_another(v: &Value) -> Vec<Value> {
    match v {
        Value::Bond(shared) => one_after_another(&shared.borrow()),
        Value::Array(items) | Value::Tuple(items) => items.as_ref().clone(),
        Value::Map(pairs) => pairs.iter().map(|(_, worth)| worth.clone()).collect(),
        _ => Vec::new(),
    }
}

/// The keys and worths a value holds, for a value that holds any. A
/// list stands for the numbers it is counted by.
fn named_pairs(v: &Value) -> Vec<(Value, Value)> {
    match v {
        Value::Bond(shared) => named_pairs(&shared.borrow()),
        Value::Map(pairs) => pairs.as_ref().clone(),
        Value::Array(items) | Value::Tuple(items) => items
            .iter()
            .enumerate()
            .map(|(at, worth)| (Value::Small(at as i64), worth.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn dumped(v: &Value, depth: usize, binary_reals: bool, sp: &Wording) -> String {
    let pad = "  ".repeat(depth);
    // A place whose cell some name still holds besides the one holding
    // it is shown as shared. A place whose cell nothing else holds is
    // shown plainly, since a cell no other name reaches is a value like
    // any other; a value shown on its own is never marked.
    let shared = |item: &Value| match item {
        Value::Bond(cell) if std::rc::Rc::strong_count(cell) > 1 => "&",
        _ => "",
    };
    match v {
        // A cell two names share is shown as what it holds.
        Value::Bond(shared) => dumped(&shared.borrow(), depth, binary_reals, sp),
        Value::Small(_) | Value::Huge(_) => format!("int({})", v.plain()),
        // Shown with its kind, a binary real is written in the fewest
        // digits that read back as the same number.
        // A nought below nought is written as such, whatever the width.
        Value::Real(r) if r.below && num_traits::Zero::is_zero(&r.p) => "float(-0)".to_string(),
        Value::Real(r) if binary_reals => format!("float({})", crate::value::binary_string(crate::value::as_binary(&r.p, &r.q), None)),
        Value::Real(_) | Value::Frac(_) => format!("float({})", v.plain()),
        Value::Text(s) => format!("string({}) \"{}\"", text_width(s, sp.text_is_bytes), s),
        Value::Flag(b) => format!("bool({})", b),
        Value::Array(items) | Value::Tuple(items) => {
            let mut out = format!("array({}) {{\n", items.len());
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!("{pad}  [{i}]=>\n{pad}  {}{}\n", shared(item), dumped(item, depth + 1, binary_reals, sp)));
            }
            out.push_str(&pad);
            out.push('}');
            out
        }
        Value::Map(pairs) => {
            let mut out = format!("array({}) {{\n", pairs.len());
            for (k, item) in pairs.iter() {
                // A text key is shown in quotes, a number bare.
                let shown = match k {
                    Value::Text(s) => format!("\"{}\"", s),
                    other => other.plain(),
                };
                out.push_str(&format!("{pad}  [{shown}]=>\n{pad}  {}{}\n", shared(item), dumped(item, depth + 1, binary_reals, sp)));
            }
            out.push_str(&pad);
            out.push('}');
            out
        }
        Value::Object(thing) => {
            let held = thing.fields.borrow();
            let members: Vec<&(String, Value)> = held.iter().filter(|(_, v)| standing(v)).collect();
            let mut out = format!("object({})#{} ({}) {{\n", thing.class.name, thing.mark, members.len());
            for (filed, item) in members {
                let how = marked(&thing.class, filed, sp);
                let member = crate::value::who_keeps(filed).0;
                out.push_str(&format!("{pad}  [\"{member}\"{how}]=>\n{pad}  {}{}\n", shared(item), dumped(item, depth + 1, binary_reals, sp)));
            }
            out.push_str(&pad);
            out.push('}');
            out
        }
        _ => "NULL".to_string(),
    }
}

/// The values of a literal: a map when any of them is a tie or the
/// literal asks for one, otherwise a list. An untied value takes the
/// next whole-number key, as in a list.
fn gathered(items: Vec<Value>, always_map: bool, plain_keys: bool) -> Value {
    if !always_map && !items.iter().any(|v| matches!(v, Value::Tie(_))) {
        return Value::array(items);
    }
    let mut pairs: Vec<(Value, Value)> = Vec::with_capacity(items.len());
    for item in items {
        match item {
            Value::Tie(pair) => {
                let (k, v) = (pair.0.clone(), pair.1.clone());
                let k = if plain_keys { key_taken(k) } else { k };
                put_key(&mut pairs, k, v);
            }
            v => {
                let key = next_key(&pairs);
                pairs.push((Value::Small(key), v));
            }
        }
    }
    Value::Map(Rc::new(pairs))
}

/// One past the highest whole-number key, or zero when there is none.
fn next_key(pairs: &[(Value, Value)]) -> i64 {
    pairs
        .iter()
        .filter_map(|(k, _)| match k {
            Value::Small(n) => Some(*n + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .max(0)
}

/// Write a key: over the value it already holds, or at the end.
fn put_key(pairs: &mut Vec<(Value, Value)>, key: Value, value: Value) {
    match pairs.iter_mut().find(|(k, _)| k.equals(&key)) {
        // A place holding a cell that names share is written through,
        // not written over.
        Some(slot) => match &slot.1 {
            Value::Bond(shared) => *shared.borrow_mut() = value,
            _ => slot.1 = value,
        },
        None => pairs.push((key, value)),
    }
}

/// How far a member may be reached from, as a language marks it beside
/// the name where it shows what a thing holds: nothing at all where the
/// member is open to everything, the word for a member the class shares
/// with those standing on it, or the class's own name and the word for
/// a member it keeps to itself.
fn marked(class: &Class, filed: &str, sp: &Wording) -> String {
    let (member, keeper) = crate::value::who_keeps(filed);
    if let Some(holder) = keeper {
        return match &sp.hidden_word {
            Some(word) => format!(":\"{holder}\":{word}"),
            None => String::new(),
        };
    }
    match class.reach_of(member) {
        Some((Reach::Guarded, _)) => match &sp.guarded_word {
            Some(word) => format!(":{word}"),
            None => String::new(),
        },
        _ => String::new(),
    }
}

/// Whether a thing still holds the member at a place: one taken off
/// leaves its place behind, holding nothing, so that a walk under way
/// keeps its footing.
fn standing(v: &Value) -> bool {
    !matches!(v, Value::Blank)
}

/// A value over lines, as PHP's print_r writes it: a scalar bare, an
/// array as `Array` and its places in brackets, each nested array set
/// eight spaces further in and followed by a blank line.
fn laid_out(v: &Value, indent: usize, sp: &Wording) -> String {
    let held;
    let (what, pairs): (String, Vec<(String, &Value)>) = match v {
        Value::Array(items) | Value::Tuple(items) => ("Array".to_string(), items.iter().enumerate().map(|(i, x)| (i.to_string(), x)).collect()),
        Value::Map(entries) => ("Array".to_string(), entries.iter().map(|(k, x)| (k.display(sp), x)).collect()),
        Value::Object(thing) => {
            held = thing.fields.borrow();
            let shown = |k: &String| format!("{}{}", crate::value::who_keeps(k).0, marked(&thing.class, k, sp));
            (format!("{} Object", thing.class.name), held.iter().filter(|(_, v)| standing(v)).map(|(k, x)| (shown(k), x)).collect())
        }
        other => return other.display(sp),
    };
    let pad = " ".repeat(indent);
    let mut out = format!("{what}\n{pad}(\n");
    for (key, item) in pairs {
        // What an array lays out ends its own line, so the newline
        // here is the gap PHP leaves after it; for a scalar it ends the line.
        let shown = laid_out(item, indent + 8, sp);
        out.push_str(&format!("{pad}    [{key}] => {shown}\n"));
    }
    out.push_str(&format!("{pad})\n"));
    out
}

/// Where an array now holds the cell a walk handed out. The place it
/// was handed out at is looked at first, since it is where the item
/// still is unless the body has moved it; nothing at all is answered
/// where the item is no longer in the array.
fn lies_at(walked: &Value, cell: &Rc<RefCell<Value>>, was: usize) -> Option<usize> {
    let same = |v: &Value| matches!(v, Value::Bond(other) if Rc::ptr_eq(other, cell));
    let held: &[Value] = match walked {
        Value::Array(items) | Value::Tuple(items) => items,
        Value::Map(pairs) => {
            if pairs.get(was).map_or(false, |(_, v)| same(v)) {
                return Some(was);
            }
            return pairs.iter().position(|(_, v)| same(v));
        }
        _ => return None,
    };
    if held.get(was).map_or(false, same) {
        return Some(was);
    }
    held.iter().position(same)
}

/// The place an array holds, made a shared cell so that a name fastened
/// to it writes into the array itself.
fn shared_item(held: &mut Value, at: &Value) -> Res<Rc<RefCell<Value>>> {
    if let Value::Collection(cell, _) = held { return shared_item(&mut cell.borrow_mut(), at); }
    // A thing's members are counted as an array's places are, but they
    // live behind a shared holding, so the cell is made within it.
    if let Value::Object(thing) = held {
        let i = as_index(at)?;
        let mut fields = thing.fields.borrow_mut();
        let reach = fields.len();
        let Some((_, place)) = fields.get_mut(i) else {
            return Err(format!("Array index {} out of bounds (length: {})", i, reach));
        };
        if let Value::Bond(shared) = place {
            return Ok(shared.clone());
        }
        let shared = Rc::new(RefCell::new(std::mem::replace(place, Value::Null)));
        *place = Value::Bond(shared.clone());
        return Ok(shared);
    }
    let place: &mut Value = match held {
        Value::Array(items) | Value::Tuple(items) => {
            let i = as_index(at)?;
            let items = Rc::make_mut(items);
            match items.get_mut(i) {
                Some(place) => place,
                None => return Err(format!("Array index {} out of bounds (length: {})", i, items.len())),
            }
        }
        Value::Map(pairs) => {
            // A walk counts places, so a map is reached by its position
            // as an array is, not by the key it holds there.
            let i = as_index(at)?;
            let pairs = Rc::make_mut(pairs);
            let held = pairs.len();
            match pairs.get_mut(i) {
                Some((_, value)) => value,
                None => return Err(format!("Array index {} out of bounds (length: {})", i, held)),
            }
        }
        _ => return Err("Cannot walk a value that is not an array".to_string()),
    };
    if let Value::Bond(shared) = place {
        return Ok(shared.clone());
    }
    let shared = Rc::new(RefCell::new(std::mem::replace(place, Value::Null)));
    *place = Value::Bond(shared.clone());
    Ok(shared)
}

/// The cell of the place a chain of keys names, the arrays and places
/// along the way made where they are not there yet and the language
/// makes what a write needs.
fn shared_deep(held: &mut Value, keys: &[Value], makes: bool) -> Res<Rc<RefCell<Value>>> {
    if let Value::Collection(cell, _) = held { return shared_deep(&mut cell.borrow_mut(), keys, makes); }
    let Some((last, first)) = keys.split_last() else {
        return Err("No place was named".to_string());
    };
    let mut spot = held;
    for k in first {
        spot = place_within(spot, k, makes)?;
    }
    let place = place_within(spot, last, makes)?;
    if let Value::Bond(shared) = place {
        return Ok(shared.clone());
    }
    let shared = Rc::new(RefCell::new(std::mem::replace(place, Value::Null)));
    *place = Value::Bond(shared.clone());
    Ok(shared)
}

fn place_within<'a>(held: &'a mut Value, at: &Value, makes: bool) -> Res<&'a mut Value> {
    if makes && matches!(held, Value::Null | Value::Blank | Value::Gap) {
        *held = Value::Array(Rc::new(Vec::new()));
    }
    // A list holds only the places it already has: a key that is no
    // whole number, or one past the last, makes it a map, which is what
    // a plain write into such a place does too.
    let beyond = match (&*held, as_index(at)) {
        (Value::Array(items), Ok(i)) => i >= items.len(),
        _ => true,
    };
    if matches!(held, Value::Array(_)) && beyond {
        let Value::Array(items) = &*held else { unreachable!("a list") };
        let spread = items.iter().enumerate().map(|(i, v)| (Value::Small(i as i64), v.clone())).collect();
        *held = Value::Map(Rc::new(spread));
    }
    let place: &mut Value = match held {
        Value::Array(items) | Value::Tuple(items) => {
            let i = as_index(at)?;
            let items = Rc::make_mut(items);
            let reach = items.len();
            match items.get_mut(i) {
                Some(place) => place,
                None => return Err(format!("Array index {} out of bounds (length: {})", i, reach)),
            }
        }
        Value::Map(pairs) => {
            let pairs = Rc::make_mut(pairs);
            let found = pairs.iter().position(|(k, _)| k.equals(at));
            let at_place = match found {
                Some(i) => i,
                None if makes => {
                    pairs.push((at.clone(), Value::Null));
                    pairs.len() - 1
                }
                None => return Err(format!("Undefined array key {}", at.plain())),
            };
            &mut pairs[at_place].1
        }
        _ => return Err("Cannot take a cell from a place in something that is not an array".to_string()),
    };
    Ok(place)
}

/// The number a piece of text spells, whole or fractional, with room
/// for a sign and for space around it. Anything else is not a number.
/// Text stepped along its letters: the last one moves on, `z` coming
/// round to `a` and carrying into the one before it, `Z` to `A` and `9`
/// to `0` the same way. A letter that is neither a letter nor a digit
/// stops the step where it stands, and a carry off the front puts a
/// fresh `a`, `A` or `1` there, after what came round first.
fn letters_onward(s: &str) -> String {
    if s.is_empty() {
        return "1".to_string();
    }
    let mut letters: Vec<u8> = s.as_bytes().to_vec();
    let mut at = letters.len();
    let mut carrying = true;
    while carrying && at > 0 {
        at -= 1;
        let c = letters[at];
        match c {
            b'z' => letters[at] = b'a',
            b'Z' => letters[at] = b'A',
            b'9' => letters[at] = b'0',
            b'a'..=b'y' | b'A'..=b'Y' | b'0'..=b'8' => {
                letters[at] = c + 1;
                carrying = false;
            }
            // Anything else is not stepped at all, and stops the step.
            _ => return String::from_utf8_lossy(&letters).into_owned(),
        }
    }
    if carrying {
        let fresh = match letters.first() {
            Some(b'a') => b'a',
            Some(b'A') => b'A',
            _ => b'1',
        };
        letters.insert(0, fresh);
    }
    String::from_utf8_lossy(&letters).into_owned()
}

fn number_spelled(s: &str) -> Option<Value> {
    let text = s.trim();
    // A number may carry a power of ten after it: 1e2, 1.5E-3.
    if let Some(at) = text.find(['e', 'E']) {
        let (front, back) = text.split_at(at);
        let power: i32 = back[1..].parse().ok()?;
        // A power of ten after it makes a real of it, whole or not,
        // just as it does where the number is written in a program.
        let (p, q) = match number_spelled(front)? {
            Value::Real(r) => (r.p.clone(), r.q.clone()),
            Value::Small(n) => (BigInt::from(n), BigInt::from(1)),
            Value::Huge(n) => ((*n).clone(), BigInt::from(1)),
            _ => return None,
        };
        let scale = BigInt::from(10).pow(power.unsigned_abs());
        let (p, q) = if power < 0 { (p, q * scale) } else { (p * scale, q) };
        return Some(arith::shape_number(p, q, Some(15)));
    }
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let whole = match digits.split_once('.') {
        None => return digits.parse::<BigInt>().ok().map(|n| Value::of_big(n * sign)),
        Some((whole, fraction)) if fraction.chars().all(|c| c.is_ascii_digit()) => (whole, fraction),
        Some(_) => return None,
    };
    let (before, after) = whole;
    if after.is_empty() {
        return before.parse::<BigInt>().ok().map(|n| Value::of_big(n * sign));
    }
    let scale = BigInt::from(10).pow(after.len() as u32);
    let above: BigInt = if before.is_empty() { BigInt::from(0) } else { before.parse().ok()? };
    let below: BigInt = after.parse().ok()?;
    let places = (before.len() + after.len()).max(15);
    Some(arith::shape_number((above * &scale + below) * sign, scale, Some(places)))
}

/// A value as sixty-four bits. Anything that is not a whole number is
/// cut down to one first, the way a language that works on bits expects:
/// what lies past the point is dropped towards nothing, so -1.5 stands
/// for -1, and a flag or nothing stands for 1 or 0.
fn bits_of(v: &Value) -> Res<i64> {
    let whole = match v {
        Value::Small(n) => return Ok(*n),
        Value::Flag(yes) => return Ok(i64::from(*yes)),
        Value::Null | Value::Blank | Value::Gap | Value::Fence => return Ok(0),
        Value::Text(s) => match number_spelled(s) {
            Some(n) => n,
            None => return Ok(0),
        },
        other => other.clone(),
    };
    if let Value::Huge(n) = &whole {
        return Ok(n.to_i64().unwrap_or(i64::MIN));
    }
    match arith::whole_of(&whole) {
        // Dividing whole numbers cuts towards nothing, which is what
        // dropping what lies past the point comes to. A number too wide
        // to be held in the bits at all comes to the lowest of them,
        // which is what a machine holding numbers to a width gives.
        Some(n) => Ok(n.to_i64().unwrap_or(i64::MIN)),
        None => Err("Working on bits needs a whole number".to_string()),
    }
}

/// A key as a language whose keys are plain takes one: text spelling a
/// whole number is that number, so `a['7']` and `a[7]` name one place;
/// a number with a point stands for the whole number towards nothing; a
/// flag stands for 1 or 0, and nothing for text with nothing in it.
/// Text that spells a number any other way stays as it was written.
fn key_taken(v: Value) -> Value {
    match &v {
        Value::Text(s) => match whole_spelled(s) {
            Some(n) => Value::Small(n),
            None => v,
        },
        Value::Flag(yes) => Value::Small(i64::from(*yes)),
        Value::Null | Value::Blank | Value::Gap => Value::text(""),
        Value::Frac(_) | Value::Real(_) => match bits_of(&v) {
            Ok(n) => Value::Small(n),
            Err(_) => v,
        },
        _ => v,
    }
}

/// The whole number a piece of text spells, where it spells one the way
/// a whole number is written out: digits, a minus before them at most,
/// no space around them and no nought leading.
fn whole_spelled(s: &str) -> Option<i64> {
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
/// of it: `"12"` is twelve and the whole, `"12abc"` twelve and not, and
/// `"abc"` no number at all.
fn number_opening(s: &str) -> (Option<Value>, bool) {
    if let Some(whole) = number_spelled(s) {
        return (Some(whole), true);
    }
    let text = s.trim_start();
    let mut end = 0;
    let bytes = text.as_bytes();
    if matches!(bytes.first(), Some(b'-') | Some(b'+')) {
        end = 1;
    }
    let mut seen_point = false;
    while end < bytes.len() {
        let c = bytes[end];
        if c.is_ascii_digit() {
            end += 1;
        } else if c == b'.' && !seen_point {
            seen_point = true;
            end += 1;
        } else {
            break;
        }
    }
    // A power of ten belongs to the number it follows, so long as
    // digits do follow it: `123e5xyz` opens with a number, `123exyz`
    // opens with 123.
    if end > 0 && matches!(bytes.get(end), Some(b'e') | Some(b'E')) {
        let mut after = end + 1;
        if matches!(bytes.get(after), Some(b'-') | Some(b'+')) {
            after += 1;
        }
        let digits = after;
        while bytes.get(after).map_or(false, u8::is_ascii_digit) {
            after += 1;
        }
        if after > digits {
            end = after;
        }
    }
    match number_spelled(&text[..end]) {
        Some(opening) => (Some(opening), false),
        None => (None, false),
    }
}

/// Read the contents without giving up the collection's own cell.
fn collection_contents(value: &Value) -> Value {
    match value {
        Value::Bond(cell) => cell.borrow().clone(),
        other => other.clone(),
    }
}

// Collection calls share the opening of arguments, but keep their own
// few names and their own complaints after those arguments are opened.
impl Engine<'_> {
    fn core_builtin(b: Builtin) -> bool {
        matches!(b, Builtin::InstanceOf | Builtin::Tuple | Builtin::Set | Builtin::Dict | Builtin::Sorted | Builtin::Reversed | Builtin::Enumerate | Builtin::Zip | Builtin::Map | Builtin::Filter | Builtin::All | Builtin::Minimum | Builtin::Maximum | Builtin::Absolute | Builtin::Round | Builtin::Divmod | Builtin::Power | Builtin::Hex | Builtin::Oct | Builtin::Bin | Builtin::Repr | Builtin::Bool | Builtin::Callable | Builtin::Identity | Builtin::Hash | Builtin::Iter | Builtin::Next | Builtin::HasAttr | Builtin::GetAttr | Builtin::SetAttr | Builtin::DelAttr | Builtin::Vars)
    }

    fn core_fault(&self, label: &str, piece: &str) -> String {
        self.lang.core_words.get(label).map_or_else(String::new, |words| Self::named_fault(words, piece))
    }

    fn core_cursor(source: CursorSource) -> Value {
        Value::Cursor(Rc::new(RefCell::new(CursorState { source, pending: None, finished: false, busy: false })))
    }

    fn core_iterator(&mut self, source: &Value) -> Res<Value> {
        if matches!(source, Value::Cursor(_) | Value::Generator(_)) { return Ok(source.clone()); }
        Ok(Self::core_cursor(CursorSource::Items(self.core_members(source)?, 0)))
    }

    fn core_step(&mut self, walk: &Value) -> Res<Option<Value>> {
        if let Value::Generator(state) = walk { return self.resume_generator(state, Value::Null).map_err(|f| f.told(&self.wording())); }
        let Value::Cursor(cell) = walk else { return Err(self.core_fault("core.not_iterator", &walk.core_kind())); };
        let mut source = {
            let mut state = cell.borrow_mut();
            if state.busy { return Err(self.core_fault("core.unready", "next")); }
            if let Some(value) = state.pending.take() { return Ok(Some(value)); }
            if state.finished { return Ok(None); }
            state.busy = true;
            state.source.clone()
        };
        let answer = (|| match &mut source {
            CursorSource::Items(values, place) => {
                let found = values.get(*place).cloned();
                if found.is_some() { *place += 1; }
                Ok(found)
            }
            CursorSource::Numbered(inner, count) => {
                let Some(value) = self.core_step(inner)? else { return Ok(None); };
                let numbered = Value::Tuple(Rc::new(vec![Value::of_big(count.clone()), value]));
                *count += 1;
                Ok(Some(numbered))
            }
            CursorSource::Combined(walks, work) => {
                if walks.is_empty() { return Ok(None); }
                let mut row = Vec::new();
                for inner in walks { let Some(value) = self.core_step(inner)? else { return Ok(None); }; row.push(value); }
                Ok(Some(if let Some(work) = work { self.core_apply(work, row)? } else { Value::Tuple(Rc::new(row)) }))
            }
            CursorSource::Selected(inner, test) => {
                while let Some(value) = self.core_step(inner)? {
                    let verdict = if matches!(test, Value::Null) { value.clone() } else { self.core_apply(test, vec![value.clone()])? };
                    if self.truth(&verdict) { return Ok(Some(value)); }
                }
                Ok(None)
            }
        })();
        let mut state = cell.borrow_mut();
        state.source = source;
        state.busy = false;
        if matches!(answer, Ok(None)) { state.finished = true; }
        answer
    }

    fn core_more(&mut self, walk: &Value) -> Res<bool> {
        let Value::Cursor(cell) = walk else { return Ok(false); };
        if cell.borrow().pending.is_some() { return Ok(true); }
        let value = self.core_step(walk)?;
        let more = value.is_some();
        cell.borrow_mut().pending = value;
        Ok(more)
    }

    fn core_members(&mut self, v: &Value) -> Res<Vec<Value>> {
        if matches!(v, Value::Cursor(_)) {
            let mut items = Vec::new();
            while let Some(value) = self.core_step(v)? { items.push(value); }
            return Ok(items);
        }
        self.comprehension_items(v).map_err(|_| self.core_fault("core.uniterable", &v.core_kind()))
    }

    fn core_apply(&mut self, work: &Value, mut args: Vec<Value>) -> Res<Value> {
        match work {
            Value::Native(b, word) => self.builtin(*b, word, &mut args),
            Value::ValueMethod(method) => self.value_method(&method.0, &method.1, args, Vec::new()),
            Value::Routine(p) => {
                if let Err(f) = self.invoke(p, args) {
                    self.carried = Some(f);
                    return Err(self.core_fault("core.unready", &p.ident));
                }
                self.drop_top()
            }
            _ => Err(self.core_fault("core.uncallable", &work.core_kind())),
        }
    }

    fn core_isinstance(&self, value: &Value, kind: &Value) -> Res<bool> {
        if let Value::Tuple(types) = kind {
            for t in types.iter() { if self.core_isinstance(value, t)? { return Ok(true); } }
            return Ok(false);
        }
        if let Value::Class(class) = kind {
            return Ok(matches!(value, Value::Object(o) if o.class.named(&class.name, false)));
        }
        let b = match kind {
            Value::Native(b, _) => *b,
            Value::SortOf(Sort::Null) => return Ok(matches!(value, Value::Null)),
            _ => return Err(self.core_fault("core.isinstance.amiss", "")),
        };
        Ok(match b {
            Builtin::ToInt => matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_)),
            Builtin::AsReal => matches!(value, Value::Real(_)),
            Builtin::ToText => matches!(value, Value::Text(_)),
            Builtin::Bool => matches!(value, Value::Flag(_)),
            Builtin::List => matches!(value, Value::Array(_)),
            Builtin::Tuple => matches!(value, Value::Tuple(_)),
            Builtin::Set => matches!(value, Value::Set(_)),
            Builtin::Dict => matches!(value, Value::Map(_)),
            _ => return Err(self.core_fault("core.isinstance.amiss", "")),
        })
    }

    fn core_call(&mut self, b: Builtin, name: &str, mut args: Vec<Value>, named: Vec<(String, Value)>) -> Res<Value> {
        use num_integer::Integer;
        use num_traits::{Signed, Zero};
        for value in &mut args { *value = value.contents(); }
        if named.is_empty() { if let Some(value) = self.special_builtin(b, &args)? { return Ok(value); } }
        if b == Builtin::Dict && args.len() > 1 { return Err(self.lang.map_argument_amiss.clone().unwrap_or_else(|| self.core_fault("core.arity", name))); }
        let mut key = Value::Null;
        let mut reverse = false;
        let mut default = None;
        let mut dict_kw = Vec::new();
        for (word, value) in named {
            let spells = |label: &str| self.lang.core_words.get(label).map_or(false, |words| Lang::spells(words, &word));
            if b == Builtin::Dict { dict_kw.push((Value::text(&word), value)); continue; }
            if matches!(b, Builtin::Sorted | Builtin::Minimum | Builtin::Maximum) && spells("key") { key = value; continue; }
            if b == Builtin::Sorted && spells("reverse") {
                if !matches!(value, Value::Small(_) | Value::Huge(_) | Value::Flag(_)) { return Err(self.core_fault("core.integer", &value.core_kind())); }
                reverse = self.truth(&value); continue;
            }
            if matches!(b, Builtin::Minimum | Builtin::Maximum) && spells("default") { default = Some(value); continue; }
            let place = match b {
                Builtin::Enumerate if spells("start") => 1,
                Builtin::Round if spells("round.number") => 0,
                Builtin::Round if spells("round.ndigits") => 1,
                Builtin::Power if spells("pow.base") => 0,
                Builtin::Power if spells("pow.exp") => 1,
                Builtin::Power if spells("pow.mod") => 2,
                _ => return Err(Self::named_fault(&self.lang.call_unknown, &word)),
            };
            if args.len() > place && !matches!(args[place], Value::Gap) { return Err(Self::named_fault(&self.lang.call_duplicate, &word)); }
            args.resize_with(place.max(args.len()), || Value::Gap);
            if args.len() == place { args.push(value); } else { args[place] = value; }
        }
        let arity = |lo, hi| if (lo..=hi).contains(&args.len()) && !args.iter().any(|v| matches!(v, Value::Gap)) { Ok(()) }
            else {
                let label = if lo == 1 && hi == 1 { "core.arity.one" } else if lo == hi { "core.arity.exact" } else { "core.arity" };
                let words = &self.lang.core_words[label];
                Err(if label == "core.arity.one" { format!("{}{}{}{}{}",words[0],name,words[1],args.len(),words[2]) }
                    else if label == "core.arity.exact" { format!("{}{}{}{}{}{}",words[0],name,words[1],lo,words[2],args.len()) }
                    else { self.core_fault(label,name) })
            };
        let number = |v: &Value| match v { Value::Flag(b) => Value::Small(i64::from(*b)), other => other.clone() };
        let integer = |v: &Value| match v { Value::Small(_) | Value::Huge(_) | Value::Flag(_) => v.as_big(), _ => Err(self.core_fault("core.integer", &v.core_kind())) };
        let result = match b {
            Builtin::InstanceOf => { arity(2, 2)?; Value::Flag(self.core_isinstance(&args[0], &args[1])?) }
            Builtin::Bool => { arity(0, 1)?; Value::Flag(args.first().map_or(false, |v| self.truth(v))) }
            Builtin::Callable => { arity(1, 1)?; Value::Flag(matches!(args[0], Value::Native(..) | Value::Routine(_) | Value::Class(_) | Value::ValueMethod(_) | Value::Method(..))) }
            Builtin::Repr => { arity(1, 1)?; Value::text(&args[0].core_repr()) }
            Builtin::Hash => { arity(1, 1)?; Value::Small(args[0].core_hash().ok_or_else(|| self.core_fault("core.unhashable", &args[0].core_kind()))?) }
            Builtin::Identity => {
                arity(1, 1)?;
                let id = match &args[0] {
                    Value::Array(a) | Value::Tuple(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Set(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Map(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Text(a) => a.as_ptr() as usize as u64,
                    Value::Object(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Class(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Cursor(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Routine(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Native(b, _) => *b as u64 + 16,
                    Value::Huge(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Real(a) => Rc::as_ptr(a) as usize as u64,
                    Value::Small(n) => (*n as u64).wrapping_mul(16).wrapping_add(3),
                    Value::Flag(v) => if *v { 2 } else { 1 },
                    Value::Null => 0,
                    _ => return Err(self.core_fault("core.unready", name)),
                };
                let filed = format!("{}:{}", args[0].core_kind(), id);
                let fresh = self.core_ids.len() + 1;
                Value::Small(*self.core_ids.entry(filed).or_insert(fresh) as i64)
            }
            Builtin::Tuple | Builtin::Set => {
                arity(0, 1)?;
                let mut items = args.first().map(|v| self.core_members(v)).transpose()?.unwrap_or_default();
                if b == Builtin::Set {
                    let mut unique = Vec::new();
                    for v in items { if v.core_hash().is_none() { return Err(self.core_fault("core.unhashable", &v.core_kind())); } if !unique.iter().any(|x: &Value| number(x).equals(&number(&v))) { unique.push(v); } }
                    items = unique;
                }
                if b == Builtin::Tuple { Value::Tuple(Rc::new(items)) } else { Value::Set(Rc::new(RefCell::new(self.set_from(items)?))) }
            }
            Builtin::Dict => {
                arity(0, 1)?;
                let mut pairs = match args.first() {
                    Some(Value::Map(p)) => p.as_ref().clone(),
                    Some(v) => {
                        let mut pairs = Vec::new();
                        for (at,item) in self.core_members(v)?.into_iter().enumerate() {
                            let row = self.core_members(&item).map_err(|_| self.core_fault("core.dict.sequence", &at.to_string()))?;
                            if row.len() != 2 { let w = &self.lang.core_words["core.dict.pair"]; return Err(format!("{}{}{}{}{}", w[0],at,w[1],row.len(),w[2])); }
                            pairs.push((row[0].clone(), row[1].clone()));
                        }
                        pairs
                    }
                    None => Vec::new(),
                };
                pairs.extend(dict_kw);
                let mut made: Vec<(Value, Value)> = Vec::new();
                for (k,v) in pairs {
                    if k.core_hash().is_none() && !matches!(k, Value::Null | Value::Real(_)) { return Err(self.core_fault("core.unhashable", &k.core_kind())); }
                    if let Some(p) = made.iter_mut().find(|(old,_)| number(old).equals(&number(&k))) { p.1 = v; } else { made.push((k,v)); }
                }
                Value::Map(Rc::new(made))
            }
            Builtin::Iter => {
                arity(1, 2)?;
                if args.len() == 2 { return Err(self.core_fault("core.unready", name)); }
                if matches!(args[0], Value::Cursor(_)) { return Ok(args[0].clone()); }
                self.core_iterator(&args[0])?
            }
            Builtin::Next => {
                arity(1, 2)?;
                self.core_step(&args[0])?.or_else(|| args.get(1).cloned()).ok_or_else(|| self.core_fault("core.exhausted", ""))?
            }
            Builtin::Reversed => {
                arity(1, 1)?;
                if !matches!(args[0], Value::Array(_) | Value::Tuple(_) | Value::Text(_) | Value::Counted(_)) { return Err(self.core_fault("core.unready", name)); }
                let mut items = self.core_members(&args[0])?; items.reverse();
                Self::core_cursor(CursorSource::Items(items, 0))
            }
            Builtin::Enumerate => {
                arity(1, 2)?;
                let n = args.get(1).map(integer).transpose()?.unwrap_or_else(|| BigInt::from(0));
                let walk = self.core_iterator(&args[0])?;
                Self::core_cursor(CursorSource::Numbered(walk, n))
            }
            Builtin::Zip | Builtin::Map => {
                arity(if b == Builtin::Map { 2 } else { 0 }, usize::MAX)?;
                let offset = usize::from(b == Builtin::Map);
                let mut walks = Vec::new();
                for source in &args[offset..] { walks.push(self.core_iterator(source)?); }
                Self::core_cursor(CursorSource::Combined(walks, if b == Builtin::Map { Some(args[0].clone()) } else { None }))
            }
            Builtin::Filter => {
                arity(2, 2)?;
                let walk = self.core_iterator(&args[1])?;
                Self::core_cursor(CursorSource::Selected(walk, args[0].clone()))
            }
            Builtin::All => {
                arity(1, 1)?;
                let walk = self.core_iterator(&args[0])?;
                while let Some(value) = self.core_step(&walk)? { if !self.truth(&value) { return Ok(Value::Flag(false)); } }
                Value::Flag(true)
            }
            Builtin::Minimum | Builtin::Maximum | Builtin::Sorted => {
                arity(1, if b == Builtin::Sorted { 1 } else { usize::MAX })?;
                if args.len() > 1 && default.is_some() { return Err(self.core_fault("core.default.many", "")); }
                let values = if args.len() == 1 { self.core_members(&args[0])? } else { args.clone() };
                if values.is_empty() && b != Builtin::Sorted { return default.ok_or_else(|| self.core_fault("core.empty", name)); }
                let mut ranked: Vec<(Value,Value)> = Vec::new();
                for value in values {
                    let k = if matches!(key, Value::Null) { value.clone() } else { self.core_apply(&key, vec![value.clone()])? };
                    let mut at = ranked.len();
                    while at > 0 {
                        let less = match (&k, &ranked[at-1].0) {
                            (Value::Text(a), Value::Text(z)) => a < z,
                            (a,z) => arith::order_values(&number(a), &number(z)).ok_or_else(|| self.core_fault("core.unready", name))? == std::cmp::Ordering::Less,
                        };
                        let greater = match (&k, &ranked[at-1].0) {
                            (Value::Text(a), Value::Text(z)) => a > z,
                            (a,z) => arith::order_values(&number(a), &number(z)).ok_or_else(|| self.core_fault("core.unready", name))? == std::cmp::Ordering::Greater,
                        };
                        if if reverse || b == Builtin::Maximum { greater } else { less } { at -= 1; } else { break; }
                    }
                    ranked.insert(at, (k,value));
                }
                if b == Builtin::Sorted { Value::array(ranked.into_iter().map(|(_,v)| v).collect()).held(true) } else { ranked.remove(0).1 }
            }
            Builtin::Absolute => {
                arity(1, 1)?;
                let x = number(&args[0]);
                let (p,q) = arith::parts(&x).ok_or_else(|| self.core_fault("core.unready", name))?;
                arith::shape_number(p.abs(), q, if matches!(x, Value::Real(_)) { Some(arith::DEFAULT_PLACES) } else { None })
            }
            Builtin::Hex | Builtin::Oct | Builtin::Bin => {
                arity(1, 1)?;
                let n = integer(&args[0])?;
                let (radix, prefix) = match b { Builtin::Hex => (16,"0x"), Builtin::Oct => (8,"0o"), _ => (2,"0b") };
                Value::text(&format!("{}{}{}", if n.is_negative() { "-" } else { "" }, prefix, n.abs().to_str_radix(radix)))
            }
            Builtin::Divmod => {
                arity(2, 2)?;
                let (a,z) = (number(&args[0]), number(&args[1]));
                if matches!(a, Value::Small(_) | Value::Huge(_)) && matches!(z, Value::Small(_) | Value::Huge(_)) {
                    let divisor = z.as_big()?;
                    if divisor.is_zero() { return Err(self.core_fault("core.zero", "")); }
                    let (q,r) = a.as_big()?.div_mod_floor(&divisor);
                    Value::Tuple(Rc::new(vec![Value::of_big(q),Value::of_big(r)]))
                } else {
                    let (p,q) = arith::parts(&a).ok_or_else(|| self.core_fault("core.unready", name))?;
                    let (r,s) = arith::parts(&z).ok_or_else(|| self.core_fault("core.unready", name))?;
                    let (x,y) = (crate::value::as_binary(&p,&q),crate::value::as_binary(&r,&s));
                    if y == 0.0 { return Err(self.core_fault("core.zero", "")); }
                    let mut rem = x % y;
                    let mut div = (x-rem)/y;
                    if rem != 0.0 && rem.is_sign_negative() != y.is_sign_negative() { rem += y; div -= 1.0; }
                    if rem == 0.0 { rem = 0.0f64.copysign(y); }
                    let mut floor = div.floor();
                    if div - floor > 0.5 { floor += 1.0; }
                    if div == 0.0 { floor = 0.0f64.copysign(x/y); }
                    Value::Tuple(Rc::new(vec![crate::value::real_of(floor,arith::DEFAULT_PLACES), crate::value::real_of(rem,arith::DEFAULT_PLACES)]))
                }
            }
            Builtin::Power => {
                arity(2, 3)?;
                if args.len() == 3 && !matches!(args[2], Value::Null) {
                    if args.iter().any(|v| !matches!(v, Value::Small(_) | Value::Huge(_) | Value::Flag(_))) { return Err(self.core_fault("core.power.integer", "")); }
                    let (mut a, mut exp, modulus) = (integer(&args[0])?, integer(&args[1])?, integer(&args[2])?);
                    if modulus.is_zero() { return Err(self.core_fault("core.mod.zero", "")); }
                    let positive = modulus.abs();
                    if exp.is_negative() {
                        let gcd = a.extended_gcd(&positive);
                        if gcd.gcd != BigInt::from(1) { return Err(self.core_fault("core.inverse", "")); }
                        a = gcd.x.mod_floor(&positive); exp = -exp;
                    }
                    let mut n = a.modpow(&exp, &positive);
                    if modulus.is_negative() && !n.is_zero() { n -= positive; }
                    Value::of_big(n)
                } else {
                    let base = number(&args[0]); let exp = number(&args[1]);
                    let (ep,eq) = arith::parts(&exp).ok_or_else(|| self.core_fault("core.unready", name))?;
                    if ep.is_negative() || eq != BigInt::from(1) || matches!(base, Value::Real(_)) || matches!(exp, Value::Real(_)) {
                        let (bp,bq) = arith::parts(&base).ok_or_else(|| self.core_fault("core.unready", name))?;
                        let (x,y) = (crate::value::as_binary(&bp,&bq),crate::value::as_binary(&ep,&eq));
                        if x == 0.0 && y < 0.0 { return Err(self.core_fault("core.power.zero", "")); }
                        let answer = x.powf(y);
                        if answer.is_nan() { return Err(self.core_fault("core.unready", name)); }
                        if answer.is_infinite() { return Err(self.core_fault("core.power.overflow", "")); }
                        crate::value::real_of(answer,arith::DEFAULT_PLACES)
                    } else { arith::calculate(Operation::Raise, &base, &exp).ok_or_else(|| self.core_fault("core.unready", name))?? }
                }
            }
            Builtin::Round => {
                arity(1, 2)?;
                let digits = match args.get(1) { None | Some(Value::Null) => 0, Some(n) => integer(n)?.to_i64().ok_or_else(|| self.core_fault("core.unready", name))? };
                let places = u32::try_from(digits.max(0)).ok().filter(|n| *n <= 100000).ok_or_else(|| self.core_fault("core.unready", name))?;
                let x = number(&args[0]);
                let (p, q) = arith::parts(&x).ok_or_else(|| self.core_fault("core.unready", name))?;
                if q.is_zero() { return Err(self.core_fault("core.unready", name)); }
                // Keep the library's scale, signed half, and truncating quotient.
                let scale = Value::of_big(BigInt::from(10).pow(places));
                let y = self.dyadic_numbers(&Action::Mul, &x, &scale)?;
                let twice = self.dyadic_numbers(&Action::Mul, &y, &Value::Small(2))?;
                let shifted = self.dyadic_numbers(&Action::Add, &twice, &Value::Small(if p.is_negative() { -1 } else { 1 }))?;
                let rounded = self.dyadic_numbers(&Action::IntDiv, &shifted, &Value::Small(2))?;
                if args.len() == 1 || matches!(args.get(1), Some(Value::Null)) {
                    let (top, bottom) = arith::parts(&rounded).ok_or_else(|| self.core_fault("core.unready", name))?;
                    Value::of_big(top / bottom)
                } else { self.dyadic_numbers(&Action::DivReal, &rounded, &scale)? }
            }
            Builtin::HasAttr | Builtin::GetAttr | Builtin::SetAttr | Builtin::DelAttr | Builtin::Vars => {
                arity(if b == Builtin::Vars { 1 } else { 2 }, if matches!(b, Builtin::SetAttr | Builtin::GetAttr) { 3 } else if b == Builtin::Vars { 1 } else { 2 })?;
                if b != Builtin::Vars && !matches!(args[1], Value::Text(_)) { return Err(self.core_fault("core.attribute.name", &args[1].core_kind())); }
                let Value::Object(o) = &args[0] else {
                    if b == Builtin::HasAttr { return Ok(Value::Flag(false)); }
                    if b == Builtin::GetAttr && args.len() == 3 { return Ok(args[2].clone()); }
                    return Err(self.core_fault(if b == Builtin::Vars { "core.vars" } else { "core.unready" }, if b == Builtin::Vars { "" } else { name }));
                };
                if b == Builtin::Vars { return Err(self.core_fault("core.unready", name)); }
                let Value::Text(attr) = &args[1] else { return Err(self.core_fault("core.attribute.name", &args[1].core_kind())); };
                let mut fields = o.fields.borrow_mut();
                let at = fields.iter().position(|(k,_)| k == attr.as_ref());
                if b == Builtin::GetAttr && at.is_none() && o.class.method(attr).is_some() { return Err(self.core_fault("core.unready", name)); }
                match b {
                    Builtin::SetAttr => { if args.len() != 3 { return Err(self.core_fault("core.arity", name)); } if let Some(at) = at { fields[at].1 = args[2].clone(); } else { fields.push((attr.to_string(),args[2].clone())); } Value::Null }
                    Builtin::HasAttr => Value::Flag(at.is_some() || o.class.method(attr).is_some()),
                    _ => if let Some(at) = at { if b == Builtin::DelAttr { fields.remove(at); Value::Null } else { fields[at].1.clone() } }
                        else if b == Builtin::GetAttr && args.len() == 3 { args[2].clone() }
                        else { let words = &self.lang.core_words["core.attribute"]; return Err(format!("{}{}{}{}{}", words[0], o.class.name, words[1], attr, words[2])); },
                }
            }
            _ => unreachable!(),
        };
        Ok(result)
    }
}
