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
    pub ident: String,
    pub formals: Vec<String>,
    pub formal_slots: Vec<usize>,
    pub idents: Vec<String>,
    /// Holds no idents: runs in the frame it closed over, making none.
    pub frameless: bool,
    pub traps: Traps,
    pub body: Form,
}
