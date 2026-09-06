// Instructions: the kernel's semantic normal form.
//
// Seven primitives carry all control and data flow. `Literal` and
// `Variable` are the leaves they operate on. Every source construct is
// reduced to these; `for` and `until` loops, function definitions and
// indexed assignment are desugared onto them rather than added to them.

use std::fmt;

use crate::kernel::value::Value;
use crate::schema::Op;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Return,
    Break,
    Continue,
}

/// Where an assignment writes.
#[derive(Debug, Clone)]
pub enum Target {
    Name(String),
    Index { name: String, index: Box<Instruction> },
}

#[derive(Clone)]
pub enum Instruction {
    /// 1. Execute in order; the value is the last one.
    Sequence(Vec<Instruction>),
    /// 2. Execute inside a fresh binding scope.
    Scope(Box<Instruction>),
    /// 3. Conditional execution.
    Branch {
        condition: Box<Instruction>,
        then_branch: Box<Instruction>,
        else_branch: Option<Box<Instruction>>,
    },
    /// 4. Bind or mutate.
    Assign { target: Target, value: Box<Instruction> },
    /// 5. Call a built-in (per the schema's function table) or a function value.
    Invoke { function: String, args: Vec<Instruction> },
    /// 6. Apply a kernel operation to operands.
    Operate { op: Op, operands: Vec<Instruction> },
    /// 7. Control transfer: return, break, continue.
    Transfer { kind: TransferKind, value: Option<Box<Instruction>> },
    /// Iteration: while `condition`, run `body`, then `step` (even after continue).
    Loop {
        condition: Box<Instruction>,
        body: Box<Instruction>,
        step: Option<Box<Instruction>>,
    },
    /// A constant.
    Literal(Value),
    /// A binding lookup.
    Variable(String),
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Sequence(items) => f.debug_tuple("Sequence").field(items).finish(),
            Instruction::Scope(inner) => f.debug_tuple("Scope").field(inner).finish(),
            Instruction::Branch { condition, then_branch, else_branch } => f
                .debug_struct("Branch")
                .field("condition", condition)
                .field("then", then_branch)
                .field("else", else_branch)
                .finish(),
            Instruction::Assign { target, value } => {
                f.debug_struct("Assign").field("target", target).field("value", value).finish()
            }
            Instruction::Invoke { function, args } => {
                f.debug_struct("Invoke").field("function", function).field("args", args).finish()
            }
            Instruction::Operate { op, operands } => {
                f.debug_struct("Operate").field("op", op).field("operands", operands).finish()
            }
            Instruction::Transfer { kind, value } => {
                f.debug_struct("Transfer").field("kind", kind).field("value", value).finish()
            }
            Instruction::Loop { condition, body, step } => f
                .debug_struct("Loop")
                .field("condition", condition)
                .field("body", body)
                .field("step", step)
                .finish(),
            Instruction::Literal(v) => f.debug_tuple("Literal").field(v).finish(),
            Instruction::Variable(name) => f.debug_tuple("Variable").field(name).finish(),
        }
    }
}

impl Instruction {
    pub fn binary(op: Op, left: Instruction, right: Instruction) -> Self {
        Instruction::Operate { op, operands: vec![left, right] }
    }

    pub fn unary(op: Op, operand: Instruction) -> Self {
        Instruction::Operate { op, operands: vec![operand] }
    }

    pub fn assign(name: String, value: Instruction) -> Self {
        Instruction::Assign { target: Target::Name(name), value: Box::new(value) }
    }

    pub fn transfer(kind: TransferKind, value: Option<Instruction>) -> Self {
        Instruction::Transfer { kind, value: value.map(Box::new) }
    }
}
