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

type Res<T = Value> = Result<T, Escape>;

enum Next {
    Value(Value),
    /// The program to run next, its frame already built.
    Jump(Rc<Routine>, Rc<Env>),
}

pub struct Machine<'a> {
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
    raised_on: u32,
    written_in: String,
    /// The words this language has for the kinds of complaint.
    complaint_words: Vec<(&'static str, String)>,
    /// How many seconds the run may take and when the count began;
    /// nought is no limit at all.
    allowed: usize,
    started: Option<std::time::Instant>,
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
        Machine {
            table,
            outermost: Env::make(idents.len(), None),
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
            raised_on: 0,
            allowed: 0,
            started: None,
            written_in: String::new(),
            quieted: 0,
            silenced: 0,
            inside: Vec::new(),
            holding: RefCell::new(Vec::new()),
            afterward: RefCell::new(Vec::new()),
            things: RefCell::new(Vec::new()),
            read_before: RefCell::new(std::collections::HashSet::new()),
            knows_cells: (HashMap::new(), HashMap::new(), std::collections::HashSet::new()),
            frames_named: Vec::new(),
            hearer: RefCell::new(None),
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
            loose_equals: table.single("ext.op.identical").is_some(),
        }
    }

    /// Where the program is written, which a complaint names.
    pub fn found_in(&mut self, place: &str) {
        self.written_in = place.to_string();
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
                let mut walking = v[0].clone();
                while let Some(further) = self.walk_handed(&walking)? {
                    walking = further;
                }
                match self.walks_itself(&walking) {
                    Some(_) => {
                        self.walk_asked(&walking, self.table.single("ext.op.walk.rewind").map(str::to_string))?;
                    }
                    None => self.can_be_walked(&walking)?,
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
            _ => return Err(Escape::Error(format!("{}() is no step of a walk", name))),
        })
    }

    /// A value with no places at all cannot be walked. A language with
    /// a word for a warning is told so and walks it no times, instead of
    /// having the run stopped over it.
    fn can_be_walked(&mut self, x: &Value) -> Result<(), Escape> {
        if matches!(x, Value::Vector(_) | Value::Dict(_) | Value::Thing(_)) {
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
    fn walk_handed(&mut self, x: &Value) -> Result<Option<Value>, Escape> {
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
            false => Some(handed),
        })
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
        match self.class_bound(name) {
            Some(found @ (Value::Blueprint(_) | Value::Routine(_) | Value::Bound(..))) => found.clone(),
            _ => v,
        }
    }

    /// The outermost binding of that name. Where classes go by their
    /// names however the names are written, a name nothing stands
    /// under is asked for again with its letters written small.
    fn class_bound(&self, name: &str) -> Option<Value> {
        if let found @ Some(_) = self.lookup(name) {
            return found;
        }
        match self.classes_either_way {
            true => self.lookup(&name.to_lowercase()),
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
                print!("{}", text);
            }
        }
    }

    /// The routines named to run once the program is done, in the order
    /// they were named. One that raises something stops the rest, as a
    /// fault anywhere else does.
    pub fn run_afterward(&mut self) -> Result<(), String> {
        // The clock the run was timed against is put away first: what a
        // program named to run afterward runs even where the run was
        // stopped for taking too long, and stopping it again would stop
        // something that was never given time of its own.
        self.started = None;
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
                self.invoke(p, env, given).map_err(|e| match e {
                    Escape::Error(m) => m,
                    _ => String::new(),
                })?;
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
        self.utter(&format!("\n{}: {} in {} on line {}\n", word, about, self.written_in, row));
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
                Value::text(&self.written_in.clone()),
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
        let told_of = |label: &str| self.table.single(label) == Some(told);
        let by_kind = match told {
            // Words the definition itself gave for a place outside the
            // range a value may take are known by being those very words.
            _ if told_of("ext.builtin.args.at.below") || told_of("ext.builtin.args.at.beyond") => Some("ext.system.fault.class.value"),
            _ if told.starts_with("Division by zero") => Some("ext.system.fault.class.division"),
            _ if told.starts_with("Bit shift by") => Some("ext.system.fault.class.arithmetic"),
            _ if told.starts_with("Cannot coerce") => Some("ext.system.fault.class.kind"),
            // An argument that is not of the class its parameter takes.
            _ if told.contains(" must be of type ") => Some("ext.system.fault.class.kind"),
            // Words the definition gave for an operand that can take no
            // part are known by the message opening with them.
            _ if self.table.single("ext.system.fault.operands").map_or(false, |w| told.starts_with(w)) => {
                Some("ext.system.fault.class.kind")
            }
            _ => None,
        };
        by_kind
            .and_then(|label| self.table.single(label))
            .or_else(|| self.table.single("ext.system.fault.class"))
            .map(str::to_string)
    }

    fn as_raised(&mut self, told: &str) -> Option<Value> {
        let named = self.class_of_fault(told)?;
        let Some(Value::Blueprint(of)) = self.class_bound(&named) else { return None };
        self.made += 1;
        let mut holds = of.every_field();
        match holds.iter_mut().find(|(k, _)| k == "message") {
            Some(place) => place.1 = Value::text(told),
            None => holds.push(("message".to_string(), Value::text(told))),
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

    /// How a run ended, told the way a language with a word for the end
    /// of one tells it: what stopped it, where, and how the run stood.
    /// Nothing is written where a language has no word for it.
    fn end_of_run(&self, said: &str) {
        let Some((_, word)) = self.complaint_words.iter().find(|(k, _)| *k == "fatal") else { return };
        self.utter(&format!("\n{}: {} in {}:{}\n", word, said, self.written_in, self.raised_on));
        self.utter(&format!("Stack trace:\n#0 {{main}}\n  thrown in {} on line {}\n", self.written_in, self.raised_on));
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
        let tokens = crate::scan::scan(source, self.table).map_err(Escape::Error)?;
        let tokens = crate::indent::indent(tokens, self.table).map_err(Escape::Error)?;
        let held: Vec<String> = self.frames_named.last().map_or_else(Vec::new, |p| p.idents.clone());
        let knows = (&self.knows_cells.0, &self.knows_cells.1, &self.knows_cells.2);
        let built = crate::build::build_within(&tokens, self.table, &self.idents, &held, knows, 0).map_err(Escape::Error)?;
        self.idents = built.globals;
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        frame.cells.borrow_mut().resize(built.program.idents.len().max(held.len()), Value::Unset);
        let answer = self.value_of(&built.program.body, frame)?;
        Ok(match answer {
            Value::Nil | Value::Unset => Value::Small(1),
            other => other,
        })
    }

    fn run_source(&mut self, source: &str, came_out_of: Option<String>) -> Result<Value, String> {
        let tokens = crate::scan::scan(source, self.table)?;
        let tokens = crate::indent::indent(tokens, self.table)?;
        let built = crate::build::build_from(&tokens, self.table, &self.idents, HashMap::new(), true, 0, came_out_of.as_deref().map(Rc::from))?;
        self.idents = built.globals;
        // Names the new source brought with it want room to stand in.
        self.outermost.cells.borrow_mut().resize(self.idents.len(), Value::Unset);
        let (was_in, was_on) = (self.written_in.clone(), self.row);
        if let Some(place) = came_out_of {
            self.written_in = place;
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
            Err(other) => Err(match other {
                Escape::Done => return Ok(Value::Nil),
                Escape::Stopped(told) => told,
                Escape::Thrown(raised) => format!("Uncaught {}", raised.bare()),
                _ => "A run of source ended oddly".to_string(),
            }),
        }
    }

    pub fn run_main(&mut self, body: &Form) -> Result<(), String> {
        let top = self.outermost.clone();
        match self.value_of(body, &top) {
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
                    self.utter(&format!("\n{}: {} in {} on line {}\n", word, told, self.written_in, self.row));
                }
                Err(told)
            }
            // A fault of the kernel's own is told under the class the
            // language names for one, where it names any.
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
                    Value::Dict(pairs) => Value::Dict(Rc::new(pairs.iter().filter(|(k, _)| !k.equals(&at)).cloned().collect())),
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
                // A statement is a fair place to look at the clock:
                // often enough to stop a run that runs away, seldom
                // enough that asking costs little.
                if let Some(over) = self.past_its_time() {
                    return Err(over);
                }
                self.value_of(inner, frame)
            }
            Form::Missing(slot) => {
                let f = ascend(frame, slot.up);
                let empty = matches!(f.cells.borrow()[slot.at], Value::Unset);
                Ok(Value::Flag(empty))
            }
            Form::Attempt { body, clauses, last } => {
                let ending = self.value_of(body, frame);
                // A language that names a class for the kernel's own
                // faults has one raised as a value of that class, so a
                // clause may take it like any other raised value.
                let ending = match ending {
                    Err(Escape::Error(told)) => match self.as_raised(&told) {
                        Some(made) => Err(Escape::Thrown(made)),
                        None => Err(Escape::Error(told)),
                    },
                    other => other,
                };
                let ending = match ending {
                    Err(Escape::Thrown(raised)) => {
                        // The first clause that takes this class holds it
                        // and runs; what none takes is raised again.
                        let of = match &raised {
                            Value::Thing(thing) => Some(thing.of.clone()),
                            _ => None,
                        };
                        let taken = clauses.iter().find(|clause| {
                            of.as_ref().map_or(false, |o| {
                                clause.classes.iter().any(|name| o.goes_by(name, self.classes_either_way))
                            })
                        });
                        match taken {
                            Some(clause) => {
                                if let Some(slot) = &clause.held {
                                    self.store(slot, frame, raised)?;
                                }
                                self.value_of(&clause.body, frame)
                            }
                            None => Err(Escape::Thrown(raised)),
                        }
                    }
                    other => other,
                };
                // The last part runs however the body ended, and only
                // then does whatever stopped it go on.
                if let Some(last) = last {
                    self.value_of(last, frame)?;
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
                Ok(Value::Blueprint(Rc::new(Blueprint {
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
            Form::Cycle { test, body, step, after } => {
                loop {
                    // A pass of a loop is a fair place to look at the
                    // clock as well as a statement is, since a loop whose
                    // body holds no statement at all — `for (;;) {}` —
                    // would otherwise never be looked at again.
                    if let Some(over) = self.past_its_time() {
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
                        Err(Escape::Leave(1)) => break,
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
                if let Some(done) = self.paired_call(&stands, args, frame) {
                    return done;
                }
                let (p, env) = self.routine_of(stands, target)?;
                let callee = self.env_for(&p, env, args, frame)?;
                self.drive(p, callee)
            }
            Form::Apply(Callee::Prim(op, name), args) => match op {
                Prim::Seq => {
                    let mut last = Value::Nil;
                    for a in args {
                        last = self.value_of(a, frame)?;
                    }
                    Ok(last)
                }
                Prim::Choose => {
                    let (p, env) = self.pick(args, frame)?;
                    self.invoke(p, env, Vec::new())
                }
                Prim::Walked | Prim::AloneWalk | Prim::MoreYet | Prim::AtHand | Prim::NamedHere | Prim::StepOn => {
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
                Prim::Spawn => {
                    let mut values = self.value_list(args, frame)?;
                    if values.is_empty() {
                        return Err("Nothing was given to make".to_string().into());
                    }
                    let stands = self.what_it_spells(values.remove(0));
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
                    let Value::Thing(thing) = subject else {
                        return Err(format!("Cannot call '{}' on something that is not an object", called).into());
                    };
                    let program = thing.of.program(&called).cloned();
                    let program = program.ok_or_else(|| format!("Call to undefined method {}::{}()", thing.of.name, called))?;
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
                    let holder = self.what_it_spells(values.remove(0));
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
                    let values = self.value_list(&args[1..], frame)?;
                    let want = if *op == Prim::Append { 1 } else { 2 };
                    if values.len() != want {
                        return Err(format!("{}() expects {} arguments, got {}", name, want + 1, values.len() + 1).into());
                    }
                    let (f, i) = self.locate(slot, frame)?;
                    let mut values = values;
                    let value = values.pop().unwrap();
                    let key = values.pop().map(|k| self.as_key_spoken(&k));
                    // Worked out before the place is reached, since
                    // reaching it holds the frame the name lives in.
                    let letter = self.letter_places.then(|| value.render(self.wording()));
                    // Through the shared cell when the name stands for one.
                    let shared = match &f.cells.borrow()[i] {
                        Value::Shared(cell) => Some(cell.clone()),
                        _ => None,
                    };
                    if let Some(cell) = shared {
                        let mut held = cell.borrow_mut();
                        written_into(&mut held, key, value, &slot.ident, self.builds_places, letter)?;
                        return Ok(Value::Nil);
                    }
                    let mut slots = f.cells.borrow_mut();
                    written_into(&mut slots[i], key, value, &slot.ident, self.builds_places, letter)?;
                    Ok(Value::Nil)
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
                    let values = self.value_list(args, frame)?;
                    let made = self.prim(*op, name, &values)?;
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

    /// A pair of a thing and a method's name, standing where a routine
    /// would: that method of that thing, the thing handed over first.
    /// A class in the first place names a method of the class itself.
    fn paired_call(&mut self, stands: &Value, args: &[Form], frame: &Rc<Env>) -> Option<Res<Value>> {
        if !self.spelled_stands {
            return None;
        }
        let Value::Vector(pair) = stands else { return None };
        if pair.len() != 2 {
            return None;
        }
        let called = pair[1].bare();
        let subject = self.what_it_spells(pair[0].clone());
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
            return Some(Err(format!("Call to undefined method {}::{}()", class.name, called).into()));
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
                if let Some(done) = self.paired_call(&stands, args, frame) {
                    return Ok(Next::Value(done?));
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

    fn env_for(&mut self, program: &Rc<Routine>, env: Rc<Env>, args: &[Form], caller: &Rc<Env>) -> Res<Rc<Env>> {
        let most = if self.reads_handed { usize::MAX } else { program.formals.len() };
        if args.len() > most || args.len() < program.least {
            self.value_list(args, caller)?;
            let wanted = program.formals.len();
            return Err(format!("Function {} expects {} arguments, got {}", program.ident, wanted, args.len()).into());
        }
        // Where what a call hands over can be read back, every argument
        // is worked out, even one the routine gives no name to.
        if self.reads_handed {
            let all = self.value_list(args, caller)?;
            for (at, v) in all.iter().enumerate() {
                self.of_the_class_written(program, at, v)?;
            }
            let frame = if program.frameless { env } else { Env::make(program.idents.len(), Some(env)) };
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
        let frame = Env::make(program.idents.len(), Some(env));
        for (at, (i, a)) in program.formal_slots.iter().zip(args).enumerate() {
            let v = self.value_of(a, caller)?;
            self.of_the_class_written(program, at, &v)?;
            frame.cells.borrow_mut()[*i] = v;
        }
        Ok(frame)
    }

    /// An argument is of the class its parameter was written to take. A
    /// parameter written with a kind naming no class is let be, since a
    /// language may bring such a value to the kind rather than refusing
    /// it, and nothing at all is let through, since a parameter with no
    /// value of its own may be handed nothing.
    fn of_the_class_written(&mut self, program: &Rc<Routine>, at: usize, x: &Value) -> Result<(), Escape> {
        let Some(Some(written)) = program.formal_kinds.get(at) else { return Ok(()) };
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
        Err(Escape::Error(format!(
            "{}(): Argument #{} ({}) must be of type {}, {} given, called in {} on line {}",
            program.ident,
            at + 1,
            program.formals.get(at).map_or("", String::as_str),
            written,
            handed,
            self.written_in,
            self.row
        )))
    }

    pub fn invoke(&mut self, program: Rc<Routine>, env: Rc<Env>, args: Vec<Value>) -> Res {
        let most = if self.reads_handed { usize::MAX } else { program.formals.len() };
        if args.len() > most || args.len() < program.least {
            let wanted = program.formals.len();
            return Err(format!("Function {} expects {} arguments, got {}", program.ident, wanted, args.len()).into());
        }
        if self.reads_handed {
            self.pending = args.clone();
        }
        let frame = if program.frameless {
            env
        } else {
            let frame = Env::make(program.idents.len(), Some(env));
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
        // handed takes the place of this call's too.
        let watching = self.reads_handed;
        if watching {
            let mine = std::mem::take(&mut self.pending);
            self.handed.push(mine);
        }
        let (mut program, mut frame) = (program, frame);
        // A program written in a file of its own runs as being in it: a
        // complaint names that file, and a file it asks for is sought
        // beside it, wherever the call was made.
        let elsewhere = program.written_in.as_ref().map(|place| {
            let was = std::mem::replace(&mut self.written_in, place.to_string());
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
        let mine = !program.frameless;
        if mine {
            self.frames_named.push(program.clone());
            self.inside.push(program.within.clone());
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
                    if mine {
                        if let Some(top) = self.frames_named.last_mut() {
                            *top = program.clone();
                        }
                        if let Some(here) = self.inside.last_mut() {
                            *here = program.within.clone();
                        }
                    }
                    if watching {
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
        if watching {
            self.handed.pop();
        }
        let result = outcome?;
        if let Some(k) = key {
            self.memo.insert(k, result.clone());
        }
        Ok(result)
    }

    // ---------- operations

    fn prim(&mut self, op: Prim, name: &str, v: &[Value]) -> Result<Value, String> {
        let w = self.wording();
        let n = |k: usize| -> Result<(), String> {
            if v.len() == k { Ok(()) } else { Err(format!("{}() expects {} argument{}, got {}", name, k, if k == 1 { "" } else { "s" }, v.len())) }
        };
        Ok(match op {
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
            // The steps of a walk that a thing may answer for itself are
            // worked out where a call can be made, not here.
            Prim::Walked | Prim::AloneWalk | Prim::MoreYet | Prim::AtHand | Prim::NamedHere | Prim::StepOn => {
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
            Prim::Of => {
                n(2)?;
                let called = v[1].bare();
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
                        if let Some(at) = self.member_place(&holds, &called) {
                            holds[at].1 = Value::Unset;
                        }
                        Value::Nil
                    }
                    other => return Err(format!("Cannot take property '{}' off {}", called, other.bare())),
                }
            }
            Prim::Onto => {
                n(3)?;
                let called = v[1].bare();
                match &v[0] {
                    Value::Thing(thing) => {
                        let mut holds = thing.holds.borrow_mut();
                        match self.member_place(&holds, &called) {
                            Some(at) => holds[at].1 = v[2].clone(),
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
                let stands = self.what_it_spells(v[0].clone());
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
                let stands = self.what_it_spells(v[0].clone());
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
                match &v[0] {
                    Value::Thing(thing) => Value::Flag(thing.of.goes_by(&v[1].bare(), self.classes_either_way)),
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
                    Value::Text(had) if self.letter_places => letter_put(had, &v[1], &v[2].render(w))?,
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
                n(1)?;
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
                        let near = std::path::Path::new(&self.written_in).parent().map(|place| place.join(&given));
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
                            Err(_) => return Ok(Value::Flag(false)),
                        }
                    }
                };
                return self.run_source(&source, came_out_of);
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
                match std::fs::write(place, what.as_bytes()) {
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
            Prim::Clock => {
                n(1)?;
                self.allowed = as_index(&v[0])?;
                self.started = Some(std::time::Instant::now());
                Value::Flag(true)
            }
            // Words said as a complaint of a kind the language names,
            // where the run stands.
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
                Value::Text(s) => Value::text(&letters_turned(s)),
                other => Value::Small(!self.bits_told(other)?),
            },
            // Two pieces of text meet letter by letter. The shorter one
            // says how far it goes, save where either bit will do, and
            // there the longer one carries on alone.
            Prim::BitsBoth | Prim::BitsEither | Prim::BitsOne
                if matches!(v[0], Value::Text(_)) && matches!(v[1], Value::Text(_)) =>
            {
                let (left, right) = (v[0].bare(), v[1].bare());
                let (left, right) = (left.as_bytes(), right.as_bytes());
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
            Prim::BitsUp | Prim::BitsDown => {
                let (bits, by) = (self.bits_told(&v[0])?, self.bits_told(&v[1])?);
                if by < 0 {
                    return Err("Bit shift by a negative number".to_string());
                }
                // Past sixty-four places nothing of the number is left,
                // save the sign when the bits go down.
                let far = by.min(64) as u32;
                Value::Small(if op == Prim::BitsUp {
                    bits.checked_shl(far).unwrap_or(0)
                } else {
                    bits.checked_shr(far).unwrap_or(if bits < 0 { -1 } else { 0 })
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
            Prim::Selfsame => Value::Flag(v[0].selfsame(&v[1])),
            Prim::Unlike => Value::Flag(!v[0].selfsame(&v[1])),
            Prim::Join => Value::text(&format!("{}{}", v[0].render(w), v[1].render(w))),
            Prim::At => self.element(&v[0], &v[1])?,
            // A glance has nothing to say about what is not there.
            Prim::Glance => {
                self.quieted += 1;
                let seen = self.element(&v[0], &v[1]).unwrap_or(Value::Nil);
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
            Prim::AsTruth => Value::Flag(self.stands_true(&v[0])),
            Prim::AsNothing => Value::Nil,
            Prim::AsVector => match v[0].clone() {
                held @ (Value::Vector(_) | Value::Dict(_)) => held,
                Value::Nil | Value::Unset => Value::Vector(std::rc::Rc::new(Vec::new())),
                held => Value::Vector(std::rc::Rc::new(vec![held])),
            },
            Prim::AsWhole => {
                let worth = self.worth_of(&v[0]);
                let r = math::ratio_of(&worth).unwrap_or(crate::data::Ratio { above: BigInt::from(0), beneath: BigInt::from(1), places: None, under: false });
                self.at_width(Value::from_big(&r.above / &r.beneath))
            }
            Prim::AsDecimal => {
                let worth = self.worth_of(&v[0]);
                let made = math::to_decimal(&worth, self.real_figures()).unwrap_or(Value::Small(0));
                self.at_width(made)
            }
            // Reaching in makes the place where nothing is there yet,
            // which is what a write into it asks for.
            Prim::Inward if !self.builds_places => self.element(&v[0], &v[1])?,
            Prim::Inward => {
                self.quieted += 1;
                let reached = self.element(&v[0], &v[1]).unwrap_or(Value::Nil);
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
            | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge
                // Only where every piece of text will give up a number,
                // so that what is worked out below is never text again.
                if (matches!(v[0], Value::Text(_)) || matches!(v[1], Value::Text(_)))
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
            Prim::AsText => {
                n(1)?;
                Value::text(&v[0].render(w))
            }
            Prim::AsInt => {
                n(1)?;
                let e = math::ratio_of(&v[0]).ok_or_else(|| format!("{}() requires a number argument", name))?;
                Value::from_big(e.above / e.beneath)
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
                self.element(&v[0], &v[1])?
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
            | Prim::Spawn | Prim::Ask | Prim::Bid | Prim::Hurl | Prim::Otherwise => unreachable!("handled in eval"),
        })
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

    fn element(&self, target: &Value, at: &Value) -> Result<Value, String> {
        // A place holding a cell that names share reads as whatever the
        // cell holds: the sharing lies between the names, not in the
        // value itself.
        return self.element_within(target, at).map(|found| match found {
            Value::Shared(cell) => cell.borrow().clone(),
            held => held,
        });
    }

    fn element_within(&self, target: &Value, at: &Value) -> Result<Value, String> {
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
            (Value::Text(_), Value::Text(_)) if self.letter_places => number_opening_in(at).0,
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
            _ => Err(self.no_places()),
        }
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
        Value::Text(s) => format!("string({}) \"{}\"", s.len(), s),
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
fn letter_put(had: &str, at: &Value, put: &str) -> Result<Value, String> {
    let Some(letter) = put.chars().next() else {
        return Err("Cannot write nothing into a place in text".to_string());
    };
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
    Ok(Value::text(&letters.into_iter().collect::<String>()))
}

fn written_into(held: &mut Value, key: Option<Value>, value: Value, ident: &str, builds: bool, letter: Option<String>) -> Result<(), String> {
    // Where a language writes into text, a named place in text takes a
    // letter and the name goes on holding text.
    if let (Value::Text(had), Some(put), Some(at)) = (&*held, &letter, &key) {
        *held = letter_put(had, at, put)?;
        return Ok(());
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
                return Ok(());
            }
        }
        let items = Rc::make_mut(items);
        match key {
            Some(k) => items[as_index(&k)?] = value,
            None => items.push(value),
        }
        return Ok(());
    }
    if let Value::Vector(items) = &*held {
        let spread = items.iter().enumerate().map(|(at, x)| (Value::Small(at as i64), x.clone())).collect();
        *held = Value::Dict(Rc::new(spread));
    }
    let Value::Dict(entries) = held else {
        return Err(format!("Variable '{}' is not an array", ident));
    };
    let entries = Rc::make_mut(entries);
    let key = key.unwrap_or_else(|| Value::Small(after_keys(entries)));
    set_key(entries, key, value);
    Ok(())
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

/// Every letter of a piece of text with its bits turned over.
fn letters_turned(s: &str) -> String {
    let letters: Vec<u8> = s.as_bytes().iter().map(|c| !c).collect();
    String::from_utf8_lossy(&letters).into_owned()
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
    match math::ratio_of(&number) {
        // Dividing whole numbers cuts towards nothing, which is what
        // dropping what lies past the point comes to. A number too wide
        // for the bits at all comes to the lowest of them, as it does on
        // a machine that holds numbers to a width.
        Some(r) => Ok((&r.above / &r.beneath).to_i64().unwrap_or(i64::MIN)),
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
