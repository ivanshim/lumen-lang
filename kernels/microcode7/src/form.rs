// The seven forms and what they name.
//
// A constant, a binding read, a binding written, a call: the floor of
// the fourth design's ancestor. A cycle, a dyad and a bump: the three
// the kernel lab found worth a form, each removing a routine call, an
// argument list or a form visit from a hot path.

use std::rc::Rc;

use crate::data::Value;

/// Where a binding lives: `depth` frames up, at `index`. A name that has
/// no binding there yet is read from the global slot instead.
#[derive(Debug, Clone)]
pub struct Address {
    pub ident: Rc<str>,
    pub up: usize,
    pub at: usize,
    pub fallback: Option<usize>,
}

/// What a call reaches: a kernel operation, or a program value.
#[derive(Debug)]
pub enum Callee {
    Prim(Prim, Rc<str>),
    Code(Box<Form>),
}

/// The kernel's operations. A definition maps surface names onto the
/// first group; the second group carries the control the tree lacks
/// forms for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Prim {
    // builtins a definition may spell
    Echo,
    Say,
    Out,
    /// Each argument's text in turn, nothing between, no line end.
    Tell,
    /// A named global constant, bound when built (ext.builtin.define).
    Define,
    /// An array or a map written like a call (ext.builtin.array).
    Gather,
    /// A value written over lines, PHP's print_r (ext.builtin.print_r).
    Portray,
    /// Which of two values comes first: below, alike or above.
    Rank,
    /// The first value when it is something, else the second, which is
    /// worked out only then.
    Otherwise,
    /// Give an array back without the place named (ext.builtin.unset).
    Erase,
    /// What an array holds at that place, answering nothing where it
    /// holds nothing there and where what is asked is not an array,
    /// with nothing said about it: how a language asks whether
    /// something is there without minding that it is not.
    Glance,
    /// Whether every one of these is something other than nothing.
    Standing,
    /// Say the run is over where it stands (ext.builtin.exit). Text
    /// given is written out first; a number is not.
    Quit,
    /// What an array holds at that place, an empty array where it holds
    /// nothing there: how a write reaches into a place that is not there
    /// yet, making it on the way in.
    Inward,
    /// Source read while the program runs: the text itself
    /// (ext.builtin.eval), or the text a file holds
    /// (ext.builtin.include). It is built against the globals the run
    /// already has and run where it stands, and what it answers with is
    /// what it gives back.
    Weigh,
    Bring,
    /// What a file holds, all at once; what to put into one; whether a
    /// file is there; and taking one away (ext.builtin.file.*). A
    /// language reaches outside its run only by spelling these.
    Slurp,
    Spill,
    There,
    Gone,
    /// How long the run may take from here, counted in seconds; nought
    /// takes the limit away (ext.builtin.time_limit).
    Clock,
    /// What the running call was handed, whatever of it the routine
    /// gave names to: the whole of it, how much there was, or the one
    /// standing at a place (ext.builtin.args.*).
    Handed,
    HowMany,
    HandedAt,
    /// Whether two values are one and the same, which asks more than
    /// being equal: they must also be of one kind, so 1 and 1.0 are
    /// equal without being the same.
    Selfsame,
    Unlike,
    /// The bits of a value, sixty-four of them, sign and all: both set,
    /// either set, one alone set, all turned over, and moved up or down.
    /// Two pieces of text take their bits letter by letter instead.
    BitsBoth,
    BitsEither,
    BitsOne,
    BitsOver,
    BitsUp,
    BitsDown,
    /// Each argument with its kind, as PHP's var_dump (ext.builtin.var_dump).
    Dump,
    MakeReal,
    Places,
    AsText,
    AsInt,
    AsReal,
    Length,
    CharAtIndex,
    CodeOf,
    CharOf,
    Raise,
    SortOf,
    Numer,
    Denom,
    External,
    Append,
    Fetch,
    Replace,
    Span,
    /// A map from the values given: a couple is a key and its value,
    /// anything else takes the next whole number as its key.
    MakeMap,
    /// `k => v` held as one value until a literal takes it in.
    Couple,
    /// The key, or the item, at a position: how a foreach walks.
    KeyAt,
    ItemAt,
    /// How many places an array or a map holds.
    Extent,
    /// `a[]`, a place only a store reaches.
    AtEnd,
    /// A thing of the class given, its maker run over the rest.
    Spawn,
    /// The property named by the second value, of the thing in the first.
    Of,
    /// Write that property: the thing, the name, the value.
    Onto,
    /// Call the method named second, of the thing named first.
    Ask,
    /// A constant or a kept value of the class given.
    Within,
    /// Write a kept value: the class, the name, the value.
    Into,
    /// Call a method of a named class: what it is for, the class, the
    /// name, then the arguments.
    Bid,
    /// Whether the first is a thing of the class named second.
    Akin,
    /// The name of the class of a thing, or of a class.
    Named,
    /// Raise the value given, for a clause to take.
    Hurl,
    /// An array with one more item, or with one place written: the
    /// array itself is left alone.
    Added,
    Placed,
    // operators
    Plus,
    Minus,
    Times,
    Over,
    OverReal,
    IntDiv,
    Mod,
    Power,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Invert,
    Negate,
    Join,
    At,
    MakeArray,
    // control
    Seq,
    Choose,
    Both,
    Either,
    Yield,
    Leave,
    Resume,
}

