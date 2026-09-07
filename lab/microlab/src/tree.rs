// Four forms. A constant, a binding read, a binding written, a call.
//
// Nothing else exists in the tree. Sequencing is a call of `last`, a
// branch is a call of `if` with the arms as program values, a loop is a
// program that calls itself, a bare block is a program called at once,
// and return, break and continue are calls that unwind to the program
// that catches them.

use std::rc::Rc;

use crate::value::Value;

/// Where a binding lives: `depth` frames up, at `index`. A name that has
/// no binding there yet is read from the global slot instead.
#[derive(Debug, Clone)]
pub struct Slot {
    pub name: Rc<str>,
    pub depth: usize,
    pub index: usize,
    pub global: Option<usize>,
}

/// What a call reaches: a kernel operation, or a program value.
#[derive(Debug)]
pub enum Target {
    Op(Op, Rc<str>),
    Program(Box<Node>),
}

/// The kernel's operations. A definition maps surface names onto the
/// first group; the second group carries the control the tree lacks
/// forms for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Op {
    // builtins a definition may spell
    Emit,
    Print,
    Write,
    Real,
    Precision,
    ToString,
    ToInt,
    ToReal,
    Len,
    CharAt,
    Ord,
    Chr,
    Error,
    Kind,
    Num,
    Den,
    Extern,
    Push,
    Get,
    Put,
    Range,
    // operators
    Add,
    Sub,
    Mul,
    Div,
    DivReal,
    Quot,
    Rem,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Not,
    Neg,
    Concat,
    Index,
    Array,
    // control
    Last,
    If,
    And,
    Or,
    Return,
    Break,
    Continue,
}

#[derive(Debug)]
pub enum Node {
    Literal(Value),
    Load(Slot),
    Assign(Slot, Box<Node>),
    Call(Target, Vec<Node>),
    /// Cycle 1: a loop as a form, run in the frame it appears in, instead
    /// of a program that calls itself. `after` tests after the body.
    Loop { test: Box<Node>, body: Box<Node>, step: Option<Box<Node>>, after: bool },
    /// Cycle 2: a branch as a form, its arms plain nodes in the same frame
    /// instead of program values.
    If { test: Box<Node>, then: Box<Node>, otherwise: Box<Node> },
    /// Cycle 3: an operation of two operands, evaluated without a vector
    /// of arguments.
    Binary { op: Op, name: Rc<str>, a: Box<Node>, b: Box<Node> },
    /// Cycle 4: `x = x + k`, the binding stepped in place.
    Step { slot: Slot, by: i64 },
}

/// What a program stops when it is raised inside: a function stops a
/// return, the program a loop is made of stops a break, a loop's body
/// stops a continue, and a branch arm stops nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Catch {
    Nothing,
    Return,
    Break,
    Continue,
}

#[derive(Debug)]
pub struct Program {
    pub name: String,
    pub params: Vec<String>,
    pub param_slots: Vec<usize>,
    pub names: Vec<String>,
    pub catches: Catch,
    pub body: Node,
}
