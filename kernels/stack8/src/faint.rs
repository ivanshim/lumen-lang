//! Weak holds, last words and the finding of unreachable rounds.
//!
//! Every value that lives behind a pointer is owned by counting: the
//! last strong hold to go takes the value with it. A weak hold is the
//! standard library's own downgraded pointer, so it costs nothing to
//! keep and cannot keep anything alive. What this module adds is the
//! bookkeeping around such holds: which of them want a word when their
//! value goes, which objects have last words of their own to say, and
//! how a round of values holding one another, which counting alone
//! never frees, is found and broken when the program asks.
//!
//! Nothing here can reach the engine. A value going away only leaves a
//! note in a thread-local queue; the engine reads the queue at its next
//! step and does the calling.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use crate::code::Routine;
use crate::value::{Class, CursorSource, Descriptor, Ending, Generator, Instance, Members, Phase, Value};

/// What a weak hold points at: one of the kinds a program may hold weakly.
#[derive(Debug, Clone)]
pub enum Hold {
    Native(crate::code::Builtin, Rc<str>),
    Object(Weak<Instance>),
    Container(Weak<RefCell<Value>>),
    Class(Weak<Class>),
    Generator(Weak<RefCell<Generator>>),
    Set(Weak<RefCell<Members>>),
    Routine(Weak<Routine>),
    Method(Weak<Instance>, Weak<Routine>),
}

impl Hold {
    /// The value again, while it is still there.
    pub fn revive(&self) -> Option<Value> {
        Some(match self {
            Hold::Native(operation, spelling) => Value::Native(*operation, spelling.clone()),
            Hold::Object(w) => Value::Object(w.upgrade()?),
            Hold::Container(w) => Value::Bond(w.upgrade()?),
            Hold::Class(w) => Value::Class(w.upgrade()?),
            Hold::Generator(w) => Value::Generator(w.upgrade()?),
            Hold::Set(w) => Value::Set(w.upgrade()?),
            Hold::Routine(w) => Value::Routine(w.upgrade()?),
            Hold::Method(o, r) => Value::Method(o.upgrade()?, r.upgrade()?),
        })
    }

    pub fn gone(&self) -> bool {
        match self {
            Hold::Native(..) => false,
            Hold::Object(w) => w.strong_count() == 0,
            Hold::Container(w) => w.strong_count() == 0,
            Hold::Class(w) => w.strong_count() == 0,
            Hold::Generator(w) => w.strong_count() == 0,
            Hold::Set(w) => w.strong_count() == 0,
            Hold::Routine(w) => w.strong_count() == 0,
            Hold::Method(o, r) => o.strong_count() == 0 || r.strong_count() == 0,
        }
    }
}

/// A weak hold as the program sees it: what it points at, the program's
/// own reference object that carries it, and the routine to call with
/// that object once the value goes.
#[derive(Debug)]
pub struct Faint {
    pub hold: Hold,
    pub bearer: Weak<Instance>,
    pub told: Option<Value>,
}

thread_local! {
    /// The name a class gives its last words, where the language has one.
    static LAST_WORD: RefCell<Option<String>> = const { RefCell::new(None) };
    /// How many weak holds still want a word when their value goes.
    static WATCHING: Cell<usize> = const { Cell::new(0) };
    /// Those holds.
    static WATCHED: RefCell<Vec<Rc<Faint>>> = const { RefCell::new(Vec::new()) };
    /// Whether anything at all is waiting for the engine's next step.
    static PENDING: Cell<bool> = const { Cell::new(false) };
    /// Whether a value somebody may be watching has gone since the last look.
    static DIED: Cell<bool> = const { Cell::new(false) };
    /// Objects rebuilt for their last words, with the routine to say them.
    static LAST_WORDS: RefCell<Vec<(Rc<Instance>, Value)>> = const { RefCell::new(Vec::new()) };
    /// Suspended walks rebuilt so that their last parts may run.
    static UNFINISHED: RefCell<Vec<Rc<RefCell<Generator>>>> = const { RefCell::new(Vec::new()) };
    /// Objects whose last words were said already, by where they live.
    static SPOKEN: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    /// Everything whose going somebody could notice: the weakly held
    /// values and the objects with last words. Only rounds that hold one
    /// of these are worth looking for.
    static CANDIDATES: RefCell<(Vec<Hold>, usize)> = const { RefCell::new((Vec::new(), 8)) };
}

