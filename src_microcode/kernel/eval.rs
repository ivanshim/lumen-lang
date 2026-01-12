// Runtime value types
//
// Language-agnostic representation of all possible values.
// No language-specific behavior here.

use std::fmt;
use num_bigint::BigInt;
use num_traits::Signed;

/// Runtime value
/// These are the only things that exist at runtime.
#[derive(Debug, Clone)]
pub enum Value {
    Number(BigInt),
    Rational {
        numerator: BigInt,
        denominator: BigInt,
    },
    Real {
        numerator: BigInt,
        denominator: BigInt,
        precision: usize, // Number of significant digits
    },
    String(String),
    Bool(bool),
    Null,
    Range {
        start: BigInt,
        end: BigInt,
    },
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
                write!(f, "{}", n)
            }
            Value::Rational { numerator, denominator } => {
                // If denominator is 1, display as integer
                if denominator == &BigInt::from(1) {
                    write!(f, "{}", numerator)
                } else {
                    write!(f, "{}/{}", numerator, denominator)
                }
            }
            Value::Real { numerator, denominator, precision } => {
                // For real, display the decimal representation with significant digit precision
                let int_part = numerator / denominator;
                let remainder = numerator.clone() - (&int_part * denominator);
                if remainder == BigInt::from(0) {
                    write!(f, "{}", int_part)
                } else {
                    // Compute decimal places needed for precision
                    let mut decimal_str = String::new();
                    let digit_count = int_part.to_string().len();
                    let target_digits = *precision;
                    let mut rem = remainder.abs();

                    // If int part has fewer digits than target, include fractional digits
                    let mut frac_digits = if digit_count >= target_digits {
                        0
                    } else {
                        target_digits - digit_count
                    };

                    // Compute fractional part
                    let denom = denominator.clone();
                    while frac_digits > 0 && rem > BigInt::from(0) {
                        rem = rem * BigInt::from(10);
                        let digit = &rem / &denom;
                        decimal_str.push_str(&digit.to_string());
                        rem = &rem - (&digit * &denom);
                        frac_digits -= 1;
                    }

                    write!(f, "{}.{}", int_part, decimal_str)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Null => write!(f, "none"),
            Value::Range { start, end } => {
                write!(f, "{}..{}", start, end)
            }
            Value::Function { params, body_ref: _ } => {
                write!(f, "<function({})>", params.join(", "))
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Rational { numerator: a_num, denominator: a_denom }, Value::Rational { numerator: b_num, denominator: b_denom }) => {
                // Cross-multiply: a/b == c/d ⟺ ad == bc
                a_num * b_denom == b_num * a_denom
            }
            (Value::Real { numerator: a_num, denominator: a_denom, .. }, Value::Real { numerator: b_num, denominator: b_denom, .. }) => {
                // Compare exact rational values
                a_num * b_denom == b_num * a_denom
            }
            // Allow comparison between real and rational/number
            (Value::Real { numerator: r_num, denominator: r_denom, .. }, Value::Rational { numerator: q_num, denominator: q_denom }) |
            (Value::Rational { numerator: q_num, denominator: q_denom }, Value::Real { numerator: r_num, denominator: r_denom, .. }) => {
                r_num * q_denom == q_num * r_denom
            }
            (Value::Real { numerator: r_num, denominator: r_denom, .. }, Value::Number(n)) |
            (Value::Number(n), Value::Real { numerator: r_num, denominator: r_denom, .. }) => {
                r_num == n && r_denom == &BigInt::from(1)
            }
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Range { start: a_start, end: a_end }, Value::Range { start: b_start, end: b_end }) => {
                a_start == b_start && a_end == b_end
            }
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
            Value::Number(n) => n != &BigInt::from(0),
            Value::Rational { numerator, .. } => numerator != &BigInt::from(0),
            Value::Real { numerator, .. } => numerator != &BigInt::from(0),
            Value::String(s) => !s.is_empty(),
            Value::Range { .. } => true,
            Value::Function { .. } => true,
        }
    }

    /// Try to coerce to number
    pub fn to_number(&self) -> Result<BigInt, String> {
        match self {
            Value::Number(n) => Ok(n.clone()),
            Value::Rational { .. } => Err("Cannot coerce rational to integer".to_string()),
            Value::Real { numerator, denominator, .. } => {
                // Truncate toward zero: integer division
                Ok(numerator / denominator)
            }
            Value::Bool(true) => Ok(BigInt::from(1)),
            Value::Bool(false) => Ok(BigInt::from(0)),
            Value::Null => Ok(BigInt::from(0)),
            Value::String(s) => s.parse::<BigInt>()
                .map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::Range { .. } => Err("Cannot coerce range to number".to_string()),
            Value::Function { .. } => Err("Cannot coerce function to number".to_string()),
        }
    }
}
