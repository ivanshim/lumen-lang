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
use crate::data::{Blueprint, Thing, Env, Kind, Value, Names};

/// The label under which a language spells each kind of complaint.
pub const COMPLAINT_LABELS: [(&str, &str); 3] = [
    ("warning", "ext.system.complaint.warning"),
    ("notice", "ext.system.complaint.notice"),
    ("deprecated", "ext.system.complaint.deprecated"),
];

/// The label under which a language spells each kind of value.
pub const KIND_LABELS: [(&str, Kind); 7] = [
    ("system.kind.integer", Kind::Whole), ("system.kind.rational", Kind::Fraction), ("system.kind.real", Kind::Decimal),
    ("system.kind.string", Kind::Chars), ("system.kind.boolean", Kind::Truth), ("system.kind.array", Kind::Vector),
    ("system.kind.null", Kind::Nothing),
];

pub enum Escape {
    Error(String),
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
    /// in, which a complaint names.
    row: u32,
    written_in: String,
    /// The words this language has for the kinds of complaint.
    complaint_words: Vec<(&'static str, String)>,
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
            written_in: String::new(),
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
    fn grumble(&self, kind: &str, about: &str) {
        let Some((_, word)) = self.complaint_words.iter().find(|(k, _)| *k == kind) else { return };
        println!("\n{}: {} in {} on line {}", word, about, self.written_in, self.row);
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
        if !self.holds_reals_to_width() {
            return v;
        }
        let Value::Frac(e) = &v else { return v };
        match crate::data::binary_worth(crate::data::nearest_binary(&e.above, &e.beneath)) {
            Some((above, beneath)) => math::make_number(above, beneath, Some(self.real_figures())),
            // Past every number of that width, and so left as it is.
            None => v,
        }
    }

    fn wording(&self) -> Names<'a> {
        Names {
            truth: self.table.single("literal.true").unwrap_or("true"),
            falsity: self.table.single("literal.false").unwrap_or("false"),
            // A language may show nothing as no text at all, as PHP does,
            // rather than as the word a program writes for it.
            real_figures: self.table.count("ext.system.real.bits").and(self.table.count("ext.system.real.digits")),
            nil: match self.table.flag("literal.null.silent") {
                true => "",
                false => self.table.single("literal.null").unwrap_or("null"),
            },
        }
    }

