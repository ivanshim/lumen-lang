// src_lumen/values.rs
//
// Lumen-specific value types.
// These are the concrete implementations of the kernel's RuntimeValue trait.
// Only Lumen code knows what numbers, booleans, and strings mean.

use crate::kernel::runtime::RuntimeValue;
use std::any::Any;
use num_bigint::BigInt;
use num_integer::gcd;
use num_traits::Signed;

/// Lumen rational number value - stored as (numerator, denominator) in canonical reduced form
/// Always stored reduced: gcd(numerator, denominator) = 1, denominator > 0
/// If denominator is 1, this represents an integer
#[derive(Debug, Clone, PartialEq)]
pub struct LumenRational {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

impl LumenRational {
    /// Create a rational from numerator and denominator, automatically reduced to canonical form
    pub fn new(num: BigInt, denom: BigInt) -> Self {
        // Handle zero denominator
        if denom == BigInt::from(0) {
            panic!("Denominator cannot be zero");
        }

        // If numerator is zero, return 0/1
        if num == BigInt::from(0) {
            return Self {
                numerator: BigInt::from(0),
                denominator: BigInt::from(1),
            };
        }

        // Normalize sign: always keep denominator positive
        let (numerator, denominator) = if denom < BigInt::from(0) {
            (-num, -denom)
        } else {
            (num, denom)
        };

        // Reduce to canonical form by dividing by GCD
        let g = gcd(numerator.clone(), denominator.clone());
        Self {
            numerator: numerator / &g,
            denominator: denominator / &g,
        }
    }

    /// Check if this rational is actually an integer (denominator is 1)
    pub fn is_integer(&self) -> bool {
        self.denominator == BigInt::from(1)
    }

    /// Get as integer if denominator is 1, otherwise None
    pub fn as_integer(&self) -> Option<&BigInt> {
        if self.is_integer() {
            Some(&self.numerator)
        } else {
            None
        }
    }
}

impl RuntimeValue for LumenRational {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        if self.is_integer() {
            format!("Rational({})", self.numerator)
        } else {
            format!("Rational({}/{})", self.numerator, self.denominator)
        }
    }

