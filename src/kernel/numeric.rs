// src/kernel/numeric.rs
//
// Utilities for language modules to work with numeric string representations.
// The kernel does not interpret numbers - these are utilities for language modules only.

use crate::kernel::registry::LumenResult;

/// Parse a numeric string to f64.
/// Language modules use this to convert string representations to working numbers.
pub fn parse_number(s: &str) -> LumenResult<f64> {
    s.parse::<f64>()
        .map_err(|_| format!("Invalid number: {}", s))
}

/// Convert f64 back to string representation.
/// Language modules use this to convert computed results back to Value representation.
pub fn format_number(n: f64) -> String {
    // Try to display as integer if it has no fractional part
    if n.fract() == 0.0 && !n.is_infinite() {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

/// Helper for numeric comparisons.
/// Returns true if left < right
pub fn compare_lt(left: &str, right: &str) -> LumenResult<bool> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(l < r)
}

/// Helper for numeric comparisons.
/// Returns true if left <= right
pub fn compare_le(left: &str, right: &str) -> LumenResult<bool> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(l <= r)
}

/// Helper for numeric comparisons.
/// Returns true if left > right
pub fn compare_gt(left: &str, right: &str) -> LumenResult<bool> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(l > r)
}

/// Helper for numeric comparisons.
/// Returns true if left >= right
pub fn compare_ge(left: &str, right: &str) -> LumenResult<bool> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(l >= r)
}

/// Add two numeric strings.
pub fn add(left: &str, right: &str) -> LumenResult<String> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(format_number(l + r))
}

/// Subtract two numeric strings.
pub fn subtract(left: &str, right: &str) -> LumenResult<String> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(format_number(l - r))
}

/// Multiply two numeric strings.
pub fn multiply(left: &str, right: &str) -> LumenResult<String> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    Ok(format_number(l * r))
}

/// Divide two numeric strings.
pub fn divide(left: &str, right: &str) -> LumenResult<String> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    if r == 0.0 {
        return Err("Division by zero".to_string());
    }
    Ok(format_number(l / r))
}

/// Modulo operation on two numeric strings.
pub fn modulo(left: &str, right: &str) -> LumenResult<String> {
    let l = parse_number(left)?;
    let r = parse_number(right)?;
    if r == 0.0 {
        return Err("Modulo by zero".to_string());
    }
    Ok(format_number(l % r))
}

/// Negate a numeric string.
pub fn negate(n: &str) -> LumenResult<String> {
    let num = parse_number(n)?;
    Ok(format_number(-num))
}
