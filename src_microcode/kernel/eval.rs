// Runtime value types
//
// Language-agnostic representation of all possible values.
// No language-specific behavior here.

use std::fmt;

/// Runtime value
/// These are the only things that exist at runtime.
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
    Function {
        params: Vec<String>,
        // Body is stored as-is, execution happens in the execute layer
        body_ref: String,  // reference to function registry, not the body itself
    },
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => {
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Null => write!(f, "none"),
            Value::Function { params, body_ref: _ } => {
                write!(f, "<function({})>", params.join(", "))
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < 1e-10,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

impl Value {
    /// Coerce to boolean (language-agnostic rules)
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Function { .. } => true,
        }
    }

    /// Try to coerce to number
    pub fn to_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::Bool(true) => Ok(1.0),
            Value::Bool(false) => Ok(0.0),
            Value::Null => Ok(0.0),
            Value::String(s) => s.parse::<f64>()
                .map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::Function { .. } => Err("Cannot coerce function to number".to_string()),
        }
    }
}
