// Lumen numeric utilities
// Arbitrary-precision integer and rational operations using BigInt

use crate::kernel::registry::LumenResult;
use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;

/// Parse a numeric string to either BigInt (integer) or a rational representation (numerator, denominator)
/// Returns (numerator, denominator) where denominator is 1 for integers
/// Supports both base-10 (e.g., "123.45") and base-N (e.g., "16@FF.AB^C") literals
pub fn parse_number_rational(s: &str) -> LumenResult<(BigInt, BigInt)> {
    // Check if this is a base-N literal (contains '@')
    if s.contains('@') {
        return parse_base_n_literal(s);
    }

    // Base-10 parsing (existing logic)
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

/// Parse a base-N numeric literal: <base>@<digits>[.<fraction>][^<exponent>]
/// Examples: 16@FF, 2@1011, 36@1234.wxyz, 10@123.45^6
/// Returns (numerator, denominator) where denominator is 1 for integers
fn parse_base_n_literal(s: &str) -> LumenResult<(BigInt, BigInt)> {
    // Find the '@' separator
    let at_pos = s.find('@')
        .ok_or_else(|| format!("Invalid base-N literal: missing '@' in '{}'", s))?;

    // Parse base (always in decimal)
    let base_str = &s[..at_pos];
    let base: u32 = base_str.parse()
        .map_err(|_| format!("Invalid base in literal '{}': base must be decimal integer", s))?;

    // Validate base range [2, 36]
    if base < 2 || base > 36 {
        return Err(format!("Invalid base {}: must be between 2 and 36", base).into());
    }

    // Parse the rest: <digits>[.<fraction>][^<exponent>]
    let rest = &s[at_pos + 1..];

    if rest.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits after '@'", s).into());
    }

    // Split by '^' for exponent
    let (mantissa_str, exp_str) = if let Some(exp_pos) = rest.find('^') {
        let mantissa = &rest[..exp_pos];
        let exp = &rest[exp_pos + 1..];
        if exp.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after '^'", s).into());
        }
        (mantissa, Some(exp))
    } else {
        (rest, None)
    };

    // Split mantissa by '.' for fractional part
    let (int_str, frac_str) = if let Some(dot_pos) = mantissa_str.find('.') {
        let int_part = &mantissa_str[..dot_pos];
        let frac_part = &mantissa_str[dot_pos + 1..];
        if frac_part.is_empty() {
            return Err(format!("Invalid base-N literal '{}': missing digits after '.'", s).into());
        }
        (int_part, Some(frac_part))
    } else {
        (mantissa_str, None)
    };

    if int_str.is_empty() {
        return Err(format!("Invalid base-N literal '{}': missing digits before '.' or '^'", s).into());
    }

    // Parse integer part
    let int_value = parse_digits_in_base(int_str, base)
        .map_err(|e| format!("Invalid base-N literal '{}': {}", s, e))?;

    // Parse fractional part if present
    let (numerator, denominator) = if let Some(frac) = frac_str {
        let frac_value = parse_digits_in_base(frac, base)
            .map_err(|e| format!("Invalid base-N literal '{}': {}", s, e))?;

        // fractional value = frac_value / base^frac_digits
        let frac_digits = frac.len() as u32;
        let frac_denominator = BigInt::from(base).pow(frac_digits);

        // Combined: int_value + frac_value/frac_denominator
        // = (int_value * frac_denominator + frac_value) / frac_denominator
        let combined_numerator = int_value * &frac_denominator + frac_value;
        (combined_numerator, frac_denominator)
    } else {
        // Integer literal (no fraction)
        (int_value, BigInt::from(1))
    };

    // Apply exponent if present
    let (final_numerator, final_denominator) = if let Some(exp) = exp_str {
        let exp_value = parse_digits_in_base(exp, base)
            .map_err(|e| format!("Invalid base-N literal '{}': exponent {}", s, e))?;

        // Convert exponent to u32
        let exp_u32 = exp_value.to_u32()
            .ok_or_else(|| format!("Invalid base-N literal '{}': exponent too large", s))?;

        // Multiply by base^exponent
        let multiplier = BigInt::from(base).pow(exp_u32);
        (numerator * multiplier, denominator)
    } else {
        (numerator, denominator)
    };

    Ok((final_numerator, final_denominator))
}

/// Parse a string of digits in the given base
/// Digits: 0-9 for values 0-9, a-z/A-Z for values 10-35
fn parse_digits_in_base(digits: &str, base: u32) -> LumenResult<BigInt> {
    let mut result = BigInt::from(0);
    let base_bigint = BigInt::from(base);

    for ch in digits.chars() {
        let digit_value = match ch {
            '0'..='9' => (ch as u32) - ('0' as u32),
            'a'..='z' => (ch as u32) - ('a' as u32) + 10,
            'A'..='Z' => (ch as u32) - ('A' as u32) + 10,
            _ => return Err(format!("invalid digit '{}' for base {}", ch, base).into()),
        };

        if digit_value >= base {
            return Err(format!("digit '{}' (value {}) is not valid in base {}", ch, digit_value, base).into());
        }

        result = result * &base_bigint + BigInt::from(digit_value);
    }

    Ok(result)
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
