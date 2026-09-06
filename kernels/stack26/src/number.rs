// The numeric tower: integers, exact fractions and reals.
//
// Two machine-word integers take the fast path through checked native
// arithmetic; anything else is computed on big fractions. A real on either
// side makes the result real, at the left real's precision. The floor is
// `+ - * // /`, equality and less-than; remainder, power, negation and the
// other comparisons are derived from those at the call sites.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::value::{Fraction, RealNumber, Value};

/// Significant digits of a real that no real operand gave a precision.
pub const DEFAULT_DIGITS: usize = 15;

/// A number viewed as a fraction, with the precision it carries if real.
pub struct Number {
    top: BigInt,
    bottom: BigInt,
    digits: Option<usize>,
    whole: bool,
}

impl Number {
    pub fn of(value: &Value) -> Option<Number> {
        Some(match value {
            Value::Int(n) => Number { top: BigInt::from(*n), bottom: BigInt::one(), digits: None, whole: true },
            Value::Big(n) => Number { top: (**n).clone(), bottom: BigInt::one(), digits: None, whole: true },
            Value::Ratio(f) => Number { top: f.top.clone(), bottom: f.bottom.clone(), digits: None, whole: false },
            Value::Real(r) => Number { top: r.top.clone(), bottom: r.bottom.clone(), digits: Some(r.digits), whole: false },
            _ => return None,
        })
    }

    pub fn cmp(&self, other: &Number) -> Ordering {
        (&self.top * &other.bottom).cmp(&(&other.top * &self.bottom))
    }

    fn value(self) -> Value {
        make(self.top, self.bottom, self.digits)
    }
}

/// Lowest terms with a positive denominator.
fn reduce(top: BigInt, bottom: BigInt) -> (BigInt, BigInt) {
    let (top, bottom) = if bottom.is_negative() { (-top, -bottom) } else { (top, bottom) };
    let g = top.gcd(&bottom);
    if g.is_one() {
        (top, bottom)
    } else {
        (&top / &g, &bottom / &g)
    }
}

/// A value from a fraction: an integer when it divides out and no real was
/// involved, a fraction otherwise, or a real at the given precision.
pub fn make(top: BigInt, bottom: BigInt, digits: Option<usize>) -> Value {
    match digits {
        Some(digits) => {
            if top.is_zero() {
                return Value::Real(Rc::new(RealNumber { top, bottom: BigInt::one(), digits }));
            }
            let (top, bottom) = reduce(top, bottom);
            Value::Real(Rc::new(RealNumber { top, bottom, digits }))
        }
        None => {
            if top.is_zero() {
                return Value::Int(0);
            }
            let (top, bottom) = reduce(top, bottom);
            if bottom.is_one() {
                Value::integer(top)
            } else {
                Value::Ratio(Rc::new(Fraction { top, bottom }))
            }
        }
    }
}

/// A real from any number, keeping its value and taking the given precision.
pub fn to_real(value: &Value, digits: usize) -> Option<Value> {
    let n = Number::of(value)?;
    Some(make(n.top, n.bottom, Some(digits)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arith {
    Add,
    Sub,
    Mul,
    Div,
    RealDiv,
    Quot,
    Rem,
    Pow,
}

fn small(value: &Value) -> Option<i64> {
    match value {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}

/// Arithmetic on two numbers, or None when either is not a number.
pub fn arith(op: Arith, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if let (Some(x), Some(y)) = (small(a), small(b)) {
        let fast = match op {
            Arith::Add => x.checked_add(y),
            Arith::Sub => x.checked_sub(y),
            Arith::Mul => x.checked_mul(y),
            Arith::Quot if y != 0 => x.checked_div(y),
            Arith::Rem if y != 0 => x.checked_div(y).and_then(|q| q.checked_mul(y)).and_then(|p| x.checked_sub(p)),
            _ => None,
        };
        if let Some(r) = fast {
            return Some(Ok(Value::Int(r)));
        }
    }
    let a = Number::of(a)?;
    let b = Number::of(b)?;
    Some(slow(op, &a, &b))
}

fn slow(op: Arith, a: &Number, b: &Number) -> Result<Value, String> {
    let digits = a.digits.or(b.digits);
    let zero_divisor = || "Division by zero".to_string();
    Ok(match op {
        Arith::Add | Arith::Sub if a.whole && b.whole => {
            let r = if op == Arith::Add { &a.top + &b.top } else { &a.top - &b.top };
            Value::integer(r)
        }
        Arith::Mul if a.whole && b.whole => Value::integer(&a.top * &b.top),
        Arith::Add => make(&a.top * &b.bottom + &b.top * &a.bottom, &a.bottom * &b.bottom, digits),
        Arith::Sub => make(&a.top * &b.bottom - &b.top * &a.bottom, &a.bottom * &b.bottom, digits),
        Arith::Mul => make(&a.top * &b.top, &a.bottom * &b.bottom, digits),
        Arith::Div => {
            if b.top.is_zero() {
                return Err(zero_divisor());
            }
            make(&a.top * &b.bottom, &a.bottom * &b.top, digits)
        }
        Arith::RealDiv => {
            if b.top.is_zero() {
                return Err(zero_divisor());
            }
            make(&a.top * &b.bottom, &a.bottom * &b.top, Some(digits.unwrap_or(DEFAULT_DIGITS)))
        }
        Arith::Quot => {
            // The integer quotient of the exact values, toward zero.
            if b.top.is_zero() {
                return Err(zero_divisor());
            }
            make((&a.top * &b.bottom) / (&b.top * &a.bottom), BigInt::one(), digits)
        }
        Arith::Rem => {
            // Derived: a - b * (a // b).
            let q = Number::of(&slow(Arith::Quot, a, b)?).expect("number");
            let p = Number::of(&slow(Arith::Mul, b, &q)?).expect("number");
            slow(Arith::Sub, a, &p)?
        }
        Arith::Pow => {
            // Derived: squaring on the exponent's integer part.
            let mut e = (&b.top / &b.bottom).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = Number { top: a.top.clone(), bottom: a.bottom.clone(), digits: a.digits, whole: a.whole };
            let mut acc = Number { top: BigInt::one(), bottom: BigInt::one(), digits: a.digits, whole: a.digits.is_none() };
            while e > 0 {
                if e & 1 == 1 {
                    acc = Number::of(&slow(Arith::Mul, &acc, &base)?).expect("number");
                }
                e >>= 1;
                if e > 0 {
                    base = Number::of(&slow(Arith::Mul, &base, &base)?).expect("number");
                }
            }
            acc.value()
        }
    })
}

/// Less-than on two numbers, or None when either is not a number.
pub fn less(a: &Value, b: &Value) -> Option<bool> {
    if let (Some(x), Some(y)) = (small(a), small(b)) {
        return Some(x < y);
    }
    Some(Number::of(a)?.cmp(&Number::of(b)?) == Ordering::Less)
}

/// Numerator and denominator of any number: an integer is over one.
pub fn parts(value: &Value) -> Option<(BigInt, BigInt)> {
    let n = Number::of(value)?;
    Some((n.top, n.bottom))
}

/// The precision a real carries.
pub fn digits_of(value: &Value) -> Option<usize> {
    match value {
        Value::Real(r) => Some(r.digits),
        _ => None,
    }
}
