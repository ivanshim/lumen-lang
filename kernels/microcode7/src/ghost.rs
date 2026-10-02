//! Weak holds on things, farewells, and the reaping of unreachable rounds.
//!
//! The machine owns its things by counting holds; the last hold to be
//! let go takes the thing with it. A weak hold is the library's own
//! downgraded pointer: free to keep, and never enough to keep a thing.
//! This module keeps the books around such holds -- which of them want
//! telling when their thing goes, which things have a farewell method
//! to run, and how a round of things holding one another (which counting
//! never frees) is found and cut when the program asks for it.
//!
//! Nothing in here can reach the machine. Something going away leaves a
//! note in a thread-local list; the machine reads the list at its next
//! step and makes the calls.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

use crate::data::{Adornment, Blueprint, Env, IteratorKind, SetStore, Thing, Value};
use crate::exec::Suspension;
use crate::form::Routine;

/// What a weak hold is on.
#[derive(Clone)]
pub enum Ghost {
    StaticKind(Value),
    Thing(Weak<Thing>),
    Blueprint(Weak<Blueprint>),
    Walk(Weak<RefCell<Suspension>>),
    Set(Weak<RefCell<SetStore>>),
    Bound(Weak<Routine>, Weak<Env>),
    Routine(Weak<Routine>),
    Method(Weak<Routine>, Weak<Thing>, Weak<crate::data::MethodMark>),
    WrappedMethod(Weak<Vec<Value>>),
}

impl Ghost {
    /// The thing itself, for as long as it is still about.
    pub fn revive(&self) -> Option<Value> {
        match self {
            Ghost::StaticKind(kind) => Some(kind.clone()),
            Ghost::Thing(w) => w.upgrade().map(Value::Thing),
            Ghost::Blueprint(w) => w.upgrade().map(Value::Blueprint),
            Ghost::Walk(w) => w.upgrade().map(Value::Generator),
            Ghost::Set(w) => w.upgrade().map(Value::Set),
            Ghost::Bound(p, e) => Some(Value::Bound(p.upgrade()?, e.upgrade()?)),
            Ghost::Routine(w) => w.upgrade().map(Value::Routine),
            Ghost::Method(p, t, identity) => Some(Value::Method(p.upgrade()?, t.upgrade()?, identity.upgrade()?)),
            Ghost::WrappedMethod(parts) => Some(Value::Wrapped(3, parts.upgrade()?.into())),
        }
    }

    pub fn departed(&self) -> bool {
        self.revive().is_none()
    }
}

/// A weak hold as the program carries it: what it is on, the program's
/// own reference object bearing it, and what to call with that object
/// once the thing has gone.
pub struct Dim {
    pub ghost: Ghost,
    pub bearer: Weak<Thing>,
    pub notify: RefCell<Option<Value>>,
    pub hash: RefCell<Option<Value>>,
    pub detached: Cell<bool>,
}


impl Dim {
    pub fn revive(&self) -> Option<Value> {
        (!self.detached.get()).then(|| self.ghost.revive()).flatten()
    }
    pub fn departed(&self) -> bool { self.detached.get() || self.ghost.departed() }
}

pub fn detach_garbage(knots: &[Knot]) {
    let places: HashSet<_> = knots.iter().filter_map(Knot::place).collect();
    REFS.with(|registry| {
        let watchers: Vec<_> = registry.borrow().iter().filter_map(Weak::upgrade).collect();
        for dim in watchers {
            let target = dim.revive().and_then(|v| Knot::Held(v).place());
            let discarded = dim.bearer.upgrade().is_some_and(|t| places.contains(&(Rc::as_ptr(&t) as usize)));
            if !discarded && !target.is_some_and(|p| places.contains(&p)) { continue; }
            dim.detached.set(true);
            if discarded {
                *dim.notify.borrow_mut() = None;
            }
            LOST.with(|mark| mark.set(true));
            STIRRED.with(|mark| mark.set(true));
        }
    });
}

