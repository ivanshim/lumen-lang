// The five words. A program is a list of them; the machine runs the list
// over one data stack. There is nothing else: no jump but the conditional
// one, no call word, no stack shufflers, no loop. Everything a language
// spells is assembled from these five.

use std::rc::Rc;

use crate::values::Value;

/// A binding's address: candidate local slots (innermost first) and the
/// global slot of the same name. A load reads the first local that holds
/// a value and falls through to the global; a store writes the first
/// local, or the global when there is none.
#[derive(Debug, Clone)]
pub struct Slot {
    pub name: Rc<str>,
    pub locals: Vec<usize>,
    pub global: usize,
    /// A load that moves the value out and leaves a hole, so the value
    /// is unshared while an operation rewrites it; the next store to the
    /// same name fills the hole.
    pub take: bool,
}

/// Kernel operations a language can spell. `Apply` names one of these.
#[derive(Debug, Clone)]
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
    /// Whether a value is true, as a boolean.
    Truth,
    /// Run the program on top with the arguments under it; a function
    /// pushes its result, a postfix program leaves what it pushed.
    Call(Rc<str>),
    /// Run the program on top with no arguments.
    Eval,
    /// Run the value on top if it is a program; push it back otherwise.
    Run,
    /// The arguments as an array.
    Array,
    /// Everything above the nearest mark as an array, the mark removed.
    Gather,
    Native(Native, Rc<str>),
}

/// Builtins a definition names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Native {
    Emit,
    Print,
    Write,
    Real,
    Precision,
    Text,
    Int,
    ToReal,
    Len,
    CharAt,
    Ord,
    Chr,
    Fail,
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
    Load(Slot),
    /// Pop into a binding.
    Store(Slot),
    /// Pop the arguments, apply the operation, push its result.
    Apply(Op, usize),
    /// Pop; when the value is not true, continue at the index.
    Unless(usize),
}

/// A compiled program.
#[derive(Debug)]
pub struct Program {
    pub name: String,
    pub params: Vec<String>,
    /// Every local slot's name, the parameters first.
    pub names: Vec<String>,
    /// A function leaves one value, its result; a postfix program leaves
    /// whatever it pushed.
    pub yields: bool,
    pub words: Vec<Word>,
}