    pub fn run_main(&mut self, body: &Form) -> Result<(), String> {
        let top = self.outermost.clone();
        match self.value_of(body, &top) {
            Ok(_) | Err(Escape::Yield(_)) | Err(Escape::Leave(_)) | Err(Escape::Resume(_)) => Ok(()),
            // A value nobody took is a fault, told the way PHP tells it.
            Err(Escape::Thrown(Value::Thing(thing))) => {
                let told = thing.holds.borrow().iter().find(|(k, _)| k == "message").map(|(_, x)| x.bare());
                Err(match told {
                    Some(told) => format!("Uncaught {}: {}", thing.of.name, told),
                    None => format!("Uncaught {}", thing.of.name),
                })
            }
            Err(Escape::Thrown(v)) => Err(format!("Uncaught {}", v.bare())),
            Err(Escape::Error(e)) => Err(e),
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
            Form::Forget(slot) => {
                let f = ascend(frame, slot.up);
                f.cells.borrow_mut()[slot.at] = Value::Unset;
                Ok(Value::Nil)
            }
            Form::OnLine(row, inner) => {
                self.row = *row;
                self.value_of(inner, frame)
            }
            Form::Missing(slot) => {
                let f = ascend(frame, slot.up);
                let empty = matches!(f.cells.borrow()[slot.at], Value::Unset);
                Ok(Value::Flag(empty))
            }
            Form::Attempt { body, clauses, last } => {
                let ending = self.value_of(body, frame);
                let ending = match ending {
                    Err(Escape::Thrown(raised)) => {
                        // The first clause that takes this class holds it
                        // and runs; what none takes is raised again.
                        let of = match &raised {
                            Value::Thing(thing) => Some(thing.of.clone()),
                            _ => None,
                        };
                        let taken = clauses.iter().find(|clause| {
                            of.as_ref().map_or(false, |o| clause.classes.iter().any(|name| o.built_on(name)))
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
                    methods: plan.methods.clone(),
                    constants,
                    shared: RefCell::new(shared),
                })))
            }
            Form::Cycle { test, body, step, after } => {
                loop {
                    if !after && !self.value_of(test, frame)?.is_true() {
                        break;
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
                    if *after && self.value_of(test, frame)?.is_true() {
                        break;
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
                let (p, env) = self.bound(target, frame)?;
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
                Prim::Both | Prim::Either => {
                    let left = self.value_of(&args[0], frame)?.is_true();
                    if (*op == Prim::Both && !left) || (*op == Prim::Either && left) {
                        return Ok(Value::Flag(left));
                    }
                    // The right side is read in place and evaluated only here.
                    let right = match self.value_of(&args[1], frame)? {
                        Value::Bound(p, env) => self.invoke(p, env, Vec::new())?,
                        v => v,
                    };
                    Ok(Value::Flag(right.is_true()))
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
                    Err(Escape::Thrown(raised))
                }
                Prim::Spawn => {
                    let mut values = self.value_list(args, frame)?;
                    if values.is_empty() {
                        return Err("Nothing was given to make".to_string().into());
                    }
                    let Value::Blueprint(class) = values.remove(0) else {
                        return Err("Only a class can be made into a thing".to_string().into());
                    };
                    self.made += 1;
                    let thing = Rc::new(Thing { of: class.clone(), holds: RefCell::new(class.every_field()), turn: self.made });
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
                    let holder = values.remove(0);
                    let called = values.remove(0).bare();
                    let Value::Blueprint(class) = holder else {
                        return Err(format!("Cannot call '{}' on something that is not a class", called).into());
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
                    let key = values.pop().map(|k| self.as_key(&k));
                    // Through the shared cell when the name stands for one.
                    let shared = match &f.cells.borrow()[i] {
                        Value::Shared(cell) => Some(cell.clone()),
                        _ => None,
                    };
                    if let Some(cell) = shared {
                        let mut held = cell.borrow_mut();
                        written_into(&mut held, key, value, &slot.ident)?;
                        return Ok(Value::Nil);
                    }
                    let mut slots = f.cells.borrow_mut();
                    written_into(&mut slots[i], key, value, &slot.ident)?;
                    Ok(Value::Nil)
                }
                op => {
                    let values = self.value_list(args, frame)?;
                    Ok(self.prim(*op, name, &values)?)
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
        match self.value_of(node, frame)? {
            Value::Bound(p, env) => Ok((p, env)),
            Value::Unset => Err("Unknown function".to_string().into()),
            _ => match node {
                Form::Read(slot) => Err(format!("'{}' is not a function", slot.ident).into()),
                _ => Err("eval needs a program".to_string().into()),
            },
        }
    }

    /// One step in tail position: a value, or the program to run next.
    fn advance(&mut self, node: &Form, frame: &Rc<Env>) -> Res<Next> {
        match node {
            Form::Apply(Callee::Code(target), args) => {
                let (p, env) = self.bound(target, frame)?;
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
        let test = self.value_of(&args[0], frame)?.is_true();
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
        for (i, a) in program.formal_slots.iter().zip(args) {
            let v = self.value_of(a, caller)?;
            frame.cells.borrow_mut()[*i] = v;
        }
        Ok(frame)
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
        let mut caught: u8 = 0;
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
                    _ => return Err("Cannot walk a value that is not an array".to_string()),
                }
            }
            Prim::Extent => {
                n(1)?;
                match &v[0] {
                    Value::Vector(items) => Value::Small(items.len() as i64),
                    Value::Dict(entries) => Value::Small(entries.len() as i64),
                    _ => return Err("Cannot walk a value that is not an array".to_string()),
                }
            }
            Prim::AtEnd => return Err("An empty index belongs on the left of an assignment".to_string()),
            Prim::Of => {
                n(2)?;
                let called = v[1].bare();
                match &v[0] {
                    Value::Thing(thing) => {
                        let found = thing.holds.borrow().iter().find(|(k, _)| *k == called).map(|(_, x)| x.clone());
                        match found {
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
            Prim::Onto => {
                n(3)?;
                let called = v[1].bare();
                match &v[0] {
                    Value::Thing(thing) => {
                        let mut holds = thing.holds.borrow_mut();
                        match holds.iter_mut().find(|(k, _)| *k == called) {
                            Some(place) => place.1 = v[2].clone(),
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
                match &v[0] {
                    Value::Blueprint(class) => match class.constant(&called) {
                        Some(x) => x.clone(),
                        None => match class.keeper(&called) {
                            Some(keeper) => {
                                let held = keeper.shared.borrow().iter().find(|(k, _)| *k == called).map(|(_, x)| x.clone());
                                held.expect("the keeper holds it")
                            }
                            None => return Err(format!("Undefined constant {}::{}", class.name, called)),
                        },
                    },
                    other => return Err(format!("Cannot reach '{}' in {}", called, other.bare())),
                }
            }
            Prim::Into => {
                n(3)?;
                let called = v[1].bare();
                match &v[0] {
                    Value::Blueprint(class) => {
                        let keeper = class.keeper(&called).unwrap_or(class);
                        let mut shared = keeper.shared.borrow_mut();
                        match shared.iter_mut().find(|(k, _)| *k == called) {
                            Some(place) => place.1 = v[2].clone(),
                            None => shared.push((called, v[2].clone())),
                        }
                        Value::Nil
                    }
                    other => return Err(format!("Cannot write '{}' in {}", called, other.bare())),
                }
            }
            Prim::Akin => {
                n(2)?;
                match &v[0] {
                    Value::Thing(thing) => Value::Flag(thing.of.built_on(&v[1].bare())),
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
                    _ => return Err(format!("{}() requires an array", name)),
                }
            }
            Prim::Placed => {
                n(3)?;
                match &v[0] {
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
                    _ => return Err(format!("{}() requires an array", name)),
                }
            }
            Prim::Gather => return Err(format!("{}() is a literal, not a call", name)),
            Prim::Handed | Prim::HowMany | Prim::HandedAt => {
                let Some(handed) = self.handed.last() else {
                    return Err(format!("{}() belongs inside a function", name));
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
                        let at = as_index(&v[0])?;
                        match handed.get(at) {
                            Some(x) => x.clone(),
                            None => return Err(format!("{}(): nothing was handed over at {}", name, at)),
                        }
                    }
                }
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
                print!("{}", over_lines(&v[0], 0));
                Value::Flag(true)
            }
            Prim::Invert => Value::Flag(!v[0].is_true()),
            // Turning text over works letter by letter; anything else is
            // read as a whole number of sixty-four bits first.
            Prim::BitsOver => match &v[0] {
                Value::Text(s) => Value::text(&letters_turned(s)),
                other => Value::Small(!sixty_four(other)?),
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
                let (left, right) = (sixty_four(&v[0])?, sixty_four(&v[1])?);
                Value::Small(match op {
                    Prim::BitsBoth => left & right,
                    Prim::BitsEither => left | right,
                    _ => left ^ right,
                })
            }
            Prim::BitsUp | Prim::BitsDown => {
                let (bits, by) = (sixty_four(&v[0])?, sixty_four(&v[1])?);
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
            Prim::Negate => match math::compute(Calc::Minus, &Value::Small(0), &v[0]) {
                Some(r) => r?,
                None => return Err("Cannot negate non-numeric value".to_string()),
            },
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
                    (Value::Flag(_), _) | (_, Value::Flag(_)) => left.is_true() == right.is_true(),
                    (one, other) | (other, one) if empty(one) && counts(other) => !other.is_true(),
                    (one, Value::Text(s)) | (Value::Text(s), one) if empty(one) => s.is_empty(),
                    (one, Value::Vector(items)) | (Value::Vector(items), one) if empty(one) => items.is_empty(),
                    (one, Value::Dict(pairs)) | (Value::Dict(pairs), one) if empty(one) => pairs.is_empty(),
                    (Value::Text(_), Value::Text(_)) => match (number_spelled_in(left), number_spelled_in(right)) {
                        (Some(x), Some(y)) => x.equals(&y),
                        _ => left.equals(right),
                    },
                    (Value::Text(s), other) | (other, Value::Text(s)) if counts(other) => match number_spelled_in(left).or_else(|| number_spelled_in(right)) {
                        Some(x) => x.equals(other),
                        None => **s == other.bare(),
                    },
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
            // Adding text joins it only where the language has no
            // operator of its own for joining; where it has one, adding
            // is arithmetic.
            Prim::Plus
                if !self.table.has_any("op.concat") && (matches!(v[0], Value::Text(_)) || matches!(v[1], Value::Text(_))) =>
            {
                Value::text(&format!("{}{}", v[0].render(w), v[1].render(w)))
            }
            // Text that spells a number is worked with as that number,
            // fractions included, so long as one side spells one.
            Prim::Plus | Prim::Minus | Prim::Times | Prim::Over | Prim::OverReal | Prim::IntDiv | Prim::Mod | Prim::Power
            | Prim::Lt | Prim::Le | Prim::Gt | Prim::Ge
                if number_spelled_in(&v[0]).is_some() || number_spelled_in(&v[1]).is_some() =>
            {
                let pair = [
                    number_spelled_in(&v[0]).unwrap_or_else(|| v[0].clone()),
                    number_spelled_in(&v[1]).unwrap_or_else(|| v[1].clone()),
                ];
                return self.prim(op, name, &pair);
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
                    Value::Text(s) => print!("{}", s),
                    _ => return Err(format!("{}() requires a string argument", name)),
                }
                Value::Nil
            }
            Prim::Say => {
                println!("{}", self.show(v));
                Value::Nil
            }
            Prim::Out => {
                print!("{}", self.show(v));
                Value::Nil
            }
            Prim::Tell => {
                let w = self.wording();
                let text: String = v.iter().map(|x| x.render(w)).collect();
                print!("{}", text);
                Value::Nil
            }
            Prim::Define => return Err(format!("{}() needs a quoted name as its first argument", name)),
            Prim::Dump => {
                for x in v {
                    println!("{}", with_kind(x, 0, self.table.count("ext.system.real.bits").is_some()));
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
                    "print_native" => println!("{}", x.render(w)),
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

    fn element(&self, target: &Value, at: &Value) -> Result<Value, String> {
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
        if let Value::Dict(entries) = target {
            let at = &self.as_key(at);
            let found = entries.iter().find(|(k, _)| k.equals(at));
            return match found {
                Some((_, v)) => Ok(v.clone()),
                None => missing(format!("Undefined array key {}", at.bare()), at),
            };
        }
        let i = match as_index(at) {
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
            _ => Err("Cannot index non-array value".to_string()),
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
fn with_kind(v: &Value, level: usize, binary_reals: bool) -> String {
    let lead = "  ".repeat(level);
    match v {
        Value::Small(_) | Value::Huge(_) => format!("int({})", v.bare()),
        // Shown with its kind, a binary real is written in the fewest
        // figures that read back as the same number.
        Value::Frac(e) if binary_reals => format!("float({})", crate::data::figured(crate::data::nearest_binary(&e.above, &e.beneath), None)),
        Value::Frac(_) => format!("float({})", v.bare()),
        Value::Text(s) => format!("string({}) \"{}\"", s.len(), s),
        Value::Flag(b) => format!("bool({})", b),
        Value::Vector(items) => {
            let entries: Vec<String> = items.iter().enumerate().map(|(i, x)| format!("{lead}  [{i}]=>\n{lead}  {}\n", with_kind(x, level + 1, binary_reals))).collect();
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
                    format!("{lead}  [{key}]=>\n{lead}  {}\n", with_kind(x, level + 1, binary_reals))
                })
                .collect();
            format!("array({}) {{\n{}{lead}}}", entries.len(), shown.concat())
        }
        Value::Thing(thing) => {
            let held = thing.holds.borrow();
            let shown: Vec<String> = held
                .iter()
                .map(|(member, x)| format!("{lead}  [\"{member}\"]=>\n{lead}  {}\n", with_kind(x, level + 1, binary_reals)))
                .collect();
            format!("object({})#{} ({}) {{\n{}{lead}}}", thing.of.name, thing.turn, held.len(), shown.concat())
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
        Some(entry) => entry.1 = value,
        None => entries.push((key, value)),
    }
}

/// A value over lines the way PHP's print_r writes it: a scalar on its
/// own, an array as the word `Array` with its places in brackets, every
/// array within set eight spaces further along and followed by a gap.
fn over_lines(v: &Value, along: usize) -> String {
    let held;
    let (called, places): (String, Vec<(String, &Value)>) = match v {
        Value::Vector(items) => ("Array".to_string(), items.iter().enumerate().map(|(at, x)| (at.to_string(), x)).collect()),
        Value::Dict(entries) => ("Array".to_string(), entries.iter().map(|(k, x)| (k.bare(), x)).collect()),
        Value::Thing(thing) => {
            held = thing.holds.borrow();
            (format!("{} Object", thing.of.name), held.iter().map(|(k, x)| (k.clone(), x)).collect())
        }
        other => return other.bare(),
    };
    let lead = " ".repeat(along);
    let mut out = format!("{called}\n{lead}(\n");
    for (key, item) in places {
        // An array within ends its own line, so this newline is the gap
        // that follows it; after a scalar it is the end of the line.
        let shown = over_lines(item, along + 8);
        out.push_str(&format!("{lead}    [{key}] => {shown}\n"));
    }
    out.push_str(&format!("{lead})\n"));
    out
}

/// Write a value into an array held in a binding: at a key, or at the
/// end when no key is given. A list written where it already reaches
/// stays a list; any other key turns it into a map, its places becoming
/// the keys.
fn written_into(held: &mut Value, key: Option<Value>, value: Value, ident: &str) -> Result<(), String> {
    let stays = match (&*held, &key) {
        (Value::Vector(items), Some(k)) => as_index(k).map_or(false, |at| at < items.len()),
        (Value::Vector(_), None) => true,
        _ => false,
    };
    if let (Value::Vector(items), true) = (&mut *held, stays) {
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

/// The number a piece of text spells, whole or fractional, with room for
/// a sign and for space around it; anything that is not such text spells
/// no number at all.
fn number_spelled_in(v: &Value) -> Option<Value> {
    let Value::Text(s) = v else { return None };
    let text = s.trim();
    // A number may carry a power of ten after it: 1e2, 1.5E-3.
    if let Some(at) = text.find(['e', 'E']) {
        let (front, back) = text.split_at(at);
        let power: i32 = back[1..].parse().ok()?;
        let base = number_spelled_in(&Value::text(front))?;
        let scale = Value::from_big(BigInt::from(10).pow(power.unsigned_abs()));
        let how = if power >= 0 { Calc::Times } else { Calc::OverReal };
        return math::compute(how, &base, &scale)?.ok();
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
        Value::Huge(n) => return Ok(n.to_i64().unwrap_or(0)),
        Value::Text(_) => match number_spelled_in(v) {
            Some(n) => n,
            None => return Ok(0),
        },
        other => other.clone(),
    };
    match math::ratio_of(&number) {
        // Dividing whole numbers cuts towards nothing, which is what
        // dropping what lies past the point comes to.
        Some(r) => Ok((&r.above / &r.beneath).to_i64().unwrap_or(0)),
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
