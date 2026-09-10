// A run may ask for binary reals with decimal names that read back whole.
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use std::cell::Cell;
thread_local! { static BRIEF: Cell<bool> = const { Cell::new(false) }; }
pub fn choose(brief: bool) { BRIEF.set(brief); }
pub fn chosen() -> bool { BRIEF.get() }

pub fn read(n: &BigInt, d: &BigInt) -> f64 {
    let negative = n.sign() == num_bigint::Sign::Minus || (n.is_zero() && d.is_negative());
    let sign_bit = u64::from(negative) << 63;
    let pack = |power: u64, digits: u64| f64::from_bits(sign_bit | power << 52 | digits);
    if d.is_zero() { return if n.is_zero() { f64::NAN } else { pack(2047, 0) }; }
    if n.is_zero() { return pack(0, 0); }
    let mut numerator = n.abs();
    let mut denominator = d.abs();
    let mut order = numerator.bits() as i64 - denominator.bits() as i64;
    match order {
        1025.. => return pack(2047, 0),
        ..=-1076 => return pack(0, 0),
        _ => (),
    }
    let below_power = match order {
        0.. => numerator < (&denominator << order as usize),
        _ => (&numerator << order.unsigned_abs() as usize) < denominator,
    };
    order -= i64::from(below_power);
    let units = (order - 52).max(-1074);
    if units < 0 { numerator <<= units.unsigned_abs() as usize; }
    else { denominator <<= units as usize; }
    let mut kept = (&numerator / &denominator).to_u64().unwrap();
    let excess = (&numerator % &denominator) * 2;
    if excess > denominator || (excess == denominator && kept & 1 != 0) { kept += 1; }
    if kept >= 0x20000000000000 { order += 1; kept /= 2; }
    if order >= 1024 { return pack(2047, 0); }
    let biased = if kept < 0x10000000000000 { 0 } else { (order.max(-1022) + 1023) as u64 };
    pack(biased, kept & 0xfffffffffffff)
}

pub fn keep(number: f64) -> (BigInt, BigInt) {
    let code = number.to_bits();
    let negative = code >> 63 != 0;
    if !number.is_finite() {
        return (BigInt::from(if number.is_nan() { 0 } else if negative { -1 } else { 1 }), BigInt::from(0));
    }
    if number == 0.0 { return (BigInt::from(0), BigInt::from(if negative { -1 } else { 1 })); }
    let biased = ((code >> 52) & 0x7ff) as i32;
    let mut upper = BigInt::from(code & 0xfffffffffffff);
    if biased != 0 { upper += BigInt::from(1u64 << 52); }
    if negative { upper = -upper; }
    let mut lower = BigInt::from(1);
    let step = biased.max(1) - 1075;
    match step { 0.. => upper <<= step as usize, _ => lower <<= -step as usize }
    (upper, lower)
}

pub fn write(number: f64) -> String {
    if number.is_nan() { return String::from("nan"); }
    if number.is_infinite() { return String::from(if number < 0.0 { "-inf" } else { "inf" }); }
    let mut precision = 1;
    let mut spelling;
    loop {
        spelling = format!("{number:.width$e}", width = precision - 1);
        if spelling.parse::<f64>().unwrap().to_bits() == number.to_bits() || precision == 17 { break; }
        precision += 1;
    }
    let split = spelling.rfind('e').unwrap();
    let exponent = spelling[split + 1..].parse::<i32>().unwrap();
    spelling.truncate(split);
    if !(-4..=15).contains(&exponent) { return format!("{spelling}e{exponent:+03}"); }
    let sign = if spelling.starts_with('-') { spelling.remove(0); "-" } else { "" };
    spelling.retain(|c| c != '.');
    let boundary = exponent + 1;
    while boundary > spelling.len() as i32 { spelling.push('0'); }
    if boundary <= 0 { spelling = "0".repeat((1 - boundary) as usize) + &spelling; }
    let at = boundary.max(1) as usize;
    spelling.insert(at, '.');
    if spelling.ends_with('.') { spelling.push('0'); }
    sign.to_owned() + &spelling
}
