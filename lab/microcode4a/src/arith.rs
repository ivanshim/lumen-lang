// Arithmetic: the floor `+ - * // /`, equality and less-than on exact
// numbers, with a native fast path for two machine integers.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::value::{Exact, Value};

pub const DIGITS: usize = 15;

pub fn exact(v: &Value) -> Option<Exact> {
    Some(match v {
        Value::Int(n) => Exact { num: BigInt::from(*n), den: BigInt::one(), digits: None },
        Value::Big(n) => Exact { num: (**n).clone(), den: BigInt::one(), digits: None },
        Value::Exact(e) => (**e).clone(),
        _ => return None,
    })
}

pub fn build(num: BigInt, den: BigInt, digits: Option<usize>) -> Value {
    if num.is_zero() {
        return match digits {
            Some(d) => Value::Exact(Rc::new(Exact { num, den: BigInt::one(), digits: Some(d) })),
            None => Value::Int(0),
        };
    }
    let (num, den) = if den.is_negative() { (-num, -den) } else { (num, den) };
    let g = num.gcd(&den);
    let (num, den) = if g.is_one() { (num, den) } else { (&num / &g, &den / &g) };
    if digits.is_none() && den.is_one() {
        Value::big(num)
    } else {
        Value::Exact(Rc::new(Exact { num, den, digits }))
    }
}

pub fn real_of(v: &Value, digits: usize) -> Option<Value> {
    let e = exact(v)?;
    Some(build(e.num, e.den, Some(digits)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sum {
    Add,
    Sub,
    Mul,
    Div,
    DivReal,
    Quot,
    Rem,
    Pow,
}

pub fn apply(op: Sum, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        let (x, y) = (*x, *y);
        let r = match op {
            Sum::Add => x.checked_add(y),
            Sum::Sub => x.checked_sub(y),
            Sum::Mul => x.checked_mul(y),
            Sum::Quot if y != 0 => x.checked_div(y),
            Sum::Rem if y != 0 => x.checked_div(y).and_then(|q| q.checked_mul(y)).and_then(|p| x.checked_sub(p)),
            _ => None,
        };
        if let Some(r) = r {
            return Some(Ok(Value::Int(r)));
        }
    }
    Some(slow(op, &exact(a)?, &exact(b)?))
}

fn slow(op: Sum, a: &Exact, b: &Exact) -> Result<Value, String> {
    let digits = a.digits.or(b.digits);
    let ints = a.den.is_one() && b.den.is_one() && digits.is_none();
    match op {
        Sum::Add if ints => Ok(Value::big(&a.num + &b.num)),
        Sum::Sub if ints => Ok(Value::big(&a.num - &b.num)),
        Sum::Mul if ints => Ok(Value::big(&a.num * &b.num)),
        Sum::Add => Ok(build(&a.num * &b.den + &b.num * &a.den, &a.den * &b.den, digits)),
        Sum::Sub => Ok(build(&a.num * &b.den - &b.num * &a.den, &a.den * &b.den, digits)),
        Sum::Mul => Ok(build(&a.num * &b.num, &a.den * &b.den, digits)),
        Sum::Div | Sum::DivReal | Sum::Quot if b.num.is_zero() => Err("Division by zero".to_string()),
        Sum::Div => Ok(build(&a.num * &b.den, &a.den * &b.num, digits)),
        Sum::DivReal => Ok(build(&a.num * &b.den, &a.den * &b.num, Some(digits.unwrap_or(DIGITS)))),
        Sum::Quot => Ok(build((&a.num * &b.den) / (&b.num * &a.den), BigInt::one(), digits)),
        Sum::Rem => {
            let q = exact(&slow(Sum::Quot, a, b)?).unwrap();
            let p = exact(&slow(Sum::Mul, b, &q)?).unwrap();
            slow(Sum::Sub, a, &p)
        }
        Sum::Pow => {
            let mut e = (&b.num / &b.den).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = a.clone();
            let mut acc = Exact { num: BigInt::one(), den: BigInt::one(), digits: a.digits };
            while e > 0 {
                if e & 1 == 1 {
                    acc = exact(&slow(Sum::Mul, &acc, &base)?).unwrap();
                }
                e >>= 1;
                if e > 0 {
                    base = exact(&slow(Sum::Mul, &base, &base)?).unwrap();
                }
            }
            Ok(build(acc.num, acc.den, acc.digits))
        }
    }
}

pub fn less(a: &Value, b: &Value) -> Option<bool> {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        return Some(x < y);
    }
    let (a, b) = (exact(a)?, exact(b)?);
    Some((&a.num * &b.den).cmp(&(&b.num * &a.den)) == Ordering::Less)
}