#[derive(Debug)]
pub enum Form {
    Const(Value),
    Read(Address),
    Write(Address, Box<Form>),
    Apply(Callee, Vec<Form>),
    /// A loop as a form, run in the frame it appears in, instead
    /// of a program that calls itself. `after` tests after the body.
    Cycle { test: Box<Form>, body: Box<Form>, step: Option<Box<Form>>, after: bool },
    /// An operation of two operands, evaluated without a vector
    /// of arguments.
    Dyad { op: Prim, name: Rc<str>, a: Input, b: Input },
    /// `x = x + k`, the binding stepped in place.
    Bump { slot: Address, by: i64 },
    /// A class declaration: what is known while building, and a value
    /// for each property, kept value and constant the plan names.
    Class { plan: Rc<Plan>, values: Vec<Form> },
    /// A body run with clauses ready to take what it raises, and a last
    /// part that runs however the body ends.
    Attempt { body: Box<Form>, clauses: Vec<Clause>, last: Option<Box<Form>> },
    /// Whether the call left this binding without a value.
    Missing(Address),
    /// A statement together with the line of the source it was written
    /// on, so that a complaint can say where it happened.
    OnLine(u32, Box<Form>),
    /// The binding's own cell, made shareable if it is not already, so
    /// another name can be tied to it.
    Share(Address),
    /// Tie a name to a shared cell, past whatever it held before.
    Tie(Address, Box<Form>),
    /// Make what the array in this binding holds at that place a shared
    /// cell, and give it back: how a walk hands out its items.
    ShareItem(Address, Box<Form>),
    /// Leave this binding as though nothing were ever written to it.
    Forget(Address),
    /// Find this value with whatever it has to say about itself kept
    /// quiet: how a language that lets a program silence one piece of
    /// itself says which piece.
    Muted(Box<Form>),
    /// The binding whose name this value spells, found while the run
    /// goes: how a language reads a name it works out.
    Called(Box<Form>),
    /// Write the second into the binding whose name the first spells.
    CallWrite(Box<Form>, Box<Form>),
}

/// One catch: the classes it takes, where it holds what it caught, and
/// what it does with it.
#[derive(Debug)]
pub struct Clause {
    pub classes: Vec<String>,
    pub held: Option<Address>,
    pub body: Form,
}

/// A class as the builder knows it. What it is built on, and the value
/// of every member, are worked out when the declaration runs.
#[derive(Debug)]
pub struct Plan {
    pub name: String,
    /// How many classes of method names only follow the one this class
    /// is built on, among the values written for it.
    pub answers: usize,
    pub field_names: Vec<String>,
    pub shared_names: Vec<String>,
    pub constant_names: Vec<String>,
    pub methods: Vec<(String, Rc<Routine>)>,
    pub extends: bool,
}

/// An operand of a dyad that is a binding or a
/// constant is read directly, without a visit to a node.
#[derive(Debug)]
pub enum Input {
    Form(Box<Form>),
    Address(Address),
    Const(Value),
}

impl Input {
    pub fn of(node: Form) -> Input {
        match node {
            Form::Read(slot) => Input::Address(slot),
            Form::Const(v) if !matches!(v, Value::Routine(_)) => Input::Const(v),
            other => Input::Form(Box::new(other)),
        }
    }
}

/// What a program stops when it is raised inside: a function stops a
/// return, the program a loop is made of stops a break, a loop's body
/// stops a continue, and a branch arm stops nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Traps {
    Naught,
    Yields,
    Leaves,
    Resumes,
}

#[derive(Debug)]
pub struct Routine {
    /// How many arguments must be given; the rest carry a value of their
    /// own, written by the body's first forms.
    pub least: usize,
    pub ident: String,
    pub formals: Vec<String>,
    pub formal_slots: Vec<usize>,
    pub idents: Vec<String>,
    /// Holds no idents: runs in the frame it closed over, making none.
    pub frameless: bool,
    pub traps: Traps,
    /// The file this program was written in, where it came of text read
    /// as the run went. A call of it is a call into that file: a
    /// complaint names it, and a file it asks for is sought beside it.
    /// Nothing where the program is the run's own.
    pub written_in: Option<Rc<str>>,
    pub body: Form,
}
