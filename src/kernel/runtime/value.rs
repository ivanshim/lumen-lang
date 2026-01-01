// src/runtime/value.rs
//
// Runtime Value representation.
// This is NOT a feature file. It must stay small and stable.
// Features may *use* Value, but should not expand it casually.

use std::fmt;

#[derive(Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "Number({})", n),
            Value::Bool(b) => write!(f, "Bool({})", b),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => {
                // Print integers cleanly when possible (nice UX, stable semantics).
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
        }
    }
}
