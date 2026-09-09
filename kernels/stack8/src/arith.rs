// Exact arithmetic. Two small integers go through checked native
// arithmetic; anything else is a big fraction. A real on either side makes
// the result real, at the left real's precision. Floor: add, subtract,
// multiply, divide, integer quotient, compare; the rest are derived here.

use std::cmp::Ordering;
use std::rc::Rc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::value::{Frac, Real, Value};

/// Significant digits when no operand says.
pub const DEFAULT_PLACES: usize = 15;

/// Any number as p/q, with its precision when real.
#[derive(Clone)]
pub struct Exact {
    pub p: BigInt,
    pub q: BigInt,
    pub places: Option<usize>,
}

impl Exact {
    pub fn from_value(v: &Value) -> Option<Exact> {
        Some(match v {
            Value::Small(n) => Exact { p: BigInt::from(*n), q: BigInt::one(), places: None },
            Value::Huge(n) => Exact { p: (**n).clone(), q: BigInt::one(), places: None },
            Value::Frac(r) => Exact { p: r.p.clone(), q: r.q.clone(), places: None },
            Value::Real(r) => Exact { p: r.p.clone(), q: r.q.clone(), places: Some(r.places) },
            _ => return None,
        })
    }

    fn is_whole(&self) -> bool {
        self.q.is_one() && self.places.is_none()
    }

    /// Whether this stands outside the numbers, which only a real of a
    /// width ever does: nought beneath says so.
    fn outside(&self) -> bool {
        self.q.is_zero()
    }

    /// Whether it is the one no number answers to.
    fn no_number(&self) -> bool {
        self.q.is_zero() && self.p.is_zero()
    }

    fn cmp_exact(&self, other: &Exact) -> Ordering {
        (&self.p * &other.q).cmp(&(&other.p * &self.q))
    }
}

/// A value from p/q: reduced; an integer when it divides out and nothing
/// was real; a real at the given precision otherwise.
pub fn shape_number(p: BigInt, q: BigInt, places: Option<usize>) -> Value {
    shape_signed(p, q, places, false)
}

/// The same, told besides whether a nought came of working with a
/// number below nought, which a real of a width keeps.
pub fn shape_signed(p: BigInt, q: BigInt, places: Option<usize>, below: bool) -> Value {
    // Nought beneath marks what stands outside the numbers. There is
    // nothing to bring to lowest terms there, and the top is held to
    // its sign alone so that two of them made different ways are the
    // one value.
    if q.is_zero() {
        let places = places.unwrap_or(DEFAULT_PLACES);
        return Value::Real(Rc::new(Real { floating: false, p: p.signum(), q, places, below: false }));
    }
    if p.is_zero() {
        return match places {
            Some(places) => Value::Real(Rc::new(Real { floating: false, p, q: BigInt::one(), places, below })),
            None => Value::Small(0),
        };
    }
    let (p, q) = if q.is_negative() { (-p, -q) } else { (p, q) };
    let g = p.gcd(&q);
    let (p, q) = if g.is_one() { (p, q) } else { (&p / &g, &q / &g) };
    match places {
        Some(places) => Value::Real(Rc::new(Real { floating: false, p, q, places, below: false })),
        None if q.is_one() => Value::of_big(p),
        None => Value::Frac(Rc::new(Frac { p, q })),
    }
}

