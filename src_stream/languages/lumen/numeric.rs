// Lumen numeric utilities
// Arbitrary-precision integer and rational operations using BigInt

use crate::kernel::registry::LumenResult;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;

/// Parse a numeric string to either BigInt (integer) or a rational representation (numerator, denominator)
/// Returns (numerator, denominator) where denominator is 1 for integers
pub fn parse_number_rational(s: &str) -> LumenResult<(BigInt, BigInt)> {
    if let Some(dot_pos) = s.find('.') {
        // Parse as decimal/rational
        let before_dot = &s[..dot_pos];
        let after_dot = &s[dot_pos + 1..];

        // Count decimal places to determine denominator
        let decimal_places = after_dot.len();
        let denominator = BigInt::from(10).pow(decimal_places as u32);

        // Combine: "1.5" -> "15" with denominator 10
        let integer_part: BigInt = if before_dot.is_empty() || before_dot == "-" {
            BigInt::from(0)
        } else {
            before_dot.parse::<BigInt>()
                .map_err(|_| format!("Failed to parse number: {}", s))?
        };

        let fractional_part: BigInt = after_dot.parse::<BigInt>()
            .map_err(|_| format!("Failed to parse number: {}", s))?;

        // Combine integer and fractional parts
        let is_negative = before_dot.starts_with('-');
        let numerator = if is_negative {
            integer_part * &denominator - fractional_part
        } else {
            integer_part * &denominator + fractional_part
        };

        Ok((numerator, denominator))
    } else {
        // Parse as integer - return with denominator 1
        let num = s.parse::<BigInt>()
            .map_err(|_| format!("Failed to parse number: {}", s))?;
        Ok((num, BigInt::from(1)))
    }
}

/// Parse a numeric string to BigInt (for backward compatibility)
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
