// src_mini_rust/values.rs
//
// Mini-Rust-specific value types.
// These are the concrete implementations of the kernel's RuntimeValue trait.

use crate::src_stream::src_stream::kernel::runtime::RuntimeValue;
use std::any::Any;

/// Mini-Rust number value - stored as string to preserve precision
#[derive(Debug, Clone, PartialEq)]
pub struct MiniRustNumber {
    pub value: String,
}

impl MiniRustNumber {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for MiniRustNumber {
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
        if let Some(other_num) = other.as_any().downcast_ref::<MiniRustNumber>() {
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
pub struct MiniRustBool {
    pub value: bool,
}

impl MiniRustBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl RuntimeValue for MiniRustBool {
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
        if let Some(other_bool) = other.as_any().downcast_ref::<MiniRustBool>() {
            Ok(self.value == other_bool.value)
        } else {
            Err("Cannot compare boolean with non-boolean".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Helper to extract a MiniRustNumber if the value is one.
pub fn as_number(val: &dyn RuntimeValue) -> Result<&MiniRustNumber, String> {
    val.as_any()
        .downcast_ref::<MiniRustNumber>()
        .ok_or_else(|| "Expected a number value".to_string())
}

/// Helper to extract a MiniRustBool if the value is one.
pub fn as_bool(val: &dyn RuntimeValue) -> Result<&MiniRustBool, String> {
    val.as_any()
        .downcast_ref::<MiniRustBool>()
        .ok_or_else(|| "Expected a boolean value".to_string())
}