pub fn to_real(v: &Value, places: usize) -> Option<Value> {
    let f = Exact::from_value(v)?;
    Some(shape_number(f.p, f.q, Some(places)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    Plus,
    Minus,
    Times,
    Over,
    OverReal,
    Floor,
    Remainder,
    Raise,
}

/// `a calc b`, or None when either is not a number.
pub fn calculate(calc: Operation, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        let (x, y) = (*x, *y);
        let quick = match calc {
            Operation::Plus => x.checked_add(y),
            Operation::Minus => x.checked_sub(y),
            Operation::Times => x.checked_mul(y),
            Operation::Floor if y != 0 => x.checked_div(y),
            Operation::Remainder if y != 0 => x.checked_rem(y),
            _ => None,
        };
        if let Some(r) = quick {
            return Some(Ok(Value::Small(r)));
        }
    }
    let (a, b) = (Exact::from_value(a)?, Exact::from_value(b)?);
    Some(precise(calc, &a, &b))
}

fn precise(calc: Operation, a: &Exact, b: &Exact) -> Result<Value, String> {
    let places = a.places.or(b.places);
    // Where either side stands outside the numbers the whole working is
    // done at the width: there is nothing exact left to keep hold of,
    // and what the width itself gives is what such a language answers.
    if a.outside() || b.outside() {
        // A division by nought is a division by nought still, and is
        // stopped as one, whatever stands on the other side of it.
        if matches!(calc, Operation::Over | Operation::OverReal | Operation::Floor) && !b.outside() && b.p.is_zero() {
            return Err("Division by zero".to_string());
        }
        let (x, y) = (crate::value::as_binary(&a.p, &a.q), crate::value::as_binary(&b.p, &b.q));
        let got = match calc {
            Operation::Plus => x + y,
            Operation::Minus => x - y,
            Operation::Times => x * y,
            Operation::Over | Operation::OverReal => x / y,
            Operation::Floor => (x / y).trunc(),
            Operation::Remainder => x % y,
            Operation::Raise => x.powf(y),
        };
        return Ok(crate::value::real_of(got, places.unwrap_or(DEFAULT_PLACES)));
    }
    let cross = |sign: i32| &a.p * &b.q + sign * (&b.p * &a.q);
    if a.is_whole() && b.is_whole() {
        match calc {
            Operation::Plus => return Ok(Value::of_big(&a.p + &b.p)),
            Operation::Minus => return Ok(Value::of_big(&a.p - &b.p)),
            Operation::Times => return Ok(Value::of_big(&a.p * &b.p)),
            _ => {}
        }
    }
    if matches!(calc, Operation::Over | Operation::OverReal | Operation::Floor) && b.p.is_zero() {
        return Err("Division by zero".to_string());
    }
    // Multiplying or dividing a nought by a number below nought leaves
    // the nought below nought, which a real of a width writes apart.
    let opposed = a.p.is_negative() != b.p.is_negative();
    Ok(match calc {
        Operation::Plus => shape_number(cross(1), &a.q * &b.q, places),
        Operation::Minus => shape_number(cross(-1), &a.q * &b.q, places),
        Operation::Times => shape_signed(&a.p * &b.p, &a.q * &b.q, places, opposed),
        Operation::Over => shape_signed(&a.p * &b.q, &a.q * &b.p, places, opposed),
        Operation::OverReal => shape_signed(&a.p * &b.q, &a.q * &b.p, Some(places.unwrap_or(DEFAULT_PLACES)), opposed),
        Operation::Floor => shape_number((&a.p * &b.q) / (&b.p * &a.q), BigInt::one(), places),
        Operation::Remainder => {
            // a - b * (a // b)
            let q = Exact::from_value(&precise(Operation::Floor, a, b)?).expect("a number");
            let bq = Exact::from_value(&precise(Operation::Times, b, &q)?).expect("a number");
            precise(Operation::Minus, a, &bq)?
        }
        Operation::Raise => {
            // By squaring, on the exponent's integer part. An exponent
            // below nought raises by as much and gives one over what
            // came of that, which a ratio holds exactly.
            let whole = &b.p / &b.q;
            let beneath = whole.is_negative();
            let mut n = whole.abs().to_u64().ok_or_else(|| "Exponent too large".to_string())?;
            let mut base = a.clone();
            let mut acc = Exact { p: BigInt::one(), q: BigInt::one(), places: a.places };
            while n > 0 {
                if n & 1 == 1 {
                    acc = Exact::from_value(&precise(Operation::Times, &acc, &base)?).expect("a number");
                }
                n >>= 1;
                if n > 0 {
                    base = Exact::from_value(&precise(Operation::Times, &base, &base)?).expect("a number");
                }
            }
            if beneath && acc.p.is_zero() {
                return Err("Division by zero".to_string());
            }
            match beneath {
                true => shape_number(acc.q, acc.p, acc.places),
                false => shape_number(acc.p, acc.q, acc.places),
            }
        }
    })
}

/// The bits of a whole number moved along, up or down, by so many
/// places. A count below nought is no move at all and answers with
/// nothing, for the language to say what it says of such a shift.
/// Sixty-four places or more leave nothing of the number behind, save
/// that moving down keeps the sign, so one below nought falls to -1
/// rather than to 0.
pub fn moved_bits(up: bool, bits: i64, by: i64) -> Option<i64> {
    if by < 0 {
        return None;
    }
    let places = by.min(64) as u32;
    Some(match up {
        true => bits.checked_shl(places).unwrap_or(0),
        false => bits.checked_shr(places).unwrap_or(if bits < 0 { -1 } else { 0 }),
    })
}

/// The order of two numbers, or None when either is not one.
pub fn order_values(a: &Value, b: &Value) -> Option<Ordering> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        return Some(x.cmp(y));
    }
    let (a, b) = (Exact::from_value(a)?, Exact::from_value(b)?);
    if a.outside() || b.outside() {
        // Nothing at all stands in an order beside the one no number
        // answers to, so there is no order to give back.
        if a.no_number() || b.no_number() {
            return None;
        }
        // What lies past every number stands above or below all the
        // rest by its side, and beside another of its own side.
        let side = |e: &Exact| match e.outside() {
            true => e.p.signum().to_i64().unwrap_or(0),
            false => 0,
        };
        return Some(side(&a).cmp(&side(&b)));
    }
    Some(a.cmp_exact(&b))
}

/// Numerator and denominator; an integer is over one. What stands
/// outside the numbers has neither.
pub fn parts(v: &Value) -> Option<(BigInt, BigInt)> {
    let f = Exact::from_value(v)?;
    match f.outside() {
        true => None,
        false => Some((f.p, f.q)),
    }
}

/// What lies before the point, dropped towards nothing. What stands
/// outside the numbers has nothing there and comes to nought, which is
/// what a language holding reals to a width makes of it.
pub fn whole_of(v: &Value) -> Option<BigInt> {
    let f = Exact::from_value(v)?;
    Some(match f.outside() {
        true => BigInt::zero(),
        false => f.p / f.q,
    })
}