/// Tell the module the name a class gives its last words. Until this
/// is called, nothing that dies is looked at.
pub fn name_last_word(word: Option<String>) {
    let _ = LAST_WORD.try_with(|w| *w.borrow_mut() = word);
}

/// Whether the engine has anything to settle before its next step.
pub fn pending() -> bool {
    PENDING.try_with(|p| p.get()).unwrap_or(false)
}

fn wake() {
    let _ = PENDING.try_with(|p| p.set(true));
}

/// A value somebody may hold weakly has gone.
fn note_death() {
    if WATCHING.try_with(|w| w.get()).unwrap_or(0) > 0 {
        let _ = DIED.try_with(|d| d.set(true));
        wake();
    }
}

/// The routine a class says its last words with, where it has one.
pub fn last_word_of(class: &Class) -> Option<Value> {
    LAST_WORD.try_with(|w| {
        let name = w.borrow();
        let name = name.as_deref()?;
        if let Some(program) = class.method(name) {
            return Some(Value::Routine(program.clone()));
        }
        let holder = class.holder(name)?;
        holder.shared.borrow().iter().find(|(n, _)| n == name).map(|(_, v)| v.clone())
    }).ok().flatten()
}

/// Whether an object's last words are still to be said; saying so
/// marks them said, so they are said once.
pub fn first_words(object: &Rc<Instance>) -> bool {
    let here = Rc::as_ptr(object) as usize;
    SPOKEN.try_with(|s| s.borrow_mut().insert(here)).unwrap_or(false)
}

/// An object is going. One with last words still unsaid is rebuilt
/// around its own fields and queued for the engine to speak for it.
pub fn departing(dying: &mut Instance) {
    let here = dying as *const Instance as usize;
    // Once the thread's own storage is gone, at the very end, nothing
    // is queued: there is no engine left to speak.
    let spoken = SPOKEN.try_with(|s| s.borrow_mut().remove(&here)).unwrap_or(true);
    if !spoken {
        if let Some(words) = last_word_of(&dying.class) {
            let fields = std::mem::take(dying.fields.get_mut());
            let again = Rc::new(Instance { replacement_class: RefCell::new(dying.replacement_class.borrow().clone()), class: dying.class.clone(), fields: RefCell::new(fields), mark: dying.mark });
            let _ = SPOKEN.try_with(|s| s.borrow_mut().insert(Rc::as_ptr(&again) as usize));
            let _ = LAST_WORDS.try_with(|q| q.borrow_mut().push((again, words)));
            wake();
        }
    }
    note_death();
}

/// A walk is going. One asleep inside a try is rebuilt around its own
/// frame and queued, so that its last parts run as the language asks.
pub fn walk_departing(dying: &mut Generator) {
    let asleep = dying.started && !dying.closed && dying.program.is_some() && !dying.resume.is_empty();
    if asleep && LAST_WORD.try_with(|w| w.borrow().is_some()).unwrap_or(false) {
        let again = Generator {
            name: dying.name.clone(), qualified: dying.qualified.clone(), trace_frame: dying.trace_frame.take(),
            program: dying.program.clone(),
            frame: std::mem::take(&mut dying.frame),
            stack: std::mem::take(&mut dying.stack),
            pc: dying.pc,
            started: true,
            closed: false,
            waiting: dying.waiting,
            handed: dying.handed.take(),
            returned: std::mem::replace(&mut dying.returned, Value::Null),
            delegate: dying.delegate.take(),
            sent: std::mem::replace(&mut dying.sent, Value::Null),
            items: std::mem::take(&mut dying.items),
            current: dying.current.take(),
            watched: dying.watched.take(),
            resume: std::mem::take(&mut dying.resume),
            resuming: dying.resuming,
            held: std::mem::take(&mut dying.held),
            hurled: dying.hurled.take(),
            walked: dying.walked.take(),
        };
        dying.closed = true;
        let _ = UNFINISHED.try_with(|q| q.borrow_mut().push(Rc::new(RefCell::new(again))));
        wake();
    }
    note_death();
}

/// A set, a class or a routine is going: only the watchers care.
pub fn plain_departing() {
    note_death();
}

