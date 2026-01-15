// src_python/values.rs
// Mini-PythonCore-specific value types.

use crate::kernel::runtime::RuntimeValue;
use std::any::Any;

#[derive(Debug, Clone, PartialEq)]
pub struct PythonCoreNumber {
    pub value: String,
}

impl PythonCoreNumber {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl RuntimeValue for PythonCoreNumber {
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
        if let Some(other_num) = other.as_any().downcast_ref::<PythonCoreNumber>() {
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

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PythonCoreBool {
    pub value: bool,
}

impl PythonCoreBool {
    pub fn new(value: bool) -> Self {
        Self { value }
    }
}

impl RuntimeValue for PythonCoreBool {
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
        if let Some(other_bool) = other.as_any().downcast_ref::<PythonCoreBool>() {
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

pub fn as_number(val: &dyn RuntimeValue) -> Result<&PythonCoreNumber, String> {
    val.as_any()
        .downcast_ref::<PythonCoreNumber>()
        .ok_or_else(|| "Expected a number value".to_string())
}

pub fn as_bool(val: &dyn RuntimeValue) -> Result<&PythonCoreBool, String> {
    val.as_any()
        .downcast_ref::<PythonCoreBool>()
        .ok_or_else(|| "Expected a boolean value".to_string())
}
