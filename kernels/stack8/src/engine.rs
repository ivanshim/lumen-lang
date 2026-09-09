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
use crate::value::{Class, Instance, Reach, Sort, Value, Wording};
use crate::code::{Operand, Builtin, Action, Routine, Cell, Instr};

pub struct Engine<'a> {
    lang: &'a Lang,
    world: Vec<Value>,
    /// The names of the globals, kept whole so that source read while
    /// the program runs can be assembled against the same ones.
    registry: crate::compile::Registry,
    data: Vec<Value>,
    memo: HashMap<String, Value>,
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
        Engine {
            lang,
            world: vec![Value::Blank; idents.len()],
            data: Vec::new(),
            memo: HashMap::new(),
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

    /// A value with no places at all cannot be walked. A language with
    /// a word for a warning is told so and walks it no times, rather
    /// than having the run stopped over it.
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
        if matches!(held, Value::Array(_) | Value::Map(_) | Value::Object(_)) {
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
            if let Value::Bond(shared) = &frame[s] {
                return Ok(shared.borrow().clone());
            }
            if !matches!(frame[s], Value::Blank) {
                return Ok(if slot.moving { std::mem::replace(&mut frame[s], Value::Gap) } else { frame[s].clone() });
            }
        }
        if let Value::Bond(shared) = &self.world[slot.far] {
            return Ok(shared.borrow().clone());
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
            let shared = Rc::new(RefCell::new(Value::Null));
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
            if matches!(frame[s], Value::Bond(_)) {
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
        if matches!(self.world[slot.far], Value::Blank) && self.warns_about(&slot.ident) {
            self.complain(Complaint::Warning, &format!("Undefined variable {}", slot.ident));
            return Ok(&self.nothing);
        }
        match &self.world[slot.far] {
            Value::Blank => Err(format!("Undefined variable: {}", slot.ident)),
            v => Ok(v),
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

    pub fn invoke(&mut self, program: &Rc<Routine>, args: Vec<Value>) -> Flow<()> {
        let n = args.len();
        self.data.extend(args);
        self.invoke_top(program, n)
    }

    /// A call whose arguments are the top `n` of the data stack: they move
    /// straight into the frame, one allocation instead of two.
    pub fn invoke_top(&mut self, program: &Rc<Routine>, n: usize) -> Flow<()> {
        // Where a language can read what a call was given, a call may
        // give more than the routine names; the rest is kept aside.
        let most = if self.lang.spare_args { usize::MAX } else { program.formals.len() };
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
            frame[*slot] = held.clone();
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
        let mut pc = 0;
        let mut counted = 0u32;
        // Where a raised value is caught, how deep the stack was when
        // the guard was set, and how much quiet was asked for then.
        let mut guards: Vec<(usize, usize, usize)> = Vec::new();
        while pc < instrs.len() {
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
                Instr::Const(v) => self.data.push(v.clone()),
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
                        .map(|&s| frame[s].clone())
                        .find(|v| !matches!(v, Value::Blank))
                        .unwrap_or_else(|| self.world[slot.far].clone());
                    self.data.push(match held {
                        Value::Bond(shared) => shared.borrow().clone(),
                        other => other,
                    });
                }
                Instr::Write(slot) => {
                    let v = self.drop_top()?;
                    self.store_cell(slot, frame, v)?;
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
                    let empty = matches!(frame[*slot], Value::Blank);
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
                    if !self.truth(&held) {
                        pc = *to;
                        continue;
                    }
                }
                Instr::SkipCmp { op, a, b, to } => {
                    let bt = if matches!(b, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let at = if matches!(a, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let (av, bv) = self.both(a, b, &at, &bt, frame)?;
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
                            let told = self.dyadic(op, av, bv)?;
                            self.truth(&told)
                        }
                    };
                    if !holds {
                        pc = *to;
                        continue;
                    }
                }
                Instr::Dyad { op, a, b } => {
                    let bt = if matches!(b, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let at = if matches!(a, Operand::Top) { Some(self.drop_top()?) } else { None };
                    let (av, bv) = self.both(a, b, &at, &bt, frame)?;
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
                        None => self.dyadic(op, av, bv)?,
                    };
                    self.data.push(r);
                }
                Instr::Bump { slot, by } => {
                    // A shared cell holds its value elsewhere, so it is
                    // read and written the long way.
                    if self.shares_cell(slot, frame) {
                        let held = self.load_cell(slot, frame)?;
                        let sum = self.dyadic(&Action::Add, &held, by)?;
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
                            let r = self.dyadic(&Action::Add, &v, by)?;
                            self.store_cell(slot, frame, r)?;
                        }
                    }
                }
            }
            pc += 1;
        }
        Ok(())
    }

    fn perform(&mut self, op: &Action, argc: usize) -> Flow<()> {
        let result = match op {
            Action::Not => {
                let held = self.drop_top()?;
                Value::Flag(!self.truth(&held))
            }
            Action::AsBool => {
                let held = self.drop_top()?;
                Value::Flag(self.truth(&held))
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
                let at = self.key_quietly(&named);
                let holder = self.drop_top()?;
                let Value::Bond(cell) = holder else {
                    return Err("Cannot take a place out of something that is not an array".into());
                };
                let mut inside = cell.borrow_mut();
                let left = match &*inside {
                    Value::Array(items) if !self.lang.del_words.is_empty() => {
                        let raw = at.as_big()?.to_i64().ok_or_else(|| self.lang.del_unrun.clone())?;
                        let i = if raw < 0 { items.len() as i64 + raw } else { raw };
                        if i < 0 || i as usize >= items.len() { return Err(self.lang.del_unrun.clone().into()); }
                        let mut left = items.as_ref().clone();
                        left.remove(i as usize);
                        Value::array(left)
                    }
                    Value::Array(items) => {
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
                    Value::Map(pairs) => Value::Map(Rc::new(pairs.iter().filter(|(k, _)| !k.equals(&at)).cloned().collect())),
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
            Action::BitTurn => {
                let v = self.drop_top()?;
                match &v {
                    Value::Text(s) => {
                        let out: Vec<u8> = self.lang.bytes_of(s).iter().map(|c| !c).collect();
                        Value::text(&self.lang.text_of(&out))
                    }
                    _ => Value::Small(!self.bits_said(&v)?),
                }
            }
            Action::Negate => {
                // 0 - x, so a real keeps its precision.
                let v = self.drop_top()?;
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
                        arith::shape_signed(now.p.clone(), now.q.clone(), Some(now.places), !was.below)
                    }
                    // Turning the lowest whole number about takes it past
                    // the width the language holds, as adding to the
                    // highest one does.
                    _ => self.within_width(turned),
                }
            }
            Action::Invoke(name) => {
                let top = self.drop_top()?;
                let callee = self.what_it_spells(top);
                return match callee {
                    Value::Routine(p) => self.invoke_top(&p, argc - 1),
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
                match &pair[0] {
                    Value::Array(items) => match items.get(at) {
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
            Action::Grab(name) => match self.drop_top()? {
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
                            held[at].1 = Value::Blank;
                        }
                        Value::Null
                    }
                    v => return Err(format!("Cannot take property '{}' off {}", name, v.plain()).into()),
                }
            }
            Action::Plant(name) => {
                let mut pair = self.drop_many(2)?;
                let value = pair.pop().expect("the value");
                match pair.pop().expect("the object") {
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
                let Value::Object(o) = args.remove(0) else {
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
            Action::Hurl => {
                self.hurled_at.set(self.line);
                return Err(Fault::Thrown(self.drop_top()?));
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
            Action::WalkFrom => {
                let mut handed = self.drop_top()?;
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
                match self.walk_asked(&pair[0], self.lang.walk_more.clone())? {
                    Some(answer) => Value::Flag(self.truth(&answer)),
                    None => {
                        let reach = match &pair[0] {
                            Value::Array(items) => items.len(),
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
                match self.walk_asked(&pair[0], named)? {
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
                Value::Array(items) => Value::Small(items.len() as i64),
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
                let result = self.builtin(*builtin, name, &mut args);
                self.buffer = args;
                match self.carried.take() {
                    Some(fled) => return Err(fled),
                    None => result?,
                }
            }
            dyadic => {
                let b = self.drop_top()?;
                let a = self.drop_top()?;
                self.dyadic(dyadic, &a, &b)?
            }
        };
        self.data.push(result);
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

    fn dyadic(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
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
        let sp = self.wording();
        let joined = || Value::text(&format!("{}{}", a.display(&sp), b.display(&sp)));
        Ok(match op {
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
                            Value::Array(items) => items.is_empty(),
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
            Action::Eq => Value::Flag(a.equals(b)),
            Action::Ne => Value::Flag(!a.equals(b)),
            Action::Same => Value::Flag(a.identical(b)),
            Action::Unsame => Value::Flag(!a.identical(b)),
            Action::Join => joined(),
            Action::At => self.element(a, b, Reading::Plain)?,
            Action::Apart => self.element(a, b, Reading::Apart)?,
            Action::Toward => self.element(a, b, Reading::Toward)?,
            // Reaching inside makes the place on the way where nothing
            // is there yet, which is what a write to it means.
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
    fn dyadic_numbers(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
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
                match arith::calculate(calc, a, b) {
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
            Value::Array(items) => items.iter().enumerate().map(|(i, x)| (Value::Small(i as i64), x.clone())).collect(),
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
        // A place holding a cell two names share reads as what the cell
        // holds, since the sharing is between the names and not
        // something the value itself carries.
        let seen = |v: Value| match v {
            Value::Bond(shared) => shared.borrow().clone(),
            held => held,
        };
        return self.element_held(target, at, how).map(seen);
    }

    fn element_held(&self, target: &Value, at: &Value, how: Reading) -> Res<Value> {
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
            Value::Array(items) => match items.get(i) {
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

    /// print and write: the values joined by spaces, or a template holding
    /// the definition's placeholders filled from the rest.
    fn render(&self, values: &[Value]) -> String {
        let sp = self.wording();
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
                        Some(v) => out.push_str(&v.display(&sp)),
                        None => out.push_str(&s[at..at + width]),
                    }
                    s = &s[at + width..];
                }
                out.push_str(s);
                return out;
            }
        }
        values.iter().map(|v| v.display(&sp)).collect::<Vec<_>>().join(" ")
    }

    fn builtin(&mut self, builtin: Builtin, name: &str, args: &mut Vec<Value>) -> Res<Value> {
        let sp = self.wording();
        let arity = |n: usize| -> Res<()> {
            if args.len() == n {
                return Ok(());
            }
            Err(format!("{}() expects {} argument{}, got {}", name, n, if n == 1 { "" } else { "s" }, args.len()))
        };
        Ok(match builtin {
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
            Builtin::ToText => {
                arity(1)?;
                Value::text(&args[0].display(&sp))
            }
            Builtin::ToInt => {
                arity(1)?;
                let whole = arith::whole_of(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::of_big(whole)
            }
            Builtin::AsReal => {
                arity(1)?;
                match &args[0] {
                    v @ Value::Real(_) => v.clone(),
                    v => arith::to_real(v, arith::DEFAULT_PLACES).ok_or_else(|| format!("{}() requires a number argument", name))?,
                }
            }
            Builtin::Length => {
                arity(1)?;
                match &args[0] {
                    Value::Text(s) => Value::Small(s.chars().count() as i64),
                    Value::Array(items) => Value::Small(items.len() as i64),
                    Value::Map(pairs) => Value::Small(pairs.len() as i64),
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
                let v = args.pop().expect("the value");
                let at = self.key(&args.pop().expect("the key"));
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
                        if let Value::Bond(shared) = &items[i] {
                            *shared.borrow_mut() = v;
                            return Ok(Value::Array(items));
                        }
                        Rc::make_mut(&mut items)[i] = v;
                        Value::Array(items)
                    }
                    // Any other key makes it a map, its places the keys.
                    Value::Array(items) => {
                        let mut pairs: Vec<(Value, Value)> =
                            items.iter().enumerate().map(|(i, x)| (Value::Small(i as i64), x.clone())).collect();
                        put_key(&mut pairs, at, v);
                        Value::Map(Rc::new(pairs))
                    }
                    Value::Map(mut pairs) => {
                        put_key(Rc::make_mut(&mut pairs), at, v);
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
                    Value::Array(items) => kept.extend(items.iter().map(|v| (number(), v.clone()))),
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
                        let raw = at.as_big()?.to_i64().ok_or_else(|| self.lang.del_unrun.clone())?;
                        let i = if raw < 0 { items.len() as i64 + raw } else { raw };
                        if i < 0 || i as usize >= items.len() { return Err(self.lang.del_unrun.clone().into()); }
                        let mut left = items.as_ref().clone();
                        left.remove(i as usize);
                        Value::array(left)
                    }
                    Value::Array(items) => {
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
        Value::Array(items) => items.as_ref().clone(),
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
        Value::Array(items) => items
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
        Value::Array(items) => {
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
        Value::Array(items) => ("Array".to_string(), items.iter().enumerate().map(|(i, x)| (i.to_string(), x)).collect()),
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
        Value::Array(items) => items,
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
        Value::Array(items) => {
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
        Value::Array(items) => {
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
