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
    // Nought beneath marks a worth standing past the numbers. Nothing
    // there can be brought down to lowest terms, and the top is held to
    // its sign alone, so that two got by different roads are one worth.
    if beneath.is_zero() {
        let places = places.or(Some(DEFAULT_PLACES));
        return Value::Frac(Rc::new(Ratio { above: above.signum(), beneath, places, under: false }));
    }
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
    let (x, y) = (ratio_of(a)?, ratio_of(b)?);
    // Where either side stands past the numbers there is nothing exact
    // left to keep hold of, so the whole working is done at the width,
    // and what the width answers is what the language answers.
    if x.past_numbers() || y.past_numbers() {
        return Some(at_the_width(op, &x, &y));
    }
    Some(precise(op, &x, &y))
}

/// A working done at the width itself, each side brought to the nearest
/// real of the width before it is worked.
fn at_the_width(op: Calc, a: &Ratio, b: &Ratio) -> Result<Value, String> {
    // Dividing by nought is dividing by nought still, and is stopped as
    // one, whatever stands on the other hand of it.
    if matches!(op, Calc::Over | Calc::OverReal | Calc::IntDiv) && !b.past_numbers() && b.above.is_zero() {
        return Err("Division by zero".to_string());
    }
    let one = crate::data::nearest_binary(&a.above, &a.beneath);
    let two = crate::data::nearest_binary(&b.above, &b.beneath);
    let got = match op {
        Calc::Plus => one + two,
        Calc::Minus => one - two,
        Calc::Times => one * two,
        Calc::Over | Calc::OverReal => one / two,
        Calc::IntDiv => (one / two).trunc(),
        Calc::Remainder => one % two,
        Calc::Power => one.powf(two),
    };
    Ok(crate::data::worth_of_binary(got, a.places.or(b.places).unwrap_or(DEFAULT_PLACES)))
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
            // An exponent under nought raises by as much and answers
            // with one over what came of that, which a ratio holds
            // exactly.
            let whole = &b.above / &b.beneath;
            let under_nought = whole.is_negative();
            let mut e = whole.abs().to_u64().ok_or_else(|| "Exponent too large".to_string())?;
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
            if under_nought && acc.above.is_zero() {
                return Err("Division by zero".to_string());
            }
            match under_nought {
                true => Ok(make_number(acc.beneath, acc.above, acc.places)),
                false => Ok(make_number(acc.above, acc.beneath, acc.places)),
            }
        }
    }
}

/// A whole number's bits carried so many places upward or downward.
/// Nothing comes back for a count under nought, that being no carrying
/// at all, and the language then says of it whatever it says. Past
/// sixty-four places nothing of the number is left, save the sign on
/// the way down: what stood under nought settles at -1.
pub fn carried_bits(bits: i64, places: i64, upward: bool) -> Option<i64> {
    if places < 0 {
        return None;
    }
    let far = places.min(64) as u32;
    if upward {
        return Some(bits.checked_shl(far).unwrap_or(0));
    }
    let emptied = if bits < 0 { -1 } else { 0 };
    Some(bits.checked_shr(far).unwrap_or(emptied))
}

pub fn below(a: &Value, b: &Value) -> Option<bool> {
    if let (Value::Small(x), Value::Small(y)) = (a, b) {
        return Some(x < y);
    }
    let (a, b) = (ratio_of(a)?, ratio_of(b)?);
    if a.past_numbers() || b.past_numbers() {
        // What nothing is equal to comes before nothing and after
        // nothing. What lies past every number stands on its own hand:
        // above or below all the numbers, and level with its like.
        if a.answers_none() || b.answers_none() {
            return Some(false);
        }
        let hand = |e: &Ratio| match e.past_numbers() {
            true => e.above.signum().to_i64().unwrap_or(0),
            false => 0,
        };
        return Some(hand(&a) < hand(&b));
    }
    Some((&a.above * &b.beneath).cmp(&(&b.above * &a.beneath)) == Ordering::Less)
}

/// Whether one of two worths is the one nothing is equal to, which
/// leaves the pair in no order whatever: every question of which comes
/// first is answered no, put whichever way round.
pub fn no_order(a: &Value, b: &Value) -> bool {
    let loose = |v: &Value| matches!(v, Value::Frac(e) if e.answers_none());
    loose(a) || loose(b)
}

/// What lies before the point, cut towards nothing. A worth past the
/// numbers has nothing there and comes to nought, which is what a
/// language holding its reals to a width makes of it.
pub fn whole_part(v: &Value) -> Option<BigInt> {
    let e = ratio_of(v)?;
    Some(match e.past_numbers() {
        true => BigInt::zero(),
        false => e.above / e.beneath,
    })
}

/// A real-valued working named by word, over the reals of the width:
/// what it gives, or nothing at all where no working goes by that name.
pub fn worked(named: &str, one: f64, two: f64) -> Option<f64> {
    Some(match named {
        "atan2" => one.atan2(two),
        "hypot" => one.hypot(two),
        "pow" => one.powf(two),
        // Dividing at the width answers with what lies past every
        // number instead of stopping the run, which is the whole of why
        // a language asks for it here.
        "fdiv" => one / two,
        "sqrt" => one.sqrt(),
        "exp" => one.exp(),
        "expm1" => one.exp_m1(),
        "log" => one.ln(),
        "log1p" => one.ln_1p(),
        "log10" => one.log10(),
        "log2" => one.log2(),
        "sin" => one.sin(),
        "cos" => one.cos(),
        "tan" => one.tan(),
        "asin" => one.asin(),
        "acos" => one.acos(),
        "atan" => one.atan(),
        "sinh" => one.sinh(),
        "cosh" => one.cosh(),
        "tanh" => one.tanh(),
        "asinh" => one.asinh(),
        "acosh" => one.acosh(),
        "atanh" => one.atanh(),
        _ => return None,
    })
}

/// How many worths a working is handed after its name.
pub fn worked_takes(named: &str) -> usize {
    match named {
        "atan2" | "hypot" | "pow" | "fdiv" => 2,
        _ => 1,
    }
}
