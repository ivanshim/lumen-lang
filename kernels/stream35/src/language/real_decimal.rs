// A binary real is held as an exact fraction between operations. Its
// decimal name is the first rounded spelling that reads back unchanged.
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

pub fn enabled() -> bool { super::definition::def().shortest_reals }

pub fn nearest(numerator: &BigInt, denominator: &BigInt) -> f64 {
    let negative = numerator.is_negative() != denominator.is_negative();
    let with_sign = |v: f64| if negative { -v } else { v };
    if denominator.is_zero() {
        return if numerator.is_zero() { f64::NAN } else { with_sign(f64::INFINITY) };
    }
    if numerator.is_zero() { return with_sign(0.0); }
    let mut above = numerator.abs();
    let mut below = denominator.abs();
    let distance = above.bits() as i64 - below.bits() as i64;
    if distance > 1024 { return with_sign(f64::INFINITY); }
    if distance < -1075 { return with_sign(0.0); }
    let at_least = if distance < 0 { (&above << distance.unsigned_abs() as usize) >= below }
        else { above >= (&below << distance as usize) };
    let mut exponent = if at_least { distance } else { distance - 1 };
    let binary_place = exponent.max(-1022) - 52;
    if binary_place.is_negative() { above <<= binary_place.unsigned_abs() as usize; }
    else { below <<= binary_place as usize; }
    let quotient = &above / &below;
    let remainder = &above - &quotient * &below;
    let comparison = (remainder << 1usize).cmp(&below);
    let mut mantissa = quotient.to_u64().unwrap();
    if comparison.is_gt() || (comparison.is_eq() && mantissa % 2 == 1) { mantissa += 1; }
    if mantissa == 9007199254740992 { mantissa /= 2; exponent += 1; }
    let code = if exponent > 1023 { 0x7ff0000000000000 }
        else if mantissa < 4503599627370496 { mantissa }
        else { (((exponent.max(-1022) + 1023) as u64) << 52) + mantissa - 4503599627370496 };
    with_sign(f64::from_bits(code))
}

pub fn exact(worth: f64) -> (BigInt, BigInt) {
    if worth.is_nan() { return (BigInt::from(0), BigInt::from(0)); }
    let sign = if worth.is_sign_negative() { -1 } else { 1 };
    if worth.is_infinite() { return (BigInt::from(sign), BigInt::from(0)); }
    if worth == 0.0 { return (BigInt::from(0), BigInt::from(sign)); }
    let encoding = worth.abs().to_bits();
    let exponent = (encoding >> 52) as i32;
    let mut numerator = BigInt::from(encoding & 0xfffffffffffff);
    if exponent > 0 { numerator += BigInt::from(4503599627370496u64); }
    numerator *= sign;
    let power = exponent.max(1) - 1075;
    let denominator = if power >= 0 { numerator <<= power as usize; BigInt::from(1) }
        else { BigInt::from(1) << (-power) as usize };
    (numerator, denominator)
}

pub fn spelling(x: f64) -> String {
    if x.is_nan() { return "nan".to_string(); }
    if x.is_infinite() { return if x > 0.0 { "inf" } else { "-inf" }.to_string(); }
    let mut found = format!("{x:.16e}");
    for length in 1usize..17 {
        let possibility = format!("{:.*e}", length - 1, x);
        if possibility.parse::<f64>().unwrap().to_bits() == x.to_bits() { found = possibility; break; }
    }
    let index = found.find('e').unwrap();
    let exponent: i32 = found[index + 1..].parse().unwrap();
    let coefficient = &found[..index];
    match exponent {
        -4..=15 => {
            let figures = coefficient.trim_start_matches('-').replace('.', "");
            let at = exponent + 1;
            let mut result = if x.is_sign_negative() { String::from("-") } else { String::new() };
            if at <= 0 {
                result += "0.";
                result += &"0".repeat(-at as usize);
                result += &figures;
            } else {
                for position in 0..figures.len().max(at as usize) {
                    if position == at as usize { result.push('.'); }
                    result.push(figures.as_bytes().get(position).copied().unwrap_or(b'0') as char);
                }
                if at as usize >= figures.len() { result += ".0"; }
            }
            result
        }
        _ => format!("{}e{}{:02}", coefficient, if exponent < 0 { '-' } else { '+' }, exponent.abs()),
    }
}
