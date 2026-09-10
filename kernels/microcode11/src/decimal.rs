// Binary reals and their shortest decimal names. The selector belongs to
// the run; the porter continues to read exact source numbers.
use std::cell::Cell;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

thread_local! { static ACTIVE: Cell<bool> = const { Cell::new(false) }; }
pub fn select(shortest: bool) { ACTIVE.with(|cell| cell.set(shortest)); }
pub fn active() -> bool { ACTIVE.with(Cell::get) }

pub fn binary(top: &BigInt, bottom: &BigInt) -> f64 {
    let negative = top.is_negative() != bottom.is_negative();
    let sign = if negative { 1u64 << 63 } else { 0 };
    if bottom.is_zero() {
        return if top.is_zero() { f64::NAN } else { f64::from_bits(sign | (2047u64 << 52)) };
    }
    if top.is_zero() { return f64::from_bits(sign); }
    let a = top.abs();
    let b = bottom.abs();
    let mut exponent = a.bits() as i64 - b.bits() as i64;
    if exponent > 1024 { return f64::from_bits(sign | (2047u64 << 52)); }
    if exponent < -1075 { return f64::from_bits(sign); }
    let less = if exponent >= 0 { a < (&b << exponent as usize) } else { (&a << -exponent as usize) < b };
    if less { exponent -= 1; }
    let shift = 52 - exponent.max(-1022);
    let (n, d) = if shift >= 0 { (a << shift as usize, b) } else { (a, b << -shift as usize) };
    let (mut mantissa, remainder) = n.div_rem(&d);
    let twice = remainder << 1usize;
    if twice > d || (twice == d && mantissa.is_odd()) { mantissa += 1; }
    let mut m = mantissa.to_u64().expect("fifty-four bits suffice");
    if m == 1u64 << 53 { m >>= 1; exponent += 1; }
    if exponent > 1023 { return f64::from_bits(sign | (2047u64 << 52)); }
    let field = if exponent < -1022 && m < 1u64 << 52 { 0 } else { (exponent.max(-1022) + 1023) as u64 };
    f64::from_bits(sign | (field << 52) | (m & ((1u64 << 52) - 1)))
}

pub fn ratio(x: f64) -> (BigInt, BigInt) {
    if !x.is_finite() { return (BigInt::from(if x.is_nan() { 0 } else if x.is_sign_negative() { -1 } else { 1 }), BigInt::zero()); }
    if x == 0.0 { return (BigInt::zero(), BigInt::from(if x.is_sign_negative() { -1 } else { 1 })); }
    let bits = x.to_bits();
    let field = ((bits >> 52) & 2047) as i32;
    let significand = (bits & ((1u64 << 52) - 1)) | if field == 0 { 0 } else { 1u64 << 52 };
    let power = if field == 0 { -1074 } else { field - 1075 };
    let mut n = BigInt::from(significand);
    if x.is_sign_negative() { n = -n; }
    if power >= 0 { return (n << power as usize, BigInt::one()); }
    let d = BigInt::one() << -power as usize;
    let common = n.gcd(&d);
    (n / &common, d / common)
}

pub fn text(x: f64) -> String {
    if !x.is_finite() { return if x.is_nan() { "nan".into() } else { x.to_string() }; }
    let mut count = 0;
    let decimal = loop {
        let trial = format!("{:.*e}", count, x);
        if trial.parse::<f64>().unwrap().to_bits() == x.to_bits() { break trial; }
        count += 1;
        assert!(count < 17);
    };
    let (coefficient, e) = decimal.split_once('e').unwrap();
    let e: i32 = e.parse().unwrap();
    if e < -4 || e >= 16 { return format!("{coefficient}e{e:+03}"); }
    let minus = if x.is_sign_negative() { "-" } else { "" };
    let digits = coefficient.trim_start_matches('-').replace('.', "");
    let place = e + 1;
    let flat = if place <= 0 { format!("0.{}{digits}", "0".repeat(-place as usize)) }
        else if place as usize >= digits.len() { format!("{digits}{}.0", "0".repeat(place as usize - digits.len())) }
        else { format!("{}.{}", &digits[..place as usize], &digits[place as usize..]) };
    format!("{minus}{flat}")
}
