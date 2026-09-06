// The tree of primitives: the kernel's normal form.
//
// Nine forms carry every construct of every language. A node keeps the
// source line it came from, so errors and the emitter can point back at
// the program. Names are resolved when the tree is built: a `Slot` says
// where a binding lives, and keeps the name for the emitter.

use std::rc::Rc;

use crate::value::Value;

/// Where a name lives: candidate local slots (innermost block first) and
/// the global slot of the same name, which a load falls through to.
#[derive(Debug, Clone)]
pub struct Slot {
    pub name: Rc<str>,
    pub locals: Vec<usize>,
    pub global: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
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
    And,
    Or,
    Not,
    Neg,
    Concat,
    Index,
    /// An array literal: any number of operands.
    Array,
}

/// The kernel's built-in functions; a definition maps surface names onto them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Native {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Return,
    Break,
    Continue,
}

#[derive(Debug)]
pub enum Callee {
    Native(Native, Rc<str>),
    Named(Slot),
    /// A program value applied directly (RPLumen's `eval`).
    Value(Box<Node>),
}

#[derive(Debug)]
pub struct Node {
    pub line: u32,
    pub form: Form,
}

#[derive(Debug)]
pub enum Form {
    /// 1. In order; the value is the last one's.
    Sequence(Vec<Node>),
    /// 2. A body whose new bindings are forgotten afterwards.
    Scope { forget: Vec<Slot>, body: Box<Node> },
    /// 3. Conditional.
    Branch { test: Box<Node>, then: Box<Node>, otherwise: Option<Box<Node>> },
    /// 4. While `test`, run `body`, then `step`, also after a continue.
    Loop { test: Box<Node>, body: Box<Node>, step: Option<Box<Node>> },
    /// 5. Bind a name.
    Assign { to: Slot, value: Box<Node> },
    /// 6. Write an element of the named array.
    AssignIndex { to: Slot, index: Box<Node>, value: Box<Node> },
    /// 7. Call a builtin or a program.
    Call { callee: Callee, args: Vec<Node> },
    /// 8. Apply an operation.
    Operate { op: Op, args: Vec<Node> },
    /// 9. Leave a loop or a program.
    Leave { how: Exit, value: Option<Box<Node>> },
    /// Leaves: a constant (a program among them) and a binding.
    Literal(Value),
    Load(Slot),
}

impl Node {
    pub fn new(line: u32, form: Form) -> Node {
        Node { line, form }
    }

    pub fn seq(line: u32, items: Vec<Node>) -> Node {
        Node { line, form: Form::Sequence(items) }
    }
}

/// A program: the body of a function or of an RPLumen `« ... »`.
#[derive(Debug)]
pub struct Program {
    pub name: String,
    pub params: Vec<String>,
    /// The slot each parameter lands in.
    pub param_slots: Vec<usize>,
    /// Every slot's name.
    pub slot_names: Vec<String>,
    pub body: Node,
}
