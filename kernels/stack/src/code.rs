// The machine: a program is a flat list of words over one data stack.
//
// Every construct of every language compiles to these words. Names are
// resolved when compiling: a local is a slot in the running frame, a
// global is a slot in the program-wide table, and a load names both so a
// slot that holds nothing yet falls through to the global of the same
// name, as a callee reading its caller's world does in Lumen.

use std::rc::Rc;

use crate::value::Value;

/// Where a name lives.
#[derive(Debug, Clone)]
pub struct Place {
    /// Candidate local slots, innermost block first; empty at the top level.
    pub locals: Vec<usize>,
    /// The global slot of the same name.
    pub global: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    RealDiv,
    Quot,
    Rem,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Concat,
    Index,
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Emit,
    Print,
    Write,
    Real,
    Precision,
    ToText,
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
}

#[derive(Debug, Clone)]
pub enum Word {
    /// Push a constant.
    Lit(Value),
    /// Push a binding's value.
    Load(Place, Rc<str>),
    /// Pop into a binding.
    Store(Place),
    /// Pop a value and an index; write the element of the named array; push null.
    PutAt(Place, Rc<str>),
    /// Pop a value; append it to the named array; push null.
    Append(Place, Rc<str>),
    /// Forget what a bare block bound, on leaving it.
    Forget { locals: Vec<usize>, globals: Vec<usize> },
    /// Pop two (or one) operands, push the result.
    Op(Op),
    /// Pop the top; push whether it was true.
    Truth,
    /// Pop the arguments; call the program bound to the name; push its result.
    Call { place: Place, name: Rc<str>, argc: usize },
    /// Pop the arguments; apply a kernel builtin; push its result.
    Apply { builtin: Builtin, name: Rc<str>, argc: usize },
    Jump(usize),
    /// Pop; jump when false.
    Unless(usize),
    /// Pop; jump when true.
    When(usize),
    /// Pop the value to return and leave the program.
    Return,
    /// Leave the program with the result so far (an expression statement's value).
    Exit,
    /// Pop and remember as the result so far.
    Result,
    Dup,
    Drop,
    Swap,
    Over,
    Rot,
    /// Pop a program and run it.
    Eval,
    /// Run the program bound to the name, or push the binding's value.
    Run(Place, Rc<str>),
    /// Remember the stack depth.
    Mark,
    /// Gather everything above the last mark into an array.
    Gather,
    /// Pop n values into an array.
    Collect(usize),
}

/// A compiled program: parameters, room for locals, and its words.
#[derive(Debug)]
pub struct Program {
    pub name: String,
    pub params: Vec<String>,
    /// Every slot's name, the parameters first.
    pub names: Vec<String>,
    pub slots: usize,
    pub words: Vec<Word>,
}
