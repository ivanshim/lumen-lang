// Arithmetic over the numeric tower.
//
// The floor is `+ - * // /`, equality and less-than on exact fractions;
// remainder, power, negation and the remaining comparisons are derived by
// the callers. Two small integers take native checked arithmetic first.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::value::{Ratio, Value};

pub const DEFAULT_DIGITS: usize = 15;

/// Any number as a ratio.
pub fn ratio(v: &Value) -> Option<Ratio> {
    Some(match v {
        Value::Small(n) => Ratio { num: BigInt::from(*n), den: BigInt::one(), digits: None },
        Value::Large(n) => Ratio { num: (**n).clone(), den: BigInt::one(), digits: None },
        Value::Fraction(f) => (**f).clone(),
        _ => return None,
    })
}

fn lowest_terms(num: BigInt, den: BigInt) -> (BigInt, BigInt) {
    let (num, den) = if den.is_negative() { (-num, -den) } else { (num, den) };
    let g = num.gcd(&den);
    if g.is_one() { (num, den) } else { (&num / &g, &den / &g) }
}

/// A value from a ratio, integer when it can be and no real was involved.
pub fn value(num: BigInt, den: BigInt, digits: Option<usize>) -> Value {
    if num.is_zero() {
        return match digits {
            Some(d) => Value::Fraction(Rc::new(Ratio { num, den: BigInt::one(), digits: Some(d) })),
            None => Value::Small(0),
        };
    }
    let (num, den) = lowest_terms(num, den);
    if digits.is_none() && den.is_one() {
        Value::from_big(num)
    } else {
        Value::Fraction(Rc::new(Ratio { num, den, digits }))
    }
}

pub fn as_real(v: &Value, digits: usize) -> Option<Value> {
    let r = ratio(v)?;
    Some(value(r.num, r.den, Some(digits)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Add,
    Sub,
    Mul,
    Div,
    DivReal,
    Quot,
    Rem,
    Pow,
}

/// Arithmetic; None when an operand is not a number.
pub fn compute(kind: Kind, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        let (x, y) = (*x, *y);
        let quick = match kind {
            Kind::Add => x.checked_add(y),
            Kind::Sub => x.checked_sub(y),
            Kind::Mul => x.checked_mul(y),
            Kind::Quot if y != 0 => x.checked_div(y),
            Kind::Rem if y != 0 => x.checked_div(y).and_then(|q| q.checked_mul(y)).and_then(|p| x.checked_sub(p)),
            _ => None,
        };
        if let Some(r) = quick {
            return Some(Ok(Value::Small(r)));
        }
    }
    let a = ratio(a)?;
    let b = ratio(b)?;
    Some(exact(kind, &a, &b))
}

fn exact(kind: Kind, a: &Ratio, b: &Ratio) -> Result<Value, String> {
    let digits = a.digits.or(b.digits);
    let whole = a.den.is_one() && b.den.is_one() && digits.is_none();
    let by_zero = || Err("Division by zero".to_string());
    match kind {
        Kind::Add if whole => Ok(Value::from_big(&a.num + &b.num)),
        Kind::Sub if whole => Ok(Value::from_big(&a.num - &b.num)),
        Kind::Mul if whole => Ok(Value::from_big(&a.num * &b.num)),
        Kind::Add => Ok(value(&a.num * &b.den + &b.num * &a.den, &a.den * &b.den, digits)),
        Kind::Sub => Ok(value(&a.num * &b.den - &b.num * &a.den, &a.den * &b.den, digits)),
        Kind::Mul => Ok(value(&a.num * &b.num, &a.den * &b.den, digits)),
        Kind::Div if b.num.is_zero() => by_zero(),
        Kind::Div => Ok(value(&a.num * &b.den, &a.den * &b.num, digits)),
        Kind::DivReal if b.num.is_zero() => by_zero(),
        Kind::DivReal => Ok(value(&a.num * &b.den, &a.den * &b.num, Some(digits.unwrap_or(DEFAULT_DIGITS)))),
        Kind::Quot if b.num.is_zero() => by_zero(),
        Kind::Quot => Ok(value((&a.num * &b.den) / (&b.num * &a.den), BigInt::one(), digits)),
        Kind::Rem => {
            // a - b * (a // b)
            let q = ratio(&exact(Kind::Quot, a, b)?).expect("number");
            let p = ratio(&exact(Kind::Mul, b, &q)?).expect("number");
            exact(Kind::Sub, a, &p)
        }
        Kind::Pow => {
            // Squaring on the exponent's integer part.
            let mut e = (&b.num / &b.den).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = a.clone();
            let mut acc = Ratio { num: BigInt::one(), den: BigInt::one(), digits: a.digits };
            while e > 0 {
                if e & 1 == 1 {
                    acc = ratio(&exact(Kind::Mul, &acc, &base)?).expect("number");
                }
                e >>= 1;
                if e > 0 {
                    base = ratio(&exact(Kind::Mul, &base, &base)?).expect("number");
                }
            }
            Ok(value(acc.num, acc.den, acc.digits))
        }
    }
}

/// a < b on numbers; None when an operand is not a number.
pub fn below(a: &Value, b: &Value) -> Option<bool> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        return Some(x < y);
    }
    let (a, b) = (ratio(a)?, ratio(b)?);
    Some((&a.num * &b.den).cmp(&(&b.num * &a.den)) == Ordering::Less)
}