thread_local! {
    static FAREWELL_NAME: RefCell<Option<String>> = const { RefCell::new(None) };
    static LISTENING: Cell<usize> = const { Cell::new(0) };
    static REFS: RefCell<Vec<Weak<Dim>>> = const { RefCell::new(Vec::new()) };
    static LISTENERS: RefCell<Vec<Rc<Dim>>> = const { RefCell::new(Vec::new()) };
    static STIRRED: Cell<bool> = const { Cell::new(false) };
    static LOST: Cell<bool> = const { Cell::new(false) };
    static FAREWELLS: RefCell<Vec<(Rc<Thing>, Value)>> = const { RefCell::new(Vec::new()) };
    static HALF_WALKS: RefCell<Vec<Rc<RefCell<Suspension>>>> = const { RefCell::new(Vec::new()) };
    static BIDDEN: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
    static NOTABLE: RefCell<(Vec<Ghost>, usize)> = const { RefCell::new((Vec::new(), 8)) };
}

/// The method a class bids farewell with, where the language has one.
/// Until it is named, nothing that goes is looked at.
pub fn set_farewell_name(word: Option<String>) {
    let _ = FAREWELL_NAME.try_with(|n| *n.borrow_mut() = word);
}

/// Whether farewells are a thing in this language at all.
pub fn bidding() -> bool {
    FAREWELL_NAME.try_with(|n| n.borrow().is_some()).unwrap_or(false)
}

/// Whether the machine has anything to attend to before its next step.
pub fn stirred() -> bool {
    let gone_method = LISTENERS.try_with(|queue| queue.try_borrow().map_or(false, |held| held.iter().any(|listener| matches!(listener.ghost, Ghost::WrappedMethod(_)) && listener.departed()))).unwrap_or(false);
    if gone_method { anything_departing(); }
    STIRRED.try_with(|s| s.get()).unwrap_or(false)
}

fn stir() {
    let _ = STIRRED.try_with(|s| s.set(true));
}

/// Something a listener might be listening for has gone.
pub fn anything_departing() {
    if LISTENING.try_with(|l| l.get()).unwrap_or(0) == 0 { return; }
    let _ = LOST.try_with(|l| l.set(true));
    stir();
}

/// The farewell method of a class, looked for among its programs first
/// and then among the values it keeps for itself.
pub fn farewell_of(class: &Blueprint) -> Option<Value> {
    let name = FAREWELL_NAME.try_with(|n| n.borrow().clone()).ok().flatten()?;
    if let Some(program) = class.program(&name) {
        return Some(Value::Routine(program.clone()));
    }
    let keeper = class.keeper(&name)?;
    let kept = keeper.shared.borrow();
    kept.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone())
}

/// Marks a thing's farewell as bidden, answering whether it was not
/// already: a farewell is bidden once.
pub fn first_farewell(thing: &Rc<Thing>) -> bool {
    let where_it_lives = Rc::as_ptr(thing) as usize;
    BIDDEN.try_with(|b| b.borrow_mut().insert(where_it_lives)).unwrap_or(false)
}

/// A thing is going. If its class bids farewell and this one has not
/// yet, it is put back together around its own members and queued for
/// the machine to bid it.
pub fn thing_departing(going: &mut Thing) {
    let here = going as *const Thing as usize;
    // With the thread's storage gone (at the very end) nothing is
    // queued: there is no machine left to run anything.
    let bidden = BIDDEN.try_with(|b| b.borrow_mut().remove(&here)).unwrap_or(true);
    if !bidden {
        if let Some(farewell) = farewell_of(&going.of) {
            let members = std::mem::take(going.holds.get_mut());
            let again = Rc::new(Thing { reclassified: RefCell::new(going.reclassified.borrow().clone()), of: going.of.clone(), holds: RefCell::new(members), turn: going.turn });
            let _ = BIDDEN.try_with(|b| b.borrow_mut().insert(Rc::as_ptr(&again) as usize));
            let _ = FAREWELLS.try_with(|f| f.borrow_mut().push((again, farewell)));
            stir();
        }
    }
    anything_departing();
}

