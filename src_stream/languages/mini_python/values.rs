// src_mini_python/values.rs
// Mini-Python-specific value types.

use crate::src_stream::kernel::runtime::RuntimeValue;
use std::any::Any;

#[derive(Debug, Clone, PartialEq)]
pub struct MiniPythonNumber {
    pub value: String,
}

impl MiniPythonNumber {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for MiniPythonNumber {
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
        if let Some(other_num) = other.as_any().downcast_ref::<MiniPythonNumber>() {
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
pub struct MiniPythonBool {
    pub value: bool,
}

impl MiniPythonBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl RuntimeValue for MiniPythonBool {
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
        if let Some(other_bool) = other.as_any().downcast_ref::<MiniPythonBool>() {
            Ok(self.value == other_bool.value)
        } else {
            Err("Cannot compare boolean with non-boolean".to_string())
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub fn as_number(val: &dyn RuntimeValue) -> Result<&MiniPythonNumber, String> {
    val.as_any()
        .downcast_ref::<MiniPythonNumber>()
        .ok_or_else(|| "Expected a number value".to_string())
}

pub fn as_bool(val: &dyn RuntimeValue) -> Result<&MiniPythonBool, String> {
    val.as_any()
        .downcast_ref::<MiniPythonBool>()
        .ok_or_else(|| "Expected a boolean value".to_string())
}
