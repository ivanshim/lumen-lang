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
            // Derived: a - b * (a // b), so the identity a == b * (a // b) + a % b holds.
            let quotient = to_num(&arith(Op::Quot, a, b)?).expect("a number");
            let product = to_num(&arith(Op::Mul, b, &quotient)?).expect("a number");
            arith(Op::Sub, a, &product)
        }
        Op::Pow => {
            // Derived: multiplication by squaring on the exponent's integer
            // part, so a real base keeps its precision through the products.
            let mut exponent = (&b.numerator / &b.denominator).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = a.clone();
            let mut acc = to_num(&from_fraction(BigInt::from(1), BigInt::from(1), a.precision)).expect("a number");
            while exponent > 0 {
                if exponent & 1 == 1 {
                    acc = to_num(&arith(Op::Mul, &acc, &base)?).expect("a number");
                }
                exponent >>= 1;
                if exponent > 0 {
                    base = to_num(&arith(Op::Mul, &base, &base)?).expect("a number");
                }
            }
            Ok(from_fraction(acc.numerator, acc.denominator, acc.precision))
        }
        _ => Err(format!("{:?} is not an arithmetic operation", op)),
    }
}

/// Exact ordering of two numbers by cross-multiplication.
pub fn compare(a: &Num, b: &Num) -> std::cmp::Ordering {
    (&a.numerator * &b.denominator).cmp(&(&b.numerator * &a.denominator))
}

