// The eight words. A routine is a list of them; the engine runs the list
// over one data stack. Five are the floor of stack5: a constant, a read,
// a write, an action on the stack's top, a conditional skip. Three are
// fused from runs of those, where the kernel lab measured that a word
// earns its place: an operator whose operands come straight from cells,
// a cell stepped in place, and a comparison with its skip.

use std::rc::Rc;

use crate::value::Value;

/// A binding's address: candidate local slots (innermost first) and the
/// global slot of the same name. A load reads the first local that holds
/// a value and falls through to the global; a store writes the first
/// local, or the global when there is none.
#[derive(Debug, Clone)]
pub struct Cell {
    pub ident: Rc<str>,
    pub near: Vec<usize>,
    pub far: usize,
    /// A load that moves the value out and leaves a hole, so the value
    /// is unshared while an operation rewrites it; the next store to the
    /// same name fills the hole.
    pub moving: bool,
}

/// Kernel operations a language can spell. `Apply` names one of these.
#[derive(Debug, Clone)]
pub enum Action {
    Add,
    Sub,
    Mul,
    Div,
    DivReal,
    IntDiv,
    Mod,
    Power,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Join,
    At,
    Not,
    Negate,
    /// Whether a value is true, as a boolean.
    AsBool,
    /// Run the program on top with the arguments under it; a function
    /// pushes its result, a postfix program leaves what it pushed.
    Invoke(Rc<str>),
    /// Run the program on top with no arguments.
    Evaluate,
    /// Run the value on top if it is a program; push it back otherwise.
    Execute,
    /// The arguments as an array.
    MakeArray,
    /// A map from the values above: every tie a pair, everything else
    /// keyed by its position among the untied.
    MakeMap,
    /// `k => v` as one value, until a literal gathers it.
    Tie,
    /// The key, or the value, of the pair at a position: how a foreach
    /// walks an array or a map.
    KeyAt,
    ValueAt,
    /// `a[]`, which only an assignment may write to.
    AtEnd,
    /// How many places an array or a map holds.
    Extent,
    /// Which of two values comes first: below, alike, or above.
    Rank,
    /// Whether two values are the very same: of one kind, and alike
    /// within it. 1 and 1.0 are equal but not the same.
    Same,
    Unsame,
    /// The bits of two whole numbers taken together, and the bits of one
    /// turned over. A number is read as sixty-four bits, sign and all.
    BitBoth,
    BitEither,
    BitOne,
    BitTurn,
    /// The bits moved up or down that many places.
    BitUp,
    BitDown,
    /// Whether the value above is nothing at all.
    Nothing,
    /// What an array holds at that place, answering nothing where it
    /// holds nothing there, or where what is asked is not an array at
    /// all, and saying nothing about it either way: how a language asks
    /// whether something is there without minding that it is not.
    Peek,
    /// Build the class this plan describes; what it stands on, if it
    /// stands on anything, is the value below.
    Forge(Rc<Plan>),
    /// A new object of the class below the arguments, its maker run.
    Make,
    /// The property of that name, of the object above.
    Grab(Rc<str>),
    /// Write that property: the object, then the value.
    Plant(Rc<str>),
    /// Call that method of the object below the arguments.
    Send(Rc<str>),
    /// A constant or a class's own value, of the class above.
    Reach(Rc<str>),
    /// Write a class's own value: the class, then the value.
    Sow(Rc<str>),
    /// Call that method of a named class: the object it is for, the
    /// class, then the arguments.
    Summon(Rc<str>),
    /// Whether the object above is of that class, or of one beneath it.
    Kindred(Rc<str>),
    /// The name of the class of the value above.
    Titled,
    /// Raise the value above as a fault to be caught.
    Hurl,
    /// Whether the value above is of any of those classes; it is consumed.
    Matches(Rc<Vec<String>>),
    /// Push the value above a second time.
    Twin,
    /// Everything above the nearest mark as an array, the mark removed.
    Collect,
    Builtin(Builtin, Rc<str>),
}

