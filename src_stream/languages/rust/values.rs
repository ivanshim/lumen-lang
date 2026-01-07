// src_rust/values.rs
//
// Mini-Rust-specific value types.
// These are the concrete implementations of the kernel's RuntimeValue trait.

use crate::kernel::runtime::RuntimeValue;
use std::any::Any;

/// Mini-Rust number value - stored as string to preserve precision
#[derive(Debug, Clone, PartialEq)]
pub struct RustNumber {
    pub value: String,
}

impl RustNumber {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for RustNumber {
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
        if let Some(other_num) = other.as_any().downcast_ref::<RustNumber>() {
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

/// Mini-Rust boolean value
#[derive(Debug, Clone, PartialEq)]
pub struct RustBool {
    pub value: bool,
}

impl RustBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl RuntimeValue for RustBool {
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
        if let Some(other_bool) = other.as_any().downcast_ref::<RustBool>() {
            Ok(self.value == other_bool.value)
        } else {
            Err("Cannot compare boolean with non-boolean".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Helper to extract a RustNumber if the value is one.
pub fn as_number(val: &dyn RuntimeValue) -> Result<&RustNumber, String> {
    val.as_any()
        .downcast_ref::<RustNumber>()
        .ok_or_else(|| "Expected a number value".to_string())
}

/// Helper to extract a RustBool if the value is one.
pub fn as_bool(val: &dyn RuntimeValue) -> Result<&RustBool, String> {
    val.as_any()
        .downcast_ref::<RustBool>()
        .ok_or_else(|| "Expected a boolean value".to_string())
}
