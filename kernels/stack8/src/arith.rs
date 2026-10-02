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

/// Any number as p/q, with its precision when real. `below` keeps
/// whether a real that stands at nought came of working with a
/// number below nought, the one thing p/q alone cannot say of a
/// nought (a whole number's own zero carries no such sign).
#[derive(Clone)]
pub struct Exact {
    pub p: BigInt,
    pub q: BigInt,
    pub places: Option<usize>,
    pub below: bool,
}

impl Exact {
    pub fn from_value(v: &Value) -> Option<Exact> {
        Some(match v {
            Value::Small(n) => Exact { p: BigInt::from(*n), q: BigInt::one(), places: None, below: false },
            Value::Huge(n) => Exact { p: (**n).clone(), q: BigInt::one(), places: None, below: false },
            Value::Frac(r) => Exact { p: r.p.clone(), q: r.q.clone(), places: None, below: false },
            Value::Real(r) => Exact { p: r.p.clone(), q: r.q.clone(), places: Some(r.places), below: r.below },
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
        return Value::Real(Rc::new(Real { floating: false, p: p.signum(), q, places, below, point: false }));
    }
    if p.is_zero() {
        return match places {
            Some(places) => Value::Real(Rc::new(Real { floating: false, p, q: BigInt::one(), places, below, point: false })),
            None => Value::Small(0),
        };
    }
    let (p, q) = if q.is_negative() { (-p, -q) } else { (p, q) };
    let g = p.gcd(&q);
    let (p, q) = if g.is_one() { (p, q) } else { (&p / &g, &q / &g) };
    match places {
        Some(places) => Value::Real(Rc::new(Real { floating: false, p, q, places, below: false, point: false })),
        None if q.is_one() => Value::of_big(p),
        None => Value::Frac(Rc::new(Frac { p, q })),
    }
}

pub fn to_real(v: &Value, places: usize) -> Option<Value> {
    let f = Exact::from_value(v)?;
    Some(shape_signed(f.p, f.q, Some(places), matches!(v, Value::Real(r) if r.below)).with_point(v.keeps_point()))
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
    let point = a.keeps_point() || b.keeps_point();
    let (a, b) = (Exact::from_value(a)?, Exact::from_value(b)?);
    if !a.outside() && !b.outside() {
        if let Some(told) = too_wide_to_meet_a_real(&a, &b) {
            return Some(Err(told));
        }
    }
    Some(precise(calc, &a, &b).map(|v| v.with_point(point)))
}

/// Where an exact number is met by a real of the width, it is carried to
/// the width before the two are worked together, the way handing a
/// whole number to a real-valued working carries it there first. A
/// whole number too great for any real of the width to hold cannot be
/// carried, and the meeting is stopped rather than let the width's own
/// standing-outside-the-numbers answer for a number that is not.
fn too_wide_to_meet_a_real(a: &Exact, b: &Exact) -> Option<String> {
    let exact = match (a.places.is_some(), b.places.is_some()) {
        (false, true) => a,
        (true, false) => b,
        _ => return None,
    };
    if exact.p.is_zero() {
        return None;
    }
    match crate::value::as_binary(&exact.p, &exact.q).is_infinite() {
        true => Some("OverflowError: int too large to convert to float".to_string()),
        false => None,
    }
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
        // A sum of two noughts below nought is a nought below nought
        // itself, the one case addition's own sign is not simply
        // above nought: everywhere else, even a number cancelled
        // exactly by its own opposite, the sum stands above nought.
        Operation::Plus => shape_signed(cross(1), &a.q * &b.q, places, a.p.is_zero() && b.p.is_zero() && a.below && b.below),
        Operation::Minus => shape_number(cross(-1), &a.q * &b.q, places),
        Operation::Times => shape_signed(&a.p * &b.p, &a.q * &b.q, places, opposed),
        Operation::Over => shape_signed(&a.p * &b.q, &a.q * &b.p, places, opposed),
        Operation::OverReal => shape_signed(&a.p * &b.q, &a.q * &b.p, Some(places.unwrap_or(DEFAULT_PLACES)), opposed),
        Operation::Floor => shape_number((&a.p * &b.q) / (&b.p * &a.q), BigInt::one(), places),
        Operation::Remainder => {
            if places.is_some() {
                let mut left = crate::value::as_binary(&a.p, &a.q);
                let mut right = crate::value::as_binary(&b.p, &b.q);
                if left == 0.0 && a.below { left = -0.0; }
                if right == 0.0 && b.below { right = -0.0; }
                let mut rest = left % right;
                if rest == 0.0 { rest = 0.0_f64.copysign(right); }
                else if rest.is_sign_negative() != right.is_sign_negative() { rest += right; }
                return Ok(crate::value::real_of(rest, places.unwrap_or(DEFAULT_PLACES)));
            }
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
            let mut acc = Exact { p: BigInt::one(), q: BigInt::one(), places: a.places, below: false };
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

/// Work ordinary real arithmetic at the configured binary width.
/// Quotient and remainder retain the shared truncating arithmetic.
/// Three reals joined as `left * right + third`, rounded the one time
/// C99's fma names it for. Once `third` is itself the value nothing
/// answers to, no other question about the three is worth asking, and
/// an answer of the same value is given back straight away. Short of
/// that, two shapes of the joining are refused outright: a nought met
/// by an unbounded multiplier, which no working settles a sign for,
/// and an unbounded product met by an unboundedness of the contrary
/// sign, which no finite width could hold steady between the two.
pub fn fused(left: f64, right: f64, third: f64) -> Result<f64, String> {
    if third.is_nan() {
        return Ok(f64::NAN);
    }
    let plain = left * right;
    if plain.is_nan() && !left.is_nan() && !right.is_nan() {
        return Err("ValueError: invalid operation in fma".to_string());
    }
    let neither_is_nan = !left.is_nan() && !right.is_nan();
    let past_bound = neither_is_nan && ((left.is_infinite() && right != 0.0) || (right.is_infinite() && left != 0.0));
    if past_bound && third.is_infinite() {
        let joined_sign = left.is_sign_positive() == right.is_sign_positive();
        if joined_sign != third.is_sign_positive() {
            return Err("ValueError: invalid operation in fma".to_string());
        }
    }
    let joined = left.mul_add(right, third);
    let every_side_bounded = left.is_finite() && right.is_finite() && third.is_finite();
    if joined.is_infinite() && every_side_bounded {
        return Err("OverflowError: overflow in fma".to_string());
    }
    Ok(joined)
}

pub fn binary_work(calc: Operation, a: &Value, b: &Value) -> Option<Result<Value, String>> {
    let (x, y) = (Exact::from_value(a)?, Exact::from_value(b)?);
    if calc == Operation::OverReal && x.is_whole() && y.is_whole() {
        return Some(integer_quotient(&x.p, &y.p));
    }
    if x.places.is_none() && y.places.is_none() { return None; }
    let binary = |v: &Value, e: &Exact| {
        if matches!(v, Value::Real(r) if r.below && r.p.is_zero()) { if e.q.is_zero() { -f64::NAN } else { -0.0 } }
        else { crate::value::as_binary(&e.p, &e.q) }
    };
    let (left, right) = (binary(a, &x), binary(b, &y));
    let result = match calc {
        Operation::Plus => left + right,
        Operation::Minus => left - right,
        Operation::Times => left * right,
        Operation::Over | Operation::OverReal => left / right,
        Operation::Floor | Operation::Remainder | Operation::Raise => return None,
    };
    Some(Ok(crate::value::real_of(result, DEFAULT_PLACES)))
}

/// Round an integer quotient directly at the binary result's spacing.
fn integer_quotient(a: &BigInt, b: &BigInt) -> Result<Value, String> {
    if b.is_zero() { return Err("Division by zero".to_string()); }
    let negative = a.is_negative() != b.is_negative();
    let signed = |x: f64| if negative { -x } else { x };
    let (n, d) = (a.abs(), b.abs());
    let mut exponent = n.bits() as i64 - d.bits() as i64;
    let overflow = || "OverflowError: integer division result too large for a float".to_string();
    if exponent > 1024 { return Err(overflow()); }
    if n.is_zero() || exponent < -1075 { return Ok(crate::value::real_of(signed(0.0), DEFAULT_PLACES)); }
    let below_power = if exponent >= 0 { n < (&d << exponent as usize) }
        else { (&n << (-exponent) as usize) < d };
    if below_power { exponent -= 1; }
    let spacing = (exponent - 52).max(-1074);
    let (top, bottom) = if spacing < 0 { (n << (-spacing) as usize, d) }
        else { (n, d << spacing as usize) };
    let (mut significand, remainder) = top.div_rem(&bottom);
    let twice = remainder << 1usize;
    if twice > bottom || twice == bottom && significand.is_odd() { significand += 1; }
    let unit = if spacing < -1022 { f64::from_bits(1u64 << (spacing + 1074) as u32) }
        else { 2.0f64.powi(spacing as i32) };
    let result = significand.to_f64().unwrap_or(f64::INFINITY) * unit;
    if result.is_infinite() { return Err(overflow()); }
    Ok(crate::value::real_of(signed(result), DEFAULT_PLACES))
}

// CPython v3.14.8 mathmodule.c vector_norm: scaled double-length squares
// and a differential correction to the square root.
pub fn vector_norm(input: &[f64]) -> f64 {
    let mut largest = 0.0_f64;
    let mut nan = false;
    for &v in input { largest = largest.max(v.abs()); nan |= v.is_nan(); }
    if largest.is_infinite() { return largest; }
    if nan { return f64::NAN; }
    if largest == 0.0 || input.len() <= 1 { return largest; }
    let bits = largest.to_bits();
    let biased = (bits >> 52) as i32;
    let exponent = if biased == 0 { 63 - bits.leading_zeros() as i32 - 1073 } else { biased - 1022 };
    if exponent < -1023 {
        let normal: Vec<f64> = input.iter().map(|x| x / f64::MIN_POSITIVE).collect();
        return f64::MIN_POSITIVE * vector_norm(&normal);
    }
    let power = -exponent;
    let scale = if power >= -1022 { f64::from_bits(((power + 1023) as u64) << 52) }
                else { f64::from_bits(1_u64 << (power + 1074)) };
    let (mut high, mut products, mut additions) = (1.0, 0.0, 0.0);
    for &v in input {
        let x = v * scale;
        let square = x * x;
        let tail = x.mul_add(x, -square);
        let next = high + square;
        additions += (high - next) + square;
        products += tail;
        high = next;
    }
    let mut root = (high - 1.0 + (products + additions)).sqrt();
    let negative_square = -root * root;
    let residue = (-root).mul_add(root, -negative_square);
    let next = high + negative_square;
    additions += (high - next) + negative_square;
    products += residue;
    root += (next - 1.0 + (products + additions)) / (2.0 * root);
    root / scale
}
