// The part of the machine that goes with whoever is running the program:
// the faults held while handlers run, where the sleeping body's share of
// them begins, and the escape a source read in let slip. The machine keeps
// one, so that another runner could be handed a set of its own.

use crate::data::Value;
use crate::exec::Escape;

pub(crate) struct Activation {
    pub(crate) holding_fault: Vec<Value>,
    // How many of the held faults belong to whoever asked a sleeping
    // body for its next value, so that what the body puts away while it
    // sleeps is found again at the same remove.
    pub(crate) holding_below: usize,
    // What a source read in got away with: the reading answers with a
    // value or a note, so a throw or an ending is kept here and let
    // go again where the reading stood.
    pub(crate) got_away: Option<Escape>,
}

impl Activation {
    pub(crate) fn begin() -> Self {
        Activation { holding_fault: vec![], holding_below: 0, got_away: None }
    }
}
