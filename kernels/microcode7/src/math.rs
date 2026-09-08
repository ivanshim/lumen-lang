// Arithmetic: the floor `+ - * // /`, equality and less-than on exact
// numbers, with a native fast path for two machine integers.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::data::{Ratio, Value};

pub const DEFAULT_PLACES: usize = 15;

pub fn ratio_of(v: &Value) -> Option<Ratio> {
    Some(match v {
        Value::Small(n) => Ratio { above: BigInt::from(*n), beneath: BigInt::one(), places: None, under: false },
        Value::Huge(n) => Ratio { above: (**n).clone(), beneath: BigInt::one(), places: None, under: false },
        Value::Frac(e) => (**e).clone(),
        _ => return None,
    })
}

pub fn make_number(above: BigInt, beneath: BigInt, places: Option<usize>) -> Value {
    made_number(above, beneath, places, false)
}

/// The same, said besides whether a nought came of working with
/// something under nought, which a real of a width holds on to.
pub fn made_number(above: BigInt, beneath: BigInt, places: Option<usize>, under: bool) -> Value {
    if above.is_zero() {
        return match places {
            Some(d) => Value::Frac(Rc::new(Ratio { above, beneath: BigInt::one(), places: Some(d), under })),
            None => Value::Small(0),
        };
    }
    let (above, beneath) = if beneath.is_negative() { (-above, -beneath) } else { (above, beneath) };
    let g = above.gcd(&beneath);
    let (above, beneath) = if g.is_one() { (above, beneath) } else { (&above / &g, &beneath / &g) };
    if places.is_none() && beneath.is_one() {
        Value::from_big(above)
    } else {
        Value::Frac(Rc::new(Ratio { above, beneath, places, under: false }))
    }
}

pub fn to_decimal(v: &Value, places: usize) -> Option<Value> {
    let e = ratio_of(v)?;
    Some(make_number(e.above, e.beneath, Some(places)))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Calc {
    Plus,
    Minus,
    Times,
    Over,
    OverReal,
    IntDiv,
    Remainder,
    Power,
}

pub fn compute(op: Calc, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        let (x, y) = (*x, *y);
        let r = match op {
            Calc::Plus => x.checked_add(y),
            Calc::Minus => x.checked_sub(y),
            Calc::Times => x.checked_mul(y),
            Calc::IntDiv if y != 0 => x.checked_div(y),
            Calc::Remainder if y != 0 => x.checked_div(y).and_then(|q| q.checked_mul(y)).and_then(|p| x.checked_sub(p)),
            _ => None,
        };
        if let Some(r) = r {
            return Some(Ok(Value::Small(r)));
        }
    }
    Some(precise(op, &ratio_of(a)?, &ratio_of(b)?))
}

fn precise(op: Calc, a: &Ratio, b: &Ratio) -> Result<Value, String> {
    let places = a.places.or(b.places);
    let ints = a.beneath.is_one() && b.beneath.is_one() && places.is_none();
    match op {
        Calc::Plus if ints => Ok(Value::from_big(&a.above + &b.above)),
        Calc::Minus if ints => Ok(Value::from_big(&a.above - &b.above)),
        Calc::Times if ints => Ok(Value::from_big(&a.above * &b.above)),
        Calc::Plus => Ok(make_number(&a.above * &b.beneath + &b.above * &a.beneath, &a.beneath * &b.beneath, places)),
        Calc::Minus => Ok(make_number(&a.above * &b.beneath - &b.above * &a.beneath, &a.beneath * &b.beneath, places)),
        // Working a nought together with something under nought leaves
        // the nought under nought, which a real of a width writes apart.
        Calc::Times => Ok(made_number(&a.above * &b.above, &a.beneath * &b.beneath, places, a.above.is_negative() != b.above.is_negative())),
        Calc::Over | Calc::OverReal | Calc::IntDiv if b.above.is_zero() => Err("Division by zero".to_string()),
        Calc::Over => Ok(made_number(&a.above * &b.beneath, &a.beneath * &b.above, places, a.above.is_negative() != b.above.is_negative())),
        Calc::OverReal => Ok(made_number(&a.above * &b.beneath, &a.beneath * &b.above, Some(places.unwrap_or(DEFAULT_PLACES)), a.above.is_negative() != b.above.is_negative())),
        Calc::IntDiv => Ok(make_number((&a.above * &b.beneath) / (&b.above * &a.beneath), BigInt::one(), places)),
        Calc::Remainder => {
            let q = ratio_of(&precise(Calc::IntDiv, a, b)?).unwrap();
            let p = ratio_of(&precise(Calc::Times, b, &q)?).unwrap();
            precise(Calc::Minus, a, &p)
        }
        Calc::Power => {
            let mut e = (&b.above / &b.beneath).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = a.clone();
            let mut acc = Ratio { above: BigInt::one(), beneath: BigInt::one(), places: a.places, under: false };
            while e > 0 {
                if e & 1 == 1 {
                    acc = ratio_of(&precise(Calc::Times, &acc, &base)?).unwrap();
                }
                e >>= 1;
                if e > 0 {
                    base = ratio_of(&precise(Calc::Times, &base, &base)?).unwrap();
                }
            }
            Ok(make_number(acc.above, acc.beneath, acc.places))
        }
    }
}

pub fn below(a: &Value, b: &Value) -> Option<bool> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        return Some(x < y);
    }
    let (a, b) = (ratio_of(a)?, ratio_of(b)?);
    Some((&a.above * &b.beneath).cmp(&(&b.above * &a.beneath)) == Ordering::Less)
}
