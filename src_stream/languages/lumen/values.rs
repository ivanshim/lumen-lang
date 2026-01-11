// src_lumen/values.rs
//
// Lumen-specific value types.
// These are the concrete implementations of the kernel's RuntimeValue trait.
// Only Lumen code knows what numbers, booleans, and strings mean.

use crate::kernel::runtime::RuntimeValue;
use std::any::Any;
use num_bigint::BigInt;
use num_integer::gcd;

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
}