/// A walk asleep inside a try is going: the machine rebuilt it around
/// its own frame, and hands it here to be shut in its turn.
pub fn walk_departing(again: Rc<RefCell<Suspension>>) {
    let _ = HALF_WALKS.try_with(|h| h.borrow_mut().push(again));
    stir();
}

/// Everything the machine owes now: farewells to bid, walks to shut,
/// and listeners whose thing has gone, each with its bearer. The flag
/// is cleared; more may gather while the machine works, so it asks
/// again until nothing comes back.
pub fn gather() -> (Vec<(Rc<Thing>, Value)>, Vec<Rc<RefCell<Suspension>>>, Vec<(Value, Value)>) {
    let _ = STIRRED.try_with(|s| s.set(false));
    let farewells = FAREWELLS.try_with(|f| std::mem::take(&mut *f.borrow_mut())).unwrap_or_default();
    let walks = HALF_WALKS.try_with(|h| std::mem::take(&mut *h.borrow_mut())).unwrap_or_default();
    let mut notices = Vec::new();
    let lost = LOST.try_with(|l| l.replace(false)).unwrap_or(false);
    if lost {
        let _ = LISTENERS.try_with(|l| {
            let mut all = l.borrow_mut();
            let mut still = Vec::with_capacity(all.len());
            for dim in all.drain(..).rev() {
                if dim.bearer.strong_count() == 0 { continue; }
                if !dim.departed() {
                    still.push(dim);
                } else if let (Some(bearer), Some(notify)) = (dim.bearer.upgrade(), dim.notify.borrow_mut().take()) {
                    notices.push((notify, Value::Thing(bearer)));
                }
            }
            let _ = LISTENING.try_with(|n| n.set(still.len()));
            still.reverse();
            *all = still;
        });
    }
    (farewells, walks, notices)
}

/// The weak hold a value admits of, through whatever cell holds it, or
/// nothing for a kind that cannot be held weakly.
pub fn ghost_of(value: &Value) -> Option<Ghost> {
    Some(match value {
        Value::Shared(cell) | Value::Mutable(cell, _) => return ghost_of(&cell.borrow()),
        Value::Thing(t) => Ghost::Thing(Rc::downgrade(t)),
        Value::Intrinsic(code, _) if code.names_a_kind() => Ghost::StaticKind(value.clone()),
        Value::Blueprint(b) => Ghost::Blueprint(Rc::downgrade(b)),
        Value::Generator(g) => Ghost::Walk(Rc::downgrade(g)),
        Value::Set(s) => Ghost::Set(Rc::downgrade(s)),
        Value::Bound(p, e) => Ghost::Bound(Rc::downgrade(p), Rc::downgrade(e)),
        Value::Routine(p) => Ghost::Routine(Rc::downgrade(p)),
        Value::Method(p, t, identity) => Ghost::Method(Rc::downgrade(p), Rc::downgrade(t), Rc::downgrade(identity)),
        Value::Wrapped(3, parts) => Ghost::WrappedMethod(Rc::downgrade(parts)),
        _ => return None,
    })
}

/// A weak hold for the program to carry. Its thing is noted as one whose
/// going could be noticed; a hold that wants telling is listened for.
pub fn dim(ghost: Ghost, bearer: Weak<Thing>, notify: Option<Value>) -> Value {
    note(ghost.clone());
    let wants_telling = notify.is_some();
    let held = Rc::new(Dim { ghost, bearer, notify: RefCell::new(notify), hash: RefCell::new(None), detached: Cell::new(false) });
    REFS.with(|refs| {
        let mut refs = refs.borrow_mut();
        refs.retain(|weak| weak.strong_count() != 0);
        refs.push(Rc::downgrade(&held));
    });
    if wants_telling {
        let _ = LISTENERS.try_with(|l| l.borrow_mut().push(held.clone()));
        let _ = LISTENING.try_with(|n| n.set(n.get() + 1));
    }
    Value::Dim(held)
}

