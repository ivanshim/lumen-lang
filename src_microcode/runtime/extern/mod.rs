// External/foreign function call system
//
// Handles dispatch to capabilities defined in language schemas.
// This is intentionally minimal and data-driven.

use crate::src_microcode::kernel::eval::Value;
use crate::schema::LanguageSchema;

/// Execute an external capability call
///
/// The schema defines the syntax and available capabilities.
/// The kernel interpreter uses this to dispatch calls.
pub fn execute_extern(
    capability: &str,
    args: Vec<Value>,
    _schema: &LanguageSchema,
) -> Result<Value, String> {
    match capability {
        "print_native" => {
            if args.len() != 1 {
                return Err(format!(
                    "print_native expects 1 argument, got {}",
                    args.len()
                ));
            }
            // Print to stdout and return the value
            println!("{}", args[0]);
            Ok(args[0].clone())
        }

        "debug_info" => {
            if args.len() != 1 {
                return Err(format!(
                    "debug_info expects 1 argument, got {}",
                    args.len()
                ));
            }
            // Print debug representation
            eprintln!("[DEBUG] {}", args[0]);
            Ok(args[0].clone())
        }

        "value_type" => {
            if args.len() != 1 {
                return Err(format!(
                    "value_type expects 1 argument, got {}",
                    args.len()
                ));
            }
            // Return type information
            let type_name = match &args[0] {
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Bool(_) => "bool",
                Value::Null => "null",
            };
            Ok(Value::String(type_name.to_string()))
        }

        _ => Err(format!("Unknown external capability: {}", capability)),
    }
}
