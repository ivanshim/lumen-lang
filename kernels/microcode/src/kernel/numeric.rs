// Exact numeric tower shared by every operation.
//
// Every numeric value is (numerator, denominator) with a positive
// denominator. Integers have denominator 1, rationals are reduced, and reals
// are rationals that also carry a display precision. Operations promote:
// a real on either side makes the result real (keeping the left real's
// precision when both are real); otherwise integer inputs give integers for
// closed operations and reduced rationals elsewhere.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero};

use crate::kernel::value::Value;
use crate::schema::Op;

#[derive(Clone, Debug)]
pub struct Num {
    pub numerator: BigInt,
    pub denominator: BigInt,
    /// `None` for integers and rationals; `Some(precision)` for reals.
    pub precision: Option<usize>,
    pub is_integer: bool,
}

impl Num {
    fn int(n: BigInt) -> Self {
        Num { numerator: n, denominator: BigInt::from(1), precision: None, is_integer: true }
    }
}

/// View a value as a number, if it is one.
pub fn to_num(value: &Value) -> Option<Num> {
    match value {
        Value::Number(n) => Some(Num::int(n.clone())),
        Value::Rational { numerator, denominator } => Some(Num {
            numerator: numerator.clone(),
            denominator: denominator.clone(),
            precision: None,
            is_integer: false,
        }),
        Value::Real { numerator, denominator, precision } => Some(Num {
            numerator: numerator.clone(),
            denominator: denominator.clone(),
            precision: Some(*precision),
            is_integer: false,
        }),
        _ => None,
    }
}

/// Reduce a fraction to lowest terms with a positive denominator, returning
/// an integer value when the denominator is 1.
pub fn rational(numerator: BigInt, denominator: BigInt) -> Value {
    if numerator.is_zero() {
        return Value::Number(BigInt::zero());
    }
    let (n, d) = normalize(numerator, denominator);
    if d == BigInt::from(1) {
        Value::Number(n)
    } else {
        Value::Rational { numerator: n, denominator: d }
    }
}

/// Reduce a fraction and keep it real with the given precision.
pub fn real(numerator: BigInt, denominator: BigInt, precision: usize) -> Value {
    if numerator.is_zero() {
        return Value::Real { numerator: BigInt::zero(), denominator: BigInt::from(1), precision };
    }
    let (n, d) = normalize(numerator, denominator);
    Value::Real { numerator: n, denominator: d, precision }
}

fn normalize(numerator: BigInt, denominator: BigInt) -> (BigInt, BigInt) {
    let (n, d) = if denominator < BigInt::zero() { (-numerator, -denominator) } else { (numerator, denominator) };
    let g = n.gcd(&d);
    (&n / &g, &d / &g)
}

/// Build a value from a fraction, honouring the promotion rules.
/// Significant digits of a real made without a real operand.
const DEFAULT_PRECISION: usize = 15;

fn from_fraction(numerator: BigInt, denominator: BigInt, precision: Option<usize>) -> Value {
    match precision {
        Some(p) => real(numerator, denominator, p),
        None => rational(numerator, denominator),
    }
}

fn result_precision(a: &Num, b: &Num) -> Option<usize> {
    a.precision.or(b.precision)
}

/// Binary arithmetic on two numbers.
pub fn arith(op: Op, a: &Num, b: &Num) -> Result<Value, String> {
    let precision = result_precision(a, b);
    let zero = BigInt::zero();
    match op {
        Op::Add | Op::Sub | Op::Mul if a.is_integer && b.is_integer && precision.is_none() => {
            let r = match op {
                Op::Add => &a.numerator + &b.numerator,
                Op::Sub => &a.numerator - &b.numerator,
                _ => &a.numerator * &b.numerator,
            };
            Ok(Value::Number(r))
        }
        Op::Add => Ok(from_fraction(
            &a.numerator * &b.denominator + &b.numerator * &a.denominator,
            &a.denominator * &b.denominator,
            precision,
        )),
        Op::Sub => Ok(from_fraction(
            &a.numerator * &b.denominator - &b.numerator * &a.denominator,
            &a.denominator * &b.denominator,
            precision,
        )),
        Op::Mul => Ok(from_fraction(&a.numerator * &b.numerator, &a.denominator * &b.denominator, precision)),
        Op::Div => {
            if b.numerator.is_zero() {
                return Err("Division by zero".to_string());
            }
            Ok(from_fraction(&a.numerator * &b.denominator, &a.denominator * &b.numerator, precision))
        }
        Op::DivReal => {
            // Division whose result is a real, at the operands' precision or the default.
            if b.numerator.is_zero() {
                return Err("Division by zero".to_string());
            }
            let p = precision.unwrap_or(DEFAULT_PRECISION);
            Ok(from_fraction(&a.numerator * &b.denominator, &a.denominator * &b.numerator, Some(p)))
        }
        Op::Quot => {
            // Integer quotient of the exact values, truncating toward zero.
            if b.numerator.is_zero() {
                return Err("Division by zero".to_string());
            }
            let q = (&a.numerator * &b.denominator) / (&b.numerator * &a.denominator);
            Ok(from_fraction(q, BigInt::from(1), precision))
        }
        Op::Rem => {
            // Remainder of the integer parts.
            let ai = &a.numerator / &a.denominator;
            let bi = &b.numerator / &b.denominator;
            if bi == zero {
                return Err("Modulo by zero".to_string());
            }
            Ok(from_fraction(ai % bi, BigInt::from(1), precision))
        }
        Op::Pow => {
            // Base may be any number; the exponent's integer part is used.
            let exp = (&b.numerator / &b.denominator).to_u32().ok_or_else(|| "Exponent too large".to_string())?;
            let n = a.numerator.pow(exp);
            let d = a.denominator.pow(exp);
            Ok(from_fraction(n, d, a.precision))
        }
        _ => Err(format!("{:?} is not an arithmetic operation", op)),
    }
}

/// Exact ordering of two numbers by cross-multiplication.
pub fn compare(a: &Num, b: &Num) -> std::cmp::Ordering {
    (&a.numerator * &b.denominator).cmp(&(&b.numerator * &a.denominator))
}

pub fn negate(value: &Value) -> Result<Value, String> {
    match value {
        Value::Number(n) => Ok(Value::Number(-n)),
        Value::Rational { numerator, denominator } => {
            Ok(Value::Rational { numerator: -numerator, denominator: denominator.clone() })
        }
        Value::Real { numerator, denominator, precision } => {
            Ok(Value::Real { numerator: -numerator, denominator: denominator.clone(), precision: *precision })
        }
        _ => Err("Cannot negate non-numeric value".to_string()),
    }
}
