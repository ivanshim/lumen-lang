// Exact arithmetic. Two small integers go through checked native
// arithmetic; anything else is a big fraction. A real on either side makes
// the result real, at the left real's precision. Floor: add, subtract,
// multiply, divide, integer quotient, compare; the rest are derived here.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::values::{Ratio, Real, Value};

/// Significant digits when no operand says.
pub const PLACES: usize = 15;

/// Any number as p/q, with its precision when real.
#[derive(Clone)]
pub struct Frac {
    pub p: BigInt,
    pub q: BigInt,
    pub places: Option<usize>,
}

impl Frac {
    pub fn of(v: &Value) -> Option<Frac> {
        Some(match v {
            Value::Int(n) => Frac { p: BigInt::from(*n), q: BigInt::one(), places: None },
            Value::Big(n) => Frac { p: (**n).clone(), q: BigInt::one(), places: None },
            Value::Ratio(r) => Frac { p: r.p.clone(), q: r.q.clone(), places: None },
            Value::Real(r) => Frac { p: r.p.clone(), q: r.q.clone(), places: Some(r.places) },
            _ => return None,
        })
    }

    fn integral(&self) -> bool {
        self.q.is_one() && self.places.is_none()
    }

    fn order(&self, other: &Frac) -> Ordering {
        (&self.p * &other.q).cmp(&(&other.p * &self.q))
    }
}

/// A value from p/q: reduced; an integer when it divides out and nothing
/// was real; a real at the given precision otherwise.
pub fn form(p: BigInt, q: BigInt, places: Option<usize>) -> Value {
    if p.is_zero() {
        return match places {
            Some(places) => Value::Real(Rc::new(Real { p, q: BigInt::one(), places })),
            None => Value::Int(0),
        };
    }
    let (p, q) = if q.is_negative() { (-p, -q) } else { (p, q) };
    let g = p.gcd(&q);
    let (p, q) = if g.is_one() { (p, q) } else { (&p / &g, &q / &g) };
    match places {
        Some(places) => Value::Real(Rc::new(Real { p, q, places })),
        None if q.is_one() => Value::whole(p),
        None => Value::Ratio(Rc::new(Ratio { p, q })),
    }
}

pub fn as_real(v: &Value, places: usize) -> Option<Value> {
    let f = Frac::of(v)?;
    Some(form(f.p, f.q, Some(places)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calc {
    Add,
    Sub,
    Mul,
    Div,
    RealDiv,
    Quot,
    Rem,
    Pow,
}

/// `a calc b`, or None when either is not a number.
pub fn compute(calc: Calc, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        let (x, y) = (*x, *y);
        let quick = match calc {
            Calc::Add => x.checked_add(y),
            Calc::Sub => x.checked_sub(y),
            Calc::Mul => x.checked_mul(y),
            Calc::Quot if y != 0 => x.checked_div(y),
            Calc::Rem if y != 0 => x.checked_rem(y),
            _ => None,
        };
        if let Some(r) = quick {
            return Some(Ok(Value::Int(r)));
        }
    }
    let (a, b) = (Frac::of(a)?, Frac::of(b)?);
    Some(exact(calc, &a, &b))
}

fn exact(calc: Calc, a: &Frac, b: &Frac) -> Result<Value, String> {
    let places = a.places.or(b.places);
    let cross = |sign: i32| &a.p * &b.q + sign * (&b.p * &a.q);
    if a.integral() && b.integral() {
        match calc {
            Calc::Add => return Ok(Value::whole(&a.p + &b.p)),
            Calc::Sub => return Ok(Value::whole(&a.p - &b.p)),
            Calc::Mul => return Ok(Value::whole(&a.p * &b.p)),
            _ => {}
        }
    }
    if matches!(calc, Calc::Div | Calc::RealDiv | Calc::Quot) && b.p.is_zero() {
        return Err("Division by zero".to_string());
    }
    Ok(match calc {
        Calc::Add => form(cross(1), &a.q * &b.q, places),
        Calc::Sub => form(cross(-1), &a.q * &b.q, places),
        Calc::Mul => form(&a.p * &b.p, &a.q * &b.q, places),
        Calc::Div => form(&a.p * &b.q, &a.q * &b.p, places),
        Calc::RealDiv => form(&a.p * &b.q, &a.q * &b.p, Some(places.unwrap_or(PLACES))),
        Calc::Quot => form((&a.p * &b.q) / (&b.p * &a.q), BigInt::one(), places),
        Calc::Rem => {
            // a - b * (a // b)
            let q = Frac::of(&exact(Calc::Quot, a, b)?).expect("a number");
            let bq = Frac::of(&exact(Calc::Mul, b, &q)?).expect("a number");
            exact(Calc::Sub, a, &bq)?
        }
        Calc::Pow => {
            // By squaring, on the exponent's integer part.
            let mut n = (&b.p / &b.q).to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = a.clone();
            let mut acc = Frac { p: BigInt::one(), q: BigInt::one(), places: a.places };
            while n > 0 {
                if n & 1 == 1 {
                    acc = Frac::of(&exact(Calc::Mul, &acc, &base)?).expect("a number");
                }
                n >>= 1;
                if n > 0 {
                    base = Frac::of(&exact(Calc::Mul, &base, &base)?).expect("a number");
                }
            }
            form(acc.p, acc.q, acc.places)
        }
    })
}

/// The order of two numbers, or None when either is not one.
pub fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    if let (Value::Int(x), Value::Int(y)) = (a, b) {
        return Some(x.cmp(y));
    }
    Some(Frac::of(a)?.order(&Frac::of(b)?))
}

/// Numerator and denominator; an integer is over one.
pub fn split(v: &Value) -> Option<(BigInt, BigInt)> {
    let f = Frac::of(v)?;
    Some((f.p, f.q))
}