    fn as_display_string(&self) -> String {
        if self.is_integer() {
            self.numerator.to_string()
        } else {
            format!("{}/{}", self.numerator, self.denominator)
        }
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_rat) = other.as_any().downcast_ref::<LumenRational>() {
            // Both in canonical form, so direct comparison works
            Ok(self.numerator == other_rat.numerator && self.denominator == other_rat.denominator)
        } else if let Some(other_num) = other.as_any().downcast_ref::<LumenNumber>() {
            // Compare rational with integer
            Ok(self.is_integer() && self.numerator == other_num.value)
        } else {
            Err("Cannot compare rational with non-numeric value".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Lumen number value - stored as BigInt for arbitrary precision
#[derive(Debug, Clone, PartialEq)]
pub struct LumenNumber {
    pub value: BigInt,
}

impl LumenNumber {
    pub fn new(value: BigInt) -> Self {
        Self { value }
    }
}

impl RuntimeValue for LumenNumber {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Number({})", self.value)
    }

    fn as_display_string(&self) -> String {
        self.value.to_string()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_num) = other.as_any().downcast_ref::<LumenNumber>() {
            Ok(self.value == other_num.value)
        } else {
            Err("Cannot compare number with non-number".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Lumen boolean value
#[derive(Debug, Clone, PartialEq)]
pub struct LumenBool {
    pub value: bool,
}

impl LumenBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl RuntimeValue for LumenBool {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Bool({})", self.value)
    }

    fn as_display_string(&self) -> String {
        if self.value { "true" } else { "false" }.to_string()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_bool) = other.as_any().downcast_ref::<LumenBool>() {
            Ok(self.value == other_bool.value)
        } else {
            Err("Cannot compare boolean with non-boolean".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Lumen string value
#[derive(Debug, Clone, PartialEq)]
pub struct LumenString {
    pub value: String,
}

impl LumenString {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for LumenString {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("String(\"{}\")", self.value)
    }

    fn as_display_string(&self) -> String {
        self.value.clone()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_str) = other.as_any().downcast_ref::<LumenString>() {
            Ok(self.value == other_str.value)
        } else {
            Err("Cannot compare string with non-string".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Helper to extract a LumenRational if the value is one.
pub fn as_rational(val: &dyn RuntimeValue) -> Result<&LumenRational, String> {
    val.as_any()
        .downcast_ref::<LumenRational>()
        .ok_or_else(|| "Expected a rational value".to_string())
}

/// Helper to extract a LumenNumber if the value is one.
/// Used by arithmetic and comparison operations.
pub fn as_number(val: &dyn RuntimeValue) -> Result<&LumenNumber, String> {
    val.as_any()
        .downcast_ref::<LumenNumber>()
        .ok_or_else(|| "Expected a number value".to_string())
}

/// Helper to extract a LumenBool if the value is one.
pub fn as_bool(val: &dyn RuntimeValue) -> Result<&LumenBool, String> {
    val.as_any()
        .downcast_ref::<LumenBool>()
        .ok_or_else(|| "Expected a boolean value".to_string())
}

/// Helper to extract a LumenString if the value is one.
pub fn as_string(val: &dyn RuntimeValue) -> Result<&LumenString, String> {
    val.as_any()
        .downcast_ref::<LumenString>()
        .ok_or_else(|| "Expected a string value".to_string())
}

/// Lumen real number value - decimal approximation with configurable precision
/// Stored as (numerator, denominator) with an associated precision in significant digits
/// The value is maintained exactly internally but displayed/compared with specified precision
#[derive(Debug, Clone, PartialEq)]
pub struct LumenReal {
    pub numerator: BigInt,
    pub denominator: BigInt,
    pub precision: usize, // Number of significant digits
}

impl LumenReal {
    /// Create a real from a numerator and denominator with specified precision
    /// Precision specifies significant digits (default 15)
    pub fn new(num: BigInt, denom: BigInt, precision: usize) -> Self {
        // Handle zero denominator
        if denom == BigInt::from(0) {
            panic!("Denominator cannot be zero");
        }

        // If numerator is zero, return 0 with specified precision
        if num == BigInt::from(0) {
            return Self {
                numerator: BigInt::from(0),
                denominator: BigInt::from(1),
                precision,
            };
        }

        // Normalize sign: always keep denominator positive
        let (numerator, denominator) = if denom < BigInt::from(0) {
            (-num, -denom)
        } else {
            (num, denom)
        };

        // Reduce to canonical form by dividing by GCD
        let g = gcd(numerator.clone(), denominator.clone());
        Self {
            numerator: numerator / &g,
            denominator: denominator / &g,
            precision,
        }
    }

    /// Convert to integer by truncating toward zero
    pub fn to_integer(&self) -> BigInt {
        &self.numerator / &self.denominator
    }

    /// Get string representation with the stored precision
    /// This renders the decimal with significant figures truncated/rounded
    pub fn as_decimal_string(&self) -> String {
        // Simple approach: compute as fixed-point and format
        // For true significant digit handling, we'd need more sophisticated rounding
        // For now, we'll compute the result and show it with reasonable precision

        let int_part = self.to_integer();
        let remainder = self.numerator.clone() - (&int_part * &self.denominator);

        if remainder == BigInt::from(0) {
            return int_part.to_string();
        }

        // Compute decimal places needed for precision
        let mut decimal_str = String::new();
        let mut digit_count = int_part.to_string().len();
        let target_digits = self.precision;
        let mut remainder = remainder.abs();

        // If int part has fewer digits than target, include fractional digits
        let mut frac_digits = if digit_count >= target_digits {
            0
        } else {
            target_digits - digit_count
        };

        // Compute fractional part
        let mut has_nonzero = false;
        while frac_digits > 0 && remainder > BigInt::from(0) {
            remainder = remainder * BigInt::from(10);
            let digit = &remainder / &self.denominator;
            if digit > BigInt::from(0) || has_nonzero {
                if !has_nonzero {
                    decimal_str.push('.');
                    has_nonzero = true;
                }
                decimal_str.push_str(&digit.to_string());
            }
            remainder = remainder - (&digit * &self.denominator);
            frac_digits -= 1;
        }

        format!("{}{}", int_part, decimal_str)
    }
}

impl RuntimeValue for LumenReal {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Real({}/{}, precision={})", self.numerator, self.denominator, self.precision)
    }

    fn as_display_string(&self) -> String {
        self.as_decimal_string()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_real) = other.as_any().downcast_ref::<LumenReal>() {
            // Compare the exact rational values (precision doesn't affect equality of stored value)
            Ok(self.numerator == other_real.numerator && self.denominator == other_real.denominator)
        } else if let Some(other_rat) = other.as_any().downcast_ref::<LumenRational>() {
            // Compare real with rational
            Ok(self.numerator == other_rat.numerator && self.denominator == other_rat.denominator)
        } else if let Some(other_num) = other.as_any().downcast_ref::<LumenNumber>() {
            // Compare real with integer
            Ok(self.numerator == other_num.value && self.denominator == BigInt::from(1))
        } else {
            Err("Cannot compare real with non-numeric value".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Helper to extract a LumenReal if the value is one.
pub fn as_real(val: &dyn RuntimeValue) -> Result<&LumenReal, String> {
    val.as_any()
        .downcast_ref::<LumenReal>()
        .ok_or_else(|| "Expected a real value".to_string())
}

/// Lumen none (null/unit) value
#[derive(Debug, Clone, PartialEq)]
pub struct LumenNone;

impl RuntimeValue for LumenNone {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        "none".to_string()
    }

    fn as_display_string(&self) -> String {
        "none".to_string()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if other.as_any().downcast_ref::<LumenNone>().is_some() {
            Ok(true)
        } else {
            Err("Cannot compare none with non-none value".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Lumen array value - heterogeneous collection of values
#[derive(Debug, Clone, PartialEq)]
pub struct LumenArray {
    pub elements: Vec<Box<dyn RuntimeValue>>,
}

impl LumenArray {
    pub fn new(elements: Vec<Box<dyn RuntimeValue>>) -> Self {
        Self { elements }
    }

    pub fn empty() -> Self {
        Self {
            elements: Vec::new(),
        }
    }
}

impl RuntimeValue for LumenArray {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        let elements_str = self
            .elements
            .iter()
            .map(|e| e.as_debug_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("Array([{}])", elements_str)
    }

    fn as_display_string(&self) -> String {
        let elements_str = self
            .elements
            .iter()
            .map(|e| e.as_display_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{}]", elements_str)
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_arr) = other.as_any().downcast_ref::<LumenArray>() {
            if self.elements.len() != other_arr.elements.len() {
                return Ok(false);
            }
            for (a, b) in self.elements.iter().zip(other_arr.elements.iter()) {
                match a.eq_value(b.as_ref()) {
                    Ok(true) => continue,
                    Ok(false) => return Ok(false),
                    Err(e) => return Err(e),
                }
            }
            Ok(true)
        } else {
            Err("Cannot compare array with non-array".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Helper to extract a LumenArray if the value is one.
pub fn as_array(val: &dyn RuntimeValue) -> Result<&LumenArray, String> {
    val.as_any()
        .downcast_ref::<LumenArray>()
        .ok_or_else(|| "Expected an array value".to_string())
}