/// Builtins a definition names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Echo,
    Say,
    Out,
    /// Every argument's text, run together, no line end (ext.builtin.echo).
    Tell,
    /// A named global constant (ext.builtin.define); bound at compile time.
    Define,
    /// An array or map written as a call (ext.builtin.array).
    Pack,
    /// A value laid out over lines, PHP's print_r (ext.builtin.print_r).
    Layout,
    /// Take a binding, or a place in an array, away (ext.builtin.unset).
    Erase,
    /// Whether each of those bindings, or places in an array, holds
    /// something other than nothing, asking without minding that a
    /// binding was never written or a place is not there
    /// (ext.builtin.isset).
    Held,
    /// Each argument with its kind, PHP's var_dump (ext.builtin.var_dump).
    Dump,
    /// Source read while the program runs: the text itself
    /// (ext.builtin.eval), or the text a file holds
    /// (ext.builtin.include). It is assembled against the same globals
    /// and run where it stands, and what it gives back is its answer.
    Eval,
    Include,
    /// What a file holds, all of it at once; what to write into one;
    /// whether a file is there at all; and taking one away
    /// (ext.builtin.file.*). Only a language that spells these reaches
    /// outside the run at all.
    FileRead,
    FileWrite,
    FileThere,
    FileGone,
    /// How long the run may take from here, in seconds; nought lifts
    /// the limit (ext.builtin.time_limit).
    TimeLimit,
    /// What the running call was given, however much of it the routine
    /// named: all of it as an array, how much there was, or the one at a
    /// position (ext.builtin.args.*).
    Given,
    GivenCount,
    GivenAt,
    MakeReal,
    Places,
    ToText,
    ToInt,
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
}

/// An operand a fused word reads without the stack: a binding or a constant.
#[derive(Debug, Clone)]
pub enum Operand {
    Cell(Cell),
    Const(Value),
    /// The top of the stack, popped.
    Top,
}

#[derive(Debug, Clone)]
pub enum Instr {
    /// Push a constant.
    Const(Value),
    /// Push a binding's value.
    Read(Cell),
    /// Pop into a binding.
    Write(Cell),
    /// Pop the arguments, apply the operation, push its result.
    Act(Action, usize),
    /// Pop; when the value is not true, continue at the index.
    Skip(usize),
    /// Whether the call left the frame's slot without a value.
    Missing(usize),
    /// Whether the global at this place has never been written.
    Unwritten(usize),
    /// Which line of the source the instrs after this one came from,
    /// so that a complaint can say where it happened.
    Line(u32),
    /// Start keeping complaints quiet, or stop: how a language that
    /// lets a program hush what one piece of it has to say about
    /// itself says where the quiet begins and ends.
    Hush(bool),
    /// Make this binding a shared cell if it is not one already, and
    /// push that cell, so another name can be fastened to it.
    Bond(Cell),
    /// Pop a shared cell and put it in this binding, so the two names
    /// stand for one cell from here on.
    Fasten(Cell),
    /// Pop a place, make what the array in this binding holds there a
    /// shared cell, and push that cell: how a walk hands out its items
    /// for writing.
    BondItem(Cell),
    /// Leave this binding as though nothing were ever written to it.
    Forget(Cell),
    /// From here to the matching Unguard, a raised value is caught: the
    /// stack goes back to its depth here, the value is pushed, and the
    /// run goes on at the index.
    Guard(usize),
    Unguard,
    /// A binary operation whose operands come from bindings, constants or
    /// the stack, the result pushed: an operator that never touches the
    /// stack for a binding.
    Dyad { op: Action, a: Operand, b: Operand },
    /// A binding stepped by a constant in place.
    Bump { slot: Cell, by: Value },
    /// A comparison of two operands and a jump when it fails: the test of
    /// a loop or a branch in one word.
    SkipCmp { op: Action, a: Operand, b: Operand, to: usize },
}

/// A compiled program.
#[derive(Debug)]
pub struct Routine {
    pub ident: String,
    pub formals: Vec<String>,
    /// How many arguments must be given; the rest have a value of their
    /// own, written by the program's own first instrs.
    pub least: usize,
    /// Every local slot's name, the parameters first.
    pub idents: Vec<String>,
    /// A function leaves one value, its result; a postfix program leaves
    /// whatever it pushed.
    pub returns_value: bool,
    /// The program's own body, which nothing called: what a call was
    /// given cannot be read from within it.
    pub body_of_all: bool,
    pub instrs: Vec<Instr>,
}

/// What a class declaration comes to: everything about the class that is
/// known while compiling. What it stands on is looked up when it runs.
#[derive(Debug)]
pub struct Plan {
    pub name: String,
    /// How many classes of method names only stand after the one this
    /// class is built on, when the class is forged.
    pub answers: usize,
    /// The names of the properties, of the values the class keeps for
    /// itself and of its constants; a value for each is on the stack, in
    /// that order, when the class is forged.
    pub field_names: Vec<String>,
    pub shared_names: Vec<String>,
    pub constant_names: Vec<String>,
    pub methods: Vec<(String, Rc<Routine>)>,
    /// Whether a class to stand on is given first.
    pub extends: bool,
}
