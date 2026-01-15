// Microcode Kernel: 7 Primitives (semantic normal form)
//
// These are the ONLY semantic operations in the kernel.
// All language-specific meaning is pushed to schema and value layers.
//
// Primitives represent minimal, canonical forms:
// 1. Sequence - execute instructions in order
// 2. Scope - push/pop binding context
// 3. Branch - conditional execution
// 4. Assign - bind/mutate a name
// 5. Invoke - call external function
// 6. Operate - dispatch unary/binary operator
// 7. Transfer - control flow (return/break/continue)
//
// Each primitive is stateless data. Semantics come from:
// - Instruction structure (the "what")
// - Schema tables (the "how")
// - Value types (the "with what")
// - Environment (the "in what context")

use crate::kernel::eval::Value;

/// Control transfer kinds (for Transfer primitive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Return,
    Break,
    Continue,
}

/// Operator kinds (for Operate primitive)
#[derive(Debug, Clone)]
pub enum OperateKind {
    Unary(String),   // operator name
    Binary(String),  // operator name
}

/// Instruction: One node in the semantic normal form.
/// Each instruction is one of 7 primitives, nothing more.
#[derive(Debug, Clone)]
pub enum Instruction {
    // 1. Sequence: execute Vec<Instruction> in order, return last value
    Sequence(Vec<Instruction>),

    // 2. Scope: push scope, execute instruction, pop scope
    Scope(Box<Instruction>),

    // 3. Branch: if cond then_instr else else_instr
    Branch {
        condition: Box<Instruction>,
        then_instr: Box<Instruction>,
        else_instr: Option<Box<Instruction>>,
    },

    // 4. Assign: name = value_instr
    Assign {
        name: String,
        value: Box<Instruction>,
    },

    // 5. Invoke: call external function
    //    All actual semantics come from the schema and external registry
    Invoke {
        function: String,  // fully qualified function name
        args: Vec<Instruction>,
    },

    // 6. Operate: apply operator to operands
    //    Operator semantics defined in schema
    Operate {
        kind: OperateKind,
        operands: Vec<Instruction>,
    },

    // 7. Transfer: control flow signal
    Transfer {
        kind: TransferKind,
        value: Option<Box<Instruction>>,
    },

    // Literals: not a "primitive" but necessary
    // (represents final values before evaluation)
    Literal(Value),

    // Variable reference: look up name in environment
    Variable(String),

    // Loop: while condition { body }
    Loop {
        condition: Box<Instruction>,
        body: Box<Instruction>,
    },

    // ForLoop: for var in iterable { body }
    ForLoop {
        var: String,
        iterable: Box<Instruction>,
        body: Box<Instruction>,
    },

    // UntilLoop: until condition { body } (do-until: execute body first, then check condition)
    UntilLoop {
        condition: Box<Instruction>,
        body: Box<Instruction>,
    },

    // Function definition: store in registry
    // (This is metadata, not execution)
    // memoizable: true only if explicitly marked by language semantics
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Box<Instruction>,
        memoizable: bool,
    },
}

impl Instruction {
    /// Helper: sequence of instructions
    pub fn sequence(instrs: Vec<Instruction>) -> Self {
        Instruction::Sequence(instrs)
    }

    /// Helper: literal value
    pub fn literal(value: Value) -> Self {
        Instruction::Literal(value)
    }

    /// Helper: variable reference
    pub fn variable(name: String) -> Self {
        Instruction::Variable(name)
    }

    /// Helper: assignment
    pub fn assign(name: String, value: Instruction) -> Self {
        Instruction::Assign {
            name,
            value: Box::new(value),
        }
    }

    /// Helper: binary operation
    pub fn binary(op: String, left: Instruction, right: Instruction) -> Self {
        Instruction::Operate {
            kind: OperateKind::Binary(op),
            operands: vec![left, right],
        }
    }

    /// Helper: unary operation
    pub fn unary(op: String, operand: Instruction) -> Self {
        Instruction::Operate {
            kind: OperateKind::Unary(op),
            operands: vec![operand],
        }
    }

    /// Helper: function call
    pub fn invoke(function: String, args: Vec<Instruction>) -> Self {
        Instruction::Invoke { function, args }
    }

    /// Helper: if-then-else
    pub fn branch(
        condition: Instruction,
        then_instr: Instruction,
        else_instr: Option<Instruction>,
    ) -> Self {
        Instruction::Branch {
            condition: Box::new(condition),
            then_instr: Box::new(then_instr),
            else_instr: else_instr.map(Box::new),
        }
    }

    /// Helper: return statement
    pub fn return_stmt(value: Option<Instruction>) -> Self {
        Instruction::Transfer {
            kind: TransferKind::Return,
            value: value.map(Box::new),
        }
    }

    /// Helper: break statement
    pub fn break_stmt() -> Self {
        Instruction::Transfer {
            kind: TransferKind::Break,
            value: None,
        }
    }

    /// Helper: continue statement
    pub fn continue_stmt() -> Self {
        Instruction::Transfer {
            kind: TransferKind::Continue,
            value: None,
        }
    }

    /// Helper: loop
    pub fn loop_stmt(condition: Instruction, body: Instruction) -> Self {
        Instruction::Loop {
            condition: Box::new(condition),
            body: Box::new(body),
        }
    }

    /// Helper: for loop
    pub fn for_loop(var: String, iterable: Instruction, body: Instruction) -> Self {
        Instruction::ForLoop {
            var,
            iterable: Box::new(iterable),
            body: Box::new(body),
        }
    }

    /// Helper: until loop
    pub fn until_loop(condition: Instruction, body: Instruction) -> Self {
        Instruction::UntilLoop {
            condition: Box::new(condition),
            body: Box::new(body),
        }
    }

    /// Helper: scope push
    pub fn scope(instr: Instruction) -> Self {
        Instruction::Scope(Box::new(instr))
    }

    /// Helper: construct array from elements
    pub fn construct_array(elements: Vec<Instruction>) -> Self {
        Instruction::Invoke {
            function: "__construct_array".to_string(),
            args: elements,
        }
    }
}