/// What the engine has to do now: the objects with last words to say,
/// the walks to close, and the watchers whose value has gone, each with
/// the reference object to hand it. Clears the flag; more may gather
/// while the engine works, so it asks again until nothing comes.
pub fn settle() -> (Vec<(Rc<Instance>, Value)>, Vec<Rc<RefCell<Generator>>>, Vec<(Value, Value)>) {
    let _ = PENDING.try_with(|p| p.set(false));
    let words = LAST_WORDS.try_with(|q| std::mem::take(&mut *q.borrow_mut())).unwrap_or_default();
    let walks = UNFINISHED.try_with(|q| std::mem::take(&mut *q.borrow_mut())).unwrap_or_default();
    let mut gone = Vec::new();
    if DIED.try_with(|d| d.replace(false)).unwrap_or(false) {
        let _ = WATCHED.try_with(|w| {
            let mut watched = w.borrow_mut();
            let mut kept = Vec::with_capacity(watched.len());
            for faint in watched.drain(..) {
                if !faint.hold.gone() {
                    kept.push(faint);
                    continue;
                }
                if let (Some(bearer), Some(told)) = (faint.bearer.upgrade(), &faint.told) {
                    gone.push((told.clone(), Value::Object(bearer)));
                }
            }
            let _ = WATCHING.try_with(|n| n.set(kept.len()));
            *watched = kept;
        });
    }
    (words, walks, gone)
}

/// A weak hold on a value, or nothing for a kind that cannot be held so.
pub fn hold_of(value: &Value) -> Option<Hold> {
    match value {
        Value::Bond(cell) | Value::Binding(cell) | Value::Collection(cell, _) => hold_of(&cell.borrow()),
        Value::Object(o) => Some(Hold::Object(Rc::downgrade(o))),
        Value::Native(op, word) if op.names_kind() => Some(Hold::Native(*op, word.clone())),
        Value::Class(c) => Some(Hold::Class(Rc::downgrade(c))),
        Value::Generator(g) => Some(Hold::Generator(Rc::downgrade(g))),
        Value::Set(s) => Some(Hold::Set(Rc::downgrade(s))),
        Value::Routine(r) => Some(Hold::Routine(Rc::downgrade(r))),
        Value::Method(o, r) => Some(Hold::Method(Rc::downgrade(o), Rc::downgrade(r))),
        _ => None,
    }
}

/// Make a weak hold the program can carry, remembering it where it
/// asks for a word, and its value as one whose going may be noticed.
pub fn make(hold: Hold, bearer: Weak<Instance>, told: Option<Value>) -> Value {
    remember(hold.clone());
    let faint = Rc::new(Faint { hold, bearer, told });
    if faint.told.is_some() {
        let _ = WATCHED.try_with(|w| w.borrow_mut().push(faint.clone()));
        let _ = WATCHING.try_with(|n| n.set(n.get() + 1));
    }
    Value::Faint(faint)
}

/// Remember a value whose going could be noticed. The list is pruned
/// of the dead whenever it has doubled since it was last pruned.
pub fn remember(hold: Hold) {
    let _ = CANDIDATES.try_with(|c| {
        let mut c = c.borrow_mut();
        if c.0.len() >= c.1 {
            c.0.retain(|h| !h.gone());
            c.1 = (c.0.len() * 2).max(8);
        }
        c.0.push(hold);
    });
}

/// Mutable containers can be the incoming side of a finalizable cycle.
/// Keep a weak candidate even when the container has no weakref protocol.
pub fn track_container(cell: &Rc<RefCell<Value>>) {
    if LAST_WORD.try_with(|word| word.borrow().is_some()).unwrap_or(false) {
        remember(Hold::Container(Rc::downgrade(cell)));
    }
}

/// One value the graph knows: the strong hold the graph itself keeps,
/// the others it holds directly, and how many holds on it the graph
/// accounts for.
struct Node {
    held: Value,
    reaches: Vec<usize>,
    inward: usize,
    marked: bool,
}

/// The values reachable from the candidates, each once, with every hold
/// between them counted. A hold the graph cannot account for is one from
/// outside it: the stack, a frame, the engine. Anything with no such
/// hold, and nothing reaching it from anything that has one, is
/// unreachable by the program.
pub struct Graph {
    nodes: HashMap<usize, Node>,
}

