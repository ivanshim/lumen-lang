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
    /// Every local slot's name, the parameters first.
    pub idents: Vec<String>,
    /// A function leaves one value, its result; a postfix program leaves
    /// whatever it pushed.
    pub returns_value: bool,
    pub instrs: Vec<Instr>,
}
