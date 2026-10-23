//! What belongs to the one thread of the program that is running: the
//! faults it is handling and the fault a source read in carried out. The
//! engine owns one of these, so a thread of its own can be given its own.

use crate::engine::Fault;
use crate::value::Value;

pub(crate) struct ThreadState {
    /// The faults being handled, innermost last.
    pub(crate) caught: Vec<Value>,
    /// How many of the held faults belong to whoever asked a suspended
    /// body for its next value. A body's own held faults stand above
    /// them, so that it sees its own first and the caller's behind, and
    /// so that what it puts away while it sleeps is found again at the
    /// same remove however deep the caller stood.
    pub(crate) held_base: usize,
    /// A fault raised inside a source read in, carried out past the
    /// reading: what the reading answers with is a note and no fault of
    /// its own, so a throw or an ending is kept here and raised again
    /// where the reading stood.
    pub(crate) carried: Option<Fault>,
}

impl ThreadState {
    pub(crate) fn new() -> Self {
        ThreadState { caught: Vec::new(), held_base: 0, carried: None }
    }
}
