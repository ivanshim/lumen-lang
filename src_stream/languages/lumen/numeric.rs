// Lumen numeric utilities
// Arbitrary-precision integer operations using BigInt

use crate::kernel::registry::LumenResult;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;

/// Parse a numeric string to BigInt
pub fn parse_number(s: &str) -> LumenResult<BigInt> {
    s.parse::<BigInt>()
        .map_err(|_| format!("Failed to parse number: {}", s).into())
}

/// Add two BigInts
pub fn add(a: &BigInt, b: &BigInt) -> LumenResult<BigInt> {
    Ok(a + b)
}

/// Subtract two BigInts
pub fn subtract(a: &BigInt, b: &BigInt) -> LumenResult<BigInt> {
    Ok(a - b)
}

/// Multiply two BigInts
pub fn multiply(a: &BigInt, b: &BigInt) -> LumenResult<BigInt> {
    Ok(a * b)
}

/// Divide two BigInts (integer division, truncated towards zero)
pub fn divide(a: &BigInt, b: &BigInt) -> LumenResult<BigInt> {
    if b == &BigInt::from(0) {
        return Err("Division by zero".into());
    }
    Ok(a / b)
}

/// Modulo operation on two BigInts
pub fn modulo(a: &BigInt, b: &BigInt) -> LumenResult<BigInt> {
    if b == &BigInt::from(0) {
        return Err("Modulo by zero".into());
    }
    Ok(a % b)
}

/// Exponentiation (power) operation on two BigInts
/// Note: exponent must fit in u32 for practical purposes
pub fn power(a: &BigInt, b: &BigInt) -> LumenResult<BigInt> {
    let exp = b.to_u32()
        .ok_or_else(|| "Exponent too large".to_string())?;
    Ok(a.pow(exp))
}

/// Negate a BigInt
pub fn negate(a: &BigInt) -> LumenResult<BigInt> {
    Ok(-a)
}

/// Compare less than
pub fn compare_lt(a: &BigInt, b: &BigInt) -> LumenResult<bool> {
    Ok(a < b)
}

/// Compare less than or equal
pub fn compare_le(a: &BigInt, b: &BigInt) -> LumenResult<bool> {
    Ok(a <= b)
}

/// Compare greater than
pub fn compare_gt(a: &BigInt, b: &BigInt) -> LumenResult<bool> {
    Ok(a > b)
}

/// Compare greater than or equal
pub fn compare_ge(a: &BigInt, b: &BigInt) -> LumenResult<bool> {
    Ok(a >= b)
}
