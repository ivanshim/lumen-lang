// Mini-Python numeric utilities
// Private helper module for numeric string operations

use crate::src_stream::src_stream::kernel::registry::LumenResult;

/// Parse a numeric string to f64
pub fn parse_number(s: &str) -> LumenResult<f64> {
    s.parse::<f64>()
        .map_err(|_| format!("Failed to parse number: {}", s).into())
}

/// Format a number back to string
pub fn format_number(n: f64) -> String {
    // Format with appropriate precision
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{:.0}", n)
    } else {
        n.to_string()
    }
}

/// Add two numeric strings
pub fn add(a: &str, b: &str) -> LumenResult<String> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(format_number(av + bv))
}

/// Subtract two numeric strings
pub fn subtract(a: &str, b: &str) -> LumenResult<String> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(format_number(av - bv))
}

/// Multiply two numeric strings
pub fn multiply(a: &str, b: &str) -> LumenResult<String> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(format_number(av * bv))
}

/// Divide two numeric strings
pub fn divide(a: &str, b: &str) -> LumenResult<String> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    if bv == 0.0 {
        return Err("Division by zero".into());
    }
    Ok(format_number(av / bv))
}

/// Modulo operation on two numeric strings
pub fn modulo(a: &str, b: &str) -> LumenResult<String> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    if bv == 0.0 {
        return Err("Modulo by zero".into());
    }
    Ok(format_number(av % bv))
}

/// Negate a numeric string
pub fn negate(s: &str) -> LumenResult<String> {
    let v = parse_number(s)?;
    Ok(format_number(-v))
}

/// Compare less than
pub fn compare_lt(a: &str, b: &str) -> LumenResult<bool> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(av < bv)
}

/// Compare less than or equal
pub fn compare_le(a: &str, b: &str) -> LumenResult<bool> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(av <= bv)
}

/// Compare greater than
pub fn compare_gt(a: &str, b: &str) -> LumenResult<bool> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(av > bv)
}

/// Compare greater than or equal
pub fn compare_ge(a: &str, b: &str) -> LumenResult<bool> {
    let av = parse_number(a)?;
    let bv = parse_number(b)?;
    Ok(av >= bv)
}
