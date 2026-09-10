// The real-number policy is installed before the source is assembled.
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};
use num_integer::Integer;
std::thread_local! { static SHORTEST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
pub fn policy(short: bool) { SHORTEST.with(|setting| setting.set(short)); }
pub fn short() -> bool { SHORTEST.with(|setting| setting.get()) }

pub fn from_ratio(p: &BigInt, q: &BigInt) -> f64 {
    let sign = if p.is_negative() != q.is_negative() { 0x8000000000000000 } else { 0 };
    if p.is_zero() {
        return if q.is_zero() { f64::NAN } else { f64::from_bits(sign) };
    }
    if q.is_zero() { return f64::from_bits(sign | 0x7ff0000000000000); }
    let (top, bottom) = (p.abs(), q.abs());
    let estimate = top.bits() as i64 - bottom.bits() as i64;
    if estimate < -1075 { return f64::from_bits(sign); }
    if estimate > 1024 { return f64::from_bits(sign | 0x7ff0000000000000); }
    let (test_top, test_bottom) = if estimate < 0 { (&top << (-estimate) as usize, bottom.clone()) }
        else { (top.clone(), &bottom << estimate as usize) };
    let leading = estimate - i64::from(test_top < test_bottom);
    let scale = 52 - leading.max(-1022);
    let (dividend, divisor) = if scale < 0 { (top, bottom << (-scale) as usize) }
        else { (top << scale as usize, bottom) };
    let (whole, rest) = dividend.div_rem(&divisor);
    let midway = (&rest * 2u8).cmp(&divisor);
    let round_up = midway.is_gt() || (midway.is_eq() && whole.is_odd());
    let mut figures = whole.to_u64().unwrap() + u64::from(round_up);
    let mut power = leading;
    if figures >> 53 != 0 { figures >>= 1; power += 1; }
    let exponent = if figures >> 52 == 0 { 0 } else { power.max(-1022) + 1023 };
    let payload = if exponent >= 2047 { 0x7ff0000000000000 }
        else { (exponent as u64) << 52 | (figures & 0x000fffffffffffff) };
    f64::from_bits(sign | payload)
}

pub fn as_ratio(real: f64) -> (BigInt, BigInt) {
    let polarity = if real.is_sign_negative() { -1 } else { 1 };
    if real.is_nan() { return (0.into(), 0.into()); }
    if real.is_infinite() { return (polarity.into(), 0.into()); }
    if real == 0.0 { return (0.into(), polarity.into()); }
    let raw = real.to_bits();
    let encoded_power = (raw >> 52) & 2047;
    let significand = (raw & 0x000fffffffffffff) + if encoded_power == 0 { 0 } else { 0x0010000000000000 };
    let power = encoded_power.max(1) as i32 - 1075;
    let top = BigInt::from(significand) * polarity;
    if power < 0 { (top, BigInt::from(1) << power.unsigned_abs() as usize) }
    else { (top << power as usize, 1.into()) }
}

pub fn display(real: f64) -> String {
    if real.is_nan() { return "nan".to_owned(); }
    if !real.is_finite() { return real.to_string(); }
    let candidates = (0..17).map(|places| format!("{real:.places$e}"));
    let chosen = candidates.filter(|s| s.parse::<f64>().unwrap().to_bits() == real.to_bits()).next().unwrap();
    let halves: Vec<&str> = chosen.split('e').collect();
    let power = halves[1].parse::<i32>().unwrap();
    if !(-4..16).contains(&power) { return format!("{}e{:+03}", halves[0], power); }
    let mut digits: Vec<char> = halves[0].chars().filter(|c| c.is_ascii_digit()).collect();
    let point = power + 1;
    let body = if point <= 0 { format!("0.{}{}", "0".repeat(-point as usize), digits.iter().collect::<String>()) }
        else {
            digits.resize(digits.len().max(point as usize), '0');
            digits.insert(point as usize, '.');
            if digits.last() == Some(&'.') { digits.push('0'); }
            digits.into_iter().collect()
        };
    format!("{}{body}", if real.is_sign_negative() { "-" } else { "" })
}
