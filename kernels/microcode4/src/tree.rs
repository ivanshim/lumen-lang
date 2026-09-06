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