/// Note a thing whose going somebody could notice. Every time the list
/// has doubled since it was last swept, the departed are swept out.
pub fn note(ghost: Ghost) {
    let _ = NOTABLE.try_with(|n| {
        let mut n = n.borrow_mut();
        if n.0.len() >= n.1 {
            n.0.retain(|g| !g.departed());
            n.1 = (n.0.len() * 2).max(8);
        }
        n.0.push(ghost);
    });
}

/// One place in the web: a value, or a frame, which is no value but
/// holds values and is held by bound routines.
pub enum Knot {
    Held(Value),
    Frame(Rc<Env>),
}

impl Knot {
    /// Where the knot's pointer stands, for the kinds that live behind one.
    fn place(&self) -> Option<usize> {
        Some(match self {
            Knot::Frame(env) => Rc::as_ptr(env) as *const () as usize,
            Knot::Held(value) => match value {
                Value::Dim(d) => Rc::as_ptr(d) as *const () as usize,
                Value::Thing(t) | Value::Attributes(t) => Rc::as_ptr(t) as *const () as usize,
                Value::Blueprint(b) => Rc::as_ptr(b) as *const () as usize,
                Value::Generator(g) => Rc::as_ptr(g) as *const () as usize,
                Value::Set(s) | Value::SetCursor { source: s, .. } => Rc::as_ptr(s) as *const () as usize,
                Value::Iterator(i) => Rc::as_ptr(i) as *const () as usize,
                Value::Backtrace(t) => Rc::as_ptr(t) as *const () as usize,
                Value::Shared(c) | Value::Mutable(c, _) => Rc::as_ptr(c) as *const () as usize,
                Value::Vector(v) | Value::Tuple(v) | Value::Row(v) | Value::Arguments(v) | Value::Span(v) | Value::Wrapped(_, v) => Rc::as_ptr(v) as *const () as usize,
                Value::Dict(d) => Rc::as_ptr(d) as *const () as usize,
                Value::Couple(c) => Rc::as_ptr(c) as *const () as usize,
                Value::Member(v, _) | Value::Window(v, _) => Rc::as_ptr(v) as *const () as usize,
                Value::Cursor(c) => Rc::as_ptr(c) as *const () as usize,
                Value::Adorned(a) => Rc::as_ptr(a) as *const () as usize,
                Value::Routine(p) => Rc::as_ptr(p) as *const () as usize,
                Value::Traversal(v, _) => Rc::as_ptr(v) as *const () as usize,
                _ => return None,
            },
        })
    }

    /// How many strong holds there are on the knot's pointer.
    fn holds(&self) -> usize {
        match self {
            Knot::Frame(env) => Rc::strong_count(env),
            Knot::Held(value) => match value {
                Value::Dim(d) => Rc::strong_count(d).saturating_sub(usize::from(d.notify.borrow().is_some())),
                Value::Thing(t) | Value::Attributes(t) => Rc::strong_count(t),
                Value::Blueprint(b) => Rc::strong_count(b),
                Value::Generator(g) => Rc::strong_count(g),
                Value::Set(s) | Value::SetCursor { source: s, .. } => Rc::strong_count(s),
                Value::Iterator(i) => Rc::strong_count(i),
                Value::Backtrace(t) => Rc::strong_count(t),
                Value::Shared(c) | Value::Mutable(c, _) => Rc::strong_count(c),
                Value::Vector(v) | Value::Tuple(v) | Value::Row(v) | Value::Arguments(v) | Value::Span(v) | Value::Wrapped(_, v) => Rc::strong_count(v),
                Value::Dict(d) => Rc::strong_count(d),
                Value::Couple(c) => Rc::strong_count(c),
                Value::Member(v, _) | Value::Window(v, _) => Rc::strong_count(v),
                Value::Cursor(c) => Rc::strong_count(c),
                Value::Adorned(a) => Rc::strong_count(a),
                Value::Routine(p) => Rc::strong_count(p),
                Value::Traversal(v, _) => Rc::strong_count(v),
                _ => 0,
            },
        }
    }