/// Where a value's pointer stands, for the kinds that live behind one.
fn place_of(value: &Value) -> Option<usize> {
    Some(match value {
        Value::Object(o) | Value::Fields(o) => Rc::as_ptr(o) as *const () as usize,
        Value::Class(c) => Rc::as_ptr(c) as *const () as usize,
        Value::Generator(g) => Rc::as_ptr(g) as *const () as usize,
        Value::Set(s) | Value::SetWalk(s, _) => Rc::as_ptr(s) as *const () as usize,
        Value::Routine(r) => Rc::as_ptr(r) as *const () as usize,
        Value::Bond(c) | Value::Binding(c) | Value::Collection(c, _) => Rc::as_ptr(c) as *const () as usize,
        Value::Array(a) | Value::Tuple(a) => Rc::as_ptr(a) as *const () as usize,
        Value::Slice(parts) => Rc::as_ptr(parts) as *const () as usize,
        Value::Map(m) => Rc::as_ptr(m) as *const () as usize,
        Value::Hashed(p) | Value::Tie(p) => Rc::as_ptr(p) as *const () as usize,
        Value::ValueMethod(p) | Value::View(p) => Rc::as_ptr(p) as *const () as usize,
        Value::Adapter(a) => Rc::as_ptr(a) as *const () as usize,
        Value::Descriptor(d) => Rc::as_ptr(d) as *const () as usize,
        Value::Walking(w) => Rc::as_ptr(w) as *const () as usize,
        Value::Walk(w) => Rc::as_ptr(w) as *const () as usize,
        Value::Cursor(c) => Rc::as_ptr(c) as *const () as usize,
        Value::Trace(t) => Rc::as_ptr(t) as *const () as usize,
        _ => return None,
    })
}

/// How many strong holds there are on a value's pointer.
fn holds_on(value: &Value) -> usize {
    match value {
        Value::Object(o) | Value::Fields(o) => Rc::strong_count(o),
        Value::Class(c) => Rc::strong_count(c),
        Value::Generator(g) => Rc::strong_count(g),
        Value::Set(s) | Value::SetWalk(s, _) => Rc::strong_count(s),
        Value::Routine(r) => Rc::strong_count(r),
        Value::Bond(c) | Value::Binding(c) | Value::Collection(c, _) => Rc::strong_count(c),
        Value::Array(a) | Value::Tuple(a) => Rc::strong_count(a),
        Value::Slice(parts) => Rc::strong_count(parts),
        Value::Map(m) => Rc::strong_count(m),
        Value::Hashed(p) | Value::Tie(p) => Rc::strong_count(p),
        Value::ValueMethod(p) | Value::View(p) => Rc::strong_count(p),
        Value::Adapter(a) => Rc::strong_count(a),
        Value::Descriptor(d) => Rc::strong_count(d),
        Value::Walking(w) => Rc::strong_count(w),
        Value::Walk(w) => Rc::strong_count(w),
        Value::Cursor(c) => Rc::strong_count(c),
        Value::Trace(t) => Rc::strong_count(t),
        _ => 0,
    }
}

