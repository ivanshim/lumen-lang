// Closed set of execution primitives
//
// These are the ONLY operations the kernel knows how to execute.
// All language behavior is expressed as compositions of these primitives.
// Each primitive is data-driven by schema mappings.

use super::eval::Value;
use std::collections::HashMap;

/// Closed set of kernel primitives
#[derive(Debug, Clone)]
pub enum Primitive {
    /// sequence: execute instructions in order
    Sequence(Vec<Instruction>),

    /// block: create new scope, execute, pop scope
    Block(Vec<Instruction>),

    /// conditional: if condition → then-block else else-block
    Conditional {
        condition: Box<Instruction>,
        then_block: Box<Instruction>,
        else_block: Option<Box<Instruction>>,
    },

    /// loop: repeat block while condition is true
    Loop {
        condition: Box<Instruction>,
        block: Box<Instruction>,
    },

    /// jump: break or continue (used inside loops)
    Jump(JumpKind),

    /// assign: variable = expression
    Assign {
        name: String,
        value: Box<Instruction>,
    },

    /// call: invoke extern function
    Call {
        selector: String,
        args: Vec<Instruction>,
    },

    /// unary_op: apply unary operator
    UnaryOp {
        operator: String,
        operand: Box<Instruction>,
    },

    /// binary_op: apply binary operator
    BinaryOp {
        operator: String,
        left: Box<Instruction>,
        right: Box<Instruction>,
    },

    /// literal: constant value
    Literal(Value),

    /// variable: load variable value
    Variable(String),

    /// print: output value (special case for now)
    Print(Box<Instruction>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JumpKind {
    Break,
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

    /// Helper: block with scope
    pub fn block(instrs: Vec<Instruction>) -> Self {
        let start = instrs.first().map(|i| i.span.0).unwrap_or(0);
        let end = instrs.last().map(|i| i.span.1).unwrap_or(0);
        Self::new(Primitive::Block(instrs), start, end)
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