    /// The knots this one holds directly. A cell being written to at
    /// this moment yields none, which leans towards keeping.
    fn onward(&self, out: &mut Vec<Knot>) {
        let held = |out: &mut Vec<Knot>, v: &Value| out.push(Knot::Held(v.clone()));
        match self {
            Knot::Frame(env) => {
                if let Ok(cells) = env.cells.try_borrow() {
                    for v in cells.iter() { held(out, v); }
                }
                if let Some(outer) = &env.outer { out.push(Knot::Frame(outer.clone())); }
            }
            Knot::Held(value) => match value {
                Value::Dim(d) => if let Some(callback) = d.notify.borrow().as_ref() { held(out, callback); },
                Value::Thing(t) | Value::Attributes(t) => {
                    out.push(Knot::Held(Value::Blueprint(t.of.clone())));
                    if let Ok(members) = t.holds.try_borrow() {
                        for (_, v) in members.iter() { held(out, v); }
                    }
                }
                Value::Blueprint(b) => {
                    for (_, v) in b.fields.iter().chain(b.constants.iter()) { held(out, v); }
                    if let Ok(shared) = b.shared.try_borrow() {
                        for (_, v) in shared.iter() { held(out, v); }
                    }
                    for (_, p) in &b.methods { out.push(Knot::Held(Value::Routine(p.clone()))); }
                    for parent in b.under.iter().chain(&b.ancestry).chain(&b.parents).chain(&b.answers) {
                        out.push(Knot::Held(Value::Blueprint(parent.clone())));
                    }
                }
                Value::Generator(g) => {
                    if let Ok(g) = g.try_borrow() { g.reaches(out); }
                }
                Value::Set(s) | Value::SetCursor { source: s, .. } => {
                    if let Ok(store) = s.try_borrow() {
                        for (_, v) in &store.entries { held(out, v); }
                    }
                }
                Value::Iterator(i) => {
                    if let Ok(state) = i.try_borrow() {
                        if let Some(p) = &state.peek { held(out, p); }
                        if let Some(t) = &state.from_thing { held(out, t); }
                        match &state.kind {
                            IteratorKind::Stored { entries, .. } => for v in entries.iter() { held(out, v); },
                            IteratorKind::Living(cell, _) => out.push(Knot::Held(Value::Shared(cell.clone()))),
                            IteratorKind::Watching { window, .. } => held(out, window),
                            IteratorKind::Placed(v, _) | IteratorKind::PlacedBack(v, _) | IteratorKind::Handed(v) | IteratorKind::Count(v, _) => held(out, v),
                            IteratorKind::Summoned { work, stop, stop_exception } => { held(out, work); held(out, stop); if let Some(v) = stop_exception { held(out, v); } }
                            IteratorKind::Select(work, stop) => { held(out, work); held(out, stop); }
                            IteratorKind::Parallel { inputs, mapper, .. } => {
                                for v in inputs { held(out, v); }
                                if let Some(m) = mapper { held(out, m); }
                            }
                            IteratorKind::Stepping(..) | IteratorKind::Busy => {}
                        }
                    }
                }
                Value::Backtrace(t) => {
                    out.push(Knot::Held(Value::Thing(t.activation.clone())));
                    held(out, &t.following);
                }
                Value::Shared(c) | Value::Mutable(c, _) => {
                    if let Ok(inner) = c.try_borrow() { held(out, &inner); }
                }
                Value::Vector(v) | Value::Tuple(v) | Value::Row(v) | Value::Arguments(v) | Value::Span(v) | Value::Wrapped(_, v) => {
                    for item in v.iter() { held(out, item); }
                }
                Value::Dict(d) => {
                    for (k, v) in d.iter() { held(out, k); held(out, v); }
                }
                Value::Couple(c) => { held(out, &c.0); held(out, &c.1); }
                Value::Member(v, _) | Value::Window(v, _) => held(out, v),
                Value::Traversal(v, cell) => {
                    held(out, v);
                    if let Ok(inner) = cell.try_borrow() {
                        if let Some(v) = inner.as_ref() { held(out, v); }
                    }
                }
                Value::Cursor(c) => {
                    if let Ok(items) = c.try_borrow() {
                        for v in items.iter() { held(out, v); }
                    }
                }
                Value::Adorned(a) => {
                    let Adornment { target, extra, .. } = &**a;
                    held(out, target);
                    if let Some(v) = extra { held(out, v); }
                }
                _ => {}
            },
        }
    }

