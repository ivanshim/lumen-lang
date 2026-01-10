// src_lumen/values.rs
//
// Lumen-specific value types.
// These are the concrete implementations of the kernel's RuntimeValue trait.
// Only Lumen code knows what numbers, booleans, and strings mean.

use crate::kernel::runtime::RuntimeValue;
use std::any::Any;

/// Lumen number value - stored as string to preserve precision
#[derive(Debug, Clone, PartialEq)]
pub struct LumenNumber {
    pub value: String,
}

impl LumenNumber {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for LumenNumber {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Number(\"{}\")", self.value)
    }

    fn as_display_string(&self) -> String {
        self.value.clone()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_num) = other.as_any().downcast_ref::<LumenNumber>() {
            // Parse both as floats for numeric comparison
            let self_n: f64 = self.value.parse()
                .map_err(|_| "Invalid number format".to_string())?;
            let other_n: f64 = other_num.value.parse()
                .map_err(|_| "Invalid number format".to_string())?;
            Ok(self_n == other_n)
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

