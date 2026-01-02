// src_lumen/extern_system/capabilities.rs
//
// Built-in capability implementations.
// These are the boundary between Lumen and the host system.

use crate::kernel::registry::LumenResult;
use crate::kernel::runtime::Value;
use super::registry::ExternCapability;

/// print_native capability
/// Takes a single Value and prints it to stdout.
/// This is the impure operation that crosses the boundary.
pub struct PrintNative;

impl ExternCapability for PrintNative {
    fn name(&self) -> &'static str {
        "print_native"
    }

    fn call(&self, args: Vec<Value>) -> LumenResult<Value> {
        if args.len() != 1 {
            return Err(format!(
                "print_native expects 1 argument, got {}",
                args.len()
            ));
        }

        // Print to stdout (impure operation)
        println!("{}", args[0]);

        // Return the value that was printed
        Ok(args[0].clone())
    }
}

/// debug_info capability
/// Takes a single Value and prints it with diagnostic information.
/// Shows the representation and any metadata.
pub struct DebugInfo;

impl ExternCapability for DebugInfo {
    fn name(&self) -> &'static str {
        "debug_info"
    }

    fn call(&self, args: Vec<Value>) -> LumenResult<Value> {
        if args.len() != 1 {
            return Err(format!(
                "debug_info expects 1 argument, got {}",
                args.len()
            ));
        }

        // Print debug representation
        eprintln!("[DEBUG] {}", args[0]);

        // Return the original value
        Ok(args[0].clone())
    }
}

/// value_type capability
/// Takes a single Value and returns the type as a number string.
pub struct ValueType;

impl ExternCapability for ValueType {
    fn name(&self) -> &'static str {
        "value_type"
    }

    fn call(&self, args: Vec<Value>) -> LumenResult<Value> {
        if args.len() != 1 {
            return Err(format!(
                "value_type expects 1 argument, got {}",
                args.len()
            ));
        }

        let type_code = match &args[0] {
            Value::Number(_) => "0",   // 0 = number
            Value::Bool(_) => "1",     // 1 = boolean
            Value::String(_) => "2",   // 2 = string
        };

        Ok(Value::Number(type_code.to_string()))
    }
}

/// Create and register all built-in capabilities
pub fn register_builtins(
    registry: &mut super::registry::CapabilityRegistry,
) {
    registry.register(None, Box::new(PrintNative));
    registry.register(None, Box::new(DebugInfo));
    registry.register(None, Box::new(ValueType));
}
