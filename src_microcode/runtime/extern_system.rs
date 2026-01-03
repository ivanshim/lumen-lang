// External function dispatch system
//
// Extern calls invoke native functions outside the language runtime.
// This is the boundary where Lumen's semantic guarantees stop.

use crate::src_microcode::kernel::eval::Value;

/// Call an external function by selector
pub fn call_extern(selector: &str, args: Vec<Value>) -> Result<Value, String> {
    match selector {
        "print" => extern_print(args),
        "println" => extern_println(args),
        "input" => extern_input(args),
        "len" => extern_len(args),
        "type" => extern_type(args),
        "str" => extern_str(args),
        "num" => extern_num(args),
        _ => Err(format!("Unknown extern function: {}", selector)),
    }
}

/// Print values without newline
fn extern_print(args: Vec<Value>) -> Result<Value, String> {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    Ok(Value::Null)
}

/// Print values with newline
fn extern_println(args: Vec<Value>) -> Result<Value, String> {
    extern_print(args)?;
    println!();
    Ok(Value::Null)
}

/// Read a line from input
fn extern_input(_args: Vec<Value>) -> Result<Value, String> {
    use std::io::{self, Write};
    io::stdout().flush().ok();

    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| format!("Input error: {}", e))?;

    // Remove trailing newline
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    Ok(Value::String(line))
}

/// Get length of string or number of elements
fn extern_len(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("len() expects exactly 1 argument".to_string());
    }

    match &args[0] {
        Value::String(s) => Ok(Value::Number(s.len() as f64)),
        Value::Number(n) => Ok(Value::Number(n.abs())),
        _ => Ok(Value::Number(0.0)),
    }
}

/// Get type name of value
fn extern_type(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Err("type() expects at least 1 argument".to_string());
    }

    let type_name = match &args[0] {
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Bool(_) => "bool",
        Value::Null => "null",
    };

    Ok(Value::String(type_name.to_string()))
}

/// Convert to string
fn extern_str(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::String(String::new()));
    }

    Ok(Value::String(args[0].to_string()))
}

/// Convert to number
fn extern_num(args: Vec<Value>) -> Result<Value, String> {
    if args.is_empty() {
        return Ok(Value::Number(0.0));
    }

    match &args[0] {
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::String(s) => {
            s.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| format!("Cannot convert '{}' to number", s))
        }
        Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        Value::Null => Ok(Value::Number(0.0)),
    }
}
