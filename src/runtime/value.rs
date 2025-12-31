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
    #[allow(dead_code)]
    String(String),
    #[allow(dead_code)]
    Null,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Null => false,
        }
    }

    #[allow(dead_code)]
    pub fn as_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(format!("Expected number, got {}", self.type_name())),
        }
    }

    #[allow(dead_code)]
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(format!("Expected boolean, got {}", self.type_name())),
        }
    }

    #[allow(dead_code)]
    pub fn as_string(&self) -> Result<&str, String> {
        match self {
            Value::String(s) => Ok(s.as_str()),
            _ => Err(format!("Expected string, got {}", self.type_name())),
        }
    }

    #[allow(dead_code)]
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::String(_) => "string",
            Value::Null => "null",
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "Number({})", n),
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Null => write!(f, "Null"),
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
            Value::String(s) => write!(f, "{}", s),
            Value::Null => write!(f, "null"),
        }
    }
}
