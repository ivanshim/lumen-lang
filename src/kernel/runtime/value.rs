// src/kernel/runtime/value.rs
//
// Runtime Value representation - kernel level.
// This is NOT a feature file. It must stay small and stable.
// Features may *use* Value, but should not expand it casually.
//
// NOTE: Value is a generic container for language-specific values.
// The kernel does NOT interpret numeric values - it only transports them.
// Language modules are responsible for:
// - Creating values (including numeric conversion)
// - Interpreting values (arithmetic, comparisons, operations)

use std::fmt;

#[derive(Clone, PartialEq)]
pub enum Value {
    Number(String),    // Raw string representation - kernel is agnostic to numeric type
    Bool(bool),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "Number(\"{}\")", n),
            Value::Bool(b) => write!(f, "Bool({})", b),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        }
    }
}
