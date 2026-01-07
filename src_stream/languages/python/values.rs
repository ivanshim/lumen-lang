// src_python/values.rs
// Mini-Python-specific value types.

use crate::kernel::runtime::RuntimeValue;
use std::any::Any;

#[derive(Debug, Clone, PartialEq)]
pub struct PythonNumber {
    pub value: String,
}

impl PythonNumber {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for PythonNumber {
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
        if let Some(other_num) = other.as_any().downcast_ref::<PythonNumber>() {
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

#[derive(Debug, Clone, PartialEq)]
pub struct PythonBool {
    pub value: bool,
}

impl PythonBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl RuntimeValue for PythonBool {
    fn clone_boxed(&self) -> Box<dyn RuntimeValue> {
        Box::new(self.clone())
    }

    fn as_debug_string(&self) -> String {
        format!("Bool({})", self.value)
    }

    fn as_display_string(&self) -> String {
        if self.value { "True" } else { "False" }.to_string()
    }

    fn eq_value(&self, other: &dyn RuntimeValue) -> Result<bool, String> {
        if let Some(other_bool) = other.as_any().downcast_ref::<PythonBool>() {
            Ok(self.value == other_bool.value)
        } else {
            Err("Cannot compare boolean with non-boolean".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn as_number(val: &dyn RuntimeValue) -> Result<&PythonNumber, String> {
    val.as_any()
        .downcast_ref::<PythonNumber>()
        .ok_or_else(|| "Expected a number value".to_string())
}

pub fn as_bool(val: &dyn RuntimeValue) -> Result<&PythonBool, String> {
    val.as_any()
        .downcast_ref::<PythonBool>()
        .ok_or_else(|| "Expected a boolean value".to_string())
}