    /// A knot with no place of its own stands for the places it holds:
    /// a bound routine for its frame (the routine itself holds nothing),
    /// a bound method for its thing, a keyed pair for its two values.
    fn parts(self) -> Vec<Knot> {
        match self {
            Knot::Held(Value::Bound(_, env)) => vec![Knot::Frame(env)],
            Knot::Held(Value::Method(_, thing, _)) => vec![Knot::Held(Value::Thing(thing))],
            Knot::Held(Value::Keyed(k, v)) => vec![Knot::Held((*k).clone()), Knot::Held((*v).clone())],
            other => vec![other],
        }
    }
}

struct Strand {
    knot: Knot,
    onward: Vec<usize>,
    inward: usize,
    reached: bool,
}

/// Everything reachable from the notable things, each once, with every
/// hold between them counted. A hold the web does not account for comes
/// from outside it -- a frame under way, the machine, a value in hand --
/// and whatever has one, or is reached from something that has one, the
/// program can still get at. The rest it cannot.
pub struct Web {
    strands: HashMap<usize, Strand>,
}

impl Web {
    /// Woven from the notable things still about.
    pub fn from_notable() -> Web {
        let roots: Vec<Knot> = NOTABLE.try_with(|n| {
            let mut n = n.borrow_mut();
            n.0.retain(|g| !g.departed());
            n.1 = (n.0.len() * 2).max(8);
            n.0.iter().filter_map(|g| g.revive()).map(Knot::Held).collect()
        }).unwrap_or_default();
        Web::weave(roots)
    }

    fn weave(roots: Vec<Knot>) -> Web {
        let mut strands: HashMap<usize, Strand> = HashMap::new();
        let mut frontier: Vec<usize> = Vec::new();
        for root in roots.into_iter().flat_map(Knot::parts) {
            if let Some(place) = root.place() {
                if !strands.contains_key(&place) {
                    strands.insert(place, Strand { knot: root, onward: Vec::new(), inward: 0, reached: false });
                    frontier.push(place);
                }
            }
        }
        let mut next = Vec::new();
        while let Some(place) = frontier.pop() {
            next.clear();
            strands[&place].knot.onward(&mut next);
            let mut onward = Vec::with_capacity(next.len());
            for knot in next.drain(..).flat_map(Knot::parts) {
                let Some(at) = knot.place() else { continue };
                if let Some(strand) = strands.get_mut(&at) {
                    strand.inward += 1;
                } else {
                    strands.insert(at, Strand { knot, onward: Vec::new(), inward: 1, reached: false });
                    frontier.push(at);
                }
                onward.push(at);
            }
            if let Some(strand) = strands.get_mut(&place) { strand.onward = onward; }
        }
        Web { strands }
    }

    /// The knots the program cannot get at any more. The web's own hold
    /// on each is the one hold not counted among the program's.
    pub fn unreachable(&mut self) -> Vec<Knot> {
        let mut frontier: Vec<usize> = Vec::new();
        for (place, strand) in self.strands.iter_mut() {
            if strand.knot.holds() > strand.inward + 1 {
                strand.reached = true;
                frontier.push(*place);
            }
        }
        while let Some(place) = frontier.pop() {
            let onward = self.strands[&place].onward.clone();
            for at in onward {
                if let Some(strand) = self.strands.get_mut(&at) {
                    if !strand.reached {
                        strand.reached = true;
                        frontier.push(at);
                    }
                }
            }
        }
        let mut lost = Vec::new();
        for strand in self.strands.values() {
            if strand.reached { continue; }
            lost.push(match &strand.knot {
                Knot::Held(v) => Knot::Held(v.clone()),
                Knot::Frame(e) => Knot::Frame(e.clone()),
            });
        }
        lost
    }