/// The values a value holds directly. A bound method is two holds, on
/// the object and on the routine, and is listed as both. A cell that is
/// being written to just now yields nothing: what it holds then counts
/// as held from outside, which errs on the side of keeping.
fn reaches(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Object(o) | Value::Fields(o) => {
            out.push(Value::Class(o.class.clone()));
            if let Ok(fields) = o.fields.try_borrow() {
                out.extend(fields.iter().map(|(_, v)| v.clone()));
            }
        }
        Value::Class(c) => {
            out.extend(c.fields.iter().map(|(_, v)| v.clone()));
            out.extend(c.constants.iter().map(|(_, v)| v.clone()));
            if let Ok(shared) = c.shared.try_borrow() {
                out.extend(shared.iter().map(|(_, v)| v.clone()));
            }
            out.extend(c.methods.iter().map(|(_, p)| Value::Routine(p.clone())));
            out.extend(c.base.iter().map(|b| Value::Class(b.clone())));
            out.extend(c.lineage.iter().chain(&c.direct).chain(&c.answers).map(|b| Value::Class(b.clone())));
        }
        Value::Generator(g) => {
            if let Ok(g) = g.try_borrow() {
                out.extend(g.trace_frame.iter().map(|f| Value::Object(f.clone())));
                out.extend(g.program.iter().map(|p| Value::Routine(p.clone())));
                out.extend(g.frame.iter().cloned());
                out.extend(g.stack.iter().cloned());
                out.extend(g.items.iter().cloned());
                out.extend(g.handed.iter().cloned());
                out.push(g.returned.clone());
                out.extend(g.delegate.iter().cloned());
                out.push(g.sent.clone());
                out.extend(g.current.iter().cloned());
                out.extend(g.watched.iter().map(|(cell, _)| Value::Bond(cell.clone())));
                for step in &g.resume {
                    match &step.phase {
                        Phase::Arm(_, v) => out.push(v.clone()),
                        Phase::Last(Ending::Thrown(v), _) => out.push(v.clone()),
                        _ => {}
                    }
                }
                out.extend(g.held.iter().cloned());
                out.extend(g.hurled.iter().cloned());
            }
        }
        Value::Set(s) | Value::SetWalk(s, _) => {
            if let Ok(members) = s.try_borrow() {
                out.extend(members.held.values().cloned());
            }
        }
        Value::Routine(r) => {
            out.extend(r.held.iter().cloned());
            out.extend(r.enclosed.iter().map(|(_, v)| v.clone()));
        }
        Value::Bond(c) | Value::Binding(c) | Value::Collection(c, _) => {
            if let Ok(inner) = c.try_borrow() {
                out.push(inner.clone());
            }
        }
        Value::Array(a) | Value::Tuple(a) => out.extend(a.iter().cloned()),
        Value::Slice(parts) => out.extend(parts.iter().cloned()),
        Value::Map(m) => {
            for (k, v) in m.iter() {
                out.push(k.clone());
                out.push(v.clone());
            }
        }
        Value::Hashed(p) | Value::Tie(p) => {
            out.push(p.0.clone());
            out.push(p.1.clone());
        }
        Value::ValueMethod(p) | Value::View(p) => out.push(p.0.clone()),
        Value::Adapter(a) => out.extend(a.1.iter().cloned()),
        Value::Descriptor(d) => match &**d {
            Descriptor::Static(v) | Descriptor::Class(v) => out.push(v.clone()),
            Descriptor::Property(a, b) => {
                out.push(a.clone());
                out.extend(b.iter().cloned());
            }
            Descriptor::Bound(a, b) => {
                out.push(a.clone());
                out.push(b.clone());
            }
        },
        Value::Walking(w) => {
            if let Ok(w) = w.try_borrow() {
                out.push(w.0.clone());
                out.extend(w.1.iter().cloned());
            }
        }
        Value::Walk(w) => {
            if let Ok(w) = w.try_borrow() {
                out.extend(w.0.iter().cloned());
            }
        }
        Value::Cursor(c) => {
            if let Ok(c) = c.try_borrow() {
                out.extend(c.pending.iter().cloned());
                out.extend(c.origin.iter().cloned());
                match &c.source {
                    CursorSource::Items(items, _) => out.push(Value::Tuple(items.clone())),
                    CursorSource::Living(cell, _) => out.push(Value::Bond(cell.clone())),
                    CursorSource::Viewed(v, ..) | CursorSource::Indexed(v, _) | CursorSource::IndexedBack(v, _)
                    | CursorSource::Handed(v) | CursorSource::Numbered(v, _) => out.push(v.clone()),
                    CursorSource::Called(a, b, _) | CursorSource::Selected(a, b) => {
                        out.push(a.clone());
                        out.push(b.clone());
                        if let CursorSource::Called(_, _, Some(v)) = &c.source { out.push(v.clone()); }
                    }
                    CursorSource::Combined(rows, work, _) => {
                        out.extend(rows.iter().cloned());
                        out.extend(work.iter().cloned());
                    }
                    CursorSource::Counted(..) => {}
                }
            }
        }
        Value::Trace(t) => {
            out.push(Value::Object(t.frame.clone()));
            out.push(t.next.clone());
        }
        _ => {}
    }
}

impl Graph {
    /// Everything reachable from the remembered candidates still alive.
    pub fn from_candidates() -> Graph {
        let roots: Vec<Value> = CANDIDATES.try_with(|c| {
            let mut c = c.borrow_mut();
            c.0.retain(|h| !h.gone());
            c.1 = (c.0.len() * 2).max(8);
            c.0.iter().filter_map(|h| h.revive()).collect()
        }).unwrap_or_default();
        Graph::build(roots)
    }

