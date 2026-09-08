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
use crate::value::{Class, Instance, Sort, Wording, Value};
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
    source: String,
    /// Nothing at all, to hand back where a binding never written is
    /// read in place and the language only complains about it.
    nothing: Value,
    /// The line the last value raised was raised on, which a language
    /// that tells where a run ended names.
    hurled_at: std::cell::Cell<u32>,
    /// How long the run may take, in seconds, and when the count began;
    /// nought is no limit at all.
    limit: std::cell::Cell<usize>,
    began: std::cell::Cell<Option<std::time::Instant>>,
    /// How many pieces of the program now being found asked for quiet.
    /// Counted rather than flagged, since one hushed piece may hold
    /// another.
    hushed: std::cell::Cell<usize>,
    /// What the run has written out while it was being kept rather than
    /// let go, innermost last. A keeping within a keeping writes into
    /// the one around it when it is given up.
    holding: RefCell<Vec<String>>,
    /// The routines to run once the program's own last statement is
    /// done, each with what it is to be handed, in the order they were
    /// named.
    when_done: RefCell<Vec<(Value, Vec<Value>)>>,
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
            source: String::new(),
            nothing: Value::Null,
            hurled_at: std::cell::Cell::new(0),
            limit: std::cell::Cell::new(0),
            began: std::cell::Cell::new(None),
            hushed: std::cell::Cell::new(0),
            holding: RefCell::new(Vec::new()),
            when_done: RefCell::new(Vec::new()),
            complainer: RefCell::new(None),
            waiting: RefCell::new(Vec::new()),
            any_waiting: std::cell::Cell::new(false),
            args_cell: find(&lang.args_binding),
            memo_cell: find(&lang.memo_binding),
            registry,
        }
    }

    /// Assemble source against the globals this run already has and run
    /// it where it stands, giving back whatever it answered with.
    /// Text read while the run is going, built and run where it stands.
    /// Where it came from a file of its own, that file is where the run
    /// is written for as long as it lasts: a complaint names it, and a
    /// file it asks for in turn is looked for beside it.
    fn run_source(&mut self, source: &str, came_from: Option<String>) -> Res<Value> {
        let tokens = crate::layout::layout(crate::lex::lex(source, self.lang)?, self.lang)?;
        let program = crate::compile::compile_from(&tokens, self.lang, &mut self.registry, 0, came_from.as_deref().map(Rc::from))?;
        // Names the new source brought with it want room in the world.
        self.world.resize(self.registry.idents.len(), Value::Blank);
        let (was_written_in, was_on) = (self.source.clone(), self.line);
        if let Some(place) = came_from {
            self.source = place;
        }
        let base = self.data.len();
        let ran = self.invoke(&program, Vec::new());
        self.source = was_written_in;
        self.line = was_on;
        match ran {
            Ok(()) => {}
            Err(Fault::Note(told)) => return Err(told),
            Err(other) => return Err(other.told(&self.names())),
        }
        // What it left behind is its answer; nothing left is a plain yes.
        Ok(match self.data.len() > base {
            true => self.drop_top()?,
            false => Value::Small(1),
        })
    }

    /// Where the program is written, which a complaint names.
    pub fn written_in(&mut self, place: &str) {
        self.source = place.to_string();
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
                print!("{}", text);
            }
        }
    }

    /// The routines named to run once the program is done, in the order
    /// they were named. One that raises something stops the rest, as a
    /// fault anywhere else does.
    pub fn run_when_done(&mut self) -> Result<(), Fault> {
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
        if self.hushed.get() > 0 {
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
        self.utter(&format!("\n{}: {} in {} on line {}\n", word, message, self.source, line));
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
                self.utter(&format!("\n{}: {} in {} on line {}\n", word, told, self.source, self.line));
                return;
            }
            Fault::Thrown(raised) => raised,
            // A fault of the kernel's own is told under the class the
            // language names for one, where it names any.
            Fault::Note(told) => {
                let Some(named) = self.class_for(told) else { return };
                let at = self.line;
                self.utter(&format!("\n{}: Uncaught {}: {} in {}:{}\n", word, named, told, self.source, at));
                self.utter(&format!("Stack trace:\n#0 {{main}}\n  thrown in {} on line {}\n", self.source, at));
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
        self.utter(&format!("\n{}: {} in {}:{}\n", word, said, self.source, at));
        self.utter(&format!("Stack trace:\n#0 {{main}}\n  thrown in {} on line {}\n", self.source, at));
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
            _ if told.starts_with("Division by zero") => &self.lang.fault_division,
            _ if told.starts_with("Bit shift by") => &self.lang.fault_arithmetic,
            _ if told.starts_with("Cannot coerce") => &self.lang.fault_kind,
            _ => &None,
        };
        named.clone().or_else(|| self.lang.fault_class.clone())
    }

    fn as_fault(&mut self, told: &str) -> Option<Value> {
        let named = self.class_for(told)?;
        let Some(Value::Class(class)) = self.lookup(&named).cloned() else { return None };
        self.hurled_at.set(self.line);
        self.made += 1;
        let mut fields = class.all_fields();
        match fields.iter_mut().find(|(n, _)| n == "message") {
            Some(place) => place.1 = Value::text(told),
            None => fields.push(("message".to_string(), Value::text(told))),
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
    fn what_it_spells(&self, v: Value) -> Value {
        let Value::Text(name) = &v else { return v };
        if !self.lang.spelled_stands {
            return v;
        }
        match self.lookup(name) {
            Some(found @ (Value::Class(_) | Value::Routine(_))) => found.clone(),
            _ => v,
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
            real_digits: self.lang.real_bits.and(self.lang.real_digits),
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
        frame.extend(self.data.drain(at..));
        // What the routine does not name stays aside rather than
        // spilling into the slots its own names sit in.
        frame.truncate(program.formals.len());
        frame.resize(program.idents.len(), Value::Blank);
        let base = self.data.len();
        // A routine written in a file of its own is run as being in it:
        // a complaint names that file, and a file the routine asks for
        // is looked for beside it, wherever the call was made.
        let elsewhere = program.written_in.as_ref().map(|place| {
            let was = std::mem::replace(&mut self.source, place.to_string());
            (was, self.line)
        });
        let outcome = self.run_instrs(program, &mut frame);
        if let Some((was, on)) = elsewhere {
            self.source = was;
            self.line = on;
        }
        if watching {
            self.given.pop();
        }
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

    fn run_instrs(&mut self, program: &Rc<Routine>, frame: &mut [Value]) -> Flow<()> {
        let instrs = &program.instrs;
        // A raised value leaves the marks that end a quiet piece unrun,
        // so how much quiet stood when the piece began is what stands
        // again once it has gone.
        let quiet = self.hushed.get();
        let mut outcome = self.run_body(frame, instrs);
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

    fn run_body(&mut self, frame: &mut [Value], instrs: &[crate::code::Instr]) -> Flow<()> {
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
                Instr::Write(slot) => {
                    let v = self.drop_top()?;
                    self.store_cell(slot, frame, v)?;
                }
                Instr::Act(op, argc) => {
                    if let Err(fault) = self.perform(op, *argc) {
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
                Instr::Forget(slot) => {
                    for &s in &slot.near {
                        frame[s] = Value::Blank;
                    }
                    if slot.near.is_empty() {
                        self.world[slot.far] = Value::Blank;
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
                Instr::Line(row) => self.line = *row,
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
                        let (p, q) = arith::parts(&worth).unwrap_or((BigInt::from(0), BigInt::from(1)));
                        self.within_width(Value::of_big(p / q))
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
                        let out: Vec<u8> = s.as_bytes().iter().map(|c| !c).collect();
                        Value::text(&String::from_utf8_lossy(&out))
                    }
                    _ => Value::Small(!bits_of(&v)?),
                }
            }
            Action::Negate => {
                // 0 - x, so a real keeps its precision.
                let v = self.drop_top()?;
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
                        let subject = match self.what_it_spells(pair[0].clone()) {
                            Value::Class(_) => Value::Null,
                            held => held,
                        };
                        let mut args = self.drop_many(argc - 1)?;
                        if matches!(subject, Value::Null) {
                            let stands = self.what_it_spells(pair[0].clone());
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
                            Some((n, v)) => if key { Value::text(n) } else { v.clone() },
                            None => return Err(format!("Array index {} out of bounds (length: {})", at, held.len()).into()),
                        }
                    }
                    _ => return Err("Cannot walk a value that is not an array".to_string().into()),
                }
            }
            Action::AtEnd => return Err("An empty index belongs on the left of an assignment".to_string().into()),
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
                    methods: plan.methods.clone(),
                    shared: RefCell::new(take(&plan.shared_names)),
                    constants: take(&plan.constant_names),
                }))
            }
            Action::Make => {
                let mut args = self.drop_many(argc)?;
                let stands = self.what_it_spells(args.remove(0));
                let Value::Class(class) = stands else {
                    return Err("Only a class can be made into an object".to_string().into());
                };
                self.made += 1;
                let object = Rc::new(Instance { class: class.clone(), fields: RefCell::new(class.all_fields()), mark: self.made });
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
                    let found = o.fields.borrow().iter().find(|(n, _)| n == name.as_ref()).map(|(_, v)| v.clone());
                    match found {
                        // A property held in a shared cell reads as what
                        // the cell holds, the sharing being between the
                        // names and not in the value.
                        Some(Value::Bond(shared)) => shared.borrow().clone(),
                        Some(v) => v,
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
                        let at = match held.iter().position(|(n, _)| n == name.as_ref()) {
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
                        o.fields.borrow_mut().retain(|(n, _)| n != name.as_ref());
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
                        let mut fields = o.fields.borrow_mut();
                        match fields.iter_mut().find(|(n, _)| n == name.as_ref()) {
                            Some(place) => place.1 = value,
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
                let method = method.ok_or_else(|| format!("Call to undefined method {}::{}()", o.class.name, name))?;
                let mut all = vec![Value::Object(o)];
                all.extend(args);
                return self.invoke(&method, all);
            }
            Action::Reach(name) => match {
                let top = self.drop_top()?;
                self.what_it_spells(top)
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
                v => return Err(format!("Cannot reach '{}' in {}", name, v.plain()).into()),
            },
            Action::Sow(name) => {
                let mut pair = self.drop_many(2)?;
                let value = pair.pop().expect("the value");
                let stands = self.what_it_spells(pair.pop().expect("the class"));
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
                    v => return Err(format!("Cannot write '{}' in {}", name, v.plain()).into()),
                }
            }
            Action::Summon(name) => {
                let mut args = self.drop_many(argc)?;
                let this = args.remove(0);
                let stands = self.what_it_spells(args.remove(0));
                let Value::Class(class) = stands else {
                    return Err(format!("Cannot call '{}' on a value that is not a class", name).into());
                };
                let method = class.method(&name).cloned();
                let method = method.ok_or_else(|| format!("Call to undefined method {}::{}()", class.name, name))?;
                let mut all = vec![this];
                all.extend(args);
                return self.invoke(&method, all);
            }
            Action::Kindred(name) => match self.drop_top()? {
                Value::Object(o) => Value::Flag(o.class.descends_from(&name)),
                _ => Value::Flag(false),
            },
            Action::Twin => {
                let top = self.data.last().cloned().ok_or("Stack underflow")?;
                top
            }
            Action::Matches(names) => match self.drop_top()? {
                Value::Object(o) => Value::Flag(names.iter().any(|n| o.class.descends_from(n))),
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
                result?
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
                    (Value::Text(x), Value::Text(y)) => match (number_spelled(x), number_spelled(y)) {
                        (Some(m), Some(n)) => m.equals(&n),
                        _ => x == y,
                    },
                    (Value::Text(s), other) | (other, Value::Text(s)) if numeric(other) => match number_spelled(s) {
                        Some(n) => n.equals(other),
                        None => s.as_ref() == other.display(&sp),
                    },
                    _ => a.equals(b),
                };
                Value::Flag(matches!(op, Action::Eq) == alike)
            }
            Action::Eq => Value::Flag(a.equals(b)),
            Action::Ne => Value::Flag(!a.equals(b)),
            Action::Same => Value::Flag(a.identical(b)),
            Action::Unsame => Value::Flag(!a.identical(b)),
            Action::Join => joined(),
            Action::At => self.element(a, b)?,
            // Reaching inside makes the place on the way where nothing
            // is there yet, which is what a write to it means.
            Action::Nested if self.lang.makes_places => {
                self.hushed.set(self.hushed.get() + 1);
                let found = self.element(a, b).unwrap_or(Value::Null);
                self.hushed.set(self.hushed.get() - 1);
                match found {
                    Value::Null | Value::Blank | Value::Gap => Value::Array(std::rc::Rc::new(Vec::new())),
                    held => held,
                }
            }
            Action::Nested => self.element(a, b)?,
            // Looking has nothing to say about what is not there.
            Action::Peek => {
                self.hushed.set(self.hushed.get() + 1);
                let found = self.element(a, b).unwrap_or(Value::Null);
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
                let (x, y) = (x.as_bytes(), y.as_bytes());
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
            Action::BitBoth | Action::BitEither | Action::BitOne | Action::BitUp | Action::BitDown => {
                let (x, y) = (bits_of(a)?, bits_of(b)?);
                Value::Small(match op {
                    Action::BitBoth => x & y,
                    Action::BitEither => x | y,
                    Action::BitOne => x ^ y,
                    _ => {
                        if y < 0 {
                            return Err("Bit shift by a negative number".to_string());
                        }
                        let places = y.min(64) as u32;
                        match op {
                            Action::BitUp => x.checked_shl(places).unwrap_or(0),
                            // Moving down keeps the sign, so a negative
                            // number falls to -1 rather than to 0.
                            _ => x.checked_shr(places).unwrap_or(if x < 0 { -1 } else { 0 }),
                        }
                    }
                })
            }
            Action::Add if self.lang.concat.is_none() && (matches!(a, Value::Text(_)) || matches!(b, Value::Text(_))) => joined(),
            // Text that spells a number is worked with as that number,
            // fractions included, rather than only as a whole one. Text
            // that spells one and then goes on saying something else is
            // worth the number it opens with, and text that spells none
            // is worth nothing; a language with a word for a warning is
            // told of both rather than stopped.
            _ if matches!(a, Value::Text(_)) || matches!(b, Value::Text(_)) => {
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
                        self.complain(Complaint::Warning, "A non-well-formed numeric value encountered");
                        n
                    }
                    Some((None, _)) => {
                        self.complain(Complaint::Warning, "A non-numeric value encountered");
                        Value::Small(0)
                    }
                };
                let (x, y) = (worth(x, a), worth(y, b));
                return self.dyadic_numbers(op, &x, &y);
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
        if self.lang.real_bits.is_none() {
            return v;
        }
        let places = self.lang.real_digits.unwrap_or(arith::DEFAULT_PLACES);
        let (p, q, below) = match &v {
            Value::Real(r) => (r.p.clone(), r.q.clone(), r.below),
            Value::Frac(r) => (r.p.clone(), r.q.clone(), false),
            _ => return v,
        };
        match crate::value::from_binary(crate::value::as_binary(&p, &q)) {
            // A nought below nought keeps its minus at any width.
            Some((p, q)) => arith::shape_signed(p, q, Some(places), below),
            // Beyond every number of that width, and so left as it is.
            None => v,
        }
    }

    /// The arithmetic itself, both values already numbers as far as they
    /// can be made so. Where a language holds its reals to a width, a
    /// whole number meeting a real is brought to that width first, so
    /// that the two are added as such a language adds them.
    fn dyadic_numbers(&self, op: &Action, a: &Value, b: &Value) -> Res<Value> {
        let real_here = |v: &Value| matches!(v, Value::Real(_) | Value::Frac(_));
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
            (Value::Text(x), Value::Text(y)) => match (number_spelled(x), number_spelled(y)) {
                (Some(m), Some(n)) => exactly(&m, &n)?,
                _ => x.as_ref() < y.as_ref(),
            },
            (Value::Text(s), other) if numeric(other) => match number_spelled(s) {
                Some(n) => exactly(&n, other)?,
                None => s.as_ref() < other.display(&sp).as_str(),
            },
            (other, Value::Text(s)) if numeric(other) => match number_spelled(s) {
                Some(n) => exactly(other, &n)?,
                None => other.display(&sp).as_str() < s.as_ref(),
            },
            _ => exactly(a, b)?,
        })
    }

    /// A key as this language takes one.
    fn key(&self, at: &Value) -> Value {
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
        if matches!(v, Value::Flag(_)) {
            return v.display(&self.wording());
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

    /// How a key an array does not hold is named in a complaint: text
    /// in quotation marks, a number as it stands.
    fn key_named(&self, at: &Value) -> String {
        match at {
            Value::Text(s) => format!("\"{}\"", s),
            other => other.plain(),
        }
    }

    fn element(&self, target: &Value, at: &Value) -> Res<Value> {
        // A place holding a cell two names share reads as what the cell
        // holds, since the sharing is between the names and not
        // something the value itself carries.
        let seen = |v: Value| match v {
            Value::Bond(shared) => shared.borrow().clone(),
            held => held,
        };
        return self.element_held(target, at).map(seen);
    }

    fn element_held(&self, target: &Value, at: &Value) -> Res<Value> {
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
                number_opening(spelling).0.unwrap_or_else(|| at.clone())
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
            _ => Err("Cannot index non-array value".to_string()),
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
            Builtin::Eval | Builtin::Include => {
                arity(1)?;
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
                        let beside = std::path::Path::new(&self.source).parent().map(|near| near.join(&given));
                        let near = beside.filter(|near| near.exists());
                        came_from = Some(match &near {
                            Some(place) => place.to_string_lossy().into_owned(),
                            None => given.clone(),
                        });
                        let found = match near {
                            Some(place) => std::fs::read(place),
                            None => std::fs::read(&given),
                        };
                        match found {
                            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                            Err(_) => return Ok(Value::Flag(false)),
                        }
                    }
                };
                return self.run_source(&source, came_from);
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
                match std::fs::write(where_to, what.as_bytes()) {
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
            Builtin::TimeLimit => {
                arity(1)?;
                let seconds = as_index(&args[0])?;
                self.limit.set(seconds);
                self.began.set(Some(std::time::Instant::now()));
                Value::Flag(true)
            }
            // The routine every complaint is to be handed to, or none.
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
                    self.utter(&format!("{}\n", dumped(v, 0, self.lang.real_bits.is_some())));
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
                let (p, q) = arith::parts(&args[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::of_big(p / q)
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
                self.element(&args[0], &args[1])?
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
                    _ => return Err(format!("{}() requires an array", name)),
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
                    let Some(letter) = put.chars().next() else {
                        return Err("Cannot write nothing into a place in text".to_string());
                    };
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
                        Err(_) => match arith::parts(&at).map(|(p, q)| &p / &q).and_then(|n| n.to_i64()) {
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
                    _ => return Err(format!("{}() requires an array", name)),
                }
            }
            Builtin::Pack => return Err(format!("{}() is a literal, not a call", name)),
            // Asking is done where the program is put together, since
            // what is asked about is a name and not its value.
            Builtin::Held => return Err("Only a name or a place in an array can be asked about".into()),
            Builtin::Erase => {
                // Taking a place out of an array: the array is given back
                // without it.
                arity(2)?;
                let at = self.key(&args.pop().expect("the place"));
                match args.pop().expect("the array") {
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
                self.utter(&laid_out(&args[0], 0));
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

pub fn sort_value(kind: Sort) -> Value {
    Value::SortOf(kind)
}

pub fn places_default() -> Value {
    Value::Small(arith::DEFAULT_PLACES as i64)
}

/// A value with its kind, as PHP's var_dump shows it: a number as
/// `int(n)` or `float(x)`, text with its byte length, an array one
/// entry per line, nested arrays indented two more.
fn dumped(v: &Value, depth: usize, binary_reals: bool) -> String {
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
        Value::Bond(shared) => dumped(&shared.borrow(), depth, binary_reals),
        Value::Small(_) | Value::Huge(_) => format!("int({})", v.plain()),
        // Shown with its kind, a binary real is written in the fewest
        // digits that read back as the same number.
        // A nought below nought is written as such, whatever the width.
        Value::Real(r) if r.below && num_traits::Zero::is_zero(&r.p) => "float(-0)".to_string(),
        Value::Real(r) if binary_reals => format!("float({})", crate::value::binary_string(crate::value::as_binary(&r.p, &r.q), None)),
        Value::Real(_) | Value::Frac(_) => format!("float({})", v.plain()),
        Value::Text(s) => format!("string({}) \"{}\"", s.len(), s),
        Value::Flag(b) => format!("bool({})", b),
        Value::Array(items) => {
            let mut out = format!("array({}) {{\n", items.len());
            for (i, item) in items.iter().enumerate() {
                out.push_str(&format!("{pad}  [{i}]=>\n{pad}  {}{}\n", shared(item), dumped(item, depth + 1, binary_reals)));
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
                out.push_str(&format!("{pad}  [{shown}]=>\n{pad}  {}{}\n", shared(item), dumped(item, depth + 1, binary_reals)));
            }
            out.push_str(&pad);
            out.push('}');
            out
        }
        Value::Object(thing) => {
            let held = thing.fields.borrow();
            let mut out = format!("object({})#{} ({}) {{\n", thing.class.name, thing.mark, held.len());
            for (member, item) in held.iter() {
                out.push_str(&format!("{pad}  [\"{member}\"]=>\n{pad}  {}{}\n", shared(item), dumped(item, depth + 1, binary_reals)));
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

/// A value over lines, as PHP's print_r writes it: a scalar bare, an
/// array as `Array` and its places in brackets, each nested array set
/// eight spaces further in and followed by a blank line.
fn laid_out(v: &Value, indent: usize) -> String {
    let held;
    let (what, pairs): (String, Vec<(String, &Value)>) = match v {
        Value::Array(items) => ("Array".to_string(), items.iter().enumerate().map(|(i, x)| (i.to_string(), x)).collect()),
        Value::Map(entries) => ("Array".to_string(), entries.iter().map(|(k, x)| (k.plain(), x)).collect()),
        Value::Object(thing) => {
            held = thing.fields.borrow();
            (format!("{} Object", thing.class.name), held.iter().map(|(k, x)| (k.clone(), x)).collect())
        }
        other => return other.plain(),
    };
    let pad = " ".repeat(indent);
    let mut out = format!("{what}\n{pad}(\n");
    for (key, item) in pairs {
        // What an array lays out ends its own line, so the newline
        // here is the gap PHP leaves after it; for a scalar it ends the line.
        let shown = laid_out(item, indent + 8);
        out.push_str(&format!("{pad}    [{key}] => {shown}\n"));
    }
    out.push_str(&format!("{pad})\n"));
    out
}

/// The place an array holds, made a shared cell so that a name fastened
/// to it writes into the array itself.
fn shared_item(held: &mut Value, at: &Value) -> Res<Rc<RefCell<Value>>> {
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

/// The number a piece of text spells, whole or fractional, with room
/// for a sign and for space around it. Anything else is not a number.
fn number_spelled(s: &str) -> Option<Value> {
    let text = s.trim();
    // A number may carry a power of ten after it: 1e2, 1.5E-3.
    if let Some(at) = text.find(['e', 'E']) {
        let (front, back) = text.split_at(at);
        let power: i32 = back[1..].parse().ok()?;
        let base = number_spelled(front)?;
        let scale = Value::of_big(BigInt::from(10).pow(power.unsigned_abs()));
        let how = if power >= 0 { Operation::Times } else { Operation::OverReal };
        return arith::calculate(how, &base, &scale)?.ok();
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
    match arith::Exact::from_value(&whole) {
        // Dividing whole numbers cuts towards nothing, which is what
        // dropping what lies past the point comes to. A number too wide
        // to be held in the bits at all comes to the lowest of them,
        // which is what a machine holding numbers to a width gives.
        Some(exact) => Ok((&exact.p / &exact.q).to_i64().unwrap_or(i64::MIN)),
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
    match number_spelled(&text[..end]) {
        Some(opening) => (Some(opening), false),
        None => (None, false),
    }
}