    /// Cuts every unreachable round: each knot that can be emptied is,
    /// and counting frees the rest. What was taken out is handed back to
    /// be dropped once the web itself has gone.
    pub fn cut(lost: &[Knot]) -> Vec<Value> {
        let mut taken = Vec::new();
        for knot in lost {
            match knot {
                Knot::Frame(env) => {
                    if let Ok(mut cells) = env.cells.try_borrow_mut() {
                        taken.extend(cells.drain(..));
                    }
                }
                Knot::Held(Value::Thing(t)) | Knot::Held(Value::Attributes(t)) => {
                    if let Ok(mut members) = t.holds.try_borrow_mut() {
                        taken.extend(std::mem::take(&mut *members).into_iter().map(|(_, v)| v));
                    }
                }
                Knot::Held(Value::Blueprint(b)) => {
                    if let Ok(mut shared) = b.shared.try_borrow_mut() {
                        taken.extend(std::mem::take(&mut *shared).into_iter().map(|(_, v)| v));
                    }
                }
                Knot::Held(Value::Shared(c)) | Knot::Held(Value::Mutable(c, _)) => {
                    if let Ok(mut inner) = c.try_borrow_mut() {
                        taken.push(std::mem::replace(&mut *inner, Value::Nil));
                    }
                }
                Knot::Held(Value::Set(s)) | Knot::Held(Value::SetCursor { source: s, .. }) => {
                    if let Ok(mut store) = s.try_borrow_mut() {
                        store.keys.clear();
                        taken.extend(std::mem::take(&mut store.entries).into_iter().map(|(_, v)| v));
                    }
                }
                Knot::Held(Value::Generator(g)) => {
                    if let Ok(mut walk) = g.try_borrow_mut() {
                        taken.extend(walk.sever());
                    }
                }
                Knot::Held(Value::Iterator(i)) => {
                    if let Ok(mut state) = i.try_borrow_mut() {
                        taken.extend(state.peek.take());
                        taken.extend(state.from_thing.take());
                        let was = std::mem::replace(&mut state.kind, IteratorKind::Busy);
                        if let IteratorKind::Stored { entries, .. } = was { taken.extend(entries.iter().cloned()); }
                    }
                }
                Knot::Held(Value::Cursor(c)) => {
                    if let Ok(mut items) = c.try_borrow_mut() {
                        taken.extend(items.drain(..));
                    }
                }
                Knot::Held(Value::Traversal(_, cell)) => {
                    if let Ok(mut inner) = cell.try_borrow_mut() {
                        taken.extend(inner.take());
                    }
                }
                _ => {}
            }
        }
        taken
    }
}

/// Living public weak references, with the two reusable kinds at the head.
pub fn refs_for(subject: &Value) -> Vec<Value> {
    let mut found = REFS.with(|refs| refs.borrow().iter().rev().filter_map(|entry| {
        let dim = entry.upgrade()?;
        let target = dim.revive()?;
        if !subject.one_place(&target) { return None; }
        dim.bearer.upgrade().map(Value::Thing)
    }).collect::<Vec<_>>());
    found.sort_by_key(|item| {
        let Value::Thing(thing) = item else { return 2; };
        let no_callback = thing.holds.borrow().iter().any(|(key, held)| {
            key == "\0weak" && matches!(held, Value::Dim(dim) if dim.notify.borrow().is_none())
        });
        if no_callback {
            match thing.blueprint().name.as_str() {
                "ReferenceType" => return 0,
                "ProxyType" | "CallableProxyType" => return 1,
                _ => (),
            }
        }
        2
    });
    found
}