    fn build(roots: Vec<Value>) -> Graph {
        let mut nodes: HashMap<usize, Node> = HashMap::new();
        let mut open: Vec<usize> = Vec::new();
        for root in roots {
            let Some(place) = place_of(&root) else { continue };
            if !nodes.contains_key(&place) {
                nodes.insert(place, Node { held: root, reaches: Vec::new(), inward: 0, marked: false });
                open.push(place);
            }
        }
        let mut children = Vec::new();
        while let Some(place) = open.pop() {
            children.clear();
            reaches(&nodes[&place].held, &mut children);
            let mut reached = Vec::with_capacity(children.len());
            for child in children.drain(..) {
                // A bound method is not a place of its own but two.
                let parts: Vec<Value> = match child {
                    Value::Method(o, r) => vec![Value::Object(o), Value::Routine(r)],
                    other => vec![other],
                };
                for part in parts {
                    let Some(at) = place_of(&part) else { continue };
                    match nodes.get_mut(&at) {
                        Some(node) => node.inward += 1,
                        None => {
                            nodes.insert(at, Node { held: part, reaches: Vec::new(), inward: 1, marked: false });
                            open.push(at);
                        }
                    }
                    reached.push(at);
                }
            }
            nodes.get_mut(&place).unwrap().reaches = reached;
        }
        Graph { nodes }
    }

    /// The values nothing outside the graph holds, directly or through
    /// anything it does hold: the graph's own hold on each is the one
    /// not counted among the program's.
    pub fn unreached(&mut self) -> Vec<Value> {
        let mut open: Vec<usize> = self.nodes.iter()
            .filter(|(_, n)| holds_on(&n.held) > n.inward + 1)
            .map(|(place, _)| *place)
            .collect();
        for place in &open {
            self.nodes.get_mut(place).unwrap().marked = true;
        }
        while let Some(place) = open.pop() {
            let reached = self.nodes[&place].reaches.clone();
            for at in reached {
                let node = self.nodes.get_mut(&at).unwrap();
                if !node.marked {
                    node.marked = true;
                    open.push(at);
                }
            }
        }
        self.nodes.values().filter(|n| !n.marked).map(|n| n.held.clone()).collect()
    }

    /// Break every unreachable round: empty each mutable value in it, so
    /// that counting frees the rest. What was emptied is handed back to
    /// be dropped once the graph itself is gone.
    pub fn sever(unreached: &[Value]) -> Vec<Value> {
        let mut grave = Vec::new();
        for value in unreached {
            match value {
                Value::Object(o) | Value::Fields(o) => {
                    if let Ok(mut fields) = o.fields.try_borrow_mut() {
                        grave.extend(std::mem::take(&mut *fields).into_iter().map(|(_, v)| v));
                    }
                }
                Value::Class(c) => {
                    if let Ok(mut shared) = c.shared.try_borrow_mut() {
                        grave.extend(std::mem::take(&mut *shared).into_iter().map(|(_, v)| v));
                    }
                }
                Value::Bond(c) | Value::Binding(c) | Value::Collection(c, _) => {
                    if let Ok(mut inner) = c.try_borrow_mut() {
                        grave.push(std::mem::replace(&mut *inner, Value::Null));
                    }
                }
                Value::Set(s) | Value::SetWalk(s, _) => {
                    if let Ok(mut members) = s.try_borrow_mut() {
                        members.row.clear();
                        grave.extend(std::mem::take(&mut members.held).into_values());
                    }
                }
                Value::Generator(g) => {
                    if let Ok(mut g) = g.try_borrow_mut() {
                        g.closed = true;
                        grave.extend(g.frame.drain(..));
                        grave.extend(g.stack.drain(..));
                        grave.extend(g.items.drain(..));
                        grave.extend(g.handed.take());
                        grave.extend(g.delegate.take());
                        grave.extend(g.current.take());
                        grave.extend(g.held.drain(..));
                        grave.extend(g.hurled.take());
                        grave.push(std::mem::replace(&mut g.returned, Value::Null));
                        grave.push(std::mem::replace(&mut g.sent, Value::Null));
                        g.resume.clear();
                        g.trace_frame = None;
                    }
                }
                Value::Walking(w) => {
                    if let Ok(mut w) = w.try_borrow_mut() {
                        grave.push(std::mem::replace(&mut w.0, Value::Null));
                        grave.extend(w.1.take());
                    }
                }
                Value::Walk(w) => {
                    if let Ok(mut w) = w.try_borrow_mut() {
                        grave.extend(w.0.drain(..));
                    }
                }
                _ => {}
            }
        }
        grave
    }
}

/// Whether a walk in the graph is asleep inside a try, so that closing
/// it is the finalisation the language asks for.
pub fn asleep(walk: &Rc<RefCell<Generator>>) -> bool {
    walk.try_borrow().map_or(false, |g| g.started && !g.closed && g.program.is_some() && !g.resume.is_empty())
}
