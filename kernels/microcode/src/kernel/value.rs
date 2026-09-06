// Runtime values.
//
// The kernel's value model is closed: integers, exact rationals, reals
// (exact rationals carrying a display precision), strings, booleans, null,
// ranges, arrays, function values and kind meta-values.

use std::fmt;
use std::rc::Rc;

use num_bigint::BigInt;
use num_traits::Signed;

use crate::kernel::instruction::Instruction;

/// Kind meta-values: the closed set of value categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindValue {
    Integer,
    Rational,
    Real,
    String,
    Boolean,
    Array,
    Null,
}

impl KindValue {
    pub fn name(self) -> &'static str {
        match self {
            KindValue::Integer => "INTEGER",
            KindValue::Rational => "RATIONAL",
            KindValue::Real => "REAL",
            KindValue::String => "STRING",
            KindValue::Boolean => "BOOLEAN",
            KindValue::Array => "ARRAY",
            KindValue::Null => "NULL",
        }
    }
}

/// The words a language prints for true, false and null.
pub struct LiteralWords<'a> {
    pub true_word: &'a str,
    pub false_word: &'a str,
    pub null_word: &'a str,
}

/// A user-defined function: parameters and body, shared between the binding
/// that names it and every call frame.
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Instruction,
}

#[derive(Debug, Clone)]
pub enum Value {
    Number(BigInt),
    Rational { numerator: BigInt, denominator: BigInt },
    Real { numerator: BigInt, denominator: BigInt, precision: usize },
    String(String),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Function(Rc<Function>),
    Kind(KindValue),
}

impl Value {
    pub fn kind(&self) -> Option<KindValue> {
        Some(match self {
            Value::Number(_) => KindValue::Integer,
            Value::Rational { .. } => KindValue::Rational,
            Value::Real { .. } => KindValue::Real,
            Value::String(_) => KindValue::String,
            Value::Bool(_) => KindValue::Boolean,
            Value::Array(_) => KindValue::Array,
            Value::Null => KindValue::Null,
            Value::Kind(_) => KindValue::Null,
            Value::Function(_) => return None,
        })
    }

    /// Truthiness.
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => n != &BigInt::from(0),
            Value::Rational { numerator, .. } | Value::Real { numerator, .. } => numerator != &BigInt::from(0),
            Value::String(s) => !s.is_empty(),
            Value::Array(_) | Value::Function(_) | Value::Kind(_) => true,
        }
    }

    /// Coerce to an integer where the value has an obvious one.
    pub fn to_number(&self) -> Result<BigInt, String> {
        match self {
            Value::Number(n) => Ok(n.clone()),
            Value::Rational { .. } => Err("Cannot coerce rational to integer".to_string()),
            Value::Real { numerator, denominator, .. } => Ok(numerator / denominator),
            Value::Bool(true) => Ok(BigInt::from(1)),
            Value::Bool(false) => Ok(BigInt::from(0)),
            Value::Null => Ok(BigInt::from(0)),
            Value::String(s) => s.parse::<BigInt>().map_err(|_| format!("Cannot coerce '{}' to number", s)),
            Value::Array(_) => Err("Cannot coerce array to number".to_string()),
            Value::Function(_) => Err("Cannot coerce function to number".to_string()),
            Value::Kind(_) => Err("Cannot coerce kind meta-value to number".to_string()),
        }
    }

    /// Text for a value as the language spells it: booleans and null use
    /// the given words, arrays render their elements the same way, and
    /// everything else renders as `Display` does.
    pub fn render(&self, words: &LiteralWords) -> String {
        match self {
            Value::Bool(true) => words.true_word.to_string(),
            Value::Bool(false) => words.false_word.to_string(),
            Value::Null => words.null_word.to_string(),
            Value::Array(elements) => {
                let inner: Vec<String> = elements.iter().map(|e| e.render(words)).collect();
                format!("[{}]", inner.join(", "))
            }
            other => other.to_string(),
        }
    }

    /// Decimal rendering of a real to its precision in significant digits.
    pub fn real_to_decimal(numerator: &BigInt, denominator: &BigInt, precision: usize) -> String {
        let int_part = numerator / denominator;
        let remainder = numerator - (&int_part * denominator);
        if remainder == BigInt::from(0) {
            return int_part.to_string();
        }
        // Significant digits: the sign is not one of them.
        let digit_count = int_part.to_string().trim_start_matches('-').len();
        let mut frac_digits = precision.saturating_sub(digit_count);
        let mut rem = remainder.abs();
        let mut decimal = String::new();
        while frac_digits > 0 && rem > BigInt::from(0) {
            rem *= BigInt::from(10);
            let digit = &rem / denominator;
            decimal.push_str(&digit.to_string());
            rem -= &digit * denominator;
            frac_digits -= 1;
        }
        let sign = if numerator < &BigInt::from(0) && int_part == BigInt::from(0) { "-" } else { "" };
        format!("{}{}.{}", sign, int_part, decimal)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Rational { numerator, denominator } => {
                if denominator == &BigInt::from(1) {
                    write!(f, "{}", numerator)
                } else {
                    write!(f, "{}/{}", numerator, denominator)
                }
            }
            Value::Real { numerator, denominator, precision } => {
                write!(f, "{}", Value::real_to_decimal(numerator, denominator, *precision))
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            Value::Null => write!(f, "null"),
            Value::Array(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Value::Function(def) => write!(f, "<function({})>", def.params.join(", ")),
            Value::Kind(k) => write!(f, "{}", k.name()),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Number(a), Number(b)) => a == b,
            (Rational { numerator: an, denominator: ad }, Rational { numerator: bn, denominator: bd })
            | (Real { numerator: an, denominator: ad, .. }, Real { numerator: bn, denominator: bd, .. })
            | (Real { numerator: an, denominator: ad, .. }, Rational { numerator: bn, denominator: bd })
            | (Rational { numerator: an, denominator: ad }, Real { numerator: bn, denominator: bd, .. }) => {
                an * bd == bn * ad
            }
            (Real { numerator, denominator, .. }, Number(n)) | (Number(n), Real { numerator, denominator, .. }) => {
                numerator == n && denominator == &BigInt::from(1)
            }
            (Rational { numerator, denominator }, Number(n)) | (Number(n), Rational { numerator, denominator }) => {
                numerator == n && denominator == &BigInt::from(1)
            }
            (String(a), String(b)) => a == b,
            (Bool(a), Bool(b)) => a == b,
            (Null, Null) => true,
            (Array(a), Array(b)) => a == b,
            (Function(a), Function(b)) => Rc::ptr_eq(a, b),
            (Kind(a), Kind(b)) => a == b,
            _ => false,
        }
    }
}
