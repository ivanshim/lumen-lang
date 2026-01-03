// Value types and coercion
//
// Language-agnostic value representation.
// All values are opaque until coerced for specific operations.

use std::fmt;

/// Runtime value type (language-agnostic)
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => {
                // Format numbers sensibly
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            Value::Null => write!(f, "null"),
        }
    }
}

impl Value {
    /// Coerce to number (language-agnostic rules)
    pub fn to_number(&self) -> Result<f64, String> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::Bool(true) => Ok(1.0),
            Value::Bool(false) => Ok(0.0),
            Value::String(s) => {
                s.parse::<f64>()
                    .map_err(|_| format!("Cannot coerce '{}' to number", s))
            }
            Value::Null => Ok(0.0),
        }
    }

    /// Coerce to boolean
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Null => false,
        }
    }

    /// Coerce to string
    pub fn to_string_value(&self) -> String {
        self.to_string()
    }

    /// Test equality (language-agnostic)
    pub fn equals(&self, other: &Value) -> Result<bool, String> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok((a - b).abs() < f64::EPSILON),
            (Value::String(a), Value::String(b)) => Ok(a == b),
            (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
            (Value::Null, Value::Null) => Ok(true),
            // Cross-type coercion for comparison
            (Value::Number(_), _) | (_, Value::Number(_)) => {
                let a = self.to_number()?;
                let b = other.to_number()?;
                Ok((a - b).abs() < f64::EPSILON)
            }
            (Value::Bool(_), _) | (_, Value::Bool(_)) => {
                Ok(self.to_bool() == other.to_bool())
            }
            _ => Ok(false),
        }
    }
}
