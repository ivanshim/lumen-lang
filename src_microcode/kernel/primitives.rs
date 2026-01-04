// Closed set of execution primitives
//
// These are the ONLY operations the kernel knows how to execute.
// All language behavior is expressed as compositions of these primitives.
// Each primitive is data-driven by schema mappings.
//
// Canonical primitive set (single-word verbs):
// - Sequence: execute instructions in order
// - Scope: create new scope, execute, pop scope
// - Branch: if condition → then-block else else-block
// - Assign: variable = expression
// - Invoke: call external function
// - Operate: apply unary or binary operator
// - Transfer: return/break/continue control flow

use super::eval::Value;

/// Closed set of kernel primitives
#[derive(Debug, Clone)]
pub enum Primitive {
    /// sequence: execute instructions in order
    Sequence(Vec<Instruction>),

    /// scope: create new scope, execute, pop scope
    Scope(Vec<Instruction>),

    /// branch: if condition → then-block else else-block
    Branch {
        condition: Box<Instruction>,
        then_block: Box<Instruction>,
        else_block: Option<Box<Instruction>>,
    },

    /// assign: variable = expression
    Assign {
        name: String,
        value: Box<Instruction>,
    },

    /// invoke: call external function
    Invoke {
        selector: String,
        args: Vec<Instruction>,
    },

    /// operate: apply unary or binary operator
    Operate {
        kind: OperateKind,
        operands: Vec<Instruction>,
    },

    /// transfer: return, break, or continue
    Transfer {
        kind: TransferKind,
        value: Option<Box<Instruction>>,
    },

    /// literal: constant value
    Literal(Value),

    /// variable: load variable value
    Variable(String),

    /// loop: internal implementation detail for while/loop constructs
    /// (not part of canonical primitive set; kept for internal use during refactoring)
    Loop {
        condition: Box<Instruction>,
        block: Box<Instruction>,
    },
}

/// Type of operation for Operate primitive
#[derive(Debug, Clone)]
pub enum OperateKind {
    /// Unary operator (operator name, operand)
    Unary(String),
    /// Binary operator (operator name, left, right)
    Binary(String),
}

/// Type of transfer for Transfer primitive
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferKind {
    /// Return from function with optional value
    Return,
    /// Break from loop
    Break,
    /// Continue to next loop iteration
    Continue,
}

/// Instruction tree node (wraps primitive with metadata)
#[derive(Debug, Clone)]
pub struct Instruction {
    pub primitive: Primitive,
    pub span: (usize, usize), // byte range in source for error reporting
}

impl Instruction {
    pub fn new(primitive: Primitive, start: usize, end: usize) -> Self {
        Self {
            primitive,
            span: (start, end),
        }
    }

    /// Helper: sequence of instructions
    pub fn sequence(instrs: Vec<Instruction>) -> Self {
        let start = instrs.first().map(|i| i.span.0).unwrap_or(0);
        let end = instrs.last().map(|i| i.span.1).unwrap_or(0);
        Self::new(Primitive::Sequence(instrs), start, end)
    }

    /// Helper: scope with new variable bindings
    pub fn scope(instrs: Vec<Instruction>) -> Self {
        let start = instrs.first().map(|i| i.span.0).unwrap_or(0);
        let end = instrs.last().map(|i| i.span.1).unwrap_or(0);
        Self::new(Primitive::Scope(instrs), start, end)
    }

    /// Helper: literal value
    pub fn literal(value: Value, start: usize, end: usize) -> Self {
        Self::new(Primitive::Literal(value), start, end)
    }

    /// Helper: variable reference
    pub fn variable(name: String, start: usize, end: usize) -> Self {
        Self::new(Primitive::Variable(name), start, end)
    }
}
